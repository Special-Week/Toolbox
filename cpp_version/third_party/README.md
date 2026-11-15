# 随源码编译的第三方组件

## double-conversion

- 版本：v3.3.1；[上游源码](https://github.com/google/double-conversion/tree/v3.3.1/double-conversion)。
- 用途：双精度数字的十进制解析与格式化，由 `build.rsp` 随项目静态编译。
- 许可：[double-conversion-LICENSE.txt](../licenses/double-conversion-LICENSE.txt)，原文件版权声明保留；许可汇总已嵌入 EXE。

## PDFio 和 zlib

- 用途：PDF 解析、页数检查及 PDF 流解压，由 Makefile 随程序编译。
- 许可：[PDFio LICENSE](pdfio/LICENSE)、[PDFio NOTICE](pdfio/NOTICE) 和 [zlib LICENSE](zlib/LICENSE)；许可汇总已嵌入 EXE。

## MicroTeX 和 tinyxml2

- 用途：MicroTeX 在运行时排版公式，tinyxml2 用于解析其资源；源码和数学字体均随项目提供。
- 许可：[MicroTeX LICENSE](microtex/LICENSE)、[字体许可](microtex/res/fonts/licences/) 和 [tinyxml2 LICENSE](tinyxml2/LICENSE.txt)；许可汇总已嵌入 EXE。
