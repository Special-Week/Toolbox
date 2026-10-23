//! 专业用户说明与详细公式分开维护。不同入口的口径单独列出。
use crate::catalog::ToolKind;

pub(crate) type NoteVariant = (usize, &'static str, &'static str);

pub(crate) fn for_tool(kind: ToolKind) -> &'static str {
    match kind {
        ToolKind::Gauge(kind) => super::gauges::notes(kind),
        ToolKind::Unit(dimension) => crate::units::notes::for_dimension(dimension),
        ToolKind::WorkingThermocouple => {
            "按所选分度曲线换算，计入参考端补偿。公式温度用 °C，热电势用 mV。\n分度曲线与等级允差各有适用温区；B 型非正基准电势不作唯一反算，U 型范围为 −200～600 °C。允差电势界限按参考端 0 °C 计算。"
        }
        ToolKind::StandardThermocouple => {
            "使用所选 S/B 型校准曲线并计入参考端补偿。证书热电势填实际值（mV），不是修正量。\n系数公式温度用 °C；反算限于所选温区，B 型采用 ≥50 °C 的单调分支。"
        }
        ToolKind::IndustrialResistance => {
            "按所选分度曲线和 R₀ 换算；R₀ 为 0 °C 电阻，Cu10 的 10 Ω 标称值对应约 25 °C。\n公式温度用 °C，电阻用 Ω。等级允差按卡片温区判断，电阻界限由温度界限换算。"
        }
        ToolKind::StandardPlatinumResistor => {
            "W=R/Rtp，Rtp 为水三相点 0.01 °C 电阻。使用证书系数或固定点拟合结果，并选择对应高低温分区。\n电阻用 Ω，公式温度用 °C；换算限于已启用分区。"
        }
        ToolKind::Thermistor => {
            "多项式使用 ln(R) 与 1/T，R 用 Ω、T 用 K，系数按升幂输入。A/B 两组系数分别拟合。\n请使用证书系数及适用温区；默认系数仅为示例。"
        }
        ToolKind::PressureReference => {
            "按标准器类型计算允差。标准器须覆盖被检范围，且被检允差/标准器允差达到设定量传比。\n准确度以百分数填写，例如 0.05 表示 0.05%。"
        }
        ToolKind::TransmitterReference => {
            "压力误差折算为电信号后，与电测误差按均匀分布合成，取 k=2。\n标准压力范围、允差及合成结果均须满足设定量传比；信号与固定误差使用同一单位。"
        }
        ToolKind::HeightCorrection => {
            "ΔH 为被检表相对标准器的高度，向上为正。标准器读数移至被检高度的修正量为 −ρgΔH。\n气体可填当地密度或由标准状态密度换算；表压模式扣除环境气柱影响。"
        }
        ToolKind::AltitudePressure => {
            "按 1976 标准大气换算绝对气压与位势高度，范围 −1000～86000 m。\n结果不含当地气象修正，不等同于实际几何海拔。"
        }
        ToolKind::HumidityConversion => {
            "输入环境温度、绝对气压及一个已知湿度参数。湿球系数 A=0.000662；饱和压在 ≤0 °C 时按冰面计算，其余按水面。\n混合比以干空气质量计，比湿以湿空气总质量计；标准状态含水量按 0 °C、101.325 kPa。"
        }
        ToolKind::EarBlackbody => {
            "采用灰体多次反射平衡，假设黑体与探头两表面相互完全可见。\n亮温修正值=亮温−黑体真实温度；发射率须在 (0,1] 内。"
        }
        ToolKind::BlackbodyError => {
            "有效波长法的发射率误差定义为 T−M，结果取至 0.01 °C。背景温度须≤200 °C。"
        }
        ToolKind::RadiationCorrection => {
            "有效波长法按仪表与辐射源发射率误差之差求修正值；背景温度须≤200 °C。\n修正值先取至 0.01 °C，再加到温度计示值。"
        }
        ToolKind::Repeatability => {
            "按样本标准差计算重复性；均值标准差为 s/√n，RSD=s/|均值|×100%。\n各测量值单位一致，n≥2。"
        }
        ToolKind::RangeMethod => "按极差法估算标准差：s≈R/C，均值标准差为 s/√n，支持 2～9 次测量。",
        ToolKind::ResultantError => {
            "各分量先乘灵敏系数，再按平方和根合成，分量按不相关处理。\n最坏情况界限取各项贡献的绝对值和。"
        }
        ToolKind::ErrorAnalysis => "绝对误差 Δ=测量值−参考值，保留正负号。引用值留空时采用参考值。",
        ToolKind::Uncertainty => {
            "各项标准不确定度乘灵敏系数后合成，分量按不相关处理，U=k·uc。\nB 类正态项填证书扩展不确定度 U，其余分布填区间半宽。包含因子 k 不自动对应置信水平。"
        }
        ToolKind::Transmitter => {
            "按所选量程与线性/开方特性双向换算，输入须在量程内。\n过程量、信号单位须与仪表配置一致。"
        }
        ToolKind::BSplineInterpolation => {
            "采用等距参数节点的自然三次均匀 B 样条，Q 点的 x 可不等距且须严格递增。\n区间外沿端点曲线外推，到 x 的首个转折点停止。"
        }
        ToolKind::CalibrationCurveFit => {
            "采用等权最小二乘拟合，K0 为常数项；各列单位须一致。\nr 表示原始 x、y 的线性相关性，R² 评价所选模型；过原点拟合的 R² 可为负。"
        }
    }
}

pub(crate) fn formula(kind: ToolKind) -> String {
    let text = match kind {
        ToolKind::Gauge(kind) => return super::gauges::formula(kind),
        ToolKind::Unit(_) => "",
        ToolKind::WorkingThermocouple => {
            "E测=E(t热端)−E(t参考端)；反算先加回参考端电势。等级允差按 IEC 60584-1 及各等级温区计算，温度界限为 t±Δt。"
        }
        ToolKind::StandardThermocouple => {
            "S 型：锌、铝、铜点相对参考曲线的偏差拟合二次式，400 °C 以下按锌点偏差线性延伸。\nB 型：1100～1500 °C 五点偏差作分段及过渡插值。\nE测=E(t热端)−E(t参考端)。"
        }
        ToolKind::IndustrialResistance => {
            "自定义 CVD：t≥0 时 R=R₀(1+At+Bt²)；t<0 时 R=R₀[1+At+Bt²+C(t−100)t³]。\n允差电阻界限为 R(t−Δt) 至 R(t+Δt)。"
        }
        ToolKind::StandardPlatinumResistor => {
            "Wr=W−ΔW。高温 ΔW=a(W−1)+b(W−1)²+c(W−1)³；A6 在铝点以上加 d(W−W660)²。\n低温 a4,b4：ΔW=a4(W−1)+b4(W−1)ln(W)；a5,b5：ΔW=a5(W−1)+b5(W−1)²。再由参考函数求温度。"
        }
        ToolKind::Thermistor => {
            "电阻→温度：1/T=ΣA[i]·ln(R)^i。\n温度→电阻：ln(R)=ΣB[i]·(1/T)^i。\n支持 3～6 阶；Steinhart–Hart 形式令二次项为 0。A/B 不保证互为精确反函数；温度残差=证书温度−反算温度。"
        }
        ToolKind::PressureReference => {
            "被检允差=被检跨度×被检准确度/100。\n数字标准器允差=标准器跨度×标准器准确度/100。\n活塞压力计允差=标准器准确度/100×max(被检跨度, 标准器跨度的10%)。"
        }
        ToolKind::TransmitterReference => {
            "设压力输出跨度为 P、信号跨度为 S。压力分量 aₚ=Δp标·S/P；电测分量 aₑ=读数准确度/100·|输出上限|+固定误差。\nU=2√(aₚ²+aₑ²)/√3；通过条件 U<S×被检准确度/100/量传比，同时核对压力范围与允差。"
        }
        ToolKind::HeightCorrection => {
            "ρ 用 kg/m³，g 用 m/s²，ΔH 用 m，压差单位 Pa。\np标准−p被检=ρgΔH；气体 ρ=ρ₀·(p绝/101325)·(273.15/T)，T 用 K。\n气体按局部恒定密度近似，环境气柱按标准大气估算；等效水柱按 4 °C 水换算。"
        }
        ToolKind::AltitudePressure => {
            "分层界面为 11、20、32、47、51、71 km，按恒温或恒温度梯度关系计算。空气密度 ρ=p/(287.05287T)，p 用 Pa，T 用 K。"
        }
        ToolKind::HumidityConversion => {
            "环境及湿球温度范围 −100～100 °C。e=RH·ps/100；湿球满足 ps(T湿)−e=0.000662p(T干−T湿)。\n含湿量=e/p（L/L）；以干空气计的体积比=e/(p−e)×10⁶（μL/L）。\n非正露/霜点输入按冰面处理；结果分别列水面露点与负温霜点。RH=0 时无有限露/霜点，水气分压须小于总压。"
        }
        ToolKind::EarBlackbody => {
            "D=ε黑+(1−ε黑)ε探。\nT亮⁴=[ε黑T黑⁴+(1−ε黑)ε探T探⁴]/D。公式温度均用 K。"
        }
        ToolKind::BlackbodyError => {
            "C=1/[exp(c₂/(λT))−1]−(1−ε)/[exp(c₂/(λT背景))−1]。\nM=c₂/[λ·ln(1+ε/C)]，误差=T−M。c₂=14388 μm·K，λ 用 μm，T 用 K。\n波段有效波长：8～14 μm 取 9.6+221/T；7～18 μm 取 8.7+525/T。"
        }
        ToolKind::RadiationCorrection => {
            "ΔT=F(ε表,T示,T背景,λ)−F(ε源,T示,T背景,λ)，F 为发射率误差函数，两项均在示值温度计算。\n修正后温度=T示+ΔT。波段有效波长为 9.6+221/T 或 8.7+525/T，T 用 K，λ 用 μm。"
        }
        ToolKind::Repeatability => {
            "s=√[Σ(x−均值)²/(n−1)]；极差=最大值−最小值。均值为 0 时 RSD 无定义。"
        }
        ToolKind::RangeMethod => {
            "n=2～9 的 C 依次为 1.13、1.69、2.06、2.33、2.53、2.70、2.85、2.97。采用两位小数表值；R=最大值−最小值。"
        }
        ToolKind::ResultantError => {
            "各项贡献 q[i]=c[i]e[i]；合成值=√Σq[i]²，最坏情况界限=Σ|q[i]|。"
        }
        ToolKind::ErrorAnalysis => {
            "相对误差=Δ/参考值×100%；引用误差=Δ/引用值×100%；误差绝对值为 |Δ|。分母为 0 时对应百分误差无定义。"
        }
        ToolKind::Uncertainty => {
            "A 类：贝塞尔法 u=s/√n；极差法 u=R/(C√n)，支持 2～9 次。\nB 类：均匀 u=a/√3；三角 u=a/√6；两点 u=a；反正弦 u=a/√2；正态 u=U/k（所选 k 为 2 或 3）。\n贡献 q=c·u；分组逐层合成后乘本层灵敏系数。“最大贡献”仅用于主导分量估计。\n修约按 uc、U 的较大值确定小数位，最低为整数位；修约后 U=k×修约后的 uc。不自动计算有效自由度。"
        }
        ToolKind::Transmitter => {
            "f=(被测量−量程下限)/量程跨度，s=(信号−输出下限)/输出跨度。\n线性：s=f；开方：s=√f，反算 f=s²。"
        }
        ToolKind::BSplineInterpolation => {
            "自然端点条件：端点 x、y 关于参数 u 的二阶导数均为 0。\n采样从最小 x 按步进生成，末尾包含最大 x。"
        }
        ToolKind::CalibrationCurveFit => {
            "残差 e=y−拟合 y。r=Σ[(x−均值 x)(y−均值 y)]/√[Σ(x−均值 x)²·Σ(y−均值 y)²]。\nR²=1−Σe²/Σ(y−均值 y)²；带截距的一次拟合在 r 有定义时满足 R²=r²。过原点固定 K0=0。\nRMSE=√(Σe²/n)，残差标准差=√[Σe²/(n−p)]，p 为估计参数个数。n=p 时残差标准差无定义；响应不变时 r、R² 无定义。\n允许重复 x，但不同 x 数须足够。拟合结果须结合残差与测量范围评价；不自动加权、扣除空白或剔除异常点。"
        }
    };
    text.to_owned()
}

/// 编号仅用于选择说明，不参与计算模型。
pub(crate) fn variants(kind: ToolKind) -> &'static [NoteVariant] {
    match kind {
        ToolKind::Thermistor => &[(
            1,
            "采用 β 模型，R₀ 对应参考温度 T₀。请使用适用温区内的 β 值。",
            "1/T=1/T₀+ln(R/R₀)/β。T、T₀ 用 K，R、R₀ 用 Ω，β 用 K。",
        )],
        ToolKind::StandardThermocouple => &[(
            1,
            "温区系数按 E(t)=Σc[i]t^i 计算，t 用 °C，E 用 mV。使用证书系数及适用温区。\n校准点拟合只修正一次、二次项；反算限于所选温区，B 型采用 ≥50 °C 分支。",
            "E测=E(t热端)−E(t参考端)；反算先加回参考端电势。热电势残差=实测电势−拟合电势。",
        )],
        ToolKind::StandardPlatinumResistor => &[
            (
                1,
                "以三组温度、电阻数据拟合 CVD 系数，温度用 °C，电阻用 Ω。\n请按实际校准点和证书温区使用。",
                "t≥0：R=R₀(1+At+Bt²)；t<0：R=R₀[1+At+Bt²+C(t−100)t³]。\n三点拟合不等于验证全温区特性。",
            ),
            (
                2,
                "采用三项高温/对数低温偏差模型，W=R/Rtp，Rtp 为水三相点电阻。\n此模式不使用 d 系数；换算限于所选温区。",
                "Wr=W−ΔW。高温 ΔW=a(W−1)+b(W−1)²+c(W−1)³；低温 ΔW=a4(W−1)+b4(W−1)ln(W)。",
            ),
        ],
        ToolKind::PressureReference => &[(
            1,
            "按活塞压力计允差口径核验量程覆盖和量传比。准确度按百分数填写。",
            "被检允差=被检跨度×被检准确度/100。\n数字标准器允差=标准器跨度×标准器准确度/100。\n活塞压力计允差=标准器准确度/100×max(被检跨度, 标准器跨度的10%)。",
        )],
        ToolKind::TransmitterReference => &[(
            1,
            "分别核对压力、电信号两通道的范围覆盖与允差比。准确度均按量程跨度百分数计算。",
            "通道允差=量程跨度×准确度/100；量传比=被检通道允差/标准通道允差。两通道均须达到设定门槛。",
        )],
        ToolKind::EarBlackbody => &[(
            1,
            "采用灰体一次反射近似，两表面相互完全可见。\n亮温修正值=亮温−黑体真实温度；近似在等温时仍可能有偏差。",
            "T亮⁴=ε黑T黑⁴+(1−ε黑)[ε探T探⁴+(1−ε探)ε黑T黑⁴]。\n公式温度均用 K，发射率须在 (0,1] 内。",
        )],
        ToolKind::BlackbodyError => &[(
            1,
            "采用 Planck 辐射平衡，示值误差=亮温−真温，修正值取相反数。\n按所选单波长或波段响应计算；波段内响应假设平坦。",
            "B(T亮)=ε源B(T真)+(1−ε源)B(T背景)。B 为所选光谱响应下的辐射量，公式温度用 K。",
        )],
        ToolKind::RadiationCorrection => &[(
            1,
            "采用 Planck 辐射平衡，修正值=真温−示值，修正后温度=示值+修正值。\n按所选单波长或平坦波段响应计算。",
            "ε源B(T真)+(1−ε源)B(T背景)=ε表B(T示)+(1−ε表)B(T背景)。\n公式温度用 K，界面温度用 °C。",
        )],
        ToolKind::ResultantError => &[(
            1,
            "相对压力误差先按量程跨度折算为绝对误差，再统一单位作平方和根合成。",
            "绝对误差=量程跨度×相对误差/100；合成值=√Σe[i]²，最坏情况界限=Σ|e[i]|。",
        )],
        ToolKind::Uncertainty => &[
            (
                1,
                "A 类按重复测量评定；B 类各项共用所选分布和 B 类灵敏系数。\n分量按不相关合成，U=k·uc；正态项数值按所选 k=2 或 3 折算。",
                "uA=s/√n；uB=√Σ(a[i]/d)²。uc=√[(cA·uA)²+(cB·uB)²]。\nd：均匀 √3、三角 √6、正态 2 或 3、两点 1、反正弦 √2。\n非正态项填半宽，正态项填扩展不确定度数值。不自动推算有效自由度或置信水平。",
            ),
            (
                2,
                "A 类按重复测量评定；B 类各项分别设置分布与灵敏系数。\n正态项填证书扩展不确定度 U，其余填区间半宽。分量按不相关合成，U=k·uc。",
                "A 类 u=s/√n；B 类按所选分布换算为标准不确定度。\n各项贡献 q=c·u；uc=√Σq²。不自动推算有效自由度或置信水平。",
            ),
        ],
        _ => &[],
    }
}

pub(crate) fn selected(kind: ToolKind, mode: usize) -> (&'static str, String) {
    if let Some((_, summary, formula)) = variants(kind).iter().find(|item| item.0 == mode) {
        (summary, (*formula).to_owned())
    } else {
        (for_tool(kind), formula(kind))
    }
}
