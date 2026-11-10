# 工具箱 · 网页版

**Toolbox Web** 是面向计量、仪表与分析检测的浏览器工具箱，包含 **7 个分类、50 个工具**。支持电脑和手机，计算在本地浏览器中完成；计量规范 PDF 下载使用配套 Node.js 服务获取文件并缓存到磁盘，无需数据库。

## 功能概览

| 分类 | 主要功能 |
|---|---|
| 单位换算（22） | 压力、温度、流量、质量、长度、热工等 257 项单位换算 |
| ITS90 温标换算（5） | 工业热电偶、工业热电阻、标准热电偶、标准铂电阻、热敏电阻 |
| 压力工具（4） | 压力表与变送器标准器选型、高度差修正、气压高度换算 |
| 温湿度工具（4） | 湿度换算、耳温计黑体发射率修正、黑体辐射源发射率误差、辐射温度计示值修正 |
| 误差工具（5） | 重复性、极差法、合成误差、误差分析、不确定度计算 |
| 量规计算（6） | 公制普通螺纹 M、梯形螺纹 Tr、美制统一螺纹 UN、R/NPT 管螺纹、钢筋直螺纹、光滑量规 |
| 其他工具（4） | 变送器、三次均匀 B 样条插值、校准曲线拟合、计量规范 PDF 下载 |

完整清单见 [主项目 README](../README.md#功能分类)。支持结果复制、适用工具的 CSV 导出、分度表、曲线和残差图。


## 构建与运行

需要 **Node.js 22.12+、npm、支持 Rust 2024 Edition 的 Rust 工具链及本机链接环境**。本项目引用仓库根目录的 Rust 模型和数据，须在完整仓库中构建。

首次安装 Wasm 工具：

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.114 --locked
```

进入网页目录并安装依赖：

```powershell
cd web_version
npm ci
```

### 开发运行

```powershell
npm run dev
```

默认访问 `http://127.0.0.1:5173/`。允许局域网设备访问：

```powershell
npm run dev -- --host 0.0.0.0
```

修改共享 Rust 模型后，运行 `npm run build:wasm`，再刷新页面。

### 发布构建

```powershell
npm run build
```

此命令使用 Rust release 优化构建 Wasm、生成绑定、检查 TypeScript，并将网页、PDF 后端和启动脚本一起打包到 `dist/`。重新构建会清理旧产物，保留 `dist/.pdf-cache/` 中已有的 PDF 缓存。

本地预览构建结果：

```powershell
npm run preview
```

默认访问 `http://127.0.0.1:4173/`。需要对外监听时追加 `-- --host 0.0.0.0`。

## 部署

### 完整服务（含 PDF 下载）

将完整 `dist/` 上传到服务器，保留 `server/`、`serve.mjs` 和 `serve.py`。运行环境需要 **Node.js 22.12+**；使用 Python 启动入口时还需 Python 3。部署机无需 Rust、npm 或 `node_modules/`。

在上传后的 `dist` 目录运行：

```powershell
python serve.py
```

默认监听 **`0.0.0.0:5173`**。本机访问 `http://127.0.0.1:5173/`，局域网设备使用服务器的实际 IP。可用 `python serve.py 8080` 修改端口，或用 `--bind 127.0.0.1` 限定为本机访问。

也可直接运行 `node serve.mjs`，默认监听 `0.0.0.0:4173`；可通过 `HOST`、`PORT` 环境变量调整监听地址和端口。

### PDF 下载与缓存

在“其他工具 → 计量规范 PDF 下载”输入规范名称关键词或编号，即时筛选内置的 2,882 条规范。编号搜索忽略大小写和空格，可输入 `JJF1059.1`；结果分页展示编号、名称、类型、状态和发布、实施日期，点击对应条目的“下载 PDF”后确认下载。

搜索在浏览器本地完成，目录来自指定的五份规范清单，状态与日期请以规范网站为准。确认下载提示后，网页将选中规范的链接发送给同源 `/api/pdf` 接口，由服务器访问规范网站。服务器按链接中的 `standno` 编号匹配缓存：已有有效 PDF 就直接发送；没有或文件损坏时，获取 PDF、解析实际页数并完整保存后发送。相同编号的空格编码、页码等附加参数不影响缓存匹配，同一规范的并发请求合并获取。

使用上述完整构建包启动时，缓存默认位于 **`serve.py` 同级的 `.pdf-cache/`**，保存 PDF 和对应 JSON 信息，重启后仍可使用，不自动过期。更新部署时请保留此目录；可用 `TOOLBOX_PDF_CACHE` 环境变量指定其他位置。单个 PDF 的下载及本地缓存上限均为 100 MB；网站接口调整或限流时可能下载失败，请按网站的电子文本使用说明使用文件。

如需批量导入本地文件，将 PDF 和含有对应文件名、链接、编号、名称及 SHA-256 的 JSON 索引放入 `web_version/待处理/`，然后在 `web_version` 下运行 `node scripts/import-pdf-cache.mjs`。脚本会校验文件哈希、解析实际页数并写入 `.pdf-cache/`，原始文件保留；也可以通过 `TOOLBOX_PDF_CACHE` 指定缓存目录。

开发服务默认使用 `web_version/.pdf-cache/`，完整构建包默认使用 `dist/.pdf-cache/`。要在本机运行构建包时使用导入的缓存，可在启动前设置 `$env:TOOLBOX_PDF_CACHE = (Resolve-Path .pdf-cache).Path`；部署到其他机器时需一并复制缓存并设置该变量，或将缓存复制到部署目录的 `.pdf-cache/`。

### 静态网站

只使用计算功能时，可将网页静态资源部署到静态服务器，保持目录结构，并将 `.wasm` 作为 `application/wasm` 返回。纯静态托管无法提供 `/api/pdf`，需要 PDF 下载时使用上面的完整服务。

### GitHub Pages

此模板只发布静态网页，适用于计算功能，不提供 PDF 下载接口。

1. 将 [deploy/github-pages.yml](deploy/github-pages.yml) 复制到仓库根目录 `.github/workflows/web-pages.yml`。
2. 在仓库 **Settings → Pages** 中选择 **GitHub Actions**。
3. 在 **Actions → Deploy Toolbox Web → Run workflow** 手动发布。

## 项目结构

```text
web_version/
├── package.json / package-lock.json  # npm 命令与依赖锁定
├── index.html                       # 网页入口
├── vite.config.ts                   # Vue、Worker、资源路径及离线构建配置
├── tsconfig.json                    # TypeScript 配置
├── .gitignore                       # 依赖、构建产物和日志的忽略规则
├── core/
│   ├── Cargo.toml / Cargo.lock       # Wasm 依赖与 release 优化配置
│   └── src/
│       ├── lib.rs                   # 向浏览器导出目录、计算和采样接口
│       ├── api.rs                   # 输入校验、结果结构与分度表
│       ├── sensors.rs               # 温标与传感器校准适配
│       ├── engineering.rs           # 压力、湿度、辐射和变送器适配
│       ├── statistics.rs            # 统计误差和不确定度预算适配
│       ├── curves.rs                # 拟合、样条与分块采样接口
│       └── calculators/             # 引用仓库中的 Rust 计算模型
├── src/
│   ├── main.ts                      # Vue 应用入口
│   ├── App.vue                      # 页面状态、导航及计算调度
│   ├── schema.ts                    # 表单配置、默认值和输入模式
│   ├── input-history.ts             # 需要记忆的工具及本地输入过滤
│   ├── calibration-archives.ts      # 标准热电偶和标准铂电阻的系数档案
│   ├── engine.ts                    # 计算请求与返回结果管理
│   ├── engine.worker.ts             # Worker 内加载和调用 Wasm
│   ├── types.ts                     # 工具、输入和结果的类型定义
│   ├── notes.ts                     # 随计算模式切换说明与公式
│   ├── format.ts / export.ts        # 数字显示、结果复制与 CSV 格式
│   ├── mobile-input.ts              # 手机数字输入辅助
│   ├── style.css / styles/          # 布局、表单、结果与手机样式
│   └── components/                  # 输入控件、量规、图表、PDF 下载及 PWA
├── server/                          # PDF 获取、真实页数校验、磁盘缓存及测试
├── scripts/
│   ├── build-wasm.mjs               # 编译 Rust 并生成浏览器绑定
│   ├── sync-licenses.mjs            # 汇总发布所需的许可声明
│   ├── clean-dist.mjs               # 清理旧构建产物，保留 PDF 缓存
│   ├── package-server.mjs           # 将后端和启动脚本打包到 dist
│   ├── offline-plugin.ts           # 生成离线资源清单与版本
│   └── service-worker.js            # 离线缓存与更新处理
├── public/                          # 图标、应用清单与许可文件
└── deploy/
    ├── serve.py / serve.mjs         # 完整服务启动脚本，构建时复制到 dist
    └── github-pages.yml             # GitHub Pages 静态部署模板
```

`node_modules/` 为 npm 依赖，`pkg/` 为 Wasm 和绑定文件，`core/target/` 为 Rust 构建缓存，`dist/` 为完整部署包；均已由 Git 忽略。启动脚本请修改 `deploy/` 中的源文件，再重新构建。

## 使用说明

- **计算**：使用双精度，具体单位、公式和适用范围见各工具的“计算说明”。示例数据应替换为实际测量值或证书参数。
- **输入保存**：部分需要反复录入的工具自动保存最近输入，可用“恢复示例”重置；单位换算等快捷工具和 PDF 下载链接不保存。计算输入保存在当前浏览器的 `localStorage`，不上传服务器；清理网站数据会清除保存的输入，存储不可用时仅保留当前会话。PDF 下载时会将规范链接发送给服务器。
- **系数档案**：标准热电偶和标准铂电阻的证书参数可按名称保存和调用，档案保存在当前浏览器的 `localStorage`；清理网站数据也会删除档案。
- **离线使用**：通过 HTTPS 或 localhost 加载生产版本，等待离线资源就绪后可断网计算；普通服务器 IP 的 HTTP 访问不启用离线缓存。PDF 下载需要连接服务器，PDF 磁盘缓存不加入网页离线资源。安装能力取决于浏览器。
- **版本更新**：有新版本时可点击“更新并刷新”，刷新前保存需要保留的结果。开发服务不注册离线缓存。
- **导出**：分度表最多 5 万行；B 样条支持 2～500 个 Q 点、最多 500 万点采样。支持流式保存的浏览器可导出全部采样点，其他浏览器单次最多导出 25 万行。
- **量规**：支持复制完整报告，适用范围见 [量规数据说明](../data/gauges/README.md)。

## 许可证

项目自有代码与图标采用 [MIT 许可证](../LICENSE)。第三方许可汇总由构建脚本写入 `public/licenses.txt`，随网站发布。
