"""开发时同步 Rust 的量规数据和说明；普通 GCC 构建不需要 Python。"""
from pathlib import Path
import json
import re
import shutil

cpp = Path(__file__).resolve().parents[1]
root = cpp.parent
resource = cpp / 'resources' / 'gauges'
resource.mkdir(parents=True, exist_ok=True)
for name in ('tables.json', 'unified.json'):
    shutil.copyfile(root / 'data' / 'gauges' / name, resource / name)

source = root / 'src' / 'calculators' / 'gauges'
kinds = ['Metric', 'Trapezoidal', 'Unified', 'Pipe', 'Rebar', 'Smooth']
notes = (source / 'mod.rs').read_text(encoding='utf-8')
conventions = (source / 'conventions.rs').read_text(encoding='utf-8')
common = conventions.split('fn common(', 1)[1].split('fn specific(', 1)[0]
output = ['// 从 Rust 的量规说明同步生成；更新请运行 scripts/sync-gauges.py。',
          '#include "toolbox/gauges.hpp"', 'namespace toolbox::gauges {']

def literal(text):
    # 原始字符串保留中文与公式，避免 C++ 转义和本地代码页影响说明。
    assert ')GAUGE"' not in text
    return 'R"GAUGE(' + text + ')GAUGE"'

for function in ('explanation', 'convention'):
    output.append(f'std::string {function}(Type type) {{\n    switch (type) {{')
    for kind in kinds:
        if function == 'explanation':
            match = re.search(r'GaugeKind::' + kind + r' => \{\s*("(?:\\.|[^"\\])*")', notes)
            if not match:
                raise RuntimeError(f'未找到计算说明：{kind}')
            text = json.loads(match[1])
        else:
            match = re.search(r'GaugeKind::' + kind + r' => \(\s*r#"(.*?)"#,\s*r#"(.*?)"#,', conventions, re.S)
            if not match:
                raise RuntimeError(f'未找到口径说明：{kind}')
            heading, body = match.groups()
            group = 'Metric | GaugeKind::Rebar' if kind in ('Metric', 'Rebar') else kind
            match_common = re.search(r'GaugeKind::' + re.escape(group) + r' => \{\s*("(?:\\.|[^"\\])*")', common)
            prefix = json.loads(match_common[1]) if match_common else ''
            text = ('取值规则\n' + prefix + '\n\n' if prefix else '') + heading + '\n' + body
        output.append(f'    case Type::{kind}: return {literal(text)};')
    output.extend(['    }', '    return "";', '}'])
output.append('} // namespace toolbox::gauges\n')
(cpp / 'src/models/gauges/notes.cpp').write_text('\n'.join(output), encoding='utf-8')
print('已同步量规数据和说明。')
