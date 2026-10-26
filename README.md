# 工具箱

**Toolbox** 是面向计量、仪表及分析检测工作的离线计算工具，包含 **7 个分类、49 个工具**。

- **Rust 桌面版**：本目录，使用 egui 构建界面。
- **[C++ 桌面版](cpp_version/README.md)**：Windows 单文件程序，支持 x64、x86 构建。
- **[网页版](web_version/README.md)**：使用 Vue 3 与 Rust/WebAssembly，适配电脑和手机，可部署为静态网站。

## 功能分类

| 分类 | 工具 |
|---|---|
| 单位换算（22） | 压力、温度、质量流量、体积流量、质量、长度、面积、容积、密度、速度、力、能量、功率、传热系数、导热系数、动力粘度、运动粘度、扩散系数、表面张力、传热速度（热流密度）、扭矩、浓度 |
| ITS90 温标换算（5） | 工业热电偶、工业热电阻、标准热电偶、标准铂电阻、热敏电阻 |
| 压力工具（4） | 压力表标准器选型、压力变送器标准器选型、高度差修正、气压高度换算 |
| 温湿度工具（4） | 湿度换算、耳温计黑体发射率修正、黑体辐射源发射率误差、辐射温度计示值修正 |
| 误差工具（5） | 重复性计算、极差法计算、合成误差、误差分析、不确定度计算 |
| 量规计算（6） | 公制普通螺纹 M、梯形螺纹 Tr、美制统一螺纹 UN、R/NPT 管螺纹、钢筋直螺纹、光滑量规 |
| 其他工具（3） | 变送器、三次均匀 B 样条插值、校准曲线拟合 |

单位换算包含 256 项单位；温标工具支持双向换算、校准拟合、适用型号的等级允差和分度表。不确定度支持 A/B 类分量、独立分布与灵敏系数、分组预算；校准曲线支持一次、过原点一次、二次和三次拟合，并输出系数、相关性和残差。

## 构建与运行

需要支持 Rust 2024 Edition 的工具链。Windows 使用 MSVC 工具链时，还需安装 C++ 构建工具和 Windows SDK。在项目根目录执行：

```powershell
# 开发运行
cargo run --locked --bin Toolbox

# 发布构建
cargo build --locked --release --bin Toolbox
```

Windows 程序生成在 `target/release/Toolbox.exe`。数据表与图标已内嵌，运行时无需附带 `data/`。

## 生成 Windows 发布包

完成发布构建后，在项目根目录执行：

```powershell
New-Item -ItemType Directory -Force dist | Out-Null
Copy-Item target/release/Toolbox.exe, README.md, LICENSE, licenses/PsychroLib-LICENSE.txt dist/
Compress-Archive -Path dist/Toolbox.exe, dist/README.md, dist/LICENSE, dist/PsychroLib-LICENSE.txt -DestinationPath dist/Toolbox-windows.zip -Force
```

发布 `dist/Toolbox-windows.zip` 即可。其他版本的构建与部署见 [C++ 版 README](cpp_version/README.md) 和 [网页版 README](web_version/README.md)。

## 项目结构

```text
Toolbox/
├── Cargo.toml                # 包信息、依赖、功能开关和可执行目标
├── Cargo.lock                # 锁定依赖版本
├── build.rs                  # 构建时嵌入 Windows 图标和版本信息
├── LICENSE                   # MIT 许可证
├── .gitignore                # 构建产物与本地文件的忽略规则
├── src/
│   ├── main.rs               # 程序入口、窗口配置与界面主题
│   ├── app.rs                # 应用状态、输入保存、页面调度与后台任务
│   ├── app/dashboard.rs      # 首页工具卡片、搜索与分类筛选
│   ├── catalog.rs            # 分类、工具名称、简介及页面映射
│   ├── navigation.rs         # 侧栏导航、分类折叠与工具选择
│   ├── fonts.rs              # 中文和数学符号的字体配置
│   ├── icon.rs               # 窗口图标及页眉图片加载
│   ├── units/
│   │   ├── mod.rs            # 单位类型、换算及输入校验
│   │   ├── data.rs           # 单位名称、符号和换算系数
│   │   └── notes.rs          # 各类单位的换算约定
│   └── calculators/
│       ├── mod.rs            # 计算模块声明
│       ├── common.rs         # 共用输入控件、结果卡片和说明展示
│       ├── explanations.rs   # 各工具的计算说明及详细公式
│       ├── format.rs         # 数值解析与普通小数格式化
│       ├── numerics.rs       # 稳定统计、平方和根及有限值检查
│       ├── calibration.rs    # 多项式、区间反算和 QR 拟合
│       ├── math.rs           # CVD 基函数、方程求解和标准大气模型
│       ├── axis.rs           # 曲线坐标轴、刻度及标签
│       ├── division_table.rs # 分度表、分页预览与 CSV 导出
│       ├── range_method.rs   # 极差法系数与测量次数校验
│       ├── its90.rs          # 工业热电偶、CVD 拟合及 β 模型页面
│       ├── errors.rs         # 重复性、极差法、误差及快速不确定度页面
│       ├── thermocouple.rs / thermocouple/  # 热电偶曲线、标准热电偶及允差
│       ├── rtd.rs / rtd/                    # 工业热电阻曲线、换算及允差
│       ├── sprt.rs / sprt/                  # 标准铂电阻分区、参考函数及校准
│       ├── thermistor.rs / thermistor/      # 热敏电阻多项式、拟合及页面
│       ├── pressure.rs / pressure/          # 标准器选型、压力合成与高差修正
│       ├── humidity.rs / humidity/          # 湿度换算页面与湿空气模型
│       ├── radiation.rs / radiation/        # 发射率误差与辐射温度修正
│       ├── transmitter.rs / transmitter/    # 变送器量程、线性及开方换算
│       ├── uncertainty.rs / uncertainty/    # 不确定度分量树、预算及合成
│       ├── curve_fit.rs / curve_fit/        # 校准曲线拟合、统计量及图表
│       ├── spline.rs / spline/              # B 样条插值、曲线及采样导出
│       └── gauges/
│           ├── mod.rs        # 量规模块入口与共用计算组织
│           ├── page.rs       # 参数表单和结果表格
│           ├── lookup.rs     # 数据表加载、分档查询与缺失值处理
│           ├── specs.rs      # 常用规格建议
│           ├── thread.rs     # M、Tr 螺纹及钢筋量规尺寸计算
│           ├── imperial.rs   # 英寸解析、UN 系列及公差查询
│           ├── un_data.rs    # UN 内置系列与制造公差数据读取
│           ├── unified.rs   # UN 工作量规与校对规计算
│           ├── unified/product.rs  # UN 工件等级、公差及旋合长度
│           ├── pipe.rs       # R/NPT 管螺纹量规
│           ├── smooth.rs     # 光滑塞规、环规和卡规
│           ├── report.rs     # 结果组织、尺寸格式与报告文本
│           ├── conventions.rs  # 各类量规的技术口径
│           └── api.rs        # 无界面计算接口
├── data/
│   ├── thermocouple/         # B、E、J、K、N、R、S、T 型参考函数系数
│   └── gauges/
│       ├── tables.json       # 量规尺寸、公差、偏差和长度表
│       ├── unified.json      # UN 系列及中径制造公差 U
│       └── README.md         # 数据用途、取值约定及适用范围
├── assets/                   # SVG 原图与 PNG、ICO 程序图标
├── licenses/                 # 第三方许可证原文
├── cpp_version/
│   ├── build.cmd / build.rsp # MinGW 构建入口、源码清单与链接参数
│   ├── include/toolbox/      # 公共接口与数据结构
│   ├── src/core/            # 基础数值工具、目录和内置数据
│   ├── src/models/          # 各工具的计算模型
│   ├── src/pages/           # 参数输入、结果展示及图表
│   ├── src/gui/             # 窗口、排版、绘图、导航和导出
│   ├── resources/           # 图标、程序清单和内嵌量规数据
│   ├── third_party/         # 随源码编译的第三方组件
│   └── licenses/            # 第三方许可及程序内许可汇总
└── web_version/
    ├── package.json         # 开发与发布构建命令
    ├── core/                # Rust/Wasm 接口与共享模型适配
    ├── src/                 # Vue 页面、输入控件、图表及计算 Worker
    ├── public/              # 图标、应用清单与许可文件
    ├── scripts/             # Wasm 构建、离线缓存和许可汇总
    └── deploy/              # 静态网站部署模板
```

`名称.rs / 名称/` 表示同名入口文件与实现目录：入口组织页面或对外接口，子目录拆分模型、曲线和允差等实现。共用数值工具集中在 `calculators/`，单位及量规基础数据分别集中维护。

Rust 的编译缓存位于 `target/`，发布包位于根目录 `dist/`；C++ 的中间文件与程序分别位于 `cpp_version/build/`、`cpp_version/dist/`，均已由 Git 忽略。

## 计算与使用说明

- **精度**：采用双精度计算，显示位数不代表测量精度。量规计算值显示五位小数，mm 与 inch 均由未修约值分别换算。
- **单位约定**：温度换算针对温度值；压力单位换算不转换表压/绝压。水柱按 4 °C、汞柱按 0 °C，热量单位采用国际蒸汽表定义；`t/a` 按每年 8000 小时计算。浓度采用 1000 kg/m³ 等密度基准，滴体积为 20 滴/mL。
- **温标与校准**：按各型号的有效温区和允差等级计算；证书系数、固定点数据应按实际证书填写。SPRT 的 Rtp 为水三相点电阻，热敏电阻多项式使用 K 和 Ω。
- **压力与湿度**：高度差以被检表高于标准器为正；气压高度采用标准大气模型。混合比以干空气质量为分母，比湿以湿空气总质量为分母，标准状态含水量按 0 °C、101.325 kPa 表示。
- **辐射修正**：根据仪表响应选择有效波长或 Planck 模型，误差和修正量的正负方向以结果标签为准。
- **统计与不确定度**：重复性使用样本标准差；平方和根合成按分量不相关处理，扩展不确定度按所选包含因子计算。
- **曲线拟合**：对输入数据采用等权最小二乘法；r 为原始数据的 Pearson 相关系数，R² 为决定系数。
- **B 样条**：支持 2～500 个 Q 点，x 须严格递增；采用自然端点条件，支持端点外推和最多 500 万点采样。预览为部分数据，CSV 导出全部采样点。
- **量规**：按内置表和当前模型计算，缺失数据不补零；参考尺寸与制造界限分别列示。适用范围见 [量规数据说明](data/gauges/README.md)。

各工具的具体公式、适用范围及输入要求见页面“计算说明”。桌面版输入仅在当前进程保存。

## 许可证

项目自有代码与图标采用 [MIT 许可证](LICENSE)。第三方组件声明见 [licenses/](licenses/) 和 [C++ 版第三方说明](cpp_version/THIRD-PARTY-NOTICES.md)。
