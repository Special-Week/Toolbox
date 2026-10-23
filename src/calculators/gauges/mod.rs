//! 六类量规的纯数值模型与桌面页面；运行时不依赖网页、脚本引擎或网络。
#[cfg(not(feature = "desktop-ui"))]
pub(crate) mod api;
mod conventions;
mod imperial;
mod lookup;
#[cfg(feature = "desktop-ui")]
mod page;
mod pipe;
mod report;
mod smooth;
mod specs;
mod thread;
mod un_data;
mod unified;

use crate::catalog::GaugeKind;

/// 简要说明只保留取值依据与结果适用条件。
pub(crate) fn notes(kind: GaugeKind) -> &'static str {
    match kind {
        GaugeKind::Metric => {
            "按 GB/T 197、GB/T 3934 查询工件公差和量规要素。小径参考界限不含独立光滑量规公差。\n塞规长度按 GB/T 10920-2008，D>100 mm 时长度及手柄另定。"
        }
        GaugeKind::Trapezoidal => {
            "多线中径公差按线数修正；内螺纹小径取 4 级，单线外螺纹小径按 c/e 带查表。\n多线及未收录公差带的小径界限不计算。"
        }
        GaugeKind::Unified => {
            "工件等级决定工件极限；工作量规中径按制造公差 U 计算。\n大径、小径及校对尺寸供参考；未收录磨损储备。"
        }
        GaugeKind::Pipe => {
            "R 为 55°、NPT 为 60° 密封锥管螺纹，锥度均为 1:16。\n尺寸用 mm，制造公差、磨损及半角偏差按表头单位读取。"
        }
        GaugeKind::Rebar => {
            "基本尺寸按所选 60°、70°、75° 牙型查表，规格为 Φ16～Φ40。\n套筒、丝头分别借用 6H、6f 及 GB/T 3934 量规要素；不构成产品标准的完整符合性判定。"
        }
        GaugeKind::Smooth => {
            "适用 0<D≤500 mm、IT6～IT16；卡规与轴用环规采用同一数值。\nP～ZC 的 6、7 级孔公差带涉及未收录 Δ 修正，不计算该组合。"
        }
    }
}

pub(crate) fn formula(kind: GaugeKind) -> String {
    conventions::text(crate::catalog::ToolKind::Gauge(kind))
}
