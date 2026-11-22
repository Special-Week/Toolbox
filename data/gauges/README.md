# 量规内置数据

用于 M、Tr、UN、R/NPT 管螺纹、钢筋直螺纹和光滑量规计算。数据在编译时嵌入程序，构建前须保留；运行时无需外部数据文件。

## 数据文件

| 文件 | 内容 |
|---|---|
| `tables.json` | 公差、基本偏差、量规尺寸、长度与规格表 |
| `unified.json` | UN 系列及中径制造公差 U，公差原始单位为 inch |

Rust 计算代码位于 `src/calculators/gauges/`，C++ 数据副本位于 `cpp_version/resources/gauges/`。

## 标准出处与核对状态

2026-11-22 已核对以下标准的公开目录。内置表的来源版次、表号和逐项数值仍需与完整标准和原始录入资料对照。

| 工具 | 官方目录与待核对内容 |
|---|---|
| M、钢筋直螺纹 | [GB/T 3934-2025](https://openstd.samr.gov.cn/bzgk/std/newGbInfo?hcno=CB961A1456EDADCED21C0779BBA996DD) 已于 2026-05-01 实施，[GB/T 197-2018](https://openstd.samr.gov.cn/bzgk/std/newGbInfo?hcno=4B9C1010476FF8385A59AD3076EC3A68) 为普通螺纹公差。需核对 M 表采用的版次及新版量规尺寸变化。钢筋直螺纹借用这些公差和量规要素，钢筋专用规格表的来源仍待确认。 |
| Tr | [GB/T 8124-2004](https://openstd.samr.gov.cn/bzgk/std/newGbInfo?hcno=121A61B6B3C7DAFDCAF237AB2633FD5E) 为梯形螺纹量规技术条件，[GB/T 5796.4-2022](https://openstd.samr.gov.cn/bzgk/std/newGbInfo?hcno=515A70D2BBF5649D13A2D0F0CCF6CFFB) 为梯形螺纹公差。需对照不同线数、外小径公差带和量规长度表。 |
| UN | [ASME B1.1](https://www.asme.org/codes-standards/find-codes-standards/b1-1-unified-inch-screw-threads-un-unr-thread-form) 涵盖统一英制螺纹，[ASME B1.2](https://www.asme.org/codes-standards/find-codes-standards?designatorCategory=B1&page=1&perPage=25&sortBy=designator&sortByDir=asc&type=Standards) 涵盖相关量规。需确认 `unified.json` 的 U 值和校对规尺寸对应的版次与表号。 |
| R / NPT | R 模型注明 [JB/T 10031-2019](https://std.samr.gov.cn/hb/search/stdHBDetailed?id=C362C05F3EBAA68DE05397BE0A0A9B00)。[ASME B1.20.1](https://www.asme.org/codes-standards/find-codes-standards/b1201-pipe-threads-general-purpose-inch) 的目录确认范围包含 NPT。需核对内置尺寸和量规表，尤其是暂缺的 NPT 6 inch 环规中径。 |
| 光滑量规 | [GB/T 1957-2006](https://openstd.samr.gov.cn/bzgk/std/newGbInfo?hcno=19CD017560D9E213D73150AC76C10DE4) 规定技术条件，[JJG 343-2012](https://www.samr.gov.cn/cms_files/filemanager/samr/www/samrnew/samrgkml/nsjg/jls/202112/W020211229469641399704.pdf) 为检定规程。需逐项对照基本偏差、量规公差和等级相关 Δ 修正。 |

## 取值约定

- `99999`、`-100000` 和空值表示缺失，不能作为尺寸或零公差；负偏差保留符号。
- 尺寸计算保留双精度，最终显示五位小数；公差与长度取档以页面提示为准。
- UN 工件及工作量规中径计算不作中间修约。塞规与环规共用制造公差 U；**16 TPI、8＜D≤12 inch 的 U 取 0.0006 inch**。

## 数据范围与限制

- **M**：`td` 为外螺纹大径公差；小径参考界限不等同于独立分配制造公差的量规尺寸。
- **Tr**：内螺纹小径采用 4 级；单线外螺纹小径按 c/e 带查表，h 带及多线外小径修正未收录。
- **UN**：大径、小径和校对尺寸为补充参考；未收录独立磨损储备，校对规提供 TT、TZ、ZT、ZZ。
- **钢筋直螺纹**：套筒与丝头分别借用 6H/6f 公差；多头仅改变导程和升角。
- **光滑量规**：不计算需要等级相关 Δ 修正的 P～ZC 6、7 级孔公差带。四处孔偏差在 `lookup.rs` 修正（μm）：0～3 mm F=6；24～30 mm D=65；65～80 mm D=100；100～120 mm D=120。
- **管螺纹**：按现有 R/NPT 规格表查询；NPT 6 inch 环规原表中径存疑，暂不输出。

数据与模型不覆盖标准的全部条款，制造与判定须结合执行标准和图纸。
