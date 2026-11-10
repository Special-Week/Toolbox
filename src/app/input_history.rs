//! 本地输入续接：按工具隔离，停顿后原子写入，正常退出时补存最后一次编辑。
use crate::catalog::{ToolDef, TOOLS};
use serde_json::json;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const DEBOUNCE: Duration = Duration::from_millis(700);
const MAX_BYTES: u64 = 8 * 1024 * 1024;

pub(super) struct InputHistory {
    path: Option<PathBuf>,
    pub(super) values: HashMap<String, String>,
    pending: Option<Instant>,
    pub(super) error: bool,
}

impl InputHistory {
    pub(super) fn sibling_path(&self, name: &str) -> Option<PathBuf> {
        self.path.as_ref().map(|path| path.with_file_name(name))
    }

    pub(super) fn new() -> Self {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("XDG_STATE_HOME").map(PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/state")));
        Self::load(base.map(|p| p.join("Toolbox/rust/inputs-v1.json")))
    }

    fn load(path: Option<PathBuf>) -> Self {
        let values = (|| {
            let mut text = String::new();
            std::fs::File::open(path.as_ref()?)
                .ok()?
                .take(MAX_BYTES + 1)
                .read_to_string(&mut text)
                .ok()?;
            if text.len() as u64 > MAX_BYTES {
                return None;
            }
            let data: serde_json::Value = serde_json::from_str(&text).ok()?;
            if data["version"].as_u64() != Some(1) {
                return None;
            }
            let values = data["values"].as_object()?;
            Some(
                values
                    .iter()
                    .filter_map(|(key, value)| {
                        let (id, _) = key.split_once('.')?;
                        TOOLS.iter().find(|t| t.id == id && t.remembers_inputs())?;
                        Some((key.clone(), value.as_str()?.to_owned()))
                    })
                    .collect(),
            )
        })()
        .unwrap_or_default();
        Self {
            path,
            values,
            pending: None,
            error: false,
        }
    }

    pub(super) fn observe(&mut self, tool: &ToolDef, inputs: &HashMap<String, String>) {
        if !tool.remembers_inputs() {
            return;
        }
        let prefix = format!("{}.", tool.id);
        let current: HashMap<_, _> = inputs
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .collect();
        let old: HashMap<_, _> = self
            .values
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .collect();
        if current == old {
            return;
        }
        self.values.retain(|key, _| !key.starts_with(&prefix));
        self.values
            .extend(current.into_iter().map(|(k, v)| (k.clone(), v.clone())));
        self.pending = Some(Instant::now() + DEBOUNCE);
    }

    pub(super) fn remaining(&self) -> Option<Duration> {
        self.pending
            .map(|due| due.saturating_duration_since(Instant::now()))
    }

    pub(super) fn flush(&mut self) {
        if self.pending.is_none() {
            return;
        }
        match self.write() {
            Ok(()) => {
                self.pending = None;
                self.error = false;
            }
            Err(_) => {
                self.error = true;
                // 写入失败保持输入与待存状态，稍后重试，不打断计算。
                self.pending = Some(Instant::now() + Duration::from_secs(5));
            }
        }
    }

    fn write(&self) -> io::Result<()> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| io::Error::other("本地数据目录不可用"))?;
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("本地数据目录无效"))?;
        std::fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer(&mut file, &json!({ "version": 1, "values": self.values }))?;
        file.flush()?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ToolboxApp;
    use eframe::egui;

    fn app_at(path: PathBuf) -> ToolboxApp {
        let mut app = ToolboxApp::default();
        app.archives = crate::app::calibration_archive::CalibrationArchives::load(Some(
            path.with_file_name("calibrations-v1.json"),
        ));
        let history = InputHistory::load(Some(path));
        app.values = history.values.clone();
        app.history = history;
        app
    }

    #[test]
    fn certificate_archives_save_and_restore_both_instruments() {
        fn click(app: &mut ToolboxApp, ctx: &egui::Context, id: &'static str, text: &str) {
            let output = frame(app, ctx, id, [1220.0, 2000.0], vec![]);
            let point = text_shapes(&output)
                .into_iter()
                .find(|(label, _)| label == text)
                .unwrap_or_else(|| panic!("找不到 {text}"))
                .1
                .center();
            for pressed in [true, false] {
                frame(
                    app,
                    ctx,
                    id,
                    [1220.0, 2000.0],
                    vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ],
                );
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let history_path = dir.path().join("inputs.json");
        for (id, name, setup, coefficient) in [
            (
                "standard_thermocouple",
                "固定点证书",
                vec![
                    ("stc.type", "0"),
                    ("stc.calibration", "0"),
                    ("stc.S.cal.0", "3.44723456789"),
                ],
                "stc.S.cal.0",
            ),
            (
                "standard_thermocouple",
                "S型证书",
                vec![
                    ("stc.type", "0"),
                    ("stc.calibration", "1"),
                    ("stc.S.zone", "1"),
                    ("stc.coefficient_source", "0"),
                    ("stc.S.1.c.0", "1.32900444085"),
                ],
                "stc.S.1.c.0",
            ),
            (
                "standard_thermocouple",
                "热电偶拟合",
                vec![
                    ("stc.type", "0"),
                    ("stc.calibration", "1"),
                    ("stc.S.zone", "0"),
                    ("stc.coefficient_source", "1"),
                ],
                "stc.fit.S.0.0.t",
            ),
            (
                "standard_thermocouple",
                "B型固定点",
                vec![("stc.type", "1"), ("stc.calibration", "0")],
                "stc.B.cal.4",
            ),
            (
                "standard_thermocouple",
                "B型系数",
                vec![
                    ("stc.type", "1"),
                    ("stc.calibration", "1"),
                    ("stc.B.zone", "1"),
                    ("stc.coefficient_source", "0"),
                ],
                "stc.B.1.c.8",
            ),
            (
                "standard_platinum_resistor_sprt",
                "标铂证书",
                vec![
                    ("sprt.model", "0"),
                    ("sprt.hr", "1"),
                    ("sprt.lr", "0"),
                    ("sprt.source", "0"),
                    ("sprt.rtp", "25.123456789"),
                ],
                "sprt.rtp",
            ),
            (
                "standard_platinum_resistor_sprt",
                "银点证书",
                vec![
                    ("sprt.model", "0"),
                    ("sprt.hr", "0"),
                    ("sprt.lr", "1"),
                    ("sprt.deviation", "0"),
                    ("sprt.source", "0"),
                    ("sprt.coefficient.d", "0.0000001"),
                ],
                "sprt.coefficient.d",
            ),
            (
                "standard_platinum_resistor_sprt",
                "标铂固定点",
                vec![
                    ("sprt.model", "0"),
                    ("sprt.hr", "1"),
                    ("sprt.lr", "0"),
                    ("sprt.source", "1"),
                ],
                "sprt.fixed.231.928",
            ),
            (
                "standard_platinum_resistor_sprt",
                "银点拟合",
                vec![
                    ("sprt.model", "0"),
                    ("sprt.hr", "0"),
                    ("sprt.lr", "0"),
                    ("sprt.deviation", "0"),
                    ("sprt.source", "1"),
                ],
                "sprt.fixed.961.78",
            ),
            (
                "standard_platinum_resistor_sprt",
                "CVD拟合",
                vec![("sprt.model", "1")],
                "r0",
            ),
        ] {
            let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
            let mut app = app_at(history_path.clone());
            let ctx = context();
            for (key, value) in setup {
                app.set_value(tool, key, value.into());
            }
            frame(&mut app, &ctx, id, [1220.0, 2000.0], vec![]);
            click(&mut app, &ctx, id, "保存当前…");
            if name == "固定点证书" {
                click(&mut app, &ctx, id, "保存");
                let empty = frame(&mut app, &ctx, id, [1220.0, 2000.0], vec![]);
                assert!(text_shapes(&empty)
                    .iter()
                    .any(|(text, _)| text == "请输入档案名称"));
                assert!(!dir.path().join("calibrations-v1.json").exists());
            }
            if name != "S型证书" {
                click(&mut app, &ctx, id, "例如：仪器编号或证书日期");
            }
            frame(
                &mut app,
                &ctx,
                id,
                [1220.0, 2000.0],
                vec![egui::Event::Text(name.into())],
            );
            if name == "S型证书" {
                frame(
                    &mut app,
                    &ctx,
                    id,
                    [1220.0, 2000.0],
                    vec![egui::Event::Key {
                        key: egui::Key::Enter,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Default::default(),
                    }],
                );
            } else {
                click(&mut app, &ctx, id, "保存");
            }
            let path = dir.path().join("calibrations-v1.json");
            let data: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            let entry = data["archives"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["name"] == name)
                .unwrap();
            let saved = entry["fields"][coefficient].as_str().unwrap().to_owned();
            let mut reopened = app_at(history_path.clone());
            reopened.set_value(tool, coefficient, "999".into());
            reopened.set_value(
                tool,
                if id == "standard_thermocouple" {
                    "stc.unit"
                } else {
                    "sprt.unit"
                },
                "2".into(),
            );
            let measurement = if id == "standard_thermocouple" {
                "stc.S.0.0.t.2"
            } else {
                "sprt.range.1.0.t.2"
            };
            reopened.set_value(tool, measurement, "77".into());
            let ctx = context();
            frame(&mut reopened, &ctx, id, [1220.0, 2000.0], vec![]);
            click(&mut reopened, &ctx, id, "选择已保存的档案");
            click(&mut reopened, &ctx, id, name);
            assert_eq!(reopened.value(tool, coefficient, ""), saved);
            for (key, value) in entry["fields"].as_object().unwrap() {
                assert_eq!(reopened.value(tool, key, ""), value.as_str().unwrap());
            }
            assert_eq!(
                reopened.value(
                    tool,
                    if id == "standard_thermocouple" {
                        "stc.unit"
                    } else {
                        "sprt.unit"
                    },
                    ""
                ),
                "2"
            );
            assert_eq!(reopened.value(tool, measurement, ""), "77");
            if name == "固定点证书" {
                let before_invalid = std::fs::read(&path).unwrap();
                reopened.set_value(tool, coefficient, "-".into());
                click(&mut reopened, &ctx, id, "保存当前…");
                click(&mut reopened, &ctx, id, "例如：仪器编号或证书日期");
                frame(
                    &mut reopened,
                    &ctx,
                    id,
                    [1220.0, 2000.0],
                    vec![egui::Event::Text("临时证书".into())],
                );
                click(&mut reopened, &ctx, id, "保存");
                let invalid = frame(&mut reopened, &ctx, id, [1220.0, 2000.0], vec![]);
                assert!(text_shapes(&invalid)
                    .iter()
                    .any(|(text, _)| text.contains("请先完整填写并检查证书参数")));
                assert_eq!(std::fs::read(&path).unwrap(), before_invalid);
                click(&mut reopened, &ctx, id, "取消");
                assert_eq!(reopened.value(tool, coefficient, ""), "-");
                reopened.set_value(tool, coefficient, "3.5".into());
                let edited = frame(&mut reopened, &ctx, id, [1220.0, 2000.0], vec![]);
                assert!(text_shapes(&edited)
                    .iter()
                    .any(|(text, _)| text == "已修改"));
                let before_overwrite = std::fs::read(&path).unwrap();
                click(&mut reopened, &ctx, id, "保存当前…");
                click(&mut reopened, &ctx, id, "例如：仪器编号或证书日期");
                frame(
                    &mut reopened,
                    &ctx,
                    id,
                    [1220.0, 2000.0],
                    vec![egui::Event::Text(name.into())],
                );
                click(&mut reopened, &ctx, id, "保存");
                let overwrite = frame(&mut reopened, &ctx, id, [1220.0, 2000.0], vec![]);
                assert!(text_shapes(&overwrite)
                    .iter()
                    .any(|(text, _)| text == "覆盖系数档案"));
                assert!(text_shapes(&overwrite)
                    .iter()
                    .any(|(text, _)| text.contains("已存在")));
                assert_eq!(std::fs::read(&path).unwrap(), before_overwrite);
                click(&mut reopened, &ctx, id, "取消");
                assert_eq!(std::fs::read(&path).unwrap(), before_overwrite);
                click(&mut reopened, &ctx, id, "保存");
                click(&mut reopened, &ctx, id, "确认覆盖");
                let data: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                assert_eq!(data["archives"][0]["fields"][coefficient], "3.5");
                reopened.set_value(tool, coefficient, "3.6".into());
                click(&mut reopened, &ctx, id, "更新选中…");
                let updating = frame(&mut reopened, &ctx, id, [1220.0, 2000.0], vec![]);
                assert!(text_shapes(&updating)
                    .iter()
                    .any(|(text, _)| text == "更新系数档案"));
                assert!(text_shapes(&updating)
                    .iter()
                    .any(|(text, _)| text.contains("确定用当前证书参数更新档案")));
                assert!(!text_shapes(&updating)
                    .iter()
                    .any(|(text, _)| text.contains("已存在")));
                click(&mut reopened, &ctx, id, "确认更新");
                let data: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                assert_eq!(data["archives"][0]["fields"][coefficient], "3.6");
                let before_delete = std::fs::read(&path).unwrap();
                click(&mut reopened, &ctx, id, "删除…");
                click(&mut reopened, &ctx, id, "取消");
                assert_eq!(std::fs::read(&path).unwrap(), before_delete);
                click(&mut reopened, &ctx, id, "删除…");
                click(&mut reopened, &ctx, id, "确认删除");
                let data: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                assert!(data["archives"].as_array().unwrap().is_empty());
            }
            let before_reset = std::fs::read(&path).unwrap();
            click(&mut reopened, &ctx, id, "恢复示例");
            assert_eq!(std::fs::read(&path).unwrap(), before_reset);
        }
    }

    #[test]
    fn certificate_controls_fit_narrow_and_wide_windows() {
        fn collect_rects(shape: &egui::Shape, rects: &mut Vec<(egui::Rect, egui::Color32)>) {
            match shape {
                egui::Shape::Rect(rect) => rects.push((rect.rect, rect.fill)),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect_rects(shape, rects);
                    }
                }
                _ => {}
            }
        }
        let dir = tempfile::tempdir().unwrap();
        for id in ["standard_thermocouple", "standard_platinum_resistor_sprt"] {
            for width in [620.0, 1220.0] {
                let mut app = app_at(dir.path().join("inputs.json"));
                let ctx = context();
                frame(&mut app, &ctx, id, [width, 2000.0], vec![]);
                let output = frame(&mut app, &ctx, id, [width, 2000.0], vec![]);
                let texts = text_shapes(&output);
                for label in ["系数档案", "选择已保存的档案", "保存当前…"] {
                    let rect = texts.iter().find(|(text, _)| text == label).unwrap().1;
                    assert!(
                        rect.left() >= 0.0 && rect.right() <= width,
                        "{id} {width}: {label} {rect:?}"
                    );
                }
                let mut rects = Vec::new();
                for shape in &output.shapes {
                    collect_rects(&shape.shape, &mut rects);
                }
                let save = texts
                    .iter()
                    .find(|(text, _)| text == "保存当前…")
                    .unwrap()
                    .1;
                assert!(
                    rects.iter().any(|(rect, fill)| *fill == crate::BLUE
                        && rect.contains_rect(save)
                        && rect.width() >= 96.0
                        && (35.0..=38.0).contains(&rect.height())),
                    "保存按钮应有足够尺寸"
                );
                let selected = texts
                    .iter()
                    .find(|(text, _)| text == "选择已保存的档案")
                    .unwrap()
                    .1;
                let archive_box = rects
                    .iter()
                    .map(|(rect, _)| *rect)
                    .filter(|rect| {
                        rect.contains_rect(selected)
                            && rect.width() >= 200.0
                            && (35.0..=38.0).contains(&rect.height())
                    })
                    .min_by(|a, b| a.width().total_cmp(&b.width()))
                    .expect("档案下拉框应与表单控件等高");
                let note = texts
                    .iter()
                    .find(|(text, _)| text.starts_with("保存当前模型、温区"))
                    .unwrap()
                    .1;
                assert!(
                    note.top() >= archive_box.bottom(),
                    "说明不能与档案栏重叠：{note:?} {archive_box:?}"
                );
                if id == "standard_thermocouple" && width == 1220.0 {
                    let label = texts.iter().find(|(text, _)| text == "系数档案").unwrap().1;
                    assert!(
                        (label.center().y - archive_box.center().y).abs() < 2.0,
                        "标题应与下拉框垂直居中：{label:?} {archive_box:?}"
                    );
                    let below = texts.iter().find(|(text, _)| text == "S 型").unwrap().1;
                    let below_box = rects
                        .iter()
                        .map(|(rect, _)| *rect)
                        .filter(|rect| {
                            rect.contains_rect(below)
                                && rect.width() >= 200.0
                                && (35.0..=38.0).contains(&rect.height())
                        })
                        .min_by(|a, b| a.width().total_cmp(&b.width()))
                        .expect("应有分度号下拉框");
                    assert!(
                        (archive_box.left() - below_box.left()).abs() < 1.0,
                        "档案和表单控件应左对齐：{archive_box:?} {below_box:?}"
                    );
                    assert!(
                        (archive_box.height() - below_box.height()).abs() < 1.0,
                        "下拉框高度应一致"
                    );
                }
            }
        }
    }

    fn context() -> egui::Context {
        let ctx = egui::Context::default();
        crate::fonts::configure_fonts(&ctx);
        crate::configure_style(&ctx);
        ctx
    }

    fn frame(
        app: &mut ToolboxApp,
        ctx: &egui::Context,
        id: &'static str,
        size: [f32; 2],
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size.into())),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.render_selected(ui, id));
            },
        )
    }

    fn text_shapes(output: &egui::FullOutput) -> Vec<(String, egui::Rect)> {
        fn collect(shape: &egui::Shape, texts: &mut Vec<(String, egui::Rect)>) {
            match shape {
                egui::Shape::Text(text) => texts.push((
                    text.galley.text().to_owned(),
                    egui::Rect::from_min_size(text.pos, text.galley.size()),
                )),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, texts);
                    }
                }
                _ => {}
            }
        }
        let mut texts = Vec::new();
        for shape in &output.shapes {
            collect(&shape.shape, &mut texts);
        }
        texts
    }

    #[test]
    fn uncertainty_average_count_applies_in_budget_and_quick_modes() {
        let directory = tempfile::tempdir().unwrap();
        let tool = TOOLS
            .iter()
            .find(|tool| tool.id == "uncertainty_calculation")
            .unwrap();
        for quick in [false, true] {
            let mut app = app_at(directory.path().join(format!("{quick}.json")));
            let ctx = context();
            let set = |app: &mut ToolboxApp, key: &str, value: &str| {
                app.set_value(tool, key, value.into());
            };
            if quick {
                set(&mut app, "budget.mode", "1");
                set(&mut app, "a_samples.count", "3");
                set(&mut app, "b_widths.count", "0");
            } else {
                set(&mut app, "budget.components.count", "1");
                set(&mut app, "budget.component.0.type", "0");
                set(&mut app, "budget.component.0.samples.count", "3");
            }
            for (i, value) in ["1", "2", "3"].iter().enumerate() {
                let key = if quick {
                    format!("a_samples.value.{i}")
                } else {
                    format!("budget.component.0.sample.{i}")
                };
                set(&mut app, &key, value);
            }
            let field = if quick {
                "a_average_count"
            } else {
                "budget.component.0.average_count"
            };
            let shapes = |app: &mut ToolboxApp| {
                text_shapes(&frame(app, &ctx, tool.id, [1220.0, 2000.0], vec![]))
            };
            let result_value = |texts: &[(String, egui::Rect)], expected: &str| {
                let title = if quick {
                    "A 类标准不确定度"
                } else {
                    "合成标准不确定度 u_c"
                };
                let label = texts.iter().find(|(text, _)| text == title).unwrap().1;
                texts.iter().any(|(text, rect)| {
                    text == expected
                        && (rect.left() - label.left()).abs() < 1.0
                        && rect.top() >= label.bottom()
                        && rect.top() - label.bottom() < 15.0
                })
            };
            let default = shapes(&mut app);
            assert!(default
                .iter()
                .any(|(text, _)| text.contains("留空默认 1 次")));
            assert!(result_value(&default, "1"));
            set(&mut app, field, "4");
            assert!(result_value(&shapes(&mut app), "0.5"));
            set(&mut app, field, "0");
            assert!(shapes(&mut app)
                .iter()
                .any(|(text, _)| text.contains("取测量次数 n 须为正整数")));
        }
    }

    #[test]
    fn uncertainty_components_round_before_combining_and_blank_keeps_precision() {
        let directory = tempfile::tempdir().unwrap();
        let tool = TOOLS
            .iter()
            .find(|tool| tool.id == "uncertainty_calculation")
            .unwrap();
        for quick in [false, true] {
            let mut app = app_at(directory.path().join(format!("rounding-{quick}.json")));
            let ctx = context();
            let set = |app: &mut ToolboxApp, key: &str, value: &str| {
                app.set_value(tool, key, value.into())
            };
            let prefix = if quick {
                set(&mut app, "budget.mode", "1");
                set(&mut app, "a_samples.count", "0");
                set(&mut app, "b_widths.count", "1");
                set(&mut app, "b_widths.value.0", "1.245");
                set(&mut app, "distribution", "4");
                set(&mut app, "b_sensitivity", "2");
                "quick.b.0"
            } else {
                set(&mut app, "budget.components.count", "1");
                set(&mut app, "budget.component.0.type", "1");
                set(&mut app, "budget.component.0.half_width", "1.245");
                set(&mut app, "budget.component.0.distribution", "4");
                set(&mut app, "budget.component.0.sensitivity", "2");
                set(&mut app, "budget.rounding", "0");
                "budget.component.0"
            };
            let shapes = |app: &mut ToolboxApp| {
                text_shapes(&frame(app, &ctx, tool.id, [1220.0, 2000.0], vec![]))
            };
            let result_value = |texts: &[(String, egui::Rect)], expected: &str| {
                let label = texts
                    .iter()
                    .find(|(text, _)| text == "合成标准不确定度 u_c")
                    .unwrap()
                    .1;
                texts.iter().any(|(text, rect)| {
                    text == expected
                        && (rect.left() - label.left()).abs() < 1.0
                        && rect.top() >= label.bottom()
                        && rect.top() - label.bottom() < 15.0
                })
            };
            let default = shapes(&mut app);
            assert!(default
                .iter()
                .any(|(text, _)| text.contains("留空保留原始计算精度")));
            assert!(result_value(&default, "2.49"));
            if !quick {
                assert_eq!(app.value(tool, "budget.rounding_v2", ""), "1");
            }
            set(&mut app, &format!("{prefix}.digits"), "3");
            assert!(result_value(&shapes(&mut app), "2.5"));
            set(&mut app, &format!("{prefix}.rounding"), "0");
            assert!(result_value(&shapes(&mut app), "2.48"));
            set(&mut app, &format!("{prefix}.digits"), "18");
            assert!(shapes(&mut app)
                .iter()
                .any(|(text, _)| text.contains("有效数字须为 1～17")));
        }
    }

    #[test]
    fn quick_uncertainty_combined_rounding_controls_and_results() {
        let directory = tempfile::tempdir().unwrap();
        let tool = TOOLS
            .iter()
            .find(|tool| tool.id == "uncertainty_calculation")
            .unwrap();
        let mut app = app_at(directory.path().join("quick-result.json"));
        let ctx = context();
        for (key, value) in [
            ("budget.mode", "1"),
            ("a_samples.count", "0"),
            ("b_widths.count", "1"),
            ("b_widths.value.0", "2.45"),
            ("distribution", "4"),
            ("b_sensitivity", "1"),
            ("coverage", "1"),
            ("budget.digits", "3"),
            ("budget.rounding_v2", "0"),
        ] {
            app.set_value(tool, key, value.into());
        }
        let draw = |app: &mut ToolboxApp| {
            text_shapes(&frame(app, &ctx, tool.id, [1220.0, 2500.0], vec![]))
        };
        let has_result = |texts: &[(String, egui::Rect)], expected: &str| {
            let label = texts
                .iter()
                .find(|(text, _)| text == "修约后标准不确定度")
                .unwrap()
                .1;
            texts.iter().any(|(text, rect)| {
                text == expected
                    && (rect.left() - label.left()).abs() < 1.0
                    && rect.top() >= label.bottom()
                    && rect.top() - label.bottom() < 15.0
            })
        };
        let default = draw(&mut app);
        assert!(default.iter().any(|(text, _)| text == "合成有效数字"));
        assert!(default.iter().any(|(text, _)| text == "合成修约方式"));
        assert_eq!(app.value(tool, "quick.result.digits", ""), "1");
        assert_eq!(app.value(tool, "quick.result.rounding", ""), "1");
        assert!(has_result(&default, "2.5"));
        app.set_value(tool, "quick.result.rounding", "0".into());
        assert!(has_result(&draw(&mut app), "2.4"));
        app.set_value(tool, "quick.result.digits", "2".into());
        assert!(has_result(&draw(&mut app), "2.45"));
    }

    #[test]
    fn uncertainty_cards_use_shorter_column_and_reflow_at_narrow_widths() {
        let directory = tempfile::tempdir().unwrap();
        let tool = TOOLS
            .iter()
            .find(|tool| tool.id == "uncertainty_calculation")
            .unwrap();
        let mut app = app_at(directory.path().join("columns.json"));
        let ctx = context();
        app.set_value(tool, "budget.components.count", "2".into());
        app.set_value(tool, "budget.component.0.type", "0".into());
        app.set_value(tool, "budget.component.0.samples.count", "7".into());

        let card_rect = |output: &egui::FullOutput, name: &str| {
            let marker = text_shapes(output)
                .into_iter()
                .find(|(text, _)| text == name)
                .unwrap()
                .1;
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if rect.stroke.width > 0.0
                            && rect.rect.contains_rect(marker)
                            && rect.rect.width() > 200.0
                            && rect.rect.height() > 80.0 =>
                    {
                        Some(rect.rect)
                    }
                    _ => None,
                })
                .min_by(|a, b| a.width().total_cmp(&b.width()))
                .unwrap()
        };
        let draw = |app: &mut ToolboxApp, width| frame(app, &ctx, tool.id, [width, 6000.0], vec![]);
        draw(&mut app, 1220.0);
        let initial = draw(&mut app, 1220.0);
        let first = card_rect(&initial, "分量 1");
        let second = card_rect(&initial, "分量 2");
        assert!(first.left() + 200.0 < second.left());
        assert!((first.top() - second.top()).abs() < 1.0);
        assert!(first.height() > second.height());

        app.set_value(tool, "budget.components.count", "3".into());
        let added = draw(&mut app, 1220.0);
        let third = card_rect(&added, "分量 3");
        assert!((third.left() - second.left()).abs() < 1.0);
        assert!(third.top() >= second.bottom());
        assert!(third.right() <= 1200.0);
        let total = text_shapes(&added)
            .into_iter()
            .find(|(text, _)| text == "总合成方法")
            .unwrap()
            .1;
        assert!(total.top() >= first.bottom().max(third.bottom()));

        // 改变已有卡片高度不会让其他卡片在输入时跳到另一列。
        app.set_value(tool, "budget.component.0.type", "1".into());
        let changed = draw(&mut app, 1220.0);
        assert!((card_rect(&changed, "分量 3").left() - second.left()).abs() < 1.0);
        app.set_value(tool, "budget.components.count", "2".into());
        draw(&mut app, 1220.0);
        app.set_value(tool, "budget.components.count", "3".into());
        let readded = draw(&mut app, 1220.0);
        assert!((card_rect(&readded, "分量 3").left() - first.left()).abs() < 1.0);

        let narrow = draw(&mut app, 620.0);
        let cards: Vec<_> = (1..=3)
            .map(|i| card_rect(&narrow, &format!("分量 {i}")))
            .collect();
        assert!(cards
            .iter()
            .all(|rect| (rect.left() - cards[0].left()).abs() < 1.0));
        assert!(cards[1].top() >= cards[0].bottom() && cards[2].top() >= cards[1].bottom());
        let wide_again = draw(&mut app, 1220.0);
        assert!((card_rect(&wide_again, "分量 3").left() - first.left()).abs() < 1.0);
        assert_eq!(app.value(tool, "budget.component.0.samples.count", ""), "7");

        for (i, value) in ["1", "2", "3"].iter().enumerate() {
            app.set_value(tool, &format!("budget.component.{i}.type"), "1".into());
            app.set_value(
                tool,
                &format!("budget.component.{i}.distribution"),
                "4".into(),
            );
            app.set_value(
                tool,
                &format!("budget.component.{i}.half_width"),
                (*value).into(),
            );
        }
        for width in [620.0, 1220.0] {
            let output = draw(&mut app, width);
            let texts = text_shapes(&output);
            let label = texts
                .iter()
                .find(|(text, _)| text == "合成标准不确定度 u_c")
                .unwrap()
                .1;
            assert!(texts.iter().any(|(text, rect)| text == "3.74165739"
                && (rect.left() - label.left()).abs() < 1.0
                && rect.top() >= label.bottom()
                && rect.top() - label.bottom() < 15.0));
        }
    }

    #[test]
    fn quick_uncertainty_cards_balance_and_preserve_inputs_on_resize() {
        let directory = tempfile::tempdir().unwrap();
        let tool = TOOLS
            .iter()
            .find(|tool| tool.id == "uncertainty_calculation")
            .unwrap();
        let mut app = app_at(directory.path().join("quick-columns.json"));
        let ctx = context();
        app.set_value(tool, "budget.mode", "1".into());
        app.set_value(tool, "a_samples.count", "10".into());
        app.set_value(tool, "b_widths.count", "1".into());
        app.set_value(tool, "b_widths.value.0", "1.245".into());
        app.set_value(tool, "quick.b.0.digits", "3".into());
        let draw = |app: &mut ToolboxApp, width| frame(app, &ctx, tool.id, [width, 6000.0], vec![]);
        let card_rect = |output: &egui::FullOutput, name: &str| {
            let marker = text_shapes(output)
                .into_iter()
                .find(|(text, _)| text == name)
                .unwrap()
                .1;
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if rect.stroke.width > 0.0
                            && rect.rect.contains_rect(marker)
                            && rect.rect.width() > 200.0
                            && rect.rect.height() > 80.0 =>
                    {
                        Some(rect.rect)
                    }
                    _ => None,
                })
                .min_by(|a, b| a.width().total_cmp(&b.width()))
                .unwrap()
        };
        draw(&mut app, 1220.0);
        let initial = draw(&mut app, 1220.0);
        let a = card_rect(&initial, "A 类评定数据");
        let b1 = card_rect(&initial, "B 类分量 1");
        assert!(a.left() + 200.0 < b1.left());
        assert!((a.top() - b1.top()).abs() < 1.0 && a.height() > b1.height());

        app.set_value(tool, "b_widths.count", "2".into());
        let added = draw(&mut app, 1220.0);
        let b2 = card_rect(&added, "B 类分量 2");
        assert!((b2.left() - b1.left()).abs() < 1.0 && b2.top() >= b1.bottom());
        app.set_value(tool, "a_samples.count", "2".into());
        let resized_a = draw(&mut app, 1220.0);
        let shorter_left = card_rect(&resized_a, "A 类评定数据").bottom()
            <= card_rect(&resized_a, "B 类分量 2").bottom();
        app.set_value(tool, "b_widths.count", "3".into());
        let added_again = draw(&mut app, 1220.0);
        let b3 = card_rect(&added_again, "B 类分量 3");
        let expected_left = if shorter_left { a.left() } else { b1.left() };
        assert!((b3.left() - expected_left).abs() < 1.0);
        let total = text_shapes(&added_again)
            .into_iter()
            .find(|(text, _)| text == "合成设置")
            .unwrap()
            .1;
        let bottom = ["A 类评定数据", "B 类分量 1", "B 类分量 2", "B 类分量 3"]
            .iter()
            .map(|name| card_rect(&added_again, name).bottom())
            .fold(0.0_f32, f32::max);
        assert!(total.top() >= bottom);

        let narrow = draw(&mut app, 620.0);
        let cards: Vec<_> = ["A 类评定数据", "B 类分量 1", "B 类分量 2", "B 类分量 3"]
            .iter()
            .map(|name| card_rect(&narrow, name))
            .collect();
        assert!(cards
            .iter()
            .all(|r| (r.left() - cards[0].left()).abs() < 1.0));
        assert!(cards
            .windows(2)
            .all(|pair| pair[1].top() >= pair[0].bottom()));
        let wide_again = draw(&mut app, 1220.0);
        assert!((card_rect(&wide_again, "B 类分量 3").left() - b3.left()).abs() < 1.0);
        assert_eq!(app.value(tool, "b_widths.value.0", ""), "1.245");
        assert_eq!(app.value(tool, "quick.b.0.digits", ""), "3");
    }

    #[test]
    fn all_default_pages_render_without_errors() {
        let directory = tempfile::tempdir().unwrap();
        let ctx = context();
        assert_eq!(TOOLS.len(), 50);
        assert_eq!(
            TOOLS.iter().filter(|tool| tool.remembers_inputs()).count(),
            17
        );
        for tool in TOOLS {
            let mut app = app_at(directory.path().join(format!("{}.json", tool.id)));
            let output = frame(&mut app, &ctx, tool.id, [1220.0, 50_000.0], vec![]);
            let texts = text_shapes(&output);
            let errors: Vec<_> = texts
                .iter()
                .filter(|(text, _)| {
                    text.starts_with("⚠") && !text.starts_with("⚠ 取值说明与适用限制")
                })
                .collect();
            assert!(errors.is_empty(), "{}: {errors:?}", tool.id);
            assert!(
                texts.iter().any(|(text, _)| text
                    == if matches!(tool.kind, crate::catalog::ToolKind::PdfDownload) {
                        "清空"
                    } else {
                        "恢复示例"
                    }),
                "{}",
                tool.id
            );
            assert!(app.history.values.keys().all(|key|
                tool.remembers_inputs() && key.starts_with(&format!("{}.", tool.id))), "{}", tool.id);
        }
    }

    #[test]
    fn tool_content_panels_share_margins_at_normal_and_narrow_widths() {
        let directory = tempfile::tempdir().unwrap();
        for width in [620.0, 1220.0] {
            let mut left_edges = Vec::new();
            for (id, marker) in [
                ("blackbody_radiation_source", "辐射温度参数"),
                ("gauge_metric", "计算参数"),
                ("metrology_pdf_download", "规范名称或编号"),
            ] {
                let mut app = app_at(directory.path().join(format!("{id}.json")));
                let ctx = context();
                frame(&mut app, &ctx, id, [width, 2000.0], vec![]);
                let output = frame(&mut app, &ctx, id, [width, 2000.0], vec![]);
                let marker = text_shapes(&output)
                    .into_iter()
                    .find(|(text, _)| text == marker)
                    .unwrap()
                    .1;
                let panel = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Rect(rect)
                            if rect.stroke.width > 0.0 && rect.rect.contains_rect(marker) =>
                        {
                            Some(rect.rect)
                        }
                        _ => None,
                    })
                    .min_by(|a, b| a.width().total_cmp(&b.width()))
                    .expect("参数面板应包围标题");
                assert!(
                    panel.right() <= width - 20.0,
                    "{id} 的面板超出右边距：{panel:?}"
                );
                left_edges.push(panel.left());
            }
            assert!(
                left_edges.iter().all(|x| (x - left_edges[0]).abs() < 1.0),
                "面板应左对齐：{left_edges:?}"
            );
        }
    }

    #[test]
    fn input_and_choice_controls_align_at_desktop_width() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = app_at(directory.path().join("inputs.json"));
        let ctx = context();
        let id = "blackbody_radiation_source";
        frame(&mut app, &ctx, id, [1220.0, 2000.0], vec![]);
        let output = frame(&mut app, &ctx, id, [1220.0, 2000.0], vec![]);
        let texts = text_shapes(&output);
        let controls: Vec<_> = ["计算模型", "有效波长 (µm)"]
            .iter()
            .map(|label| {
                let label = texts.iter().find(|(text, _)| text == label).unwrap().1;
                output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Rect(rect)
                            if rect.stroke.width > 0.0
                                && rect.rect.left() > label.right()
                                && rect.rect.center().y >= label.top()
                                && rect.rect.center().y <= label.bottom()
                                && (35.0..=40.0).contains(&rect.rect.height()) =>
                        {
                            Some(rect.rect)
                        }
                        _ => None,
                    })
                    .min_by(|a, b| a.width().total_cmp(&b.width()))
                    .expect("应显示完整的表单控件边框")
            })
            .collect();
        assert!(
            (controls[0].left() - controls[1].left()).abs() < 1.0,
            "控件应左对齐：{controls:?}"
        );
        assert!(
            (controls[0].width() - controls[1].width()).abs() < 1.0,
            "控件应等宽：{controls:?}"
        );
        assert!(
            (controls[0].height() - controls[1].height()).abs() < 1.0,
            "控件应等高：{controls:?}"
        );
    }

    #[test]
    fn spline_inputs_have_visible_borders_without_focus() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = app_at(directory.path().join("inputs.json"));
        let ctx = context();
        let id = "bspline_interpolation";
        frame(&mut app, &ctx, id, [1220.0, 2000.0], vec![]);
        let output = frame(&mut app, &ctx, id, [1220.0, 2000.0], vec![]);
        let texts = text_shapes(&output);
        for value in ["0.5", "250", "2002"] {
            let text = texts
                .iter()
                .find(|(text, _)| text == value)
                .expect("应显示输入值")
                .1;
            assert!(
                output.shapes.iter().any(|shape| match &shape.shape {
                    egui::Shape::Rect(rect) =>
                        rect.stroke.width >= 1.0
                            && rect.stroke.color.r() < 190
                            && rect.stroke.color.g() < 190
                            && rect.rect.height() >= 35.0
                            && rect.rect.contains_rect(text),
                    _ => false,
                }),
                "输入值 {value} 应位于清晰可见的边框内"
            );
        }
        let point = texts
            .iter()
            .find(|(text, _)| text == "250")
            .unwrap()
            .1
            .center();
        for pressed in [true, false] {
            frame(
                &mut app,
                &ctx,
                id,
                [1220.0, 2000.0],
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        let focused = frame(&mut app, &ctx, id, [1220.0, 2000.0], vec![]);
        assert!(focused.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Rect(rect) if rect.stroke.color == crate::BLUE && rect.rect.contains(point)
        )), "聚焦后边框应变蓝");
        frame(
            &mut app,
            &ctx,
            id,
            [1220.0, 2000.0],
            vec![egui::Event::Text("9".into())],
        );
        let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
        assert_ne!(app.value(tool, "q_0_x", "250"), "250", "输入框应可正常编辑");
    }

    #[test]
    fn styled_choice_updates_the_selected_model() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = app_at(directory.path().join("inputs.json"));
        let ctx = context();
        let id = "blackbody_radiation_source";
        let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
        let size = [1220.0, 2000.0];
        let output = frame(&mut app, &ctx, id, size, vec![]);
        let choice = text_shapes(&output)
            .into_iter()
            .find(|(text, _)| text == "有效波长法")
            .unwrap()
            .1
            .center();
        for pressed in [true, false] {
            frame(
                &mut app,
                &ctx,
                id,
                size,
                vec![
                    egui::Event::PointerMoved(choice),
                    egui::Event::PointerButton {
                        pos: choice,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        let output = frame(&mut app, &ctx, id, size, vec![]);
        let option = text_shapes(&output)
            .into_iter()
            .find(|(text, _)| text == "Planck 辐射平衡")
            .expect("下拉菜单应展开")
            .1
            .center();
        for pressed in [true, false] {
            frame(
                &mut app,
                &ctx,
                id,
                size,
                vec![
                    egui::Event::PointerMoved(option),
                    egui::Event::PointerButton {
                        pos: option,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        assert_eq!(app.value(tool, "model", "0"), "1");
    }

    #[test]
    fn invalid_input_shows_a_distinct_error_card() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = app_at(directory.path().join("inputs.json"));
        let ctx = context();
        let id = "blackbody_radiation_source";
        let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
        frame(&mut app, &ctx, id, [1220.0, 2000.0], vec![]);
        app.set_value(tool, "wavelength", "0".into());
        let output = frame(&mut app, &ctx, id, [1220.0, 2000.0], vec![]);
        let error = text_shapes(&output)
            .into_iter()
            .find(|(text, _)| text.starts_with('⚠'))
            .expect("应显示错误说明")
            .1;
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_rgb(255, 244, 243) && rect.rect.contains_rect(error)
        )), "错误说明应位于提示卡片中");
    }

    #[test]
    fn rendered_reset_button_clears_saved_rows_and_keeps_other_tools() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("inputs.json");
        let ctx = context();
        let fit = TOOLS
            .iter()
            .find(|tool| tool.id == "calibration_curve_fit")
            .unwrap();
        let repeat = TOOLS
            .iter()
            .find(|tool| tool.id == "repeatability_calculation")
            .unwrap();
        let mut app = app_at(path.clone());
        app.set_value(repeat, "samples.value.0", "16.5".into());
        let _ = frame(&mut app, &ctx, repeat.id, [1220.0, 820.0], vec![]);
        app.set_value(fit, "points.count", "3".into());
        app.set_value(fit, "x_name", "现场自定义名称".into());
        app.set_value(fit, "point.99.y", "12345".into());
        for (i, (x, y)) in [(0, 1), (1, 3), (2, 5)].iter().enumerate() {
            app.set_value(fit, &format!("point.{i}.x"), x.to_string());
            app.set_value(fit, &format!("point.{i}.y"), y.to_string());
        }
        let output = frame(&mut app, &ctx, fit.id, [1220.0, 820.0], vec![]);
        app.history.flush();
        let reopened = app_at(path.clone());
        assert_eq!(reopened.value(fit, "point.2.y", ""), "5");
        assert_eq!(reopened.value(fit, "x_name", ""), "现场自定义名称");
        let texts = text_shapes(&output);
        let button = texts.iter().find(|(text, _)| text == "恢复示例").unwrap().1;
        assert!(button.center().x > 900.0);
        let title = texts.iter().find(|(text, _)| text == fit.name).unwrap().1;
        assert!((button.center().y - title.center().y).abs() < 24.0);
        let point = button.center();
        for pressed in [true, false] {
            let _ = frame(
                &mut app,
                &ctx,
                fit.id,
                [1220.0, 820.0],
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        assert_eq!(app.value(fit, "points.count", ""), "6");
        assert_eq!(app.value(fit, "point.0.y", ""), "0.197");
        assert!(!app.values.contains_key(&format!("{}.point.99.y", fit.id)));
        assert!(!app.values.contains_key(&format!("{}.x_name", fit.id)));
        assert_eq!(app.value(repeat, "samples.value.0", ""), "16.5");
        app.history.flush();
        assert_eq!(app_at(path).value(fit, "point.0.y", ""), "0.197");
    }

    #[test]
    fn quick_conversion_reopens_with_examples_and_reset_fits_narrow_width() {
        let directory = tempfile::tempdir().unwrap();
        let ctx = context();
        let mut app = app_at(directory.path().join("inputs.json"));
        let unit = TOOLS
            .iter()
            .find(|tool| tool.id == "unit_pressure")
            .unwrap();
        let _ = frame(&mut app, &ctx, unit.id, [620.0, 820.0], vec![]);
        app.set_value(unit, "value", "999".into());
        let _ = frame(
            &mut app,
            &ctx,
            "calibration_curve_fit",
            [620.0, 820.0],
            vec![],
        );
        let output = frame(&mut app, &ctx, unit.id, [620.0, 820.0], vec![]);
        assert_eq!(app.value(unit, "value", ""), "1");
        assert!(!app
            .history
            .values
            .keys()
            .any(|key| key.starts_with(unit.id)));
        let texts = text_shapes(&output);
        let button = texts.iter().find(|(text, _)| text == "恢复示例").unwrap().1;
        assert!(button.right() <= 596.0 && button.left() > 450.0);
    }

    #[test]
    fn persists_only_selected_tools_and_replaces_their_previous_fields() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("inputs.json");
        let fit = TOOLS
            .iter()
            .find(|t| t.id == "calibration_curve_fit")
            .unwrap();
        let unit = TOOLS.iter().find(|t| t.id == "unit_pressure").unwrap();
        let mut history = InputHistory::load(Some(path.clone()));
        let mut inputs = HashMap::from([
            (format!("{}.point.0", fit.id), "-".into()),
            (format!("{}.point.1", fit.id), "自定义数据".into()),
            (format!("{}.value", unit.id), "123".into()),
        ]);
        history.observe(fit, &inputs);
        history.observe(unit, &inputs);
        history.flush();
        let loaded = InputHistory::load(Some(path.clone()));
        assert_eq!(loaded.values.len(), 2);
        assert_eq!(loaded.values[&format!("{}.point.0", fit.id)], "-");

        // 恢复示例时删除整个工具命名空间，隐藏的旧数据也不能重新出现。
        inputs.retain(|key, _| !key.starts_with(&format!("{}.", fit.id)));
        inputs.insert(format!("{}.point.0", fit.id), "1".into());
        history.observe(fit, &inputs);
        history.flush();
        let loaded = InputHistory::load(Some(path));
        assert_eq!(
            loaded.values,
            HashMap::from([(format!("{}.point.0", fit.id), "1".into())])
        );
    }

    #[test]
    fn ignores_broken_unknown_and_excluded_history() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("inputs.json");
        std::fs::write(&path, "{broken").unwrap();
        assert!(InputHistory::load(Some(path.clone())).values.is_empty());
        std::fs::write(&path, r#"{"version":1,"values":{"unit_pressure.value":"123","unknown.value":"1","uncertainty_calculation.name":"预算","uncertainty_calculation.invalid":12}}"#).unwrap();
        assert_eq!(InputHistory::load(Some(path.clone())).values.len(), 1);
        std::fs::write(
            &path,
            r#"{"version":2,"values":{"uncertainty_calculation.name":"预算"}}"#,
        )
        .unwrap();
        assert!(InputHistory::load(Some(path)).values.is_empty());
    }

    #[test]
    fn keeps_pending_input_after_a_write_failure_and_can_retry() {
        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path().join("blocked");
        std::fs::write(&parent, "file instead of directory").unwrap();
        let path = parent.join("inputs.json");
        let tool = TOOLS
            .iter()
            .find(|t| t.id == "repeatability_calculation")
            .unwrap();
        let inputs = HashMap::from([(format!("{}.samples.value.0", tool.id), "12.3".into())]);
        let mut history = InputHistory::load(Some(path.clone()));
        history.observe(tool, &inputs);
        assert!(history.remaining().unwrap() > Duration::ZERO);
        history.flush();
        assert!(history.error);
        assert!(history.remaining().is_some());
        assert_eq!(history.values, inputs);
        std::fs::remove_file(parent).unwrap();
        history.flush();
        assert!(!history.error);
        assert!(history.remaining().is_none());
        assert_eq!(InputHistory::load(Some(path)).values, inputs);
    }
}
