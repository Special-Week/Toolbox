# 工具箱 · 网页版

**Toolbox Web** 是面向计量、仪表与分析检测的浏览器计算工具，包含 **7 个分类、49 个工具**。支持电脑和手机，计算在本地浏览器中完成，无需计算后端或数据库。

## 功能概览

| 分类 | 主要功能 |
|---|---|
| 单位换算（22） | 压力、温度、流量、质量、长度、热工等 256 项单位换算 |
| ITS90 温标换算（5） | 工业热电偶、工业热电阻、标准热电偶、标准铂电阻、热敏电阻 |
| 压力工具（4） | 压力表与变送器标准器选型、高度差修正、气压高度换算 |
| 温湿度工具（4） | 湿度换算、耳温计黑体发射率修正、黑体辐射源发射率误差、辐射温度计示值修正 |
| 误差工具（5） | 重复性、极差法、合成误差、误差分析、不确定度计算 |
| 量规计算（6） | 公制普通螺纹 M、梯形螺纹 Tr、美制统一螺纹 UN、R/NPT 管螺纹、钢筋直螺纹、光滑量规 |
| 其他工具（3） | 变送器、三次均匀 B 样条插值、校准曲线拟合 |

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

此命令使用 Rust release 优化构建 Wasm、生成绑定、检查 TypeScript 并打包网页，最终输出到 `dist/`。

本地预览构建结果：

```powershell
npm run preview
```

默认访问 `http://127.0.0.1:4173/`。需要对外监听时追加 `-- --host 0.0.0.0`。

## 部署

### 静态网站

将 **`dist/` 内的全部文件**上传到静态网站服务器，保持目录结构。支持域名根目录与子目录部署，服务器应将 `.wasm` 作为 `application/wasm` 返回。运行服务器无需安装 Rust 或 Node.js。

### GitHub Pages

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
│   ├── engine.ts                    # 计算请求与返回结果管理
│   ├── engine.worker.ts             # Worker 内加载和调用 Wasm
│   ├── types.ts                     # 工具、输入和结果的类型定义
│   ├── notes.ts                     # 随计算模式切换说明与公式
│   ├── format.ts / export.ts        # 数字显示、结果复制与 CSV 格式
│   ├── mobile-input.ts              # 手机数字输入辅助
│   ├── style.css / styles/          # 布局、表单、结果与手机样式
│   └── components/                  # 输入控件、量规、不确定度、图表及 PWA
├── scripts/
│   ├── build-wasm.mjs               # 编译 Rust 并生成浏览器绑定
│   ├── sync-licenses.mjs            # 汇总发布所需的许可声明
│   ├── offline-plugin.ts           # 生成离线资源清单与版本
│   └── service-worker.js            # 离线缓存与更新处理
├── public/                          # 图标、应用清单与许可文件
└── deploy/github-pages.yml          # GitHub Pages 部署模板
```

`node_modules/` 为 npm 依赖，`pkg/` 为 Wasm 和绑定文件，`core/target/` 为 Rust 构建缓存，`dist/` 为最终网站；均已由 Git 忽略。

## 使用说明

- **计算**：使用双精度，具体单位、公式和适用范围见各工具的“计算说明”。示例数据应替换为实际测量值或证书参数。
- **数据保存**：输入保存在当前浏览器，不上传服务器；清理网站数据会清除保存的输入。本地存储不可用时仅保留当前会话。
- **离线使用**：通过 HTTPS 或 localhost 加载生产版本，等待离线资源就绪后可断网计算；普通服务器 IP 的 HTTP 访问不启用离线缓存。安装能力取决于浏览器。
- **版本更新**：有新版本时可点击“更新并刷新”，刷新前保存需要保留的结果。开发服务不注册离线缓存。
- **导出**：分度表最多 5 万行；B 样条支持 2～500 个 Q 点、最多 500 万点采样。支持流式保存的浏览器可导出全部采样点，其他浏览器单次最多导出 25 万行。
- **量规**：支持复制完整报告，适用范围见 [量规数据说明](../data/gauges/README.md)。

## 许可证

项目自有代码与图标采用 [MIT 许可证](../LICENSE)。第三方许可汇总由构建脚本写入 `public/licenses.txt`，随网站发布。
