//! 本地输入续接：按工具隔离，停顿后原子写入，正常退出时补存最后一次编辑。
use crate::catalog::{TOOLS, ToolDef};
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
                assert!(
                    text_shapes(&empty)
                        .iter()
                        .any(|(text, _)| text == "请输入档案名称")
                );
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
                assert!(
                    text_shapes(&invalid)
                        .iter()
                        .any(|(text, _)| text.contains("请先完整填写并检查证书参数"))
                );
                assert_eq!(std::fs::read(&path).unwrap(), before_invalid);
                click(&mut reopened, &ctx, id, "取消");
                assert_eq!(reopened.value(tool, coefficient, ""), "-");
                reopened.set_value(tool, coefficient, "3.5".into());
                let edited = frame(&mut reopened, &ctx, id, [1220.0, 2000.0], vec![]);
                assert!(
                    text_shapes(&edited)
                        .iter()
                        .any(|(text, _)| text == "已修改")
                );
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
                assert!(
                    text_shapes(&overwrite)
                        .iter()
                        .any(|(text, _)| text == "覆盖系数档案")
                );
                assert!(
                    text_shapes(&overwrite)
                        .iter()
                        .any(|(text, _)| text.contains("已存在"))
                );
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
                assert!(
                    text_shapes(&updating)
                        .iter()
                        .any(|(text, _)| text == "更新系数档案")
                );
                assert!(
                    text_shapes(&updating)
                        .iter()
                        .any(|(text, _)| text.contains("确定用当前证书参数更新档案"))
                );
                assert!(
                    !text_shapes(&updating)
                        .iter()
                        .any(|(text, _)| text.contains("已存在"))
                );
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

    fn note_text_is_rendered(texts: &[(String, egui::Rect)], note: &str) -> bool {
        if note.is_empty() || texts.iter().any(|(text, _)| text == note) {
            return true;
        }
        note.lines().all(|line| {
            line.is_empty()
                || texts.iter().any(|(text, _)| text == line)
                || crate::calculators::formula_layout::visible_text_fragments(line)
                    .iter()
                    .filter(|part| !part.is_empty())
                    .all(|part| texts.iter().any(|(text, _)| text.contains(part)))
        })
    }

    #[test]
    fn height_process_tracks_the_result_and_disappears_for_invalid_input() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = app_at(directory.path().join("inputs.json"));
        let ctx = context();
        let id = "altitude_difference_correction";
        let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
        let size = [1220.0, 2400.0];
        let first = frame(&mut app, &ctx, id, size, vec![]);
        let texts = text_shapes(&first);
        assert!(!texts.iter().any(|(text, _)| text.contains("ρ × g × ΔH")));
        let point = texts
            .iter()
            .find(|(text, _)| text == "计算过程")
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
        let show = |app: &mut ToolboxApp| {
            for _ in 0..12 {
                frame(app, &ctx, id, size, vec![]);
            }
            text_shapes(&frame(app, &ctx, id, size, vec![]))
                .into_iter()
                .map(|(text, _)| text)
                .collect::<Vec<_>>()
        };
        let liquid = show(&mut app);
        assert!(!liquid.iter().any(|text| text == "显示完整双精度"));
        assert!(liquid.iter().any(|text| text.contains("978.899803 Pa")));
        assert!(liquid.iter().any(|text| text.contains("ρ × g × ΔH")));
        app.set_value(tool, "height", "-0.1".into());
        assert!(
            show(&mut app)
                .iter()
                .any(|text| text.contains("-978.899803 Pa"))
        );
        app.set_value(tool, "medium", "1".into());
        app.set_value(tool, "gas.state", "1".into());
        app.set_value(tool, "gas.pressure_type", "0".into());
        let gauge = show(&mut app);
        assert!(
            gauge
                .iter()
                .any(|text| text.contains("标准器绝压 = 表压 + 环境绝压"))
        );
        assert!(
            gauge.iter().any(|text| text.contains("环境气柱压力差")),
            "{gauge:?}"
        );
        app.set_value(tool, "gas.pressure_type", "1".into());
        let absolute = show(&mut app);
        assert!(
            absolute
                .iter()
                .any(|text| text.contains("标准器绝压 = 100000 Pa"))
        );
        assert!(!absolute.iter().any(|text| text.contains("环境气柱压力差")));
        app.set_value(tool, "gas.state", "0".into());
        app.set_value(tool, "height", "0".into());
        let local_gas = show(&mut app);
        assert!(
            local_gas
                .iter()
                .any(|text| text.contains("当地介质密度 ρ = 1.204"))
        );
        assert!(local_gas.iter().any(|text| text.contains("无高差")));
        assert!(!local_gas.iter().any(|text| text.contains("环境气柱压力差")));
        app.set_value(tool, "gas.state", "1".into());
        app.set_value(tool, "density.gas.normal", "0".into());
        let invalid = show(&mut app);
        assert!(
            invalid
                .iter()
                .any(|text| text.contains("密度与重力加速度必须大于 0"))
        );
        assert!(!invalid.iter().any(|text| text == "计算过程"));
    }

    #[test]
    fn pressure_tools_show_process_for_each_model_and_selection_mode() {
        let directory = tempfile::tempdir().unwrap();
        let size = [1220.0, 8000.0];
        for id in [
            "pressure_reference_selection",
            "pressure_transmitter_reference_selection",
            "altitude_pressure_converter",
        ] {
            let mut app = app_at(directory.path().join(format!("{id}.json")));
            let ctx = context();
            let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
            let show = |app: &mut ToolboxApp| {
                for _ in 0..12 {
                    frame(app, &ctx, id, size, vec![]);
                }
                text_shapes(&frame(app, &ctx, id, size, vec![]))
                    .into_iter()
                    .map(|(text, _)| text)
                    .collect::<Vec<_>>()
            };
            let open = |app: &mut ToolboxApp| {
                let out = frame(app, &ctx, id, size, vec![]);
                let point = text_shapes(&out)
                    .iter()
                    .find(|(text, _)| text == "计算过程")
                    .unwrap()
                    .1
                    .center();
                for pressed in [true, false] {
                    frame(
                        app,
                        &ctx,
                        id,
                        size,
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
            };
            open(&mut app);
            if id == "altitude_pressure_converter" {
                assert!(
                    show(&mut app)
                        .iter()
                        .any(|text| text.contains("空气密度 ρ = p /"))
                );
                app.set_value(tool, "mode", "1".into());
                assert!(show(&mut app).iter().any(|text| text.contains("二分反算")));
                app.set_value(tool, "pressure", "0".into());
                assert!(!show(&mut app).iter().any(|text| text == "计算过程"));
            } else {
                assert!(
                    show(&mut app)
                        .iter()
                        .any(|text| text.contains("量传比 = 被检允差"))
                );
                if id == "pressure_reference_selection" {
                    app.set_value(tool, "dut_low", "10.01".into());
                    app.set_value(tool, "dut_high", "10.03".into());
                    app.set_value(tool, "ref_high", "20".into());
                    let text = show(&mut app);
                    assert!(
                        text.iter().any(|s| s.contains("10.03 − 10.01 = 0.02")),
                        "{text:?}"
                    );
                    assert!(
                        !text.iter().any(|s| s.contains("0.019999999999999574")),
                        "{text:?}"
                    );
                    app.set_value(tool, "dut_low", "0".into());
                    app.set_value(tool, "dut_high", "1".into());
                    app.set_value(tool, "ref_high", "1.6".into());
                    app.set_value(tool, "selection.reference_type", "1".into());
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|text| text.contains("活塞压力计"))
                    );
                } else {
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|text| text.contains("电测分量 ="))
                    );
                }
                for mode in ["1", "2"] {
                    app.set_value(tool, "selection.mode", mode.into());
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|text| text.contains("逐一核验目录量程"))
                    );
                }
                app.set_value(tool, "selection.mode", "0".into());
                app.set_value(tool, "required_ratio", "0".into());
                assert!(!show(&mut app).iter().any(|text| text == "计算过程"));
                if id == "pressure_transmitter_reference_selection" {
                    app.set_value(tool, "required_ratio", "3".into());
                    app.set_value(tool, "selection.model", "1".into());
                    open(&mut app);
                    let legacy = show(&mut app);
                    assert!(legacy.iter().any(|text| text.contains("压力通道（量程")));
                    assert!(legacy.iter().any(|text| text.contains("电信号通道")));
                }
            }
        }
    }

    #[test]
    fn all_calculation_processes_are_directly_before_the_explanation() {
        use crate::catalog::ToolKind;
        let directory = tempfile::tempdir().unwrap();
        let mut checked = 0;
        for tool in TOOLS.iter().filter(|tool| {
            matches!(
                tool.kind,
                ToolKind::Unit(_)
                    | ToolKind::WorkingThermocouple
                    | ToolKind::IndustrialResistance
                    | ToolKind::StandardThermocouple
                    | ToolKind::StandardPlatinumResistor
                    | ToolKind::Thermistor
                    | ToolKind::PressureReference
                    | ToolKind::TransmitterReference
                    | ToolKind::HeightCorrection
                    | ToolKind::AltitudePressure
                    | ToolKind::HumidityConversion
                    | ToolKind::EarBlackbody
                    | ToolKind::BlackbodyError
                    | ToolKind::RadiationCorrection
                    | ToolKind::Repeatability
                    | ToolKind::RangeMethod
                    | ToolKind::ResultantError
                    | ToolKind::ErrorAnalysis
                    | ToolKind::Uncertainty
                    | ToolKind::Gauge(_)
                    | ToolKind::Transmitter
                    | ToolKind::BSplineInterpolation
                    | ToolKind::CalibrationCurveFit
            )
        }) {
            let mut app = app_at(directory.path().join(format!("{}.json", tool.id)));
            let ctx = context();
            let out = frame(&mut app, &ctx, tool.id, [1220.0, 18000.0], vec![]);
            let shapes = text_shapes(&out);
            let process = shapes
                .iter()
                .find(|(s, _)| s == "计算过程")
                .unwrap_or_else(|| panic!("{} 无计算过程", tool.id))
                .1;
            let note = shapes
                .iter()
                .find(|(s, _)| s == "计算说明")
                .unwrap_or_else(|| panic!("{} 无计算说明", tool.id))
                .1;
            assert!(process.bottom() < note.top(), "{} 位置不对", tool.id);
            assert!(
                note.top() - process.bottom() < 120.0,
                "{} 中间还有其他内容",
                tool.id
            );
            checked += 1;
        }
        assert_eq!(checked, 49);
        for (id, settings) in [
            (
                "standard_thermocouple",
                vec![("stc.calibration", "1"), ("stc.coefficient_source", "1")],
            ),
            ("standard_platinum_resistor_sprt", vec![("sprt.model", "1")]),
            (
                "standard_platinum_resistor_sprt",
                vec![("sprt.source", "1")],
            ),
            ("thermistor", vec![("ntc.model", "1")]),
            ("thermistor", vec![("ntc.source", "1")]),
            (
                "pressure_transmitter_reference_selection",
                vec![("selection.model", "1")],
            ),
            (
                "pressure_reference_selection",
                vec![("selection.mode", "2")],
            ),
            ("resultant_error", vec![("components.mode", "1")]),
            ("uncertainty_calculation", vec![("budget.mode", "1")]),
        ] {
            let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
            let mut app = app_at(directory.path().join(format!("{id}_variant.json")));
            for (key, value) in settings {
                app.set_value(tool, key, value.into());
            }
            let ctx = context();
            let shapes = text_shapes(&frame(&mut app, &ctx, id, [1220.0, 18000.0], vec![]));
            let process = shapes
                .iter()
                .find(|(s, _)| s == "计算过程")
                .unwrap_or_else(|| panic!("{id} 模式无计算过程: {shapes:?}"))
                .1;
            let note = shapes
                .iter()
                .find(|(s, _)| s == "计算说明")
                .unwrap_or_else(|| panic!("{id} 模式无计算说明"))
                .1;
            assert!(
                process.bottom() < note.top() && note.top() - process.bottom() < 120.0,
                "{id} 模式的位置不对"
            );
        }
    }

    #[test]
    fn every_calculation_note_renders_its_actual_summary_and_formula() {
        let directory = tempfile::tempdir().unwrap();
        let size = [1220.0, 18000.0];
        let mut checked = 0;
        for tool in TOOLS
            .iter()
            .filter(|tool| !matches!(tool.kind, crate::catalog::ToolKind::PdfDownload))
        {
            let mut app = app_at(directory.path().join(format!("note-{}.json", tool.id)));
            let ctx = context();
            let output = frame(&mut app, &ctx, tool.id, size, vec![]);
            let point = text_shapes(&output)
                .iter()
                .find(|(s, _)| s == "计算说明")
                .unwrap()
                .1
                .center();
            for pressed in [true, false] {
                frame(
                    &mut app,
                    &ctx,
                    tool.id,
                    size,
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
            for _ in 0..12 {
                frame(&mut app, &ctx, tool.id, size, vec![]);
            }
            let output = frame(&mut app, &ctx, tool.id, size, vec![]);
            let texts = text_shapes(&output);
            let (summary, formula) = crate::calculators::explanations::selected(tool.kind, 0);
            assert!(
                note_text_is_rendered(&texts, summary),
                "{} 缺少对应摘要",
                tool.id
            );
            assert!(
                note_text_is_rendered(&texts, &formula),
                "{} 缺少对应公式",
                tool.id
            );
            checked += 1;
        }
        assert_eq!(checked, 49);
    }

    #[test]
    fn model_variants_render_the_matching_calculation_note() {
        let directory = tempfile::tempdir().unwrap();
        let size = [1220.0, 18000.0];
        for (id, key, value, mode) in [
            ("standard_thermocouple", "stc.calibration", "1", 1),
            ("standard_platinum_resistor_sprt", "sprt.model", "1", 1),
            ("standard_platinum_resistor_sprt", "sprt.deviation", "1", 2),
            ("thermistor", "ntc.model", "1", 1),
            (
                "pressure_reference_selection",
                "selection.reference_type",
                "1",
                1,
            ),
            (
                "pressure_transmitter_reference_selection",
                "selection.model",
                "1",
                1,
            ),
            ("ear_thermometer_blackbody", "ear.model", "1", 1),
            ("blackbody_radiation_source", "model", "1", 1),
            ("radiation_thermometer", "model", "1", 1),
            ("resultant_error", "components.mode", "1", 1),
            ("uncertainty_calculation", "budget.mode", "1", 1),
        ] {
            let tool = TOOLS
                .iter()
                .find(|tool| tool.id == id)
                .unwrap_or_else(|| panic!("不存在工具 {id}"));
            let mut app = app_at(directory.path().join(format!("{id}-{key}.json")));
            let ctx = context();
            frame(&mut app, &ctx, id, size, vec![]);
            app.set_value(tool, key, value.into());
            let output = frame(&mut app, &ctx, id, size, vec![]);
            let point = text_shapes(&output)
                .iter()
                .find(|(s, _)| s == "计算说明")
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
            for _ in 0..12 {
                frame(&mut app, &ctx, id, size, vec![]);
            }
            let output = frame(&mut app, &ctx, id, size, vec![]);
            let texts = text_shapes(&output);
            let (summary, formula) = crate::calculators::explanations::selected(tool.kind, mode);
            assert!(
                note_text_is_rendered(&texts, summary),
                "{id} 模式 {mode} 摘要不符: {:?}",
                texts.iter().map(|(s, _)| s).collect::<Vec<_>>()
            );
            assert!(
                note_text_is_rendered(&texts, &formula),
                "{id} 模式 {mode} 公式不符"
            );
        }
    }

    #[test]
    fn gauge_process_displays_formulas_or_identifies_direct_table_lookups() {
        let directory = tempfile::tempdir().unwrap();
        let size = [1220.0, 24000.0];
        for (id, expected) in [
            (
                "gauge_metric",
                "通端工作塞规：中径名义 = 工件中径下限 + ZPL =",
            ),
            (
                "gauge_trapezoidal",
                "通端工作塞规：中径名义 = 工件中径下限 + ZPL =",
            ),
            ("gauge_unified", "通端工作"),
            ("gauge_pipe", "按所选规格查内置表"),
            (
                "gauge_rebar",
                "通端工作塞规：中径名义 = 工件中径下限 + ZPL =",
            ),
            ("gauge_smooth", "孔用止端下限 = D + ES − T1 ="),
        ] {
            let mut app = app_at(directory.path().join(format!("{id}-steps.json")));
            let ctx = context();
            let output = frame(&mut app, &ctx, id, size, vec![]);
            let point = text_shapes(&output)
                .iter()
                .find(|(s, _)| s == "计算过程")
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
            for _ in 0..12 {
                frame(&mut app, &ctx, id, size, vec![]);
            }
            let output = frame(&mut app, &ctx, id, size, vec![]);
            let text = text_shapes(&output);
            assert!(
                text.iter().any(|(s, _)| s.contains(expected)),
                "{id} 缺少 {expected}"
            );
            assert!(
                !text
                    .iter()
                    .any(|(s, _)| s.contains("以下为本次计算或查表得到的结果")),
                "{id} 仍在重复结果表"
            );
        }
    }

    #[test]
    fn spline_process_shows_one_representative_sample() {
        let directory = tempfile::tempdir().unwrap();
        let id = "bspline_interpolation";
        let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
        let mut app = app_at(directory.path().join("spline-process.json"));
        let ctx = context();
        let size = [1220.0, 10000.0];
        let output = frame(&mut app, &ctx, id, size, vec![]);
        let point = text_shapes(&output)
            .iter()
            .find(|(text, _)| text == "计算过程")
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
        for _ in 0..12 {
            frame(&mut app, &ctx, id, size, vec![]);
        }
        let texts = text_shapes(&frame(&mut app, &ctx, id, size, vec![]));
        assert_eq!(
            texts
                .iter()
                .filter(|(s, _)| s.starts_with("示例：预览采样点"))
                .count(),
            1
        );
        assert!(texts.iter().any(|(s, _)| s.contains("左端外推")));
        assert!(
            texts
                .iter()
                .any(|(s, _)| s.contains("sₓ = x末 − x₀ = 2650 − 250 = 2400"))
        );
        assert!(
            texts
                .iter()
                .any(|(s, _)| s.contains("Q8：|48340 − 2002| = 46338，所以 sᵧ = 46338"))
        );
        assert!(!texts.iter().any(|(s, _)| s.contains("示例过程无法生成")));

        app.set_value(tool, "x_min", "250".into());
        app.set_value(tool, "x_max", "260".into());
        let texts = text_shapes(&frame(&mut app, &ctx, id, size, vec![]));
        assert!(
            texts
                .iter()
                .any(|(s, _)| s.contains("示例：预览采样点 2，x = 250.5")),
            "{texts:?}"
        );
        assert!(
            texts.iter().any(|(s, _)| s.contains("Q0～Q1 插值")),
            "{texts:?}"
        );
        assert_eq!(
            texts
                .iter()
                .filter(|(s, _)| s.starts_with("示例：预览采样点"))
                .count(),
            1
        );
    }

    #[test]
    fn unit_process_explains_linear_and_temperature_conversions() {
        let directory = tempfile::tempdir().unwrap();
        let size = [1220.0, 8000.0];
        for (id, value, expected) in [
            ("unit_temperature", "100", ["目标值 = ", "1", " = 373.15 K"]),
            ("unit_pressure", "101325", ["1", "101325", ") = 1 atm"]),
        ] {
            let mut app = app_at(directory.path().join(format!("{id}.json")));
            let ctx = context();
            let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
            frame(&mut app, &ctx, id, size, vec![]);
            app.set_value(tool, "value", value.into());
            let out = frame(&mut app, &ctx, id, size, vec![]);
            let shapes = text_shapes(&out);
            let heading = shapes
                .iter()
                .find(|(text, _)| text.starts_with("换算结果 · 全部"))
                .expect("应显示换算结果标题")
                .1;
            let process_header = shapes
                .iter()
                .find(|(text, _)| text == "计算过程")
                .unwrap()
                .1;
            let note_header = shapes
                .iter()
                .find(|(text, _)| text == "计算说明")
                .unwrap()
                .1;
            assert!(process_header.top() > heading.bottom());
            assert!(
                process_header.bottom() < note_header.top(),
                "计算过程应位于计算说明之前"
            );
            let point = process_header.center();
            for pressed in [true, false] {
                frame(
                    &mut app,
                    &ctx,
                    id,
                    size,
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
            for _ in 0..12 {
                frame(&mut app, &ctx, id, size, vec![]);
            }
            let texts = text_shapes(&frame(&mut app, &ctx, id, size, vec![]));
            for part in expected {
                assert!(
                    texts.iter().any(|(text, _)| text.contains(part)),
                    "{id}: {part}: {texts:?}"
                );
            }
            if id == "unit_temperature" {
                let first = texts
                    .iter()
                    .find(|(text, _)| text.starts_with("摄氏度：目标系数"))
                    .unwrap()
                    .1;
                let second = texts
                    .iter()
                    .find(|(text, _)| text.starts_with("开尔文：目标系数"))
                    .unwrap()
                    .1;
                assert!(
                    second.top() - first.bottom() >= 15.0,
                    "不同单位的计算之间应保留空行"
                );
            }
            app.set_value(tool, "value", "200".into());
            let updated = text_shapes(&frame(&mut app, &ctx, id, size, vec![]));
            assert!(
                updated
                    .iter()
                    .any(|(text, _)| text.starts_with("输入 200 "))
            );
            app.set_value(tool, "value", "无效".into());
            let invalid = text_shapes(&frame(&mut app, &ctx, id, size, vec![]));
            assert!(!invalid.iter().any(|(text, _)| text == "计算过程"));
        }
    }

    #[test]
    fn humidity_process_follows_input_modes_and_hides_invalid_results() {
        let directory = tempfile::tempdir().unwrap();
        let id = "temperature_humidity_conversion";
        let mut app = app_at(directory.path().join("humidity.json"));
        let ctx = context();
        let size = [1220.0, 10000.0];
        let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
        let initial = frame(&mut app, &ctx, id, size, vec![]);
        let point = text_shapes(&initial)
            .iter()
            .find(|(text, _)| text == "计算过程")
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
        for (mode, expected) in [
            ("0", "输入相对湿度"),
            ("1", "输入露点的饱和水汽压"),
            ("2", "输入湿球温度的饱和水汽压"),
            ("3", "输入混合比"),
        ] {
            app.set_value(tool, "known", mode.into());
            for _ in 0..12 {
                frame(&mut app, &ctx, id, size, vec![]);
            }
            let rendered = text_shapes(&frame(&mut app, &ctx, id, size, vec![]));
            assert!(
                rendered.iter().any(|(text, _)| text.contains(expected)),
                "{mode}: {rendered:?}"
            );
            assert!(
                rendered
                    .iter()
                    .any(|(text, _)| text.contains("干空气基准体积比"))
            );
        }
        app.set_value(tool, "pressure", "0".into());
        let invalid = text_shapes(&frame(&mut app, &ctx, id, size, vec![]));
        assert!(!invalid.iter().any(|(text, _)| text == "计算过程"));
    }

    #[test]
    fn radiation_process_tracks_models_responses_and_invalid_inputs() {
        let directory = tempfile::tempdir().unwrap();
        let size = [1220.0, 10000.0];
        for id in [
            "ear_thermometer_blackbody",
            "blackbody_radiation_source",
            "radiation_thermometer",
        ] {
            let mut app = app_at(directory.path().join(format!("{id}.json")));
            let ctx = context();
            let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
            let initial = frame(&mut app, &ctx, id, size, vec![]);
            let point = text_shapes(&initial)
                .iter()
                .find(|(text, _)| text == "计算过程")
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
            let show = |app: &mut ToolboxApp| {
                for _ in 0..12 {
                    frame(app, &ctx, id, size, vec![]);
                }
                text_shapes(&frame(app, &ctx, id, size, vec![]))
                    .into_iter()
                    .map(|(text, _)| text)
                    .collect::<Vec<_>>()
            };
            if id == "ear_thermometer_blackbody" {
                assert!(show(&mut app).iter().any(|s| s.contains("多次反射平衡")));
                app.set_value(tool, "ear.model", "1".into());
                assert!(show(&mut app).iter().any(|s| s.contains("一次反射近似")));
            } else {
                assert!(show(&mut app).iter().any(|s| s.contains("采用有效波长法")));
                app.set_value(tool, "model", "1".into());
                for (response, expected) in [
                    ("0", "单一波长"),
                    ("1", "8～14 μm 平坦波段"),
                    ("2", "7～18 μm 平坦波段"),
                ] {
                    app.set_value(tool, "spectral_response", response.into());
                    let text = show(&mut app);
                    assert!(text.iter().any(|s| s.contains(expected)), "{id} {response}");
                    assert!(text.iter().any(|s| s.contains("目标 ")));
                }
            }
            app.set_value(tool, "emissivity", "0".into());
            assert!(!show(&mut app).iter().any(|text| text == "计算过程"));
        }
    }

    #[test]
    fn temperature_tools_show_current_process_and_hide_invalid_results() {
        let directory = tempfile::tempdir().unwrap();
        let size = [1220.0, 10000.0];
        for id in [
            "working_thermocouple",
            "industrial_thermal_resistance",
            "standard_thermocouple",
            "standard_platinum_resistor_sprt",
            "thermistor",
        ] {
            let mut app = app_at(directory.path().join(format!("{id}.json")));
            let ctx = context();
            let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
            let show = |app: &mut ToolboxApp| {
                for _ in 0..12 {
                    frame(app, &ctx, id, size, vec![]);
                }
                text_shapes(&frame(app, &ctx, id, size, vec![]))
                    .into_iter()
                    .map(|(text, _)| text)
                    .collect::<Vec<_>>()
            };
            let open = |app: &mut ToolboxApp| {
                let out = frame(app, &ctx, id, size, vec![]);
                let texts = text_shapes(&out);
                let point = texts
                    .iter()
                    .find(|(text, _)| text == "计算过程")
                    .unwrap_or_else(|| panic!("{id}: {texts:?}"))
                    .1
                    .center();
                for pressed in [true, false] {
                    frame(
                        app,
                        &ctx,
                        id,
                        size,
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
            };
            open(&mut app);
            match id {
                "working_thermocouple" => {
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("测得热电势 = E(热端)"))
                    );
                    let steps = show(&mut app);
                    assert!(steps.iter().any(|s| s.contains("实际使用分段 0～1372 °C")));
                    assert!(steps.iter().any(|s| s.contains("c[0] = -0.017600413686")));
                    // 指数表达式由排版引擎绘制，不再属于文本图形。
                    assert!(steps.iter().any(|s| s.contains("指数项 =")));
                    app.set_value(tool, "mode", "1".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("补偿后热电势 =")));
                    app.set_value(tool, "mode", "0".into());
                    app.set_value(tool, "tc.unit", "2".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("212 °F")));
                    app.set_value(tool, "tc.unit", "0".into());
                    app.set_value(tool, "cold", "9999".into());
                }
                "industrial_thermal_resistance" => {
                    let initial = show(&mut app);
                    assert!(initial.iter().any(|s| s.contains("CVD 系数 A =")));
                    assert!(
                        initial.iter().any(|s| s.contains("电阻 = 138.5055 Ω")),
                        "{initial:?}"
                    );
                    assert!(
                        !initial.iter().any(|s| s.contains("138.50549999999998")),
                        "{initial:?}"
                    );
                    app.set_value(tool, "rtd.direction", "1".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("有界二分法")));
                    app.set_value(tool, "rtd.direction", "0".into());
                    app.set_value(tool, "rtd.unit", "1".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("212 °F")));
                    app.set_value(tool, "rtd.unit", "0".into());
                    app.set_value(tool, "rtd.graduation", "3".into());
                    app.set_value(tool, "rtd.direction", "0".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("分段参考曲线")));
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("c[1] = 0.003974973"))
                    );
                    app.set_value(tool, "rtd.pt100_3916.r0", "0".into());
                }
                "standard_thermocouple" => {
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("固定点实测热电势"))
                    );
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("固定点 1 的插值项"))
                    );
                    app.set_value(tool, "stc.cold.0", "100".into());
                    let cold_steps = show(&mut app);
                    assert!(
                        cold_steps
                            .iter()
                            .any(|s| s.contains("参考端固定点修正") && s.contains("100 °C"))
                    );
                    assert!(cold_steps.iter().any(|s| s.contains("参考端参考电势")
                        && s.contains("固定点修正")
                        && s.contains("模型电势")));
                    app.set_value(tool, "stc.S.0.0.t.0", "200".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("S 型低于 400 °C")));
                    app.set_value(tool, "stc.direction", "1".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("补偿后电势 =")));
                    app.set_value(tool, "stc.calibration", "1".into());
                    app.set_value(tool, "stc.direction", "0".into());
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("使用输入的温区系数"))
                    );
                    app.set_value(tool, "stc.coefficient_source", "1".into());
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("温区系数由校准点拟合"))
                    );
                    app.set_value(tool, "stc.calibration", "0".into());
                    app.set_value(tool, "stc.type", "1".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("B 型标准热电偶")));
                    app.set_value(tool, "stc.B.0.0.t.0", "1150".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("过渡比例 = 0.5")));
                    app.set_value(tool, "stc.cold.0", "9999".into());
                }
                "standard_platinum_resistor_sprt" => {
                    assert!(show(&mut app).iter().any(|s| s.contains("实际电阻比 W =")));
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("高温参考函数：x ="))
                    );
                    app.set_value(tool, "sprt.direction", "1".into());
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("偏差修正后的参考电阻比"))
                    );
                    app.set_value(tool, "sprt.hr", "6".into());
                    app.set_value(tool, "sprt.direction", "0".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("低温偏差")));
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("低温参考函数：x ="))
                    );
                    app.set_value(tool, "sprt.hr", "1".into());
                    app.set_value(tool, "sprt.source", "1".into());
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("使用固定点拟合系数"))
                    );
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("固定点拟合：每点计算"))
                    );
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("固定点高温参考函数"))
                    );
                    app.set_value(tool, "sprt.model", "1".into());
                    open(&mut app);
                    let displayed = show(&mut app);
                    assert!(displayed.iter().any(|s| s.contains("对每个校准点建立")));
                    assert!(displayed.iter().any(|s| s.contains(" − 1 = ")));
                    let rendered = text_shapes(&frame(&mut app, &ctx, id, size, vec![]));
                    let points: Vec<_> = rendered
                        .iter()
                        .filter(|(text, _)| text.starts_with("校准点：t ="))
                        .map(|(_, rect)| *rect)
                        .collect();
                    assert_eq!(points.len(), 3);
                    assert!(
                        points
                            .windows(2)
                            .all(|pair| pair[1].top() - pair[0].bottom() >= 15.0),
                        "不同校准点之间应保留空行"
                    );
                    app.set_value(tool, "r0", "0".into());
                }
                "thermistor" => {
                    assert!(show(&mut app).iter().any(|s| s.contains("电阻 → 温度：")));
                    let shifted_b0 = (20000.0_f64.ln() - 3950.0 / 298.15).to_string();
                    app.set_value(tool, "ntc.b.3.false.0", shifted_b0);
                    let details = show(&mut app);
                    assert!(details.iter().any(|s| s.contains("变化率使用 B[0]")));
                    let b_resistance = details
                        .iter()
                        .find_map(|s| {
                            s.strip_prefix("B 曲线在当前温度的电阻 = ")?
                                .strip_suffix(" Ω。")?
                                .parse::<f64>()
                                .ok()
                        })
                        .expect("应展示 B 曲线在当前温度的电阻");
                    assert!((b_resistance - 20000.0).abs() < 1e-6);
                    app.set_value(tool, "mode", "1".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("温度 → 电阻：")));
                    app.set_value(tool, "ntc.temperature.0", "-20".into());
                    let boundary = show(&mut app);
                    assert!(
                        boundary
                            .iter()
                            .any(|s| s.contains("B 曲线在校准温区内取差分点"))
                    );
                    app.set_value(tool, "ntc.b.3.false.0", "1".into());
                    app.set_value(tool, "ntc.b.3.false.1", "0".into());
                    app.set_value(tool, "ntc.range.min", "-273".into());
                    app.set_value(tool, "ntc.range.max", "-270".into());
                    app.set_value(tool, "ntc.temperature.0", "-273".into());
                    let cold = show(&mut app);
                    assert!(cold.iter().any(|s| s.contains("2.71828183")), "{cold:?}");
                    assert!(cold.iter().any(|s| s.contains("在校准温区内取差分点")));
                    app.set_value(tool, "ntc.range.min", "-20".into());
                    app.set_value(tool, "ntc.range.max", "100".into());
                    app.set_value(tool, "ntc.temperature.0", "25".into());
                    app.set_value(tool, "ntc.source", "1".into());
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("校准点最小二乘拟合系数"))
                    );
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("拟合 A 时使用 x="))
                    );
                    app.set_value(tool, "ntc.model", "1".into());
                    open(&mut app);
                    assert!(show(&mut app).iter().any(|s| s.contains("β = 3950 K")));
                    app.set_value(tool, "r0", "0".into());
                }
                _ => unreachable!(),
            }
            assert!(
                !show(&mut app).iter().any(|s| s == "计算过程"),
                "{id} 无效输入不应显示旧过程"
            );
        }
    }

    #[test]
    fn thermocouple_reference_details_wrap_in_narrow_windows() {
        let directory = tempfile::tempdir().unwrap();
        let id = "working_thermocouple";
        for width in [370.0, 620.0] {
            let mut app = app_at(directory.path().join(format!("{width}.json")));
            let ctx = context();
            let size = [width, 3000.0];
            let first = frame(&mut app, &ctx, id, size, vec![]);
            let point = text_shapes(&first)
                .iter()
                .find(|(text, _)| text == "计算过程")
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
            for _ in 0..15 {
                frame(&mut app, &ctx, id, size, vec![]);
            }
            let texts = text_shapes(&frame(&mut app, &ctx, id, size, vec![]));
            let coefficients: Vec<_> = texts
                .iter()
                .filter(|(text, _)| text.starts_with("c["))
                .collect();
            assert!(!coefficients.is_empty(), "应展示参考函数系数");
            for (_, rect) in coefficients {
                assert!(
                    rect.left() >= 0.0 && rect.right() <= width,
                    "{width} 像素窗口内系数越界：{rect:?}"
                );
            }
        }
    }

    #[test]
    fn uncertainty_total_hides_float_tail_without_precision_toggle() {
        let directory = tempfile::tempdir().unwrap();
        let id = "uncertainty_calculation";
        let mut app = app_at(directory.path().join("uncertainty.json"));
        let ctx = context();
        let size = [1220.0, 10000.0];
        let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
        frame(&mut app, &ctx, id, size, vec![]);
        for (key, value) in [
            ("budget.components.count", "1"),
            ("budget.component.0.type", "1"),
            ("budget.component.0.distribution", "4"),
            ("budget.component.0.half_width", "0.1"),
            ("budget.component.0.sensitivity", "3"),
        ] {
            app.set_value(tool, key, value.into());
        }
        let click = |app: &mut ToolboxApp, label: &str| {
            let output = frame(app, &ctx, id, size, vec![]);
            let point = text_shapes(&output)
                .iter()
                .find(|(text, _)| text == label)
                .unwrap()
                .1
                .center();
            for pressed in [true, false] {
                frame(
                    app,
                    &ctx,
                    id,
                    size,
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
            for _ in 0..12 {
                frame(app, &ctx, id, size, vec![]);
            }
        };
        click(&mut app, "计算过程");
        let output = frame(&mut app, &ctx, id, size, vec![]);
        let text = text_shapes(&output);
        assert!(
            text.iter().any(|(s, _)| s.contains("u_c = 0.3。")),
            "{text:?}"
        );
        assert!(
            !text.iter().any(|(s, _)| s.contains("0.30000000000000004")),
            "{text:?}"
        );
        assert!(!text.iter().any(|(s, _)| s == "显示完整双精度"));
    }

    #[test]
    fn error_tools_show_calculation_process_for_each_mode() {
        let directory = tempfile::tempdir().unwrap();
        let size = [1220.0, 10000.0];
        for id in [
            "repeatability_calculation",
            "range_method_calculation",
            "resultant_error",
            "error_analysis",
            "uncertainty_calculation",
        ] {
            let mut app = app_at(directory.path().join(format!("{id}.json")));
            let ctx = context();
            let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
            let show = |app: &mut ToolboxApp| {
                for _ in 0..12 {
                    frame(app, &ctx, id, size, vec![]);
                }
                text_shapes(&frame(app, &ctx, id, size, vec![]))
                    .into_iter()
                    .map(|(text, _)| text)
                    .collect::<Vec<_>>()
            };
            let open = |app: &mut ToolboxApp| {
                let output = frame(app, &ctx, id, size, vec![]);
                let point = text_shapes(&output)
                    .iter()
                    .find(|(text, _)| text == "计算过程")
                    .unwrap_or_else(|| panic!("{id}: {:?}", text_shapes(&output)))
                    .1
                    .center();
                for pressed in [true, false] {
                    frame(
                        app,
                        &ctx,
                        id,
                        size,
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
            };
            open(&mut app);
            match id {
                "repeatability_calculation" => {
                    let text = show(&mut app);
                    assert!(text.iter().any(|s| s.contains("测量点 1：")));
                    assert!(text.iter().any(|s| s.contains("样本标准差 s =")));
                    assert!(text.iter().any(|s| s.contains("10.03 − 10.01 = 0.02")));
                    assert!(!text.iter().any(|s| s.contains("0.019999999999999574")));
                    app.set_value(tool, "samples.value.0", "无效".into());
                }
                "range_method_calculation" => {
                    assert!(show(&mut app).iter().any(|s| s.contains("极差 R =")));
                    app.set_value(tool, "range.input_method", "1".into());
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("输入最大值、最小值"))
                    );
                    app.set_value(tool, "range.max", "0".into());
                }
                "resultant_error" => {
                    assert!(show(&mut app).iter().any(|s| s.contains("分量 1：贡献")));
                    app.set_value(tool, "components.error.0", "-0.02".into());
                    app.set_value(tool, "components.sensitivity.0", "2".into());
                    assert!(
                        show(&mut app)
                            .iter()
                            .any(|s| s.contains("2 × -0.02 = -0.04"))
                    );
                    app.set_value(tool, "components.mode", "1".into());
                    open(&mut app);
                    assert!(show(&mut app).iter().any(|s| s.contains("统一单位：")));
                    app.set_value(tool, "pressure_error.0.high", "0".into());
                }
                "error_analysis" => {
                    let text = show(&mut app);
                    assert!(text.iter().any(|s| s.contains("100.02 − 100 = 0.02")));
                    assert!(
                        text.iter().any(|s| s.contains("误差绝对值 |Δ| = 0.02。")),
                        "{text:?}"
                    );
                    assert!(
                        !text.iter().any(|s| s.contains("0.01999999999999602")),
                        "{text:?}"
                    );
                    app.set_value(tool, "true", "0".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("相对误差无定义")));
                    app.set_value(tool, "measured", "无效".into());
                }
                "uncertainty_calculation" => {
                    assert!(show(&mut app).iter().any(|s| s.contains("B 类均匀分布")));
                    app.set_value(tool, "budget.component.0.type", "0".into());
                    let text = show(&mut app);
                    assert!(text.iter().any(|s| s.contains("贝塞尔法：取测量次数")));
                    assert!(
                        text.iter().any(|s| s.contains("0 − (-0.002) = 0.002")),
                        "{text:?}"
                    );
                    assert!(
                        text.iter().any(|s| s.contains("-0.01 − (-0.002) = -0.008")),
                        "{text:?}"
                    );
                    assert!(
                        !text.iter().any(|s| s.contains("10.017999999999999")),
                        "{text:?}"
                    );
                    let shapes = text_shapes(&frame(&mut app, &ctx, id, size, vec![]));
                    let first = shapes
                        .iter()
                        .find(|(s, _)| s.starts_with("测量点 1："))
                        .unwrap()
                        .1;
                    let second = shapes
                        .iter()
                        .find(|(s, _)| s.starts_with("测量点 2："))
                        .unwrap()
                        .1;
                    assert!(
                        second.top() - first.bottom() < 12.0,
                        "不确定度测量点之间不应有空行"
                    );
                    app.set_value(tool, "budget.component.0.a_method", "1".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("极差法：R =")));
                    app.set_value(tool, "budget.component.0.type", "2".into());
                    assert!(show(&mut app).iter().any(|s| s.contains("合成分量：")));
                    app.set_value(tool, "budget.mode", "1".into());
                    open(&mut app);
                    let text = show(&mut app);
                    assert!(text.iter().any(|s| s.contains("A 类分量：")));
                    assert!(text.iter().any(|s| s.contains("B 类合成 uB")));
                    let shapes = text_shapes(&frame(&mut app, &ctx, id, size, vec![]));
                    let first = shapes
                        .iter()
                        .find(|(s, _)| s.starts_with("测量点 1："))
                        .unwrap()
                        .1;
                    let second = shapes
                        .iter()
                        .find(|(s, _)| s.starts_with("测量点 2："))
                        .unwrap()
                        .1;
                    assert!(
                        second.top() - first.bottom() < 12.0,
                        "快捷不确定度测量点之间不应有空行"
                    );
                    app.set_value(tool, "a_samples.count", "3".into());
                    app.set_value(tool, "a_average_count", "4".into());
                    for (i, value) in ["1", "2", "3"].iter().enumerate() {
                        app.set_value(tool, &format!("a_samples.value.{i}"), (*value).into());
                    }
                    let lines = show(&mut app);
                    assert!(lines.iter().any(|s| s.contains("uA = ")));
                    assert!(lines.iter().any(|s| s.contains(" = 0.5。")));
                    app.set_value(tool, "coverage", "0".into());
                }
                _ => unreachable!(),
            }
            assert!(
                !show(&mut app).iter().any(|s| s == "计算过程"),
                "{id} 无效输入不应保留旧过程"
            );
        }
    }

    #[test]
    fn transmitter_process_follows_both_directions_and_transfer_functions() {
        let directory = tempfile::tempdir().unwrap();
        let tool = TOOLS.iter().find(|tool| tool.id == "transmitter").unwrap();
        let size = [1220.0, 8000.0];
        for (direction, function, expected) in [
            (0, 0, "输出信号 = 下限"),
            (0, 1, "信号比例 q = √r"),
            (1, 0, "被测量比例 r = q"),
            (1, 1, "被测量比例 r = q²"),
        ] {
            let mut app = app_at(
                directory
                    .path()
                    .join(format!("{direction}-{function}.json")),
            );
            let ctx = context();
            frame(&mut app, &ctx, tool.id, size, vec![]);
            app.set_value(tool, "direction", direction.to_string());
            app.set_value(tool, "function", function.to_string());
            let output = frame(&mut app, &ctx, tool.id, size, vec![]);
            let point = text_shapes(&output)
                .iter()
                .find(|(text, _)| text == "计算过程")
                .unwrap()
                .1
                .center();
            for pressed in [true, false] {
                frame(
                    &mut app,
                    &ctx,
                    tool.id,
                    size,
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
            for _ in 0..12 {
                frame(&mut app, &ctx, tool.id, size, vec![]);
            }
            let text: Vec<_> = text_shapes(&frame(&mut app, &ctx, tool.id, size, vec![]))
                .into_iter()
                .map(|(text, _)| text)
                .collect();
            assert!(
                text.iter().any(|line| line.contains(expected)),
                "{direction}/{function}: {text:?}"
            );
            if direction == 1 && function == 1 {
                assert!(
                    text.iter().any(|line| line.contains("0.5² = 0.25")),
                    "{text:?}"
                );
                assert!(text.iter().any(|line| line.contains("= 25 Pa")), "{text:?}");
            }
            app.set_value(
                tool,
                if direction == 0 {
                    "pressure.0.process_input"
                } else {
                    "signal_input"
                },
                "无效".into(),
            );
            let text = text_shapes(&frame(&mut app, &ctx, tool.id, size, vec![]));
            assert!(!text.iter().any(|(text, _)| text == "计算过程"));
        }
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
            assert!(
                default
                    .iter()
                    .any(|(text, _)| text.contains("留空默认 1 次"))
            );
            assert!(result_value(&default, "1"));
            set(&mut app, field, "4");
            assert!(result_value(&shapes(&mut app), "0.5"));
            set(&mut app, field, "0");
            assert!(
                shapes(&mut app)
                    .iter()
                    .any(|(text, _)| text.contains("取测量次数 n 须为正整数"))
            );
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
            assert!(
                default
                    .iter()
                    .any(|(text, _)| text.contains("留空保留原始计算精度"))
            );
            assert!(result_value(&default, "2.49"));
            if !quick {
                assert_eq!(app.value(tool, "budget.rounding_v2", ""), "1");
            }
            set(&mut app, &format!("{prefix}.digits"), "3");
            assert!(result_value(&shapes(&mut app), "2.5"));
            set(&mut app, &format!("{prefix}.rounding"), "0");
            assert!(result_value(&shapes(&mut app), "2.48"));
            set(&mut app, &format!("{prefix}.digits"), "18");
            assert!(
                shapes(&mut app)
                    .iter()
                    .any(|(text, _)| text.contains("有效数字须为 1～17"))
            );
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
        assert!(
            cards
                .iter()
                .all(|rect| (rect.left() - cards[0].left()).abs() < 1.0)
        );
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
        assert!(
            cards
                .iter()
                .all(|r| (r.left() - cards[0].left()).abs() < 1.0)
        );
        assert!(
            cards
                .windows(2)
                .all(|pair| pair[1].top() >= pair[0].bottom())
        );
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
    fn dropdown_items_fill_popup_and_are_clickable_at_the_right_edge() {
        fn collect_rectangles(shape: &egui::Shape, result: &mut Vec<egui::Rect>) {
            match shape {
                egui::Shape::Rect(rect) => result.push(rect.rect),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect_rectangles(shape, result);
                    }
                }
                _ => {}
            }
        }
        let directory = tempfile::tempdir().unwrap();
        for (id, selected_label, other_label, key, expected) in [
            (
                "gauge_unified",
                "塞规（内螺纹）",
                "环规（外螺纹）",
                "unified.side",
                "环规（外螺纹）",
            ),
            (
                "gauge_metric",
                "塞规（内螺纹）",
                "环规（外螺纹）",
                "thread.side",
                "环规（外螺纹）",
            ),
            (
                "gauge_pipe",
                "R · 55° 密封锥螺纹",
                "NPT · 60° 密封锥螺纹",
                "system",
                "NPT · 60° 密封锥螺纹",
            ),
            ("gauge_smooth", "孔用塞规", "轴用环规", "mode", "轴用环规"),
            (
                "blackbody_radiation_source",
                "有效波长法",
                "Planck 辐射平衡",
                "model",
                "1",
            ),
        ] {
            for width in [370.0, 620.0, 1220.0] {
                let mut app = app_at(directory.path().join(format!("{id}-menu-{width}.json")));
                let ctx = context();
                let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
                let size = [width, 2000.0];
                let output = frame(&mut app, &ctx, id, size, vec![]);
                let button = text_shapes(&output)
                    .iter()
                    .find(|(text, _)| text == selected_label)
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
                            egui::Event::PointerMoved(button),
                            egui::Event::PointerButton {
                                pos: button,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: Default::default(),
                            },
                        ],
                    );
                }
                let output = frame(&mut app, &ctx, id, size, vec![]);
                let option = text_shapes(&output)
                    .iter()
                    .find(|(text, _)| text == other_label)
                    .expect("下拉菜单应展开")
                    .1;
                let selected = text_shapes(&output)
                    .iter()
                    .filter(|(text, rect)| text == selected_label && rect.top() < option.top())
                    .max_by(|a, b| a.1.top().total_cmp(&b.1.top()))
                    .unwrap()
                    .1;
                let mut rects = Vec::new();
                for shape in &output.shapes {
                    collect_rectangles(&shape.shape, &mut rects);
                }
                rects.retain(|rect| rect.contains_rect(selected));
                let popup = rects
                    .iter()
                    .filter(|r| {
                        r.contains_rect(option) && r.top() <= selected.top() && r.height() < 360.0
                    })
                    .min_by(|a, b| a.area().total_cmp(&b.area()))
                    .expect("应有包围选项的菜单背景");
                let highlight = rects
                    .iter()
                    .filter(|r| {
                        r.height() < 50.0
                            && r.top() <= selected.top()
                            && r.bottom() >= selected.bottom()
                    })
                    .min_by(|a, b| a.area().total_cmp(&b.area()))
                    .expect("应有选中项背景");
                assert!(
                    highlight.width() >= popup.width() - 20.0,
                    "选中项应铺满菜单：popup={popup:?}, highlight={highlight:?}"
                );
                let point = egui::pos2(highlight.right() - 4.0, option.center().y);
                for pressed in [true, false] {
                    frame(
                        &mut app,
                        &ctx,
                        id,
                        size,
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
                assert_eq!(app.value(tool, key, ""), expected, "{id} {width}");
            }
        }
    }

    #[test]
    fn unified_specification_menu_fits_narrow_windows_and_keeps_three_columns() {
        let directory = tempfile::tempdir().unwrap();
        for width in [370.0, 620.0, 1220.0] {
            let mut app = app_at(directory.path().join(format!("spec-menu-{width}.json")));
            let ctx = context();
            let id = "gauge_unified";
            let size = [width, 2000.0];
            let output = frame(&mut app, &ctx, id, size, vec![]);
            let button = text_shapes(&output)
                .iter()
                .find(|(text, _)| text == "常用规格")
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
                        egui::Event::PointerMoved(button),
                        egui::Event::PointerButton {
                            pos: button,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ],
                );
            }
            let output = frame(&mut app, &ctx, id, size, vec![]);
            let rows: Vec<_> = text_shapes(&output)
                .into_iter()
                .filter(|(text, _)| text.contains("inch") && text.contains("mm"))
                .collect();
            assert!(!rows.is_empty(), "规格列表应显示英寸和毫米两列");
            for (text, rect) in &rows {
                assert!(
                    rect.left() >= 0.0 && rect.right() <= width,
                    "{width}: {text} {rect:?}"
                );
            }
            let point = rows[0].1.center();
            for pressed in [true, false] {
                frame(
                    &mut app,
                    &ctx,
                    id,
                    size,
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
            let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
            assert_eq!(app.value(tool, "size", ""), "No.0");
        }
    }

    #[test]
    fn archive_dropdown_keeps_long_names_visible_within_the_window() {
        let directory = tempfile::tempdir().unwrap();
        let long_name = format!("标准热电偶{}", "长".repeat(80));
        let required = [
            "stc.type",
            "stc.calibration",
            "stc.S.cal.0",
            "stc.S.cal.1",
            "stc.S.cal.2",
        ];
        let fields = json!({"stc.type":"0", "stc.calibration":"0", "stc.S.cal.0":"1", "stc.S.cal.1":"2", "stc.S.cal.2":"3"});
        std::fs::write(directory.path().join("calibrations-v1.json"), json!({"version":1,"archives":[{"tool":"standard_thermocouple","name":long_name,"fields":fields,"required":required}]}).to_string()).unwrap();
        for width in [370.0, 620.0, 1220.0] {
            let mut app = app_at(directory.path().join(format!("archive-{width}.json")));
            let ctx = context();
            let id = "standard_thermocouple";
            let size = [width, 2000.0];
            let output = frame(&mut app, &ctx, id, size, vec![]);
            let button = text_shapes(&output)
                .iter()
                .find(|(text, _)| text == "选择已保存的档案")
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
                        egui::Event::PointerMoved(button),
                        egui::Event::PointerButton {
                            pos: button,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ],
                );
            }
            let output = frame(&mut app, &ctx, id, size, vec![]);
            let text = text_shapes(&output)
                .into_iter()
                .find(|(text, _)| text == &long_name)
                .expect("应显示档案名称");
            assert!(
                text.1.left() >= 0.0 && text.1.right() <= width,
                "{width} 下档案名称超出窗口：{:?}",
                text.1
            );
            let point = text.1.center();
            for pressed in [true, false] {
                frame(
                    &mut app,
                    &ctx,
                    id,
                    size,
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
            let tool = TOOLS.iter().find(|tool| tool.id == id).unwrap();
            assert_eq!(
                app.value(tool, "stc.S.cal.2", ""),
                "3",
                "长名称档案应能正常选择和加载"
            );
        }
    }

    #[test]
    fn general_dropdown_wraps_long_options_and_selects_them() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = app_at(directory.path().join("long-options.json"));
        let ctx = context();
        let tool = TOOLS
            .iter()
            .find(|tool| tool.id == "error_analysis")
            .unwrap();
        let long = format!("长选项{}", "测".repeat(80));
        let options = ["短选项", long.as_str()];
        let size = [370.0, 1000.0];
        let draw = |app: &mut ToolboxApp, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size.into())),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        app.select_index(ui, tool, "test_long", "选项", &options, 0);
                    });
                },
            )
        };
        let output = draw(&mut app, vec![]);
        let button = text_shapes(&output)
            .iter()
            .find(|(text, _)| text == "短选项")
            .unwrap()
            .1
            .center();
        for pressed in [true, false] {
            draw(
                &mut app,
                vec![
                    egui::Event::PointerMoved(button),
                    egui::Event::PointerButton {
                        pos: button,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        let output = draw(&mut app, vec![]);
        let option = text_shapes(&output)
            .into_iter()
            .find(|(text, _)| text == &long)
            .expect("应显示长选项")
            .1;
        assert!(
            option.left() >= 0.0 && option.right() <= size[0],
            "长选项超出窗口：{option:?}"
        );
        let point = option.center();
        for pressed in [true, false] {
            draw(
                &mut app,
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
        assert_eq!(app.value(tool, "test_long", ""), "1");
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
        assert!(
            !app.history
                .values
                .keys()
                .any(|key| key.starts_with(unit.id))
        );
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
