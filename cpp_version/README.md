# 工具箱 C++ 版

**Toolbox** 的 Windows 桌面版，使用 C++17、Win32、Direct2D 和 DirectWrite，提供 **7 个分类、50 个工具**。支持窗口缩放和分类折叠，采用系统字体与 DPI 适配。

当前版本：**v2026.11.15**。

完整功能清单和通用计算约定见 [主项目 README](../README.md#功能分类)。本目录可独立构建。

## 构建与运行

预先将完整的 **mingw64 / mingw32** 工具链加入 PATH，确保 `gcc`、`g++`、`windres` 和 `mingw32-make` 来自同一套工具链。以下命令在 `cpp_version` 目录执行。构建会编译随源码提供的 PDFio、zlib 和数学排版依赖。

### 64 位（默认）

使用 mingw64：

```powershell
.\build.cmd
```

生成 `dist/x64/Toolbox_cpp_x64_v2026.11.15.exe`。

### 32 位

切换 PATH 到 mingw32：

```powershell
.\build.cmd x86
```

生成 `dist/x86/Toolbox_cpp_x86_v2026.11.15.exe`。

Windows 7 SP1 32 位构建方案使用 **GCC 8.1.0、POSIX/DWARF/MSVCRT** 工具链，处理器须支持 SSE2；尚未完成 Win7 真机验证。任意新版 mingw32 不保证 Win7 兼容。

### 直接使用 mingw32-make

以 64 位为例：

```powershell
mingw32-make -j4 ARCH=x64
```

32 位切换至 mingw32 工具链，并将参数改为 `ARCH=x86`。脚本默认使用 4 个并行任务，可在运行 `build.cmd` 前设置 `TOOLBOX_BUILD_JOBS` 调整。

Makefile 分架构保存对象文件和头文件依赖，后续构建只重编已变化的文件；`build.rsp` 和 `microtex.rsp` 提供源码清单，实际编译和链接参数由 Makefile 管理。文件名中的版本号取自 `resources/Toolbox.rc` 的 `FILEVERSION`；更新版本时应同时更新该文件中的版本字符串、数字版本、程序清单和窗口标题。普通构建使用已生成的 C++ 公式目录，不需要 Node.js。

修改仓库根目录 `assets/math/` 下的共享公式目录或匹配规则后，需在 `cpp_version` 目录使用 Node.js 同步并检查生成文件，再构建：

```powershell
node scripts/sync-math-catalog.mjs
node scripts/sync-math-catalog.mjs --check
```

`include/toolbox/generated_formula_catalog.inc` 和 `generated_formula_patterns.inc` 是构建所需源码，应与共享公式数据保持同步。

## 发布

发布对应架构的 **Toolbox_cpp_x86_v版本号.exe** 或 **Toolbox_cpp_x64_v版本号.exe** 即可。运行库、数据、图标和许可声明已静态链接或内嵌；Windows 系统 DLL 由操作系统提供。`build/` 仅存放编译中间文件。

## 项目结构

```text
cpp_version/
├── build.cmd                 # 构建入口，默认 x64，可选 x86
├── Makefile                  # mingw32-make 构建规则和架构隔离
├── build.rsp / microtex.rsp  # 应用与 MicroTeX 源码清单
├── .gitignore                # 构建产物和本地缓存的忽略规则
├── THIRD-PARTY-NOTICES.md     # 第三方组件与授权说明
├── include/toolbox/
│   ├── core.hpp              # 数值工具、工具目录、单位及坐标轴接口
│   ├── models.hpp            # 温标、压力、湿度等计算模型接口
│   ├── curve_fit.hpp         # 校准曲线模型、系数和统计量接口
│   ├── gauges.hpp            # 量规输入、查表和结果报告接口
│   ├── gui.hpp               # 窗口、控件、页面和绘图接口
│   ├── pdf_download.hpp      # 规范 PDF 获取、校验与保存接口
│   ├── formula_layout.hpp    # 公式识别及共享公式目录
│   ├── generated_formula_*.inc # 由共享公式数据生成的 C++ 条目
│   ├── math_engine.hpp       # MicroTeX 数学排版接口
│   └── resource_ids.h        # 代码与资源文件共用的资源编号
├── src/
│   ├── main.cpp              # Windows 程序入口与 COM 初始化
│   ├── core/
│   │   ├── catalog_data.cpp  # 分类、工具定义、单位名称与换算系数
│   │   ├── sensor_data.cpp   # 热电偶等传感器的参考系数
│   │   ├── explanations.cpp  # 计算说明和详细公式
│   │   ├── number_text.cpp   # 数值解析与普通小数格式化
│   │   ├── numerics.cpp      # 多项式、反算、QR 拟合、统计和单位换算
│   │   └── math.cpp          # CVD 拟合、标准大气和坐标刻度
│   ├── models/
│   │   ├── thermocouple.cpp  # 工业热电偶正反算、变化率和允差
│   │   ├── rtd.cpp           # 工业热电阻曲线、换算与允差
│   │   ├── calibrated.cpp    # 标准热电偶、标准铂电阻与热敏电阻校准
│   │   ├── pressure.cpp      # 压力标准器评定、高差修正和变送器换算
│   │   ├── humidity.cpp      # 饱和水气压与湿空气参数换算
│   │   ├── radiation.cpp     # 发射率误差与辐射温度修正
│   │   ├── uncertainty.cpp   # 不确定度分量及合成
│   │   ├── curve_fit.cpp     # 校准曲线拟合、系数及统计量
│   │   ├── spline.cpp        # B 样条插值、反算和采样
│   │   ├── pdf_download.cpp  # 规范网站请求、PDF 解析及原始文件保存
│   │   └── gauges/
│   │       ├── data.cpp      # 内嵌量规数据读取
│   │       ├── lookup.cpp    # 公差、偏差与尺寸分档查询
│   │       ├── form.cpp      # 量规参数配置、规格和选项联动
│   │       ├── thread.cpp    # M、Tr 螺纹及钢筋量规计算
│   │       ├── unified.cpp   # UN 工件、工作量规和校对规计算
│   │       ├── pipe.cpp      # R/NPT 管螺纹量规
│   │       ├── smooth.cpp    # 光滑塞规、环规和卡规
│   │       ├── report.cpp    # 尺寸结果、表格和报告文本
│   │       └── notes.cpp     # 量规计算说明与技术口径
│   ├── pages/
│   │   ├── router.cpp        # 按工具类型分派页面
│   │   ├── sensors.cpp / sensors.hpp  # 工业温标页面及共用传感器控件
│   │   ├── standard_couple.cpp       # 标准热电偶校准与换算页面
│   │   ├── sprt.cpp          # 标准铂电阻分区、系数和拟合页面
│   │   ├── thermistor.cpp    # 热敏电阻多项式与校准页面
│   │   ├── pressure.cpp      # 压力选型、高差及气压高度页面
│   │   ├── humidity.cpp      # 湿度换算与辐射修正页面
│   │   ├── errors.cpp        # 重复性、误差和不确定度页面
│   │   ├── gauges.cpp        # 量规参数表单和结果表格
│   │   ├── other.cpp         # 变送器与 B 样条页面
│   │   ├── pdf_download.cpp  # 规范 PDF 下载页面与提示弹窗
│   │   ├── curve_fit.cpp / curve_fit.hpp  # 校准曲线输入与结果展示
│   │   └── curve_fit_chart.cpp            # 拟合曲线和残差图
│   └── gui/
│       ├── window.cpp        # 窗口消息、输入控件、焦点与滚动处理
│       ├── scene.cpp         # 界面布局及 Direct2D 绘制
│       ├── text_layout.cpp   # DirectWrite 文字测量、换行与排版
│       ├── details.cpp / math_engine.cpp # 计算过程、说明与公式绘制
│       ├── navigation.cpp    # 首页与分类导航
│       ├── icon.cpp          # 图标资源加载
│       ├── export.cpp        # CSV 导出与后台任务
│       ├── input_history.cpp # 最近输入的本地 JSON 保存与恢复
│       ├── calibration_archive.cpp # 标准热电偶与标准铂电阻系数档案
│       └── licenses.cpp      # 程序内许可查看
├── resources/
│   ├── Toolbox.rc            # 图标、版本、程序清单、数据和许可资源入口
│   ├── Toolbox.manifest      # DPI、系统兼容和权限声明
│   ├── toolbox.ico           # 多尺寸程序图标
│   ├── standards.json        # 内置计量规范目录
│   └── gauges/
│       ├── tables.json       # 量规尺寸、公差、偏差和长度表
│       └── unified.json      # UN 系列及中径制造公差 U
├── licenses/                 # 第三方许可原文及内嵌许可汇总
├── scripts/                  # PDF 依赖辅助脚本和共享公式目录同步脚本
└── third_party/
    ├── README.md             # 第三方源码说明
    ├── double-conversion/    # 十进制解析与格式化组件
    ├── microtex/ / tinyxml2/ # 数学排版、字体及 XML 解析组件
    ├── pdfio/                # PDF 文件解析及页数检查
    └── zlib/                 # PDF 流解压依赖
```

`include/toolbox/` 定义公共接口，`src/models/` 实现计算，`src/pages/` 组织参数和结果，`src/gui/` 处理窗口与绘制。`resources/` 由 `windres` 编译并链接进 EXE，构建时须保留。

构建中间文件位于 `build/`，发布程序位于 `dist/x64/` 或 `dist/x86/`；这些目录均已由本目录的 `.gitignore` 忽略。

## 使用说明

- 计算使用 `double`，适用温区、单位、允差和模型条件见各页“计算说明”。量规模型与数据边界见 [量规数据说明](../data/gauges/README.md)。
- 可展开“计算过程”查看本次公式和数值代入；较复杂且已接入的公式通过 MicroTeX 绘制，显示修约不改变计算精度。
- 部分需要反复录入的工具自动保存最近输入，默认使用 `%LOCALAPPDATA%\Toolbox\cpp\inputs-v1.json`，可用“恢复示例”重置。单位换算等快捷工具和 PDF 下载链接不保存。
- 标准热电偶和标准铂电阻的证书参数可保存为命名档案，默认存放在 `%LOCALAPPDATA%\Toolbox\cpp\calibrations-v1.json`。
- “其他工具 → 计量规范 PDF 下载”支持按规范名称或编号搜索内置的 2,882 条规范，分页展示类型、状态及日期，选择条目后由本机下载 PDF、检查实际页数并选择位置保存。下载前显示提示，保存成功后显示确认；单文件上限 100 MB，请按网站的电子文本使用说明使用文件。
- 分度表和样条支持 CSV 导出；量规支持复制完整结果。

## 许可证

项目自有代码与图标采用 [MIT 许可证](../LICENSE)，第三方组件见 [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)。程序标题栏系统菜单中可查看“开源许可”。
