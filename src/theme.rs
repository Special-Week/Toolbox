//! 现代清爽设计系统：语义化深浅双主题色板、分类点缀色、Phosphor 图标映射与全局 Toast 通知桥接。
use crate::catalog::{Dimension, GaugeKind, Group, ToolDef, ToolKind};
use eframe::egui::{self, Color32, CornerRadius, Shadow, Stroke, Vec2};
use egui_phosphor::regular as icons;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    pub(crate) fn toggle(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub(crate) fn from_str(value: &str) -> Self {
        match value {
            "dark" => Self::Dark,
            _ => Self::Light,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Palette {
    pub(crate) dark: bool,
    pub(crate) page: Color32,
    pub(crate) surface: Color32,
    pub(crate) surface_subtle: Color32,
    pub(crate) surface_hover: Color32,
    pub(crate) input_bg: Color32,
    pub(crate) border: Color32,
    pub(crate) border_strong: Color32,
    pub(crate) ink: Color32,
    pub(crate) ink_secondary: Color32,
    pub(crate) muted: Color32,
    pub(crate) accent: Color32,
    pub(crate) accent_soft: Color32,
    pub(crate) accent_border: Color32,
    pub(crate) table_stripe: Color32,
    pub(crate) danger: Color32,
    pub(crate) danger_soft: Color32,
    pub(crate) danger_border: Color32,
    pub(crate) shadow: Shadow,
    pub(crate) popover_shadow: Shadow,
}

pub(crate) const LIGHT: Palette = Palette {
    dark: false,
    page: Color32::from_rgb(245, 247, 251),
    surface: Color32::WHITE,
    surface_subtle: Color32::from_rgb(248, 250, 253),
    surface_hover: Color32::from_rgb(241, 245, 251),
    input_bg: Color32::from_rgb(252, 253, 255),
    border: Color32::from_rgb(224, 231, 240),
    border_strong: Color32::from_rgb(168, 182, 200),
    ink: Color32::from_rgb(24, 34, 48),
    ink_secondary: Color32::from_rgb(58, 73, 94),
    muted: Color32::from_rgb(104, 118, 138),
    accent: Color32::from_rgb(37, 99, 235),
    accent_soft: Color32::from_rgb(238, 244, 255),
    accent_border: Color32::from_rgb(186, 211, 252),
    table_stripe: Color32::from_rgb(244, 247, 252),
    danger: Color32::from_rgb(205, 56, 56),
    danger_soft: Color32::from_rgb(255, 244, 243),
    danger_border: Color32::from_rgb(242, 205, 202),
    shadow: Shadow {
        offset: [0, 2],
        blur: 10,
        spread: 0,
        color: Color32::from_rgba_premultiplied(15, 23, 42, 10),
    },
    popover_shadow: Shadow {
        offset: [0, 10],
        blur: 28,
        spread: 0,
        color: Color32::from_rgba_premultiplied(15, 23, 42, 30),
    },
};

pub(crate) const DARK: Palette = Palette {
    dark: true,
    page: Color32::from_rgb(15, 23, 36),
    surface: Color32::from_rgb(24, 34, 51),
    surface_subtle: Color32::from_rgb(20, 29, 44),
    surface_hover: Color32::from_rgb(34, 47, 68),
    input_bg: Color32::from_rgb(18, 26, 40),
    border: Color32::from_rgb(46, 61, 84),
    border_strong: Color32::from_rgb(72, 92, 122),
    ink: Color32::from_rgb(236, 242, 249),
    ink_secondary: Color32::from_rgb(198, 210, 226),
    muted: Color32::from_rgb(138, 154, 178),
    accent: Color32::from_rgb(96, 165, 250),
    accent_soft: Color32::from_rgb(30, 50, 84),
    accent_border: Color32::from_rgb(56, 98, 160),
    table_stripe: Color32::from_rgb(29, 40, 59),
    danger: Color32::from_rgb(248, 113, 113),
    danger_soft: Color32::from_rgb(64, 28, 34),
    danger_border: Color32::from_rgb(120, 48, 56),
    shadow: Shadow {
        offset: [0, 3],
        blur: 12,
        spread: 0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 48),
    },
    popover_shadow: Shadow {
        offset: [0, 12],
        blur: 32,
        spread: 0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 110),
    },
};

const THEME_KEY: &str = "toolbox.theme_mode";
const TOAST_QUEUE_KEY: &str = "toolbox.toast_queue";

pub(crate) fn current_mode(ctx: &egui::Context) -> ThemeMode {
    ctx.data(|d| d.get_temp::<ThemeMode>(egui::Id::new(THEME_KEY)))
        .unwrap_or(ThemeMode::Light)
}

pub(crate) fn palette(ctx: &egui::Context) -> Palette {
    match current_mode(ctx) {
        ThemeMode::Light => LIGHT,
        ThemeMode::Dark => DARK,
    }
}

pub(crate) fn apply_theme(ctx: &egui::Context, mode: ThemeMode) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(THEME_KEY), mode));
    let p = match mode {
        ThemeMode::Light => LIGHT,
        ThemeMode::Dark => DARK,
    };

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    style.spacing.scroll = egui::style::ScrollStyle::thin();

    let mut visuals = if p.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = p.page;
    visuals.window_fill = p.surface;
    visuals.window_stroke = Stroke::new(1.0_f32, p.border);
    visuals.window_corner_radius = CornerRadius::same(12);
    visuals.window_shadow = p.popover_shadow;
    visuals.popup_shadow = p.popover_shadow;
    visuals.faint_bg_color = p.table_stripe;
    visuals.extreme_bg_color = p.input_bg;
    visuals.override_text_color = None;

    visuals.widgets.noninteractive.bg_fill = p.surface;
    visuals.widgets.noninteractive.weak_bg_fill = p.surface_subtle;
    visuals.widgets.noninteractive.fg_stroke.color = p.ink;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(10);

    visuals.widgets.inactive.bg_fill = p.surface;
    visuals.widgets.inactive.weak_bg_fill = p.surface;
    visuals.widgets.inactive.fg_stroke.color = p.ink;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border_strong);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(8);

    visuals.widgets.hovered.bg_fill = p.accent_soft;
    visuals.widgets.hovered.weak_bg_fill = p.accent_soft;
    visuals.widgets.hovered.fg_stroke.color = p.accent;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
    visuals.widgets.hovered.expansion = 0.0;

    visuals.widgets.active.bg_fill = p.accent;
    visuals.widgets.active.weak_bg_fill = p.accent;
    visuals.widgets.active.fg_stroke.color = Color32::WHITE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.active.corner_radius = CornerRadius::same(8);
    visuals.widgets.active.expansion = 0.0;

    visuals.widgets.open.bg_fill = p.accent_soft;
    visuals.widgets.open.weak_bg_fill = p.accent_soft;
    visuals.widgets.open.fg_stroke.color = p.accent;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.open.corner_radius = CornerRadius::same(8);

    visuals.selection.bg_fill = p.accent;
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);

    style.visuals = visuals;
    ctx.set_style(style);
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GroupAccent {
    pub(crate) fg: Color32,
    pub(crate) bg: Color32,
    pub(crate) icon: &'static str,
}

/// 为 7 个工具分类分配协调清爽的点缀色与语义图标，消除大面积单色调疲劳。
pub(crate) fn group_accent(group: Group, dark: bool) -> GroupAccent {
    let icon = match group {
        Group::Units => icons::ARROWS_LEFT_RIGHT,
        Group::Its90 => icons::THERMOMETER,
        Group::Pressure => icons::GAUGE,
        Group::Humidity => icons::DROP,
        Group::Errors => icons::CHART_SCATTER,
        Group::Gauges => icons::RULER,
        Group::Other => icons::WRENCH,
    };
    let (fg, bg) = if dark {
        match group {
            Group::Units => (
                Color32::from_rgb(96, 165, 250),
                Color32::from_rgb(28, 48, 82),
            ),
            Group::Its90 => (
                Color32::from_rgb(251, 146, 60),
                Color32::from_rgb(66, 40, 24),
            ),
            Group::Pressure => (
                Color32::from_rgb(56, 189, 248),
                Color32::from_rgb(20, 52, 74),
            ),
            Group::Humidity => (
                Color32::from_rgb(45, 212, 191),
                Color32::from_rgb(18, 56, 56),
            ),
            Group::Errors => (
                Color32::from_rgb(167, 139, 250),
                Color32::from_rgb(46, 36, 78),
            ),
            Group::Gauges => (
                Color32::from_rgb(148, 163, 184),
                Color32::from_rgb(40, 50, 66),
            ),
            Group::Other => (
                Color32::from_rgb(52, 211, 153),
                Color32::from_rgb(20, 56, 46),
            ),
        }
    } else {
        match group {
            Group::Units => (
                Color32::from_rgb(37, 99, 235),
                Color32::from_rgb(238, 244, 255),
            ),
            Group::Its90 => (
                Color32::from_rgb(217, 91, 20),
                Color32::from_rgb(255, 244, 235),
            ),
            Group::Pressure => (
                Color32::from_rgb(2, 132, 199),
                Color32::from_rgb(236, 248, 255),
            ),
            Group::Humidity => (
                Color32::from_rgb(13, 148, 136),
                Color32::from_rgb(236, 252, 249),
            ),
            Group::Errors => (
                Color32::from_rgb(124, 58, 237),
                Color32::from_rgb(245, 240, 255),
            ),
            Group::Gauges => (
                Color32::from_rgb(71, 85, 105),
                Color32::from_rgb(241, 245, 249),
            ),
            Group::Other => (
                Color32::from_rgb(5, 150, 105),
                Color32::from_rgb(236, 253, 245),
            ),
        }
    };
    GroupAccent { fg, bg, icon }
}

pub(crate) fn tool_icon(tool: &ToolDef) -> &'static str {
    match tool.kind {
        ToolKind::Unit(dim) => match dim {
            Dimension::Pressure => icons::GAUGE,
            Dimension::Temperature => icons::THERMOMETER_SIMPLE,
            Dimension::MassFlow | Dimension::VolumeFlow => icons::WAVES,
            Dimension::Mass => icons::SCALES,
            Dimension::Length => icons::RULER,
            Dimension::Area => icons::SQUARE_HALF_BOTTOM,
            Dimension::Volume => icons::CUBE,
            Dimension::Density => icons::STACK,
            Dimension::Velocity => icons::SPEEDOMETER,
            Dimension::Force => icons::ARROW_UP_RIGHT,
            Dimension::Energy => icons::LIGHTNING,
            Dimension::Power => icons::FIRE_SIMPLE,
            Dimension::HeatTransferCoefficient
            | Dimension::ThermalConductivity
            | Dimension::HeatTransferRate => icons::SUN_DIM,
            Dimension::DynamicViscosity | Dimension::KinematicViscosity => icons::DROP_HALF,
            Dimension::DiffusionCoefficient => icons::ATOM,
            Dimension::SurfaceTension => icons::DROP,
            Dimension::Torque => icons::ARROWS_CLOCKWISE,
            Dimension::Concentration => icons::FLASK,
        },
        ToolKind::WorkingThermocouple | ToolKind::StandardThermocouple => icons::THERMOMETER_HOT,
        ToolKind::IndustrialResistance | ToolKind::StandardPlatinumResistor => icons::THERMOMETER,
        ToolKind::Thermistor => icons::PULSE,
        ToolKind::PressureReference | ToolKind::TransmitterReference => icons::GAUGE,
        ToolKind::HeightCorrection => icons::ARROWS_VERTICAL,
        ToolKind::AltitudePressure => icons::MOUNTAINS,
        ToolKind::HumidityConversion => icons::DROP,
        ToolKind::EarBlackbody | ToolKind::BlackbodyError | ToolKind::RadiationCorrection => {
            icons::SUN
        }
        ToolKind::Repeatability | ToolKind::RangeMethod => icons::CHART_BAR,
        ToolKind::ResultantError | ToolKind::ErrorAnalysis => icons::TARGET,
        ToolKind::Uncertainty => icons::TREE_STRUCTURE,
        ToolKind::Gauge(kind) => match kind {
            GaugeKind::Smooth => icons::CIRCLE_DASHED,
            _ => icons::NUT,
        },
        ToolKind::Transmitter => icons::SLIDERS_HORIZONTAL,
        ToolKind::BSplineInterpolation => icons::BEZIER_CURVE,
        ToolKind::CalibrationCurveFit => icons::CHART_LINE,
        ToolKind::PdfDownload => icons::FILE_PDF,
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ToastLevel {
    Success,
    Info,
    Error,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingToast {
    pub(crate) level: ToastLevel,
    pub(crate) message: String,
}

fn push_toast(ctx: &egui::Context, level: ToastLevel, message: impl Into<String>) {
    let id = egui::Id::new(TOAST_QUEUE_KEY);
    ctx.data_mut(|d| {
        let queue = d.get_temp_mut_or_default::<Vec<PendingToast>>(id);
        queue.push(PendingToast {
            level,
            message: message.into(),
        });
    });
}

pub(crate) fn toast_success(ctx: &egui::Context, message: impl Into<String>) {
    push_toast(ctx, ToastLevel::Success, message);
}

pub(crate) fn toast_info(ctx: &egui::Context, message: impl Into<String>) {
    push_toast(ctx, ToastLevel::Info, message);
}

pub(crate) fn toast_error(ctx: &egui::Context, message: impl Into<String>) {
    push_toast(ctx, ToastLevel::Error, message);
}

pub(crate) fn drain_toasts(ctx: &egui::Context, toasts: &mut egui_notify::Toasts) {
    let id = egui::Id::new(TOAST_QUEUE_KEY);
    let pending: Vec<PendingToast> = ctx.data_mut(|d| {
        std::mem::take(d.get_temp_mut_or_default::<Vec<PendingToast>>(id))
    });
    for item in pending {
        let duration = Some(std::time::Duration::from_secs(3));
        match item.level {
            ToastLevel::Success => {
                toasts.success(item.message).duration(duration);
            }
            ToastLevel::Info => {
                toasts.info(item.message).duration(duration);
            }
            ToastLevel::Error => {
                toasts
                    .error(item.message)
                    .duration(Some(std::time::Duration::from_secs(5)));
            }
        }
    }
}
