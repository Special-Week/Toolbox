# 工具箱 C++ 版

**Toolbox** 的 Windows 桌面版，使用 C++17、Win32、Direct2D 和 DirectWrite，提供 **7 个分类、49 个工具**。支持窗口缩放、工具搜索和分类折叠，采用系统字体与 DPI 适配。

完整功能清单和通用计算约定见 [主项目 README](../README.md#功能分类)。本目录可独立构建。

## 构建与运行

预先将完整的 **mingw64 / mingw32** 工具链加入 PATH，确保 `gcc`、C++ 编译组件和 `windres` 来自同一套工具链。以下命令在 `cpp_version` 目录执行。

### 64 位（默认）

使用 mingw64：

```powershell
.\build.cmd
```

生成 `dist/x64/Toolbox.exe`。

### 32 位

切换 PATH 到 mingw32：

```powershell
.\build.cmd x86
```

生成 `dist/x86/Toolbox.exe`。

Windows 7 SP1 32 位构建方案使用 **GCC 8.1.0、POSIX/DWARF/MSVCRT** 工具链，处理器须支持 SSE2；尚未完成 Win7 真机验证。任意新版 mingw32 不保证 Win7 兼容。

### 直接使用 GCC

以 64 位为例：

```powershell
New-Item -ItemType Directory -Force build, dist/x64 | Out-Null
windres -c 65001 -O coff resources/Toolbox.rc -o build/Toolbox.res.o
gcc "@build.rsp" -O2 -s -o dist/x64/Toolbox.exe
```

32 位改用 mingw32，将输出目录改为 `dist/x86`，并在 GCC 命令中加入 `-msse2 -mfpmath=sse -pthread`。

`build.rsp` 保存源码清单与编译、链接参数；`gcc` 按源文件扩展名编译 C++，并显式链接 C++ 标准库。

## 发布

发布对应架构的 **Toolbox.exe** 即可。运行库、数据、图标和许可声明已静态链接或内嵌；Windows 系统 DLL 由操作系统提供。`build/` 仅存放编译中间文件。

## 项目结构

```text
cpp_version/
├── build.cmd                 # 构建入口，默认 x64，可选 x86
├── build.rsp                 # 源码清单、C++17 编译参数与静态链接库
├── .gitignore                # 构建产物和本地缓存的忽略规则
├── THIRD-PARTY-NOTICES.md     # 第三方组件与授权说明
├── include/toolbox/
│   ├── core.hpp              # 数值工具、工具目录、单位及坐标轴接口
│   ├── models.hpp            # 温标、压力、湿度等计算模型接口
│   ├── curve_fit.hpp         # 校准曲线模型、系数和统计量接口
│   ├── gauges.hpp            # 量规输入、查表和结果报告接口
│   ├── gui.hpp               # 窗口、控件、页面和绘图接口
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
│   │   ├── curve_fit.cpp / curve_fit.hpp  # 校准曲线输入与结果展示
│   │   └── curve_fit_chart.cpp            # 拟合曲线和残差图
│   └── gui/
│       ├── window.cpp        # 窗口消息、输入控件、焦点与滚动处理
│       ├── scene.cpp         # 界面布局及 Direct2D 绘制
│       ├── text_layout.cpp   # DirectWrite 文字测量、换行与排版
│       ├── navigation.cpp    # 首页、工具搜索及分类导航
│       ├── icon.cpp          # 图标资源加载
│       ├── export.cpp        # CSV 导出与后台任务
│       └── licenses.cpp      # 程序内许可查看
├── resources/
│   ├── Toolbox.rc            # 图标、版本、程序清单、数据和许可资源入口
│   ├── Toolbox.manifest      # DPI、系统兼容和权限声明
│   ├── toolbox.ico           # 多尺寸程序图标
│   └── gauges/
│       ├── tables.json       # 量规尺寸、公差、偏差和长度表
│       └── unified.json      # UN 系列及中径制造公差 U
├── scripts/
│   ├── sync-gauges.py        # 从 Rust 版同步量规数据与说明
│   └── sync-copy.py          # 从 Rust 版同步工具文案与单位说明
├── licenses/                 # 第三方许可原文及内嵌许可汇总
└── third_party/
    ├── README.md             # 第三方源码说明
    └── double-conversion/    # 十进制解析与格式化组件
```

`include/toolbox/` 定义公共接口，`src/models/` 实现计算，`src/pages/` 组织参数和结果，`src/gui/` 处理窗口与绘制。`resources/` 由 `windres` 编译并链接进 EXE，构建时须保留。

构建中间文件位于 `build/`，发布程序位于 `dist/x64/` 或 `dist/x86/`；这些目录均已由本目录的 `.gitignore` 忽略。

## 使用说明

- 计算使用 `double`，适用温区、单位、允差和模型条件见各页“计算说明”。量规模型与数据边界见 [量规数据说明](../data/gauges/README.md)。
- 输入仅在当前运行期间保存。分度表和样条支持 CSV 导出；量规支持复制完整结果。
- 开发时可运行 `python scripts/sync-gauges.py`、`python scripts/sync-copy.py` 同步 Rust 数据和说明；普通构建无需 Python 或 Rust。

## 许可证

项目自有代码与图标采用 [MIT 许可证](../LICENSE)，第三方组件见 [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)。程序标题栏系统菜单中可查看“开源许可”。
