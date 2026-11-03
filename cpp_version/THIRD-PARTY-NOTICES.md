# 第三方组件说明

本程序使用 Windows 系统提供的 Direct2D、DirectWrite、Win32 和通用控件，不随包复制 Windows 系统 DLL。

计量规范 PDF 下载使用系统 WinHTTP、BCrypt，以及静态编译的 PDFio 1.7.0（提交 `3444a8a6f99028e2f9853a857936c144d7b6d955`）、其 TTF 源码（提交 `671390aef1f9e81723da164ea4a4a72037abfd52`，均为 Apache-2.0）和 zlib 1.3.1。源码及许可证保存在 `third_party/pdfio`、`third_party/zlib`；许可全文与 PDFio NOTICE 亦已嵌入 EXE。MinGW 的 `ssize_t` 兼容性调整沿用原下载工具。

十进制文本解析和格式化使用 Google/V8 的 [double-conversion v3.3.1](https://github.com/google/double-conversion/tree/v3.3.1)，源码保存在 `third_party/double-conversion` 并直接编译进程序，不增加额外 DLL。原作者的 BSD 三条款许可证保留在源码目录 `licenses/double-conversion-LICENSE.txt`，并完整嵌入发布 EXE。

GCC 构建参数将 MinGW-w64 的 C++、GCC 及所需线程运行库静态链接进 EXE，不随程序分发外部运行库 DLL。供资源编译器使用的许可汇总保存在 `licenses/third-party-licenses.txt`，包含本说明、double-conversion、PsychroLib 以及 GCC、GCC Runtime Library Exception、MinGW-w64 和 winpthreads 的许可原文。更换工具链或第三方组件时，应核对并同步该汇总；具体条款以所用发行包中的许可证为准。

热电偶 B/E/J/K/N/R/S/T 型参考函数数据保留在编译后的静态数据中，来自 Rust 项目随附的 NIST ITS-90 参考函数数据；运行时不下载数据。

计算模型由本项目 Rust 实现迁移。源项目保留的 PsychroLib MIT 许可证见 `licenses/PsychroLib-LICENSE.txt`。

以上说明和许可证全文均可在程序窗口系统菜单的“开源许可”中阅读（右键单击标题栏或按 `Alt+空格`）。单文件分发保留这些内容，不需要额外的许可证目录。
