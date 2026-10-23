"""从 Rust 同步专业用户说明。仅在修改说明时运行，普通构建不依赖 Python。"""

from __future__ import annotations

import json
import re
from pathlib import Path


def literals(block: str) -> dict[str, str]:
    """提取说明分支的 UTF-8 字符串。"""
    pattern = (
        r"(?:ToolKind|Dimension)::(\w+) =>\s*"
        r'(?:\{\s*)?("(?:\\.|[^"\\])*")\s*(?:\}|,)'
    )
    return {name: json.loads(value) for name, value in re.findall(pattern, block)}


def wide(text: str) -> str:
    """生成宽字符原始字符串，保留中文和公式换行。"""
    if ')COPY"' in text:
        raise ValueError("说明包含保留的字符串结束标记")
    return 'LR"COPY(' + text + ')COPY"'


def main() -> None:
    """同步基本说明、模式说明与单位约定，保留 C++ 自身的修约口径。"""
    cpp = Path(__file__).resolve().parents[1]
    root = cpp.parent
    source = (root / "src/calculators/explanations.rs").read_text(encoding="utf-8")
    summary = literals(source.split("fn for_tool(", 1)[1].split("fn formula(", 1)[0])
    formula = literals(source.split("fn formula(", 1)[1].split("fn variants(", 1)[0])
    modes: dict[str, list[tuple[int, str, str]]] = {}
    for kind, body in re.findall(
        r"ToolKind::(\w+) => &\[(.*?)\],",
        source.split("fn variants(", 1)[1],
        re.DOTALL,
    ):
        modes[kind] = [
            (int(mode), json.loads(note), json.loads(detail))
            for mode, note, detail in re.findall(
                r'\(\s*(\d+),\s*("(?:\\.|[^"\\])*")'
                r',\s*("(?:\\.|[^"\\])*")\s*,?\s*\)',
                body,
                re.DOTALL,
            )
        ]
    if len(summary) != 21 or summary.keys() != formula.keys():
        raise ValueError("普通工具说明未完整提取")
    output = [
        "// 从 Rust 同步的专业用户说明。更新请运行 scripts/sync-copy.py。",
        '#include "toolbox/core.hpp"',
        '#include "toolbox/gauges.hpp"',
        "namespace toolbox {",
        "CalculationNotes calculationNotes(const Tool &tool, int variant) {",
        "    switch (tool.kind) {",
        "    case Kind::Unit: {",
        "        auto note = unitNotes(tool.dimension);",
        '        return note.empty() ? CalculationNotes{} : CalculationNotes{{L"使用条件", note}};',
        "    }",
    ]
    gauge_names = ["Metric", "Trapezoidal", "Unified", "Pipe", "Rebar", "Smooth"]
    for name in gauge_names:
        output.extend(
            [
                f"    case Kind::Gauge{name}:",
                f'        return {{{{L"使用条件", wide(gauges::explanation(gauges::Type::{name}))}},',
                f'                {{L"详细公式与取值", wide(gauges::convention(gauges::Type::{name}))}}}};',
            ]
        )
    for kind, note in summary.items():
        output.append(f"    case Kind::{kind}:")
        for mode, mode_note, detail in modes.get(kind, []):
            if mode == 2 and kind == "Uncertainty":
                continue  # 此项为 Web 快速评定的逐项设置说明。
            output.append(
                f"        if (variant == {mode}) return "
                f'{{{{L"使用条件", {wide(mode_note)}}},'
                f'{{L"详细公式与取值", {wide(detail)}}}}};'
            )
        detail = formula[kind]
        if kind == "Uncertainty":
            detail = detail.replace("最低为整数位", "可修约至十位、百位等")
        output.append(
            f'        return {{{{L"使用条件", {wide(note)}}},'
            f'{{L"详细公式与取值", {wide(detail)}}}}};'
        )
    output.extend(["    }", "    return {};", "}", "} // namespace toolbox", ""])
    (cpp / "src/core/explanations.cpp").write_text("\n".join(output), encoding="utf-8")

    unit_source = (root / "src/units/notes.rs").read_text(encoding="utf-8")
    units = literals(unit_source)
    if len(units) != 22:
        raise ValueError("单位约定未完整提取")
    catalog = cpp / "src/core/catalog_data.cpp"
    text = catalog.read_text(encoding="utf-8")
    start = text.index("std::wstring unitNotes(Dimension dimension) {")
    end = text.index("} // namespace toolbox", start)
    function = (
        "std::wstring unitNotes(Dimension dimension) {\n    switch (dimension) {\n"
    )
    function += "".join(
        f"    case Dimension::{name}: return {wide(note)};\n"
        for name, note in units.items()
    )
    function += '    }\n    return L"";\n}\n'
    catalog.write_text(text[:start] + function + text[end:], encoding="utf-8")
    print("已同步普通工具说明、模式说明与单位约定。")


if __name__ == "__main__":
    main()
