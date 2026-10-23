//! 结果数据不依赖 UI。计算值固定五位小数，输入参数保留原有显示；显示修约不写回模型。
use super::lookup::{number, valid};
use crate::calculators::format::format_number;
use serde_json::Value;

#[derive(Clone)]
pub(super) struct Cell {
    pub display: String,
    pub raw: String,
}

/// 仅用于结果排版；极小负数舍入后统一显示 0.00000，不出现负零。
pub(super) fn calculated(value: f64) -> String {
    if !valid(value) {
        return "—".into();
    }
    format!("{:.5}", if value.abs() < 0.000005 { 0.0 } else { value })
}

impl From<&str> for Cell {
    fn from(value: &str) -> Self {
        Self {
            display: value.into(),
            raw: value.into(),
        }
    }
}
impl From<String> for Cell {
    fn from(value: String) -> Self {
        Self {
            display: value.clone(),
            raw: value,
        }
    }
}
impl From<f64> for Cell {
    fn from(value: f64) -> Self {
        if !valid(value) {
            return "—".into();
        }
        Self {
            display: calculated(value),
            raw: value.to_string(),
        }
    }
}
impl Cell {
    pub fn pair(a: f64, b: f64) -> Self {
        Self {
            display: format!("{} / {}", calculated(a), calculated(b)),
            raw: format!("{a} / {b}"),
        }
    }
    pub fn parameter(value: f64) -> Self {
        if !valid(value) {
            return "—".into();
        }
        Self {
            display: format_number(value),
            raw: value.to_string(),
        }
    }
    pub fn limit(value: f64, sign: &str) -> Self {
        let cell = Self::from(value);
        if !valid(value) {
            return cell;
        }
        Self {
            display: format!("{sign} {}", cell.display),
            raw: format!("{sign} {}", cell.raw),
        }
    }
    pub fn value(value: &Value) -> Self {
        if value.is_null() {
            return "—".into();
        }
        let n = number(value);
        if n.is_finite() {
            Self::from(n)
        } else {
            value.as_str().unwrap_or("—").into()
        }
    }
}

pub(super) struct Table {
    pub title: String,
    pub headings: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
}
impl Table {
    pub fn new(title: &str, headings: &[&str]) -> Self {
        Self {
            title: title.into(),
            headings: headings.iter().map(|s| (*s).into()).collect(),
            rows: Vec::new(),
        }
    }
    pub fn row(&mut self, cells: Vec<Cell>) {
        self.rows.push(cells);
    }
}

#[derive(Default)]
pub(super) struct Report {
    pub title: String,
    pub metrics: Vec<(String, Cell)>,
    pub tables: Vec<Table>,
    pub notes: Vec<String>,
}
impl Report {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }
    pub fn metric(&mut self, name: impl Into<String>, value: impl Into<Cell>) {
        self.metrics.push((name.into(), value.into()));
    }
    pub fn parameter(&mut self, name: impl Into<String>, value: f64) {
        self.metrics.push((name.into(), Cell::parameter(value)));
    }
    pub fn note(&mut self, text: impl Into<String>) {
        self.notes.push(text.into());
    }

    /// 校验实际尺寸，不把公差偏差的正负号当作尺寸；缺失值“—”仍保留。
    /// 非标输入即使基本牙型为正，加上公差后也可能产生负的小径或磨损极限。
    pub fn checked(self) -> Result<Self, String> {
        for table in &self.tables {
            for row in &table.rows {
                for (heading, cell) in table.headings.iter().zip(row) {
                    if heading.ends_with("(mm)") {
                        for token in cell.raw.split_whitespace() {
                            if let Ok(value) = token.parse::<f64>() {
                                if !value.is_finite() || value <= 0. {
                                    return Err(format!(
                                        "当前参数及公差产生非正或无效尺寸（{}：{}），请核对直径、螺距和公差带。",
                                        table.title, heading,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(self)
    }

    /// 美制页面的 mm 与 inch 都从未修约值独立格式化，不把已显示的 mm 再除以 25.4。
    pub fn show_inches(&mut self) {
        self.note("mm 与 inch 独立修约，显示值互换可能有末位差异。");
        fn annotate(cell: &mut Cell) {
            let mut has_number = false;
            let values = cell
                .raw
                .split_whitespace()
                .map(|token| {
                    if let Ok(value) = token.parse::<f64>() {
                        has_number = true;
                        calculated(value / 25.4)
                    } else {
                        token.to_owned()
                    }
                })
                .collect::<Vec<_>>();
            if has_number {
                cell.display
                    .push_str(&format!("\n({} inch)", values.join(" ")));
            }
        }
        for (label, cell) in &mut self.metrics {
            if label.contains("(mm)") {
                annotate(cell);
            }
        }
        for table in &mut self.tables {
            for row in &mut table.rows {
                for (heading, cell) in table.headings.iter().zip(row) {
                    if heading.contains("(mm)") {
                        annotate(cell);
                    }
                }
            }
        }
    }
    /// 复制为普通文本，数值与界面一致；不再提供量规 CSV 文件导出。
    pub fn text(&self, explanation: &str, convention: &str) -> String {
        let mut out = format!("{}\n", self.title);
        for (label, value) in &self.metrics {
            out.push_str(&format!("{label}：{}\n", value.display.replace('\n', " ")));
        }
        for table in &self.tables {
            out.push_str(&format!(
                "\n{}\n{}\n",
                table.title,
                table.headings.join("\t")
            ));
            for row in &table.rows {
                out.push_str(
                    &row.iter()
                        .map(|c| c.display.replace('\n', " "))
                        .collect::<Vec<_>>()
                        .join("\t"),
                );
                out.push('\n');
            }
        }
        for note in &self.notes {
            out.push_str(&format!("取值说明：{note}\n"));
        }
        out.push_str(&format!(
            "\n计算说明\n{explanation}\n\n详细公式与取值\n{convention}\n"
        ));
        out
    }
}

/// 全部螺纹共用几何摘要；角度用度，线数只影响导程和升角。
pub(super) fn geometry(
    report: &mut Report,
    d: f64,
    p: f64,
    d2: f64,
    d1: f64,
    angle: f64,
    lines: usize,
) {
    report.parameter("基本大径 D (mm)", d);
    report.metric("基本中径 D₂ (mm)", d2);
    report.metric("基本小径 D₁ (mm)", d1);
    report.parameter("螺距 P (mm)", p);
    report.parameter("牙型角 (°)", angle);
    report.parameter("半角 (°)", angle / 2.0);
    report.parameter("线数 / 头数", lines as f64);
    report.metric("导程 L (mm)", lines as f64 * p);
    report.metric(
        "螺旋升角 ψ (°)",
        (lines as f64 * p / (std::f64::consts::PI * d2))
            .atan()
            .to_degrees(),
    );
    report.metric(
        "最佳三针直径 d₀ (mm)",
        p / (2.0 * (angle / 2.0).to_radians().cos()),
    );
}
