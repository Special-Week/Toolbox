# 温标参考数据

## 热电偶

`type_*.ref.txt` 保存 B、E、J、K、N、R、S、T 的 ITS-90 **正向**参考函数，输入为 °C，输出为 mV，参考端为 0 °C。K 型正温区含额外的指数项；各温区及多项式阶数以文件中的 `range:` 行为准。反算使用同一正向函数，证书偏差、固定点校准和工业等级允差另由相应模型处理。

发布来源为 [NIST Monograph 175（1993）](https://doi.org/10.6028/NIST.MONO.175)。[NIST SRD 60 热电偶表](https://its90.nist.gov/Thermocouples) 提供按分度号下载的原始表，每个表附有正向参考函数一节。

2026-11-22 同时核对了八个分度号的温区端点、多项式阶数、全部正向系数和 K 型指数项。此目录的数值与 C++ 的 [内嵌表](../../cpp_version/src/core/sensor_data.cpp) 均和 SRD 60 下载表一致。Rust 桌面及 Web/Wasm 直接共享此目录的数据。

## 标准铂电阻（SPRT）

[BIPM《Guide to the Realization of the ITS-90 — Platinum Resistance Thermometry》（2021）](https://www.bipm.org/documents/20126/41773843/Guide_ITS-90_5_SPRT_2021.pdf) §2.2 的式 (7)–(10) 和表 1 定义参考电阻比 `Wr` 的低温和高温参考函数。A/C 用于由温度求 `Wr`，B/D 用于参考反算，分界为水三相点 273.16 K（0.01 °C）。

2026-11-22 对照表 1 核对了 [Rust/Web](../../src/calculators/sprt/curves.rs) 与 [C++](../../cpp_version/src/core/sensor_data.cpp) 中的 A、B、C、D 共 49 个参考系数，两侧全部一致。软件按所选子温区和证书偏差系数计算，执行时的温区以工具页面所选模型为准。
