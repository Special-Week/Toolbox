//! 专业用户说明与详细公式分开维护。不同入口的口径单独列出。
use crate::catalog::{Dimension, ToolKind};

pub(crate) type NoteVariant = (usize, &'static str, &'static str);

pub(crate) fn for_tool(kind: ToolKind) -> &'static str {
    match kind {
        ToolKind::PdfDownload => "",
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
            "ΔH 为被检表相对标准器的高度，向上为正。标准器读数移至被检高度的修正量等于标准器与被检表压力差的相反数。\n气体可填当地密度或由标准状态密度换算；使用标准状态密度的表压模式扣除环境气柱影响。"
        }
        ToolKind::AltitudePressure => {
            "按 1976 标准大气换算绝对气压与位势高度，范围 −1000～86000 m。\n结果不含当地气象修正，不等同于实际几何海拔。"
        }
        ToolKind::HumidityConversion => {
            "输入环境温度、绝对气压及一个已知湿度参数。湿球系数 A=0.000662；饱和压在 ≤0 °C 时按冰面计算，其余按水面。\n霜点仅在冰面公式反算温度不高于 0 °C 时显示；混合比以干空气质量计，比湿以湿空气总质量计；标准状态含水量按 0 °C、101.325 kPa。"
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
            "各项标准不确定度乘灵敏系数后，可按不相关分量的平方和根合成，也可取最大绝对贡献；U=k·uc。\nB 类正态项填证书扩展不确定度 U，其余分布填区间半宽。包含因子 k 不自动对应置信水平。"
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
        ToolKind::PdfDownload => "",
        ToolKind::Gauge(kind) => return super::gauges::formula(kind),
        ToolKind::Unit(Dimension::Temperature) => {
            "先检查输入不得低于该温标的绝对零度，再将输入温度 value 换算为开尔文 K=max(value×fₛ+oₛ, 0)，再求目标温度 (K−oₜ)/fₜ；同单位直接返回输入值。这里是温度值换算，不适用于温差。"
        }
        ToolKind::Unit(_) => {
            "线性单位按目标值 = 输入值 × (源单位系数 fₛ / 目标单位系数 fₜ) 直接换算，同单位直接返回输入值；各系数以本类别的同一基准单位定义。"
        }
        ToolKind::WorkingThermocouple => {
            "先按分度号及温度选择对应的参考函数分段：E(t)=Σc[i]·t^i。\nK 型 0 °C 以上分段还需加 a₀·exp[a₁(t−a₂)²]；t 用 °C，E 用 mV。\nE测=E(t热端)−E(t参考端)；反算先加回参考端电势，再在正向参考函数的有效单调区间内二分求解。各分段系数和本次代入值见计算过程。\n等级允差按 IEC 60584-1 及各等级温区计算，温度界限为 t±Δt。"
        }
        ToolKind::StandardThermocouple => {
            "S 型：锌、铝、铜点相对参考曲线的偏差拟合二次式，400 °C 以下按锌点偏差线性延伸，两段在 400 °C 不保证连续。\nB 型：低于 23 °C 修正量为零，23 °C 至低于 800 °C 按首点线性计算，800 °C 起使用首点偏差；两段在 800 °C 不保证连续。1100～1500 °C 五点偏差作分段及过渡插值。固定点反算分段求解，有多个温度时提示核对校准点或使用限定温区的系数换算。\nE测=E(t热端)−E(t参考端)。"
        }
        ToolKind::IndustrialResistance => {
            "自定义 CVD：t≥0 时 R=R₀(1+At+Bt²)；t<0 时 R=R₀[1+At+Bt²+C(t−100)t³]。\n允差电阻界限为 R(t−Δt) 至 R(t+Δt)。"
        }
        ToolKind::StandardPlatinumResistor => {
            "Wr=W−ΔW。高温 ΔW=a(W−1)+b(W−1)²+c(W−1)³；A6 在 W>W660 时加 d(W−W660)²，W660 为铝点电阻比。\n低温 a4,b4：ΔW=a4(W−1)+b4(W−1)ln(W)；a5,b5：ΔW=a5(W−1)+b5(W−1)²。再由参考函数求温度。"
        }
        ToolKind::Thermistor => {
            "电阻→温度：1/T=ΣA[i]·ln(R)^i。\n温度→电阻：ln(R)=ΣB[i]·(1/T)^i。\n支持 3～6 阶；Steinhart–Hart 形式令二次项为 0。A/B 不保证互为精确反函数；温度残差=证书温度−反算温度。"
        }
        ToolKind::PressureReference => {
            "被检允差=被检跨度×被检准确度/100。\n数字标准器允差=标准器跨度×标准器准确度/100。\n活塞压力计允差=标准器准确度/100×max(被检跨度, 标准器跨度的10%)。"
        }
        ToolKind::TransmitterReference => {
            "设 P 为被检变送器压力输入量程跨度，S 为其电信号输出量程跨度，Δp标为所选压力标准器的允许误差。压力分量 aₚ=Δp标·S/P；电测分量 aₑ=读数准确度/100·max(|输出下限|,|输出上限|)+固定误差，覆盖整个输出范围。\nU=2√(aₚ²+aₑ²)/√3；通过条件 U<S×被检准确度/100/量传比，同时核对压力范围与允差。"
        }
        ToolKind::HeightCorrection => {
            "ρ 用 kg/m³，g 用 m/s²，ΔH 用 m，压差单位 Pa。\n液体、已知当地密度的气体及绝压气体：p标准−p被检=ρgΔH；标准状态密度换算为当地气体密度：ρ=ρ₀·(p绝/101325)·(273.15/T)，T 用 K。\n由标准状态密度换算的表压气体：p标准−p被检=ρgΔH−环境气柱压差；环境气柱按标准大气估算。气体按局部恒定密度近似；等效水柱按 4 °C 水换算。"
        }
        ToolKind::AltitudePressure => {
            "分层界面为 11、20、32、47、51、71 km，按恒温或恒温度梯度关系计算。空气密度 ρ=p/(287.05287T)，p 用 Pa，T 用 K。"
        }
        ToolKind::HumidityConversion => {
            "环境及湿球温度 t 用 °C、气压 p 和水汽分压 e 用 Pa，t 范围 −100～100 °C。饱和压 ps(t) 分段计算：t≤0：611.2·exp[22.46t/(272.62+t)]；0<t<1：611.2·exp[a₁t/(243.12+t)]；t≥1：10^[(10.286T−2148.4909)/(T−35.85)]，其中 T=t+273.15 K。a₁=244.12·ln[ps(1)/611.2]，使两段水面公式连续。\n已知相对湿度时 e=RH·ps(t)/100；已知露/霜点时 e=ps(t点)；已知湿球时 e=ps(t湿)−0.000662p(t−t湿)；已知混合比 m（g/kg）时 q=(m/1000)·28.959/18.0152，e=pq/(1+q)。湿球反算使用同一方程的有界求解。若在 −100 °C 至环境温度内无连续解，则提示无法反算，不输出结果。\nRH=100e/ps；体积分数=e/p（L/L）；以干空气计的体积比=e/(p−e)×10⁶（μL/L）；质量比=体积比×18.0152/28.959（μg/g）；混合比=质量比/1000（g/kg）；比湿=1000×质量比/(10⁶+质量比)（g/kg）。绝对湿度=e×18.015/(8.31451T)（g/m³）；标准状态含水量=绝对湿度×T×101325/(273.15p)。\n当 e>0，令 z=ln(e)−ln(611.2)：霜点=272.62z/(22.46−z)；e<ps(1) 时水面露点=243.12z/(a−z)，e≤611.2 Pa 时 a=17.62，否则 a=a₁；e≥ps(1) 时露点=[−2148.4909+35.85log₁₀(e)]/[log₁₀(e)−10.286]−273.15。非正露/霜点输入按冰面处理；RH=0 时无有限露/霜点。水汽分压须小于总压。由其他湿度参数反算的 RH 若因数值误差仅略微超出 0%～100%，则将水汽分压同时归整至对应边界。"
        }
        ToolKind::EarBlackbody => {
            "D=ε黑+(1−ε黑)ε探。\nT亮⁴=[ε黑T黑⁴+(1−ε黑)ε探T探⁴]/D。公式温度均用 K。"
        }
        ToolKind::BlackbodyError => {
            "T 为输入黑体温度的绝对温度，T背景为背景绝对温度，ε 为黑体发射率。C=1/[exp(c₂/(λT))−1]−(1−ε)/[exp(c₂/(λT背景))−1]。\nM=c₂/[λ·ln(1+ε/C)] 为模型反算的温度，发射率误差=T−M；ε=1 时直接取误差 0，其他情况须有 C>0。c₂=14388 μm·K，λ 用 μm，温度用 K。\n波段有效波长：8～14 μm 取 9.6+221/T；7～18 μm 取 8.7+525/T。"
        }
        ToolKind::RadiationCorrection => {
            "设 T 为示值的绝对温度、T背景为背景绝对温度，c=14388/λ。对每个发射率 ε 分别计算 C=1/[exp(c/T)−1]−(1−ε)/[exp(c/T背景)−1]，M=c/ln(1+ε/C)，F(ε)=T−M；ε=1 时直接取 F=0，其他情况须有 C>0。\n未修约修正值 ΔT=F(ε表)−F(ε源)，两项均使用同一示值温度和波长；ΔT 取至 0.01 °C 后加到示值。8～14 μm 波段取 λ=9.6+221/T，7～18 μm 波段取 λ=8.7+525/T；T 用 K，λ 用 μm，c 的单位为 K。"
        }
        ToolKind::Repeatability => {
            "s=√[Σ(xᵢ−x̄)²/(n−1)]，x̄ 为测量均值；极差=最大值−最小值。均值为 0 时 RSD 无定义。"
        }
        ToolKind::RangeMethod => {
            "R=最大值−最小值；重复性标准差 s≈R/C，平均值标准差 s均值≈s/√n。n=2～9 的 C 依次为 1.13、1.69、2.06、2.33、2.53、2.70、2.85、2.97，采用两位小数表值。"
        }
        ToolKind::ResultantError => {
            "各项贡献 q[i]=c[i]e[i]；合成值=√Σq[i]²，最坏情况界限=Σ|q[i]|。"
        }
        ToolKind::ErrorAnalysis => {
            "相对误差=Δ/参考值×100%；引用误差=Δ/引用值×100%；误差绝对值为 |Δ|。分母为 0 时对应百分误差无定义。"
        }
        ToolKind::Uncertainty => {
            "A 类：贝塞尔法 u=s/√n，s 由 2～50 个测量值计算；n 是另行填写的取测量次数，留空按 1 次。极差法 u=R/(C√m)，m 是测量值个数，支持 2～9 个测量值。\nB 类：均匀 u=a/√3；三角 u=a/√6；两点 u=a；反正弦 u=a/√2；正态 u=U/k证书（按该分量选择 2 或 3），与合成结果的包含因子 k 分别设置。\n贡献 q=c·u；分组逐层合成后乘本层灵敏系数。各组和总合成可选平方和根或最大绝对贡献；“最大贡献”仅用于主导分量估计。\n修约按 uc、U 的较大值确定小数位，可修约至十位、百位等；修约后 U=k×修约后的 uc。不自动计算有效自由度。"
        }
        ToolKind::Transmitter => {
            "f=(被测量−量程下限)/量程跨度，s=(信号−输出下限)/输出跨度。\n线性：s=f；开方：s=√f，反算 f=s²。"
        }
        ToolKind::BSplineInterpolation => {
            "每段使用四个相邻控制点 P₀～P₃ 和局部参数 0≤u≤1：C(u)=ΣBᵢ(u)Pᵢ。B₀=(1−u)³/6；B₁=(3u³−6u²+4)/6；B₂=(−3u³+3u²+3u+1)/6；B₃=u³/6。x 和 y 分别按同一基函数计算。\n内部控制点由 Pᵢ₋₁+4Pᵢ+Pᵢ₊₁=6Qᵢ 求解；自然端点要求 x、y 关于 u 的二阶导数为 0。给定 x 时先在相应分段求 x(u)=x，再代入 y(u)。外推遇 x 的首个转折即停止。\n采样从最小 x 按步进生成，末尾包含最大 x。"
        }
        ToolKind::CalibrationCurveFit => {
            "拟合 ŷ=ΣK[j]x^j，选择系数使 Σ(y−ŷ)² 最小；过原点模型固定 K0=0。计算先做坐标归一化，再以列缩放和 QR 求解，显示系数换回原单位。\n残差 e=y−拟合 y。r=Σ[(x−均值 x)(y−均值 y)]/√[Σ(x−均值 x)²·Σ(y−均值 y)²]。\nR²=1−Σe²/Σ(y−均值 y)²；带截距的一次拟合在 r 有定义时满足 R²=r²。\nRMSE=√(Σe²/n)，残差标准差=√[Σe²/(n−p)]，p 为估计参数个数。n=p 时残差标准差无定义；x 或 y 不变时 r 无定义；y 不变时 R² 无定义。\n允许重复 x；过原点一次拟合至少需要一个非零 x，含截距的模型须有足够多的不同 x。拟合结果须结合残差与测量范围评价；不自动加权、扣除空白或剔除异常点。"
        }
    };
    if matches!(kind, ToolKind::CalibrationCurveFit) {
        format!(
            "{text}\n实际拟合值和残差使用归一化模型计算，以减少原单位展开系数的数值抵消；直接代入展示系数的结果可能存在舍入差异。"
        )
    } else {
        text.to_owned()
    }
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
            "温区系数按 E(t)=Σc[i]t^i 计算，t 用 °C，E 用 mV；0 °C 特别规定为 0 mV。含 0 °C 的系数温区要求常数项 C0 为零。\n校准点拟合只修正一次、二次项；反算要求曲线在所选反算温区内严格单调，B 型采用 ≥50 °C 分支。",
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
            "B(T亮)=ε源B(T真)+(1−ε源)B(T背景)。单波长辐射量 Bλ(T) ∝ λ⁻⁵/[exp(c₂/(λT))−1]；λ 用 μm，T 用 K，c₂=hc/k，由 SI 定义常数换算为 μm·K。\n波段假设响应平坦，对 Bλ 在所选波段采用 256 等分的复合辛普森积分；共同辐射常数在平衡方程中约去。按辐射量二分反算 T亮，再计算示值误差 T亮−T真及其相反的修正值。",
        )],
        ToolKind::RadiationCorrection => &[(
            1,
            "采用 Planck 辐射平衡，修正值=真温−示值，修正后温度=示值+修正值。\n按所选单波长或平坦波段响应计算。",
            "ε源B(T真)+(1−ε源)B(T背景)=ε表B(T示)+(1−ε表)B(T背景)。单波长 Bλ(T) ∝ λ⁻⁵/[exp(c₂/(λT))−1]；λ 用 μm，T 用 K，c₂=hc/k，由 SI 定义常数换算为 μm·K。\n波段假设响应平坦，对 Bλ 在所选波段采用 256 等分的复合辛普森积分；共同辐射常数约去。解出 B(T真) 后通过二分反算 T真，修正值=T真−T示；界面温度用 °C。",
        )],
        ToolKind::ResultantError => &[(
            1,
            "相对压力误差先按量程跨度折算为绝对误差，再统一单位作平方和根合成。",
            "相对误差先按绝对误差=量程跨度×相对误差/100 换算；所有分量统一到结果单位后，合成值=√Σe[i]²。此模式只显示平方和根结果。",
        )],
        ToolKind::Uncertainty => &[
            (
                1,
                "A 类按重复测量评定；B 类各项共用所选分布和 B 类灵敏系数。\n分量按不相关合成，U=k·uc；正态项数值按所选 k=2 或 3 折算。",
                "uA=s/√n，s 用输入的重复测量值计算，n 为取测量次数，留空按 1 次；不使用 A 类时取 0。各 B 类原始值 uᵢ=a[i]/d，按各自设置修约后计算 uB=√Σuᵢ²；uA 同样按设置修约后参与 uc=√[(cA·uA)²+(cB·uB)²]。\nd：均匀 √3、三角 √6、正态 2 或 3、两点 1、反正弦 √2。\n非正态项填半宽，正态项填扩展不确定度数值。不自动推算有效自由度或置信水平。",
            ),
            (
                2,
                "A 类按重复测量评定；B 类各项分别设置分布与灵敏系数。\n正态项填证书扩展不确定度 U，其余填区间半宽。分量可按不相关项平方和根合成或取最大绝对贡献，U=k·uc。",
                "A 类贝塞尔法 u=s/√n，n 为取测量次数，留空按 1 次；极差法 u=R/(C√m)，m 为测量值个数。B 类按所选分布换算为标准不确定度。\n各项贡献 q=c·u；按选定方式取平方和根或最大绝对贡献。不自动推算有效自由度或置信水平。",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::TOOLS;

    #[test]
    fn every_calculator_and_explicit_mode_has_a_complete_selectable_explanation() {
        let mut calculators = 0;
        let mut modes = 0;
        for tool in TOOLS {
            if matches!(tool.kind, ToolKind::PdfDownload) {
                continue;
            }
            calculators += 1;
            let (summary, details) = selected(tool.kind, 0);
            assert!(
                !summary.is_empty() || !details.is_empty(),
                "{} 缺少说明",
                tool.id
            );
            assert!(!details.is_empty(), "{} 缺少公式", tool.id);
            let mut seen = std::collections::HashSet::new();
            for &(mode, summary, details) in variants(tool.kind) {
                assert!(
                    mode > 0 && seen.insert(mode),
                    "{} 模式编号无效或重复",
                    tool.id
                );
                assert!(
                    !summary.is_empty() && !details.is_empty(),
                    "{} 模式 {mode} 缺少说明",
                    tool.id
                );
                assert_eq!(selected(tool.kind, mode), (summary, details.to_owned()));
                modes += 1;
            }
            assert_eq!(selected(tool.kind, usize::MAX), selected(tool.kind, 0));
        }
        assert_eq!(calculators, 49);
        assert!(modes >= 10);
    }

    #[test]
    fn uncertainty_notes_cover_both_combination_rules() {
        let text = formula(ToolKind::Uncertainty);
        assert!(text.contains("平方和根或最大绝对贡献"));
        assert!(text.contains("k证书"));
        assert!(text.contains("分别设置"));
        assert_eq!(super::super::uncertainty::model::divisor(2), 2.0);
        assert_eq!(super::super::uncertainty::model::divisor(3), 3.0);
        let rss = super::super::uncertainty::model::combine(&[3.0, -4.0], false).unwrap();
        let maximum = super::super::uncertainty::model::combine(&[3.0, -4.0], true).unwrap();
        assert_eq!(rss, 5.0);
        assert_eq!(maximum, 4.0);
        assert!(
            selected(ToolKind::Uncertainty, 2)
                .0
                .contains("最大绝对贡献")
        );
    }

    #[test]
    fn humidity_notes_follow_saturation_and_humidity_boundaries() {
        use super::super::humidity::website::{calculate, saturation};
        let note = formula(ToolKind::HumidityConversion);
        assert!(note.contains("t≤0"));
        assert!(note.contains("0<t<1"));
        assert!(note.contains("t≥1"));
        assert!(note.contains("ln(e)−ln(611.2)"));
        assert!(note.contains("无连续解"));
        assert!(calculate(-100.0, 101325.0, 0, 50.0).is_err());
        for t in [
            -100.0_f64, -0.000001, 0.0, 0.000001, 0.5, 0.999999, 1.0, 20.0, 100.0,
        ] {
            let expected = if t <= 0.0 {
                611.2 * (22.46 * t / (272.62 + t)).exp()
            } else if t < 1.0 {
                let coefficient = 244.12 * (saturation(1.0).unwrap() / 611.2).ln();
                611.2 * (coefficient * t / (243.12 + t)).exp()
            } else {
                10.0_f64.powf((10.286 * (t + 273.15) - 2148.4909) / (t + 273.15 - 35.85))
            };
            assert_eq!(saturation(t).unwrap(), expected);
        }
        assert!(saturation(-100.0001).is_err());
        assert!(saturation(100.0001).is_err());
        assert!(calculate(20.0, 101325.0, 0, -1e-11).is_err());
        assert!(calculate(20.0, 101325.0, 0, 100.0 + 1e-11).is_err());
        let dry = calculate(20.0, 101325.0, 0, 0.0).unwrap();
        assert!(dry.dew.is_none() && dry.frost.is_none());
    }

    #[test]
    fn standard_thermocouple_note_includes_zero_reference_exception() {
        let (note, _) = selected(ToolKind::StandardThermocouple, 1);
        assert!(note.contains("0 °C 特别规定为 0 mV"));
        let couple = super::super::thermocouple::standard::StandardCouple {
            kind: "S",
            points: vec![],
            zone: Some((100.0, 200.0, vec![1.0, 2.0, 3.0])),
        };
        assert_eq!(couple.emf(0.0).unwrap(), 0.0);
        assert!(couple.emf(99.0).is_err());
        assert_eq!(
            couple.emf(100.0).unwrap(),
            1.0 + 2.0 * 100.0 + 3.0 * 100.0_f64.powi(2)
        );
    }

    #[test]
    fn platinum_note_describes_electrical_ratio_threshold() {
        let note = formula(ToolKind::StandardPlatinumResistor);
        assert!(note.contains("W>W660"));
        let model = super::super::sprt::model::Sprt {
            d: 0.01,
            ..Default::default()
        };
        let mut without = model;
        without.d = 0.0;
        assert_eq!(
            model.reference_ratio(model.w660, true),
            without.reference_ratio(model.w660, true)
        );
        let w = model.w660 + 0.5;
        assert!(
            (without.reference_ratio(w, true) - model.reference_ratio(w, true) - 0.01 * 0.25).abs()
                < 1e-14
        );
    }

    #[test]
    fn pressure_note_matches_coverage_ratio_and_piston_boundaries() {
        use super::super::pressure::{model::Range, selection_model::verify};
        let note = formula(ToolKind::PressureReference);
        assert!(note.contains("max(被检跨度, 标准器跨度的10%)"));
        let reference = Range::new(0.0, 10.0).unwrap();
        for high in [0.999, 1.0, 1.001] {
            let dut = Range::new(0.0, high).unwrap();
            let result = verify(dut, reference, 1.0, 0.1, 3.0, true).unwrap();
            assert_eq!(result.reference_tolerance, 0.1 / 100.0 * high.max(1.0));
        }
        let dut = Range::new(0.0, 1.0).unwrap();
        let same = Range::new(0.0, 1.0).unwrap();
        let at = verify(dut, same, 3.0, 1.0, 3.0, false).unwrap();
        assert!(at.coverage && at.meets);
        assert!(!verify(dut, same, 3.0, 1.0, 3.000001, false).unwrap().meets);
        assert!(
            !verify(dut, Range::new(0.1, 1.0).unwrap(), 3.0, 0.1, 3.0, false)
                .unwrap()
                .meets
        );
        let note = selected(ToolKind::TransmitterReference, 1);
        assert!(note.0.contains("两通道"));
        assert!(note.1.contains("量程跨度"));
        let note = formula(ToolKind::TransmitterReference);
        assert!(note.contains("被检变送器压力输入量程跨度"));
        let standard = Range::new(0.0, 2.0).unwrap();
        let output = Range::new(4.0, 20.0).unwrap();
        let result = super::super::pressure::selection_model::transmitter(
            dut, standard, output, 1.0, 0.01, 0.0, 0.0, 3.0,
        )
        .unwrap();
        assert!(
            (result.pressure_contribution
                - result.pressure.reference_tolerance * output.span / dut.span)
                .abs()
                < 1e-14
        );
    }

    #[test]
    fn radiation_notes_distinguish_approximation_and_planck_boundaries() {
        use super::super::radiation::{
            model::{Response, Spectrum},
            website,
        };
        let (effective, _) = selected(ToolKind::BlackbodyError, 0);
        let (planck, planck_formula) = selected(ToolKind::BlackbodyError, 1);
        assert!(effective.contains("≤200 °C"));
        assert!(planck.contains("Planck"));
        assert!(planck_formula.contains("256 等分"));
        assert!(website::error(0.99, 0.65, 1000.0, 200.0).is_ok());
        assert!(website::error(0.99, 0.65, 1000.0, 200.000001).is_err());
        assert!(website::error(0.0, 0.65, 1000.0, 20.0).is_err());
        assert_eq!(website::error(1.0, 0.65, 1000.0, 20.0).unwrap(), 0.0);
        let correction_note = formula(ToolKind::RadiationCorrection);
        assert!(correction_note.contains("F(ε)=T−M"));
        assert!(correction_note.contains("其他情况须有 C>0"));
        assert!(correction_note.contains("ΔT=F(ε表)−F(ε源)"));
        let instrument = website::error(0.95, 0.65, 1000.0, 20.0).unwrap();
        let source = website::error(0.99, 0.65, 1000.0, 20.0).unwrap();
        assert_eq!(
            website::correction(0.95, 0.99, 0.65, 1000.0, 20.0).unwrap(),
            instrument - source
        );
        assert!(Spectrum::new(Response::Single(0.1)).is_ok());
        assert!(Spectrum::new(Response::Single(100.0)).is_ok());
        assert!(Spectrum::new(Response::Single(0.099)).is_err());
        assert!(Spectrum::new(Response::Band(8.0, 8.0)).is_err());
        assert!(
            Spectrum::new(Response::Band(8.0, 14.0))
                .unwrap()
                .apparent(1000.0, 201.0, 0.99)
                .is_ok()
        );
    }
}
