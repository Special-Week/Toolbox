# 随源码编译的第三方组件

## double-conversion

- 固定版本：v3.3.1。
- 上游：https://github.com/google/double-conversion/tree/v3.3.1/double-conversion。
- 用途：十进制字符串与 IEEE 754 双精度数之间的转换，以及最短可往返文本、定点小数格式化。
- 内容：仅保留 20 个实现和头文件；第三方测试及基准程序不参与本项目构建。
- 集成：`build.rsp` 列出其中的 `.cc`，由 GCC 随项目一起编译，无需联网、动态库或另行安装。
- 授权：原文见 `../licenses/double-conversion-LICENSE.txt`，许可汇总 `../licenses/third-party-licenses.txt` 由资源编译器嵌入 EXE。

上游文件保留原版权声明和注释；应用中的中文接口说明位于 `src/core/number_text.cpp`。
