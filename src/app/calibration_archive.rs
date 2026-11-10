//! 命名证书数据与上次输入分开存储，避免恢复示例影响已保存的档案。
use super::pdf_download::action_button;
use crate::calculators::common::{form_row, parse_number};
use crate::{BLUE, INK, MUTED, RED};
use eframe::egui::{self, RichText, Vec2};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const MAX_BYTES: u64 = 8 * 1024 * 1024;
const COUPLE: &str = "standard_thermocouple";
const RESISTOR: &str = "standard_platinum_resistor_sprt";

fn validate_name(name: &str) -> Result<&str, &'static str> {
    let name = name.trim();
    if name.is_empty() {
        Err("请输入档案名称")
    } else if name.chars().count() > 100 {
        Err("档案名称不能超过 100 个字")
    } else {
        Ok(name)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Archive {
    tool: String,
    name: String,
    fields: BTreeMap<String, String>,
    required: Vec<String>,
}

pub(crate) struct SaveRequest {
    name: String,
    replace: bool,
}

enum ArchiveDialog {
    Save { name: String },
    Overwrite { name: String, return_to_save: bool },
    Delete { name: String },
}

pub(super) struct CalibrationArchives {
    path: Option<PathBuf>,
    entries: Vec<Archive>,
    error: Option<String>,
    selected: HashMap<String, String>,
    dialog: Option<ArchiveDialog>,
    dialog_error: Option<String>,
    focus_name: bool,
    last_tool: Option<String>,
    message: Option<String>,
}

fn allowed(tool: &str, key: &str) -> bool {
    match tool {
        COUPLE => {
            matches!(
                key,
                "stc.type"
                    | "stc.calibration"
                    | "stc.coefficient_source"
                    | "stc.S.zone"
                    | "stc.B.zone"
            ) || key.starts_with("stc.fit.")
                || ((key.starts_with("stc.S.") || key.starts_with("stc.B."))
                    && (key.contains(".cal.") || key.contains(".c.")))
        }
        RESISTOR => {
            matches!(
                key,
                "sprt.model"
                    | "sprt.hr"
                    | "sprt.lr"
                    | "sprt.deviation"
                    | "sprt.source"
                    | "sprt.rtp"
                    | "r0"
                    | "r1"
                    | "r2"
                    | "r3"
                    | "t1"
                    | "t2"
                    | "t3"
            ) || key.starts_with("sprt.coefficient.")
                || key.starts_with("sprt.fixed.")
        }
        _ => false,
    }
}

fn read_archives(path: &Path) -> Result<Vec<Archive>, String> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("读取系数档案失败：{e}")),
    };
    let mut text = String::new();
    file.take(MAX_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|e| format!("读取系数档案失败：{e}"))?;
    if text.len() as u64 > MAX_BYTES {
        return Err("系数档案文件过大，未读取或覆盖".into());
    }
    let root: Value =
        serde_json::from_str(&text).map_err(|_| "系数档案 JSON 格式错误，未覆盖原文件")?;
    if root["version"].as_u64() != Some(1) {
        return Err("系数档案版本不支持，未覆盖原文件".into());
    }
    let items = root["archives"]
        .as_array()
        .ok_or("系数档案格式错误，未覆盖原文件")?;
    let mut entries: Vec<Archive> = Vec::new();
    let mut names = HashSet::new();
    for item in items {
        let tool = item["tool"]
            .as_str()
            .filter(|s| matches!(*s, COUPLE | RESISTOR))
            .ok_or("系数档案工具类型无效")?;
        let name = item["name"]
            .as_str()
            .filter(|s| !s.trim().is_empty() && s.chars().count() <= 100)
            .ok_or("系数档案名称无效")?;
        let raw = item["fields"].as_object().ok_or("系数档案字段无效")?;
        let mut fields = BTreeMap::new();
        for (key, value) in raw {
            if !allowed(tool, key) {
                return Err("系数档案包含不支持的字段，未覆盖原文件".into());
            }
            fields.insert(
                key.clone(),
                value.as_str().ok_or("系数档案字段不是文本")?.to_owned(),
            );
        }
        let required: Vec<String> = item["required"]
            .as_array()
            .ok_or("系数档案缺少必需字段清单")?
            .iter()
            .map(|v| v.as_str().map(str::to_owned).ok_or("系数档案必需字段无效"))
            .collect::<Result<_, _>>()?;
        validate_fields(tool, &fields, &required)?;
        if !names.insert((tool.to_owned(), name.to_owned())) {
            return Err("系数档案存在重复名称，未覆盖原文件".into());
        }
        entries.push(Archive {
            tool: tool.into(),
            name: name.into(),
            fields,
            required,
        });
    }
    Ok(entries)
}

fn validate_fields(
    tool: &str,
    fields: &BTreeMap<String, String>,
    required: &[String],
) -> Result<(), String> {
    let index = |key: &str, count: usize| -> Result<usize, String> {
        let value = fields
            .get(key)
            .ok_or_else(|| format!("系数档案缺少字段 {key}"))?;
        let n = value
            .parse::<usize>()
            .map_err(|_| format!("系数档案的 {key} 无效"))?;
        if n >= count {
            return Err(format!("系数档案的 {key} 超出范围"));
        }
        Ok(n)
    };
    let mut expected = Vec::new();
    if tool == COUPLE {
        let kind = ["S", "B"][index("stc.type", 2)?];
        let mode = index("stc.calibration", 2)?;
        expected.extend(["stc.type".to_owned(), "stc.calibration".to_owned()]);
        if mode == 0 {
            for i in 0..if kind == "S" { 3 } else { 5 } {
                expected.push(format!("stc.{kind}.cal.{i}"));
            }
        } else {
            let zones = crate::calculators::thermocouple::standard::zones(kind);
            let zone_key = format!("stc.{kind}.zone");
            let zone = index(&zone_key, zones.len())?;
            let source = index("stc.coefficient_source", 2)?;
            expected.extend([zone_key, "stc.coefficient_source".to_owned()]);
            if source == 0 {
                for i in 0..zones[zone].2.len() {
                    expected.push(format!("stc.{kind}.{zone}.c.{i}"));
                }
            } else {
                let count = index("stc.fit.points.count", 31)?;
                if count < 2 {
                    return Err("系数档案的校准点不足".into());
                }
                expected.push("stc.fit.points.count".into());
                for i in 0..count {
                    expected.push(format!("stc.fit.{kind}.{zone}.{i}.t"));
                    expected.push(format!("stc.fit.{kind}.{zone}.{i}.e"));
                }
            }
        }
    } else if tool == RESISTOR {
        let model = index("sprt.model", 2)?;
        expected.push("sprt.model".to_owned());
        if model == 1 {
            expected.extend(
                ["r0", "t1", "r1", "t2", "r2", "t3", "r3"]
                    .into_iter()
                    .map(str::to_owned),
            );
        } else {
            let hr = index("sprt.hr", 7)?;
            let lr = index("sprt.lr", 3)?;
            let deviation = index("sprt.deviation", 2)?;
            let source = index("sprt.source", 2)?;
            if hr == 6 && lr == 2 {
                return Err("系数档案不能同时禁用高温和低温分区".into());
            }
            expected.extend(
                [
                    "sprt.hr",
                    "sprt.lr",
                    "sprt.deviation",
                    "sprt.source",
                    "sprt.rtp",
                ]
                .into_iter()
                .map(str::to_owned),
            );
            let low = &crate::calculators::sprt::ranges::LOW_RANGES[lr];
            if source == 0 {
                if hr != 6 {
                    expected.extend(
                        [
                            "sprt.coefficient.a",
                            "sprt.coefficient.b",
                            "sprt.coefficient.c",
                        ]
                        .into_iter()
                        .map(str::to_owned),
                    );
                }
                if let Some(labels) = low.coefficients {
                    expected.extend(
                        labels
                            .into_iter()
                            .map(|label| format!("sprt.coefficient.{label}")),
                    );
                }
                if deviation == 0 && hr == 0 {
                    expected.extend(
                        ["sprt.coefficient.d", "sprt.coefficient.w660"]
                            .into_iter()
                            .map(str::to_owned),
                    );
                }
            } else {
                for t in crate::calculators::sprt::ranges::high_fixed_points(hr)
                    .iter()
                    .chain(low.fixed_points)
                {
                    expected.push(format!("sprt.fixed.{t}"));
                }
                if deviation == 0 && hr == 0 {
                    expected.push("sprt.fixed.961.78".into());
                }
            }
        }
    } else {
        return Err("系数档案工具类型无效".into());
    }
    for key in expected {
        if !required.contains(&key) {
            return Err(format!("系数档案缺少必需字段 {key}"));
        }
    }
    for key in required {
        if !allowed(tool, key) {
            return Err("系数档案包含不支持的字段".into());
        }
        let value = fields
            .get(key)
            .ok_or_else(|| format!("系数档案缺少字段 {key}"))?;
        parse_number(value).map_err(|_| format!("系数档案中的 {key} 不是有效数字"))?;
    }
    Ok(())
}

fn write_archives(path: &Path, entries: &[Archive]) -> Result<(), String> {
    let parent = path.parent().ok_or("系数档案目录无效")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("无法创建系数档案目录：{e}"))?;
    let data: Vec<_> = entries.iter().map(|entry| json!({"tool":entry.tool,"name":entry.name,"fields":entry.fields,"required":entry.required})).collect();
    let mut file =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| format!("无法创建临时档案：{e}"))?;
    serde_json::to_writer_pretty(&mut file, &json!({"version":1,"archives":data}))
        .map_err(|e| format!("无法写入系数档案：{e}"))?;
    if file.as_file().metadata().map_err(|e| e.to_string())?.len() > MAX_BYTES {
        return Err("系数档案超过 8 MB，未覆盖原文件".into());
    }
    file.flush()
        .and_then(|_| file.as_file().sync_all())
        .map_err(|e| format!("无法保存系数档案：{e}"))?;
    file.persist(path)
        .map_err(|e| format!("无法替换系数档案：{}", e.error))?;
    Ok(())
}

impl CalibrationArchives {
    pub(super) fn load(path: Option<PathBuf>) -> Self {
        let loaded = path
            .as_deref()
            .ok_or_else(|| "本地配置目录不可用".to_owned())
            .and_then(read_archives);
        let (entries, error) = match loaded {
            Ok(entries) => (entries, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        Self {
            path,
            entries,
            error,
            selected: HashMap::new(),
            dialog: None,
            dialog_error: None,
            focus_name: false,
            last_tool: None,
            message: None,
        }
    }

    pub(super) fn clear_selection(&mut self, tool: &str) {
        self.selected.remove(tool);
    }

    pub(super) fn show(
        &mut self,
        ui: &mut egui::Ui,
        tool: &str,
        values: &mut HashMap<String, String>,
    ) -> Option<SaveRequest> {
        if self.last_tool.as_deref() != Some(tool) {
            self.last_tool = Some(tool.into());
            self.dialog = None;
            self.dialog_error = None;
            self.message = None;
        }
        let mut chosen = None;
        let selected_name = self.selected.get(tool).cloned();
        let dirty = selected_name
            .as_deref()
            .and_then(|name| {
                self.entries
                    .iter()
                    .find(|entry| entry.tool == tool && entry.name == name)
            })
            .is_some_and(|entry| {
                entry
                    .fields
                    .iter()
                    .any(|(key, value)| values.get(&format!("{tool}.{key}")) != Some(value))
            });
        let selected_text = selected_name.as_deref().unwrap_or("选择已保存的档案");
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(17, 0))
            .show(ui, |ui| {
                ui.spacing_mut().interact_size.y = 36.0;
                form_row(ui, "系数档案", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        egui::ComboBox::from_id_salt(("calibration_archive", tool))
                            .selected_text(selected_text)
                            .width(230.0_f32.min(ui.available_width().max(120.0)))
                            .wrap_mode(egui::TextWrapMode::Truncate)
                            .show_ui(ui, |ui| {
                                if !self.entries.iter().any(|entry| entry.tool == tool) {
                                    ui.label("暂无保存的档案");
                                }
                                for entry in self.entries.iter().filter(|entry| entry.tool == tool)
                                {
                                    if ui
                                        .selectable_label(
                                            selected_name.as_deref() == Some(entry.name.as_str()),
                                            &entry.name,
                                        )
                                        .clicked()
                                    {
                                        chosen = Some(entry.name.clone());
                                    }
                                }
                            });
                        if dirty {
                            ui.label(RichText::new("已修改").color(MUTED));
                        }
                        if ui
                            .add(action_button("保存当前…", true).min_size(Vec2::new(96.0, 36.0)))
                            .clicked()
                        {
                            self.dialog = Some(ArchiveDialog::Save {
                                name: String::new(),
                            });
                            self.dialog_error = None;
                            self.focus_name = true;
                            self.message = None;
                        }
                        if let Some(name) = self.selected.get(tool).cloned() {
                            if ui
                                .add(
                                    action_button("更新选中…", false)
                                        .min_size(Vec2::new(96.0, 36.0)),
                                )
                                .clicked()
                            {
                                self.dialog = Some(ArchiveDialog::Overwrite {
                                    name: name.clone(),
                                    return_to_save: false,
                                });
                                self.dialog_error = None;
                            }
                            if ui
                                .add(action_button("删除…", false).min_size(Vec2::new(96.0, 36.0)))
                                .clicked()
                            {
                                self.dialog = Some(ArchiveDialog::Delete { name });
                                self.dialog_error = None;
                            }
                        }
                    })
                });
            });
        if let Some(name) = chosen {
            self.apply(tool, &name, values);
        }
        if let Some(error) = &self.error {
            ui.label(RichText::new(error).color(RED));
        }
        if let Some(message) = &self.message
            && !(dirty && (message.starts_with("已调用") || message.starts_with("已保存")))
        {
            ui.label(RichText::new(message).color(if message.starts_with('已') {
                BLUE
            } else {
                RED
            }));
        }
        ui.label(RichText::new("保存当前模型、温区及对应证书参数，不保存待换算数值。调用档案会替换当前证书参数；其他温区需另存。").size(12.0).color(MUTED));
        ui.add_space(8.0);
        self.show_dialog(ui.ctx(), tool)
    }

    fn show_dialog(&mut self, ctx: &egui::Context, tool: &str) -> Option<SaveRequest> {
        let mut dialog = self.dialog.take()?;
        let mut next = None;
        let mut close = false;
        let mut request = None;
        let frame = egui::Frame::popup(&ctx.style())
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgb(220, 227, 237),
            ))
            .corner_radius(10)
            .inner_margin(24);
        let response = egui::Modal::new(egui::Id::new(("calibration_dialog", tool)))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(440.0_f32.min((ctx.screen_rect().width() - 80.0).max(200.0)));
                ui.spacing_mut().item_spacing = Vec2::new(12.0, 12.0);
                match &mut dialog {
                    ArchiveDialog::Save { name } => {
                        ui.label(RichText::new("保存系数档案").size(19.0).strong().color(INK));
                        ui.label(RichText::new("档案名称").size(13.0).color(MUTED));
                        let input = ui.add_sized(
                            [ui.available_width(), 40.0],
                            egui::TextEdit::singleline(name)
                                .hint_text("例如：仪器编号或证书日期")
                                .vertical_align(egui::Align::Center),
                        );
                        if self.focus_name {
                            input.request_focus();
                            self.focus_name = false;
                        }
                        let submit = (input.has_focus() || input.lost_focus())
                            && ui.input(|input| input.key_pressed(egui::Key::Enter));
                        if let Some(error) = &self.dialog_error {
                            ui.label(RichText::new(error).color(RED));
                        }
                        ui.allocate_ui_with_layout(
                            Vec2::new(ui.available_width(), 40.0),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui.add(action_button("保存", true)).clicked() || submit {
                                    if let Ok(trimmed) = validate_name(name).map(str::to_owned) {
                                        match self
                                            .path
                                            .as_deref()
                                            .ok_or_else(|| "本地配置目录不可用".to_owned())
                                            .and_then(read_archives)
                                        {
                                            Ok(entries)
                                                if entries.iter().any(|entry| {
                                                    entry.tool == tool && entry.name == trimmed
                                                }) =>
                                            {
                                                self.dialog_error = None;
                                                next = Some(ArchiveDialog::Overwrite {
                                                    name: trimmed,
                                                    return_to_save: true,
                                                });
                                            }
                                            Ok(_) => {
                                                self.dialog_error = None;
                                                request = Some(SaveRequest {
                                                    name: trimmed,
                                                    replace: false,
                                                });
                                            }
                                            Err(error) => self.dialog_error = Some(error),
                                        }
                                    } else {
                                        self.dialog_error =
                                            validate_name(name).err().map(str::to_owned);
                                    }
                                }
                                if ui.add(action_button("取消", false)).clicked() {
                                    close = true;
                                }
                            },
                        );
                    }
                    ArchiveDialog::Overwrite {
                        name,
                        return_to_save,
                    } => {
                        let (title, description, confirm) = if *return_to_save {
                            (
                                "覆盖系数档案",
                                format!("档案“{name}”已存在，确定用当前证书参数覆盖吗？"),
                                "确认覆盖",
                            )
                        } else {
                            (
                                "更新系数档案",
                                format!("确定用当前证书参数更新档案“{name}”吗？"),
                                "确认更新",
                            )
                        };
                        ui.label(RichText::new(title).size(19.0).strong().color(INK));
                        ui.label(description);
                        if let Some(error) = &self.dialog_error {
                            ui.label(RichText::new(error).color(RED));
                        }
                        ui.add_space(8.0);
                        ui.allocate_ui_with_layout(
                            Vec2::new(ui.available_width(), 40.0),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui.add(action_button(confirm, true)).clicked() {
                                    request = Some(SaveRequest {
                                        name: name.clone(),
                                        replace: true,
                                    });
                                }
                                if ui.add(action_button("取消", false)).clicked() {
                                    self.dialog_error = None;
                                    if *return_to_save {
                                        self.focus_name = true;
                                        next = Some(ArchiveDialog::Save { name: name.clone() });
                                    } else {
                                        close = true;
                                    }
                                }
                            },
                        );
                    }
                    ArchiveDialog::Delete { name } => {
                        ui.label(RichText::new("删除系数档案").size(19.0).strong().color(INK));
                        ui.label(format!("确定删除档案“{name}”吗？"));
                        if let Some(error) = &self.dialog_error {
                            ui.label(RichText::new(error).color(RED));
                        }
                        ui.add_space(8.0);
                        ui.allocate_ui_with_layout(
                            Vec2::new(ui.available_width(), 40.0),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui.add(action_button("确认删除", true)).clicked() {
                                    match self.remove(tool, name) {
                                        Ok(()) => {
                                            self.selected.remove(tool);
                                            self.message = Some(format!("已删除档案：{name}"));
                                            self.dialog_error = None;
                                            close = true;
                                        }
                                        Err(error) => self.dialog_error = Some(error),
                                    }
                                }
                                if ui.add(action_button("取消", false)).clicked() {
                                    close = true;
                                }
                            },
                        );
                    }
                }
            });
        if response.should_close() {
            close = true;
        }
        self.dialog = if close {
            self.dialog_error = None;
            None
        } else {
            next.or(Some(dialog))
        };
        request
    }

    fn apply(&mut self, tool: &str, name: &str, values: &mut HashMap<String, String>) {
        if let Some(entry) = self
            .entries
            .iter()
            .find(|entry| entry.tool == tool && entry.name == name)
        {
            let prefix = format!("{tool}.");
            values.retain(|key, _| {
                !key.strip_prefix(&prefix)
                    .is_some_and(|key| allowed(tool, key))
            });
            for (key, value) in &entry.fields {
                values.insert(format!("{prefix}{key}"), value.clone());
            }
            self.selected.insert(tool.into(), name.into());
            self.message = Some(format!("已调用档案：{name}"));
        }
    }

    pub(super) fn finish_save(
        &mut self,
        tool: &str,
        values: &HashMap<String, String>,
        request: Option<SaveRequest>,
        required: Option<Vec<String>>,
    ) {
        let Some(request) = request else {
            return;
        };
        let result = (|| {
            let name = validate_name(&request.name)?;
            let required = required.ok_or("请先完整填写并检查证书参数，再保存档案")?;
            let prefix = format!("{tool}.");
            let fields: BTreeMap<String, String> = required
                .iter()
                .filter_map(|key| {
                    values
                        .get(&format!("{prefix}{key}"))
                        .map(|value| (key.clone(), value.clone()))
                })
                .collect();
            validate_fields(tool, &fields, &required)?;
            let path = self.path.as_deref().ok_or("本地配置目录不可用")?;
            let mut entries = read_archives(path)?;
            let existing = entries
                .iter()
                .position(|entry| entry.tool == tool && entry.name == name);
            if existing.is_some() && !request.replace {
                return Err("同名档案已存在，请再次保存并确认覆盖".into());
            }
            if existing.is_none() && request.replace {
                return Err("所选档案不存在，请重新选择".into());
            }
            let entry = Archive {
                tool: tool.into(),
                name: name.into(),
                fields,
                required,
            };
            if let Some(index) = existing {
                entries[index] = entry;
            } else {
                entries.push(entry);
            }
            write_archives(path, &entries)?;
            self.entries = entries;
            self.error = None;
            self.selected.insert(tool.into(), name.into());
            Ok(format!("已保存档案：{name}"))
        })();
        match result {
            Ok(message) => {
                self.dialog = None;
                self.dialog_error = None;
                self.message = Some(message);
            }
            Err(error) if self.dialog.is_some() => self.dialog_error = Some(error),
            Err(error) => self.message = Some(error),
        }
    }

    fn remove(&mut self, tool: &str, name: &str) -> Result<(), String> {
        let path = self.path.as_deref().ok_or("本地配置目录不可用")?;
        let mut entries = read_archives(path)?;
        let before = entries.len();
        entries.retain(|entry| entry.tool != tool || entry.name != name);
        if entries.len() == before {
            return Err("所选档案不存在，请重新打开程序".into());
        }
        write_archives(path, &entries)?;
        self.entries = entries;
        self.error = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_name_reports_empty_and_overlong_separately() {
        assert_eq!(validate_name("  "), Err("请输入档案名称"));
        assert_eq!(
            validate_name(&"标".repeat(101)),
            Err("档案名称不能超过 100 个字")
        );
        assert_eq!(validate_name("  标铂  "), Ok("标铂"));
    }

    fn request(name: &str, replace: bool) -> Option<SaveRequest> {
        Some(SaveRequest {
            name: name.into(),
            replace,
        })
    }

    fn sample() -> (HashMap<String, String>, Vec<String>) {
        let required: Vec<String> = [
            "stc.type",
            "stc.calibration",
            "stc.S.cal.0",
            "stc.S.cal.1",
            "stc.S.cal.2",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        let values = HashMap::from([
            (format!("{COUPLE}.stc.type"), "0".into()),
            (format!("{COUPLE}.stc.calibration"), "0".into()),
            (format!("{COUPLE}.stc.S.cal.0"), "3.4472345678901234".into()),
            (format!("{COUPLE}.stc.S.cal.1"), "5.8605".into()),
            (format!("{COUPLE}.stc.S.cal.2"), "10.5822".into()),
            (format!("{COUPLE}.stc.unit"), "2".into()),
            (format!("{COUPLE}.stc.S.0.0.t.2"), "1000".into()),
        ]);
        (values, required)
    }

    #[test]
    fn saves_loads_updates_and_deletes_without_touching_measurements() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("archives.json");
        let mut archives = CalibrationArchives::load(Some(path.clone()));
        let (mut values, required) = sample();
        values.insert(format!("{COUPLE}.stc.S.1.c.0"), "123".into());
        archives.finish_save(
            COUPLE,
            &values,
            request("证书 A", false),
            Some(required.clone()),
        );
        assert!(path.exists());
        assert!(archives.message.as_deref().unwrap().starts_with("已保存"));
        let loaded = read_archives(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].fields["stc.S.cal.0"], "3.4472345678901234");
        assert!(!loaded[0].fields.contains_key("stc.unit"));
        assert!(!loaded[0].fields.contains_key("stc.S.1.c.0"));

        values.insert(format!("{COUPLE}.stc.S.cal.0"), "9".into());
        values.insert(format!("{COUPLE}.stc.B.cal.0"), "stale".into());
        values.insert(format!("{RESISTOR}.sprt.rtp"), "25".into());
        archives.apply(COUPLE, "证书 A", &mut values);
        assert_eq!(
            values[&format!("{COUPLE}.stc.S.cal.0")],
            "3.4472345678901234"
        );
        assert!(!values.contains_key(&format!("{COUPLE}.stc.B.cal.0")));
        assert_eq!(values[&format!("{COUPLE}.stc.unit")], "2");
        assert_eq!(values[&format!("{COUPLE}.stc.S.0.0.t.2")], "1000");
        assert_eq!(values[&format!("{RESISTOR}.sprt.rtp")], "25");

        values.insert(format!("{COUPLE}.stc.S.cal.0"), "3.5".into());
        let original = std::fs::read(&path).unwrap();
        archives.finish_save(
            COUPLE,
            &values,
            request("证书 A", false),
            Some(required.clone()),
        );
        assert!(archives.message.as_deref().unwrap().contains("同名"));
        assert_eq!(std::fs::read(&path).unwrap(), original);
        archives.finish_save(COUPLE, &values, request("证书 A", true), Some(required));
        assert_eq!(
            read_archives(&path).unwrap()[0].fields["stc.S.cal.0"],
            "3.5"
        );
        archives.remove(COUPLE, "证书 A").unwrap();
        assert!(read_archives(&path).unwrap().is_empty());
    }

    #[test]
    fn broken_files_and_incomplete_inputs_are_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("archives.json");
        let mut archives = CalibrationArchives::load(Some(path.clone()));
        let (mut values, required) = sample();
        archives.finish_save(COUPLE, &values, request("未完成", false), None);
        assert!(!path.exists());
        values.remove(&format!("{COUPLE}.stc.S.cal.0"));
        archives.finish_save(
            COUPLE,
            &values,
            request("缺失", false),
            Some(required.clone()),
        );
        assert!(!path.exists());
        values.insert(format!("{COUPLE}.stc.S.cal.0"), "-".into());
        archives.finish_save(
            COUPLE,
            &values,
            request("无效", false),
            Some(required.clone()),
        );
        assert!(!path.exists());
        let broken = br#"{"version":99,"archives":[]}"#;
        std::fs::write(&path, broken).unwrap();
        let mut archives = CalibrationArchives::load(Some(path.clone()));
        assert!(archives.error.is_some());
        values.insert(format!("{COUPLE}.stc.S.cal.0"), "3.44".into());
        archives.finish_save(
            COUPLE,
            &values,
            request("新档案", false),
            Some(required.clone()),
        );
        assert_eq!(std::fs::read(&path).unwrap(), broken);
        let incomplete = serde_json::to_vec(&json!({"version":1,"archives":[{
            "tool":COUPLE,"name":"不完整", "fields":{"stc.type":"0"}, "required":["stc.type"]
        }]}))
        .unwrap();
        std::fs::write(&path, &incomplete).unwrap();
        assert!(read_archives(&path).is_err());
        archives.finish_save(COUPLE, &values, request("新档案", false), Some(required));
        assert_eq!(std::fs::read(&path).unwrap(), incomplete);
    }

    #[test]
    fn dialogs_fit_small_windows_with_full_size_buttons() {
        fn collect(shape: &egui::Shape, rects: &mut Vec<egui::Rect>) {
            match shape {
                egui::Shape::Rect(rect) => rects.push(rect.rect),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, rects);
                    }
                }
                _ => {}
            }
        }
        for dialog in [
            ArchiveDialog::Save {
                name: String::new(),
            },
            ArchiveDialog::Overwrite {
                name: "证书编号".repeat(20),
                return_to_save: true,
            },
            ArchiveDialog::Delete {
                name: "证书编号".repeat(20),
            },
        ] {
            let dir = tempfile::tempdir().unwrap();
            let mut archives = CalibrationArchives::load(Some(dir.path().join("archive.json")));
            archives.dialog = Some(dialog);
            let ctx = egui::Context::default();
            crate::fonts::configure_fonts(&ctx);
            crate::configure_style(&ctx);
            let render = |archives: &mut CalibrationArchives| {
                ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            [620.0, 520.0].into(),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        archives.show_dialog(ctx, COUPLE);
                    },
                )
            };
            render(&mut archives);
            let output = render(&mut archives);
            let mut rects = Vec::new();
            for shape in &output.shapes {
                collect(&shape.shape, &mut rects);
            }
            let panel = rects
                .iter()
                .find(|r| (480.0..510.0).contains(&r.width()))
                .expect("应有对话框");
            assert!(
                panel.top() >= 0.0 && panel.bottom() <= 520.0,
                "对话框应在窗口内：{panel:?}"
            );
            assert_eq!(
                rects
                    .iter()
                    .filter(|r| panel.contains_rect(**r)
                        && (112.0..=150.0).contains(&r.width())
                        && (39.0..42.0).contains(&r.height()))
                    .count(),
                2
            );
        }
    }

    #[test]
    fn different_instruments_can_use_the_same_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("archives.json");
        let mut archives = CalibrationArchives::load(Some(path.clone()));
        let (mut values, required) = sample();
        archives.finish_save(COUPLE, &values, request("证书", false), Some(required));
        let fields = ["sprt.model", "r0", "t1", "r1", "t2", "r2", "t3", "r3"];
        for (key, value) in fields
            .iter()
            .zip(["1", "100", "-100", "60", "100", "138", "200", "175"])
        {
            values.insert(format!("{RESISTOR}.{key}"), value.into());
        }
        archives.finish_save(
            RESISTOR,
            &values,
            request("证书", false),
            Some(fields.into_iter().map(str::to_owned).collect()),
        );
        assert_eq!(read_archives(&path).unwrap().len(), 2);
        archives.apply(RESISTOR, "证书", &mut values);
        assert_eq!(values[&format!("{RESISTOR}.r0")], "100");
        assert_eq!(
            values[&format!("{COUPLE}.stc.S.cal.0")],
            "3.4472345678901234"
        );
    }
}
