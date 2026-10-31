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
        let history = InputHistory::load(Some(path));
        app.values = history.values.clone();
        app.history = history;
        app
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
    fn all_49_default_pages_calculate_without_errors() {
        let directory = tempfile::tempdir().unwrap();
        let ctx = context();
        assert_eq!(TOOLS.len(), 49);
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
                .filter(|(text, _)| text.starts_with("⚠"))
                .collect();
            assert!(errors.is_empty(), "{}: {errors:?}", tool.id);
            assert!(
                texts.iter().any(|(text, _)| text == "恢复示例"),
                "{}",
                tool.id
            );
            assert!(app.history.values.keys().all(|key|
                tool.remembers_inputs() && key.starts_with(&format!("{}.", tool.id))), "{}", tool.id);
        }
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
