// 声明式界面场景与双主题色板。控件键值稳定，计算结果刷新不会重建正在输入的编辑框。
#include "toolbox/gui.hpp"
namespace toolbox {
namespace {
bool darkThemeActive = false;
Color rgb(int r, int g, int b, float a = 1.f) { return Color(r / 255.f, g / 255.f, b / 255.f, a); }
} // namespace

void applyTheme(bool dark) {
    darkThemeActive = dark;
    if (!dark) {
        Ink = rgb(24, 34, 48);
        Muted = rgb(104, 118, 138);
        Blue = rgb(37, 99, 235);
        Soft = rgb(238, 244, 255);
        TableStripe = rgb(245, 249, 255);
        Background = rgb(245, 247, 251);
        White = rgb(255, 255, 255);
        Border = rgb(224, 231, 240);
        Green = rgb(22, 163, 74);
        Red = rgb(205, 56, 56);
        SurfaceSubtle = rgb(248, 250, 253);
        InputBg = rgb(252, 253, 255);
        BorderStrong = rgb(168, 182, 200);
        DangerSoft = rgb(255, 244, 243);
        DangerBorder = rgb(243, 200, 200);
        SuccessSoft = rgb(236, 253, 245);
        AccentText = rgb(255, 255, 255);
    } else {
        Ink = rgb(236, 242, 249);
        Muted = rgb(138, 154, 178);
        Blue = rgb(96, 165, 250);
        Soft = rgb(30, 50, 84);
        TableStripe = rgb(28, 40, 60);
        Background = rgb(15, 23, 36);
        White = rgb(24, 34, 51);
        Border = rgb(43, 58, 81);
        Green = rgb(74, 222, 128);
        Red = rgb(248, 113, 113);
        SurfaceSubtle = rgb(20, 29, 44);
        InputBg = rgb(19, 28, 43);
        BorderStrong = rgb(72, 92, 122);
        DangerSoft = rgb(46, 24, 27);
        DangerBorder = rgb(92, 40, 45);
        SuccessSoft = rgb(19, 42, 32);
        AccentText = rgb(15, 23, 36);
    }
}

GroupAccent groupAccent(Group group) {
    if (!darkThemeActive) {
        switch (group) {
        case Group::Units:
            return {rgb(37, 99, 235), rgb(238, 244, 255)};
        case Group::Its90:
            return {rgb(217, 91, 20), rgb(255, 244, 235)};
        case Group::Pressure:
            return {rgb(2, 132, 199), rgb(236, 248, 255)};
        case Group::Humidity:
            return {rgb(13, 148, 136), rgb(236, 252, 249)};
        case Group::Errors:
            return {rgb(124, 58, 237), rgb(245, 240, 255)};
        case Group::Gauges:
            return {rgb(71, 85, 105), rgb(241, 245, 249)};
        default:
            return {rgb(5, 150, 105), rgb(236, 253, 245)};
        }
    }
    switch (group) {
    case Group::Units:
        return {rgb(96, 165, 250), rgb(26, 45, 77)};
    case Group::Its90:
        return {rgb(251, 146, 60), rgb(58, 35, 21)};
    case Group::Pressure:
        return {rgb(56, 189, 248), rgb(19, 44, 61)};
    case Group::Humidity:
        return {rgb(45, 212, 191), rgb(19, 49, 46)};
    case Group::Errors:
        return {rgb(167, 139, 250), rgb(41, 30, 66)};
    case Group::Gauges:
        return {rgb(148, 163, 184), rgb(35, 45, 59)};
    default:
        return {rgb(52, 211, 153), rgb(19, 49, 38)};
    }
}

Rect intersectRect(Rect a, Rect b) {
    const float x = std::max(a.x, b.x), y = std::max(a.y, b.y);
    return {x, y, std::max(0.f, std::min(a.x + a.w, b.x + b.w) - x),
            std::max(0.f, std::min(a.y + a.h, b.y + b.h) - y)};
}

bool Rect::contains(float px, float py) const { return px >= x && py >= y && px < x + w && py < y + h; }
std::wstring &State::value(const std::string &key, const std::wstring &initial) {
    return values.emplace(key, initial).first->second;
}
int &State::choice(const std::string &key, int initial) {
    return choices.emplace(key, initial).first->second;
}
void Scene::fill(Rect r, Color c, float radius) {
    Draw d;
    d.kind = 0;
    d.rect = r;
    d.clip = clip;
    d.color = c;
    d.radius = radius;
    draws.push_back(d);
}
void Scene::border(Rect r, Color c, float radius) {
    Draw d;
    d.kind = 1;
    d.rect = r;
    d.clip = clip;
    d.color = c;
    d.radius = radius;
    draws.push_back(d);
}
void Scene::text(Rect r, const std::wstring &t, float size, Color c, bool bold, bool center, bool paragraph,
                 bool mono) {
    Draw d;
    d.kind = 2;
    d.rect = r;
    d.clip = clip;
    d.color = c;
    d.text = t;
    d.size = size;
    d.bold = bold;
    d.center = center;
    d.paragraph = paragraph;
    d.mono = mono;
    draws.push_back(std::move(d));
}
void Scene::line(float x1, float y1, float x2, float y2, Color c, float thickness) {
    Draw d;
    d.kind = 3;
    d.rect = {x1, y1, x2 - x1, y2 - y1};
    d.clip = clip;
    d.color = c;
    d.stroke = thickness;
    draws.push_back(d);
}
void Scene::button(Rect r, const std::wstring &t, std::function<void()> action, bool selected) {
    fill(r, selected ? Blue : Soft, 7);
    if (!selected) {
        border(r, Border, 7);
    }
    text(r, t, 12.5f, selected ? AccentText : Blue, selected, true);
    hits.push_back({r, clip, std::move(action)});
}
void Scene::icon(Rect r) {
    Draw d;
    d.kind = 4;
    d.rect = r;
    d.clip = clip;
    draws.push_back(d);
}
void Scene::math(Rect r, const std::wstring &fallback, const std::wstring &formula) {
    Draw d;
    d.kind = 5;
    d.rect = r;
    d.clip = clip;
    d.text = fallback;
    d.formula = formula;
    draws.push_back(std::move(d));
}
void Scene::input(const std::string &key, Rect r, const std::wstring &value) {
    fill(r, InputBg, 7);
    border(r, BorderStrong, 7);
    controls.push_back({key, r, clip, value, {}, 0});
}
void Scene::combo(const std::string &key, Rect r, const std::vector<std::wstring> &options, int selection) {
    controls.push_back({key, r, clip, L"", options, selection});
}

Page::Page(App &owner, Scene &scene, const Tool &definition, float left, float top, float width)
    : app(owner), ui(scene), state(owner.state), tool(definition), x(left), y(top), w(width) {}
std::string Page::key(const std::string &f) const { return tool.id + "." + f; }

void Page::beginSplit(float leftRatio) {
    if (split_.active)
        return;
    split_ = SplitFrame{};
    split_.active = true;
    split_.outerX = x;
    split_.outerW = w;
    split_.topY = y;
    split_.savedClip = ui.clip;
    split_.gap = 16.f;
    split_.sideBySide = w >= 860.f;
    if (split_.sideBySide) {
        split_.leftW = std::clamp(std::round((w - split_.gap) * leftRatio), 320.f, w - split_.gap - 340.f);
        split_.rightX = x + split_.leftW + split_.gap;
        split_.rightW = w - split_.leftW - split_.gap;
    } else {
        split_.leftW = w;
        split_.rightX = x;
        split_.rightW = w;
    }
    split_.leftFillIndex = ui.draws.size();
    ui.fill({x, y, split_.leftW, 0}, White, 10);
    ui.border({x, y, split_.leftW, 0}, Border, 10);
    x = split_.outerX + 18.f;
    y = split_.topY + 14.f;
    w = std::max(120.f, split_.leftW - 36.f);
    ui.clip = intersectRect(split_.savedClip, {split_.outerX, split_.savedClip.y, split_.leftW, split_.savedClip.h});
}

void Page::rightColumn() {
    if (!split_.active || split_.inRight)
        return;
    const float leftBottom = y + 14.f;
    ui.draws[split_.leftFillIndex].rect.h = std::max(48.f, leftBottom - split_.topY);
    ui.draws[split_.leftFillIndex + 1].rect.h = std::max(48.f, leftBottom - split_.topY);
    split_.leftBottom = split_.topY + ui.draws[split_.leftFillIndex].rect.h;
    split_.inRight = true;

    const float rightTop = split_.sideBySide ? split_.topY : (split_.leftBottom + 14.f);
    ui.clip = split_.savedClip;
    split_.rightFillIndex = ui.draws.size();
    ui.fill({split_.rightX, rightTop, split_.rightW, 0}, White, 10);
    ui.border({split_.rightX, rightTop, split_.rightW, 0}, Border, 10);
    x = split_.rightX + 18.f;
    y = rightTop + 14.f;
    w = std::max(120.f, split_.rightW - 36.f);
    ui.clip =
        intersectRect(split_.savedClip, {split_.rightX, split_.savedClip.y, split_.rightW, split_.savedClip.h});
}

void Page::endSplit() {
    if (!split_.active)
        return;
    if (!split_.inRight) {
        const float leftBottom = y + 14.f;
        const float h = std::max(48.f, leftBottom - split_.topY);
        ui.draws[split_.leftFillIndex].rect.h = h;
        ui.draws[split_.leftFillIndex + 1].rect.h = h;
        y = split_.topY + h + 16.f;
    } else {
        const float rightTop = ui.draws[split_.rightFillIndex].rect.y;
        const float rightBottom = y + 14.f;
        const float rh = std::max(48.f, rightBottom - rightTop);
        ui.draws[split_.rightFillIndex].rect.h = rh;
        ui.draws[split_.rightFillIndex + 1].rect.h = rh;
        if (split_.sideBySide) {
            const float matched = std::max(ui.draws[split_.leftFillIndex].rect.h, rh);
            ui.draws[split_.leftFillIndex].rect.h = matched;
            ui.draws[split_.leftFillIndex + 1].rect.h = matched;
            ui.draws[split_.rightFillIndex].rect.h = matched;
            ui.draws[split_.rightFillIndex + 1].rect.h = matched;
            y = split_.topY + matched + 16.f;
        } else {
            y = rightTop + rh + 16.f;
        }
    }
    x = split_.outerX;
    w = split_.outerW;
    ui.clip = split_.savedClip;
    split_ = SplitFrame{};
}

void Page::heading(const std::wstring &t) {
    y += 6;
    ui.text({x, y, w, 26}, t, 14.5f, Ink, true);
    y += 32;
}
void Page::note(const std::wstring &t, Color c) {
    float h = std::max(22.f, app.paragraphHeight(t, w));
    ui.text({x, y, w, h}, t, 12.5f, c, false, false, true);
    y += h + 6;
}
void Page::error(const std::wstring &t) {
    const std::wstring message = L"⚠ " + t;
    const float h = std::max(22.f, app.paragraphHeight(message, std::max(1.f, w - 24), 13));
    ui.fill({x, y, w, h + 22}, DangerSoft, 8);
    ui.border({x, y, w, h + 22}, DangerBorder, 8);
    ui.text({x + 12, y + 11, w - 24, h}, message, 13, Red, false, false, true);
    y += h + 32;
}
std::wstring Page::processValue(double value) {
    if (!std::isfinite(value))
        return L"—";
    return L"\u2063" + inputNumber(value) + L"\u2063";
}
void Page::showProcess() {
    if (!processSteps.empty()) {
        for (auto &line : processSteps)
            line = processDisplayLine(line, false);
        processSteps.push_back(L"部分计算值按 13 位有效数字显示，实际计算精度不变。");
        details("calculation.process", L"计算过程", processSteps);
    }
}
void Page::explain() {
    // 说明只读取当前选择，不改变计算参数。
    int variant = 0;
    if (tool.kind == Kind::ResultantError) {
        variant = state.choice(key("mode"), 0);
    } else if (tool.kind == Kind::Uncertainty) {
        variant = state.choice(key("budget.mode"), 0);
    } else if (tool.kind == Kind::Thermistor) {
        variant = state.choice(key("family"), 0);
    } else if (tool.kind == Kind::StandardThermocouple) {
        variant = state.choice(key("calibration"), 0);
    } else if (tool.kind == Kind::PressureReference) {
        variant = state.choice(key("reference_type"), 0);
    } else if (tool.kind == Kind::TransmitterReference) {
        variant = state.choice(key("model"), 0);
    } else if (tool.kind == Kind::EarBlackbody) {
        variant = state.choice(key("ear.model"), 0);
    } else if (tool.kind == Kind::StandardPlatinumResistor) {
        variant = state.choice(key("model"), 0);
        if (variant == 0 && state.choice(key("deviation"), 0) == 1) {
            variant = 2;
        }
    } else if (tool.kind == Kind::BlackbodyError || tool.kind == Kind::RadiationCorrection) {
        variant = state.choice(key("radiation.model"), 0);
    }
    const auto sections = calculationNotes(tool, variant);
    if (sections.empty()) {
        return;
    }
    std::vector<std::wstring> lines;
    auto appendLines = [&](const std::wstring &text) {
        std::size_t start = 0;
        while (true) {
            const auto end = text.find(L'\n', start);
            lines.push_back(text.substr(start, end == std::wstring::npos ? end : end - start));
            if (end == std::wstring::npos)
                break;
            start = end + 1;
        }
    };
    appendLines(sections.front().second);
    for (std::size_t i = 1; i < sections.size(); ++i) {
        lines.push_back(sections[i].first);
        appendLines(sections[i].second);
    }
    details("calculation.details", L"计算说明", lines);
}
std::wstring Page::textInput(const std::string &f, const std::wstring &label, const std::wstring &initial) {
    auto &value = state.value(key(f), initial);
    bool inlineLabel = w >= 380;
    float labelWidth = w >= 600 ? 182.f : 138.f;
    if (inlineLabel) {
        ui.text({x, y, labelWidth - 10, 34}, label, 12.5f, Muted);
        ui.input(key(f), {x + labelWidth, y, std::max(80.f, w - labelWidth), 34}, value);
        y += 42;
    } else {
        ui.text({x, y, w, 22}, label, 12.5f, Muted);
        y += 24;
        ui.input(key(f), {x, y, w, 34}, value);
        y += 42;
    }
    return value;
}
double Page::number(const std::string &f, const std::wstring &label, double initial) {
    auto value = textInput(f, label, inputNumber(initial));
    try {
        return parse(utf8(value));
    } catch (const std::exception &e) {
        if (inputError.empty()) {
            inputError = label + L"：" + wide(e.what());
        }
        return std::numeric_limits<double>::quiet_NaN();
    }
}
int Page::select(const std::string &f, const std::wstring &label, const std::vector<std::wstring> &options,
                 int initial) {
    int &choice = state.choice(key(f), initial);
    choice = std::clamp(choice, 0, int(options.size()) - 1);
    bool inlineLabel = w >= 380;
    float labelWidth = w >= 600 ? 182.f : 138.f;
    if (inlineLabel) {
        ui.text({x, y, labelWidth - 10, 34}, label, 12.5f, Muted);
        ui.combo(key(f), {x + labelWidth, y, std::max(80.f, w - labelWidth), 34}, options, choice);
        y += 42;
    } else {
        ui.text({x, y, w, 22}, label, 12.5f, Muted);
        y += 24;
        ui.combo(key(f), {x, y, w, 34}, options, choice);
        y += 42;
    }
    return choice;
}
int Page::count(const std::string &f, const std::wstring &label, int initial, int minimum, int maximum) {
    auto k = key(f);
    int &n = state.choice(k, initial);
    n = std::clamp(n, minimum, maximum);
    const float btnW = w < 380 ? 84.f : 96.f;
    ui.text({x, y, std::max(90.f, w - btnW * 2 - 12), 34}, label + L" · " + std::to_wstring(n) + L" 项", 13.5f,
            Ink, true);
    ui.button({x + w - btnW * 2 - 6, y, btnW, 32}, L"− 减少", [thisApp = &app, k, minimum] {
        auto &n = thisApp->state.choices[k];
        n = std::max(minimum, n - 1);
        thisApp->refresh();
    });
    ui.button({x + w - btnW, y, btnW, 32}, L"＋ 添加", [thisApp = &app, k, maximum] {
        auto &n = thisApp->state.choices[k];
        n = std::min(maximum, n + 1);
        thisApp->refresh();
    });
    y += 42;
    return n;
}
bool Page::collapse(const std::string &f, const std::wstring &t, bool initial) {
    auto k = key(f);
    bool open = state.choice(k, initial ? 1 : 0) != 0;
    ui.button({x, y, w, 34}, (open ? L"▾  " : L"▸  ") + t, [owner = &app, k] {
        owner->state.choices[k] = !owner->state.choices[k];
        owner->refresh();
    });
    y += 42;
    return open;
}
Values Page::values(const std::string &f, const std::wstring &label, int n, const Values &initial) {
    Values v;
    for (int i = 0; i < n; ++i) {
        v.push_back(number(f + "." + std::to_string(i), label + L" " + std::to_wstring(i + 1),
                           i < int(initial.size()) ? initial[i] : 0));
    }
    return v;
}
std::vector<std::pair<double, double>> Page::pairs(const std::string &f, const std::wstring &label, int n,
                                                   const std::vector<std::pair<double, double>> &initial,
                                                   float maximumHeight) {
    std::vector<std::pair<double, double>> values;
    float top = y;
    auto previousClip = ui.clip;
    bool scrolling = maximumHeight > 0 && n * 42 > maximumHeight;
    if (scrolling) {
        std::string scrollKey = key(f + ".scroll");
        int limit = int(n * 42 - maximumHeight);
        int &offset = state.choice(scrollKey, 0);
        offset = std::clamp(offset, 0, limit);
        Rect viewport{x, top, w, maximumHeight};
        ui.scrollRegions.push_back({viewport, previousClip, [owner = &app, scrollKey, limit](float delta) {
                                        int old = owner->state.choices[scrollKey];
                                        int value = std::clamp(old + int(delta), 0, limit);
                                        if (value == old) {
                                            return false;
                                        }
                                        owner->state.choices[scrollKey] = value;
                                        return true;
                                    }});
        ui.clip = intersectRect(previousClip, viewport);
        y -= offset;
    }
    float labelCol = 48.f;
    float cell = std::max(60.f, (w - labelCol - 12.f) / 2.f);
    for (int i = 0; i < n; ++i) {
        auto p = i < int(initial.size()) ? initial[i] : std::make_pair(0., 0.);
        ui.text({x, y, labelCol, 34}, label + std::to_wstring(i + 1), 12.5f, Blue, true);
        double rowY = y;
        float oldX = x, oldW = w;
        x += labelCol + 4.f;
        w = cell;
        auto read = [&](const std::string &field, double def) {
            auto &raw = state.value(key(field), inputNumber(def));
            ui.input(key(field), {x, y, w, 34}, raw);
            try {
                return parse(utf8(raw));
            } catch (const std::exception &) {
                inputError = L"请填写完整的校准点坐标";
                return std::numeric_limits<double>::quiet_NaN();
            }
        };
        double a = read(f + "." + std::to_string(i) + ".x", p.first);
        x += cell + 6.f;
        double b = read(f + "." + std::to_string(i) + ".y", p.second);
        x = oldX;
        w = oldW;
        y = float(rowY) + 42.f;
        values.push_back({a, b});
    }
    if (scrolling) {
        ui.clip = previousClip;
        y = top + maximumHeight + 8;
        ui.border({x, top, w, maximumHeight}, Border, 8);
    }
    return values;
}
void Page::check() const { require(inputError.empty(), utf8(inputError).c_str()); }

void Page::metrics(const std::wstring &title, const Metrics &entries) {
    check();
    y += 6;
    ui.text({x, y, w - 140, 26}, title, 14.5f, Ink, true);
    ui.text({x + w - 140, y + 2, 140, 22}, L"点击卡片复制数值", 11, Muted, false, true);
    y += 32;
    int columns = w > 760 ? 3 : (w > 340 ? 2 : 1);
    float gap = 10.f;
    float cell = (w - gap * (columns - 1)) / columns;
    int i = 0;
    for (const auto &e : entries) {
        float cx = x + (i % columns) * (cell + gap), cy = y + (i / columns) * 86.f;
        const bool primary = (i == 0);
        Rect card{cx, cy, cell, 76.f};
        ui.fill(card, primary ? Soft : SurfaceSubtle, 9);
        ui.border(card, primary ? Blue : Border, 9);
        if (primary) {
            ui.fill({cx + 1, cy + 8, 3.5f, 60.f}, Blue, 2);
        }
        ui.text({cx + 12, cy + 8, cell - 22, 22}, e.first, 11.5f, Muted);
        ui.text({cx + 12, cy + 31, cell - 22, 36}, e.second, 16.5f, primary ? Blue : Ink, true, false, false, true);
        ui.hits.push_back({card, ui.clip, [owner = &app, label = e.first, val = e.second] {
                               owner->copyToClipboard(val, L"已复制 " + label + L"：" + val);
                           }});
        ++i;
    }
    y += float((int(entries.size()) + columns - 1) / columns) * 86.f + 4.f;
}

void Page::table(const std::vector<std::wstring> &headings,
                 const std::vector<std::vector<std::wstring>> &rows) {
    if (headings.empty())
        return;
    const float minCol = 108.f;
    float total = std::max(w, float(headings.size()) * minCol);
    float cell = total / float(headings.size());
    bool scrolling = total > w + 1.f;
    const std::string scrollKey = key("table.x." + std::to_string(headings.size()) + "." + std::to_string(rows.size()));
    int &offset = state.choice(scrollKey, 0);
    int maximum = int(std::ceil(std::max(0.f, total - w)));
    offset = std::clamp(offset, 0, maximum);

    Rect original = ui.clip;
    ui.clip = intersectRect(original, {x, y, w, 100000.f});
    float rx = x - offset;
    ui.fill({x, y, w, 34}, SurfaceSubtle, 6);
    ui.border({x, y, w, 34}, Border, 6);
    for (std::size_t i = 0; i < headings.size(); ++i) {
        ui.text({rx + float(i) * cell + 8, y, cell - 16, 34}, headings[i], 12.5f, Muted, true);
    }
    y += 36;
    for (std::size_t j = 0; j < rows.size(); ++j) {
        ui.fill({x, y, w, 34}, j % 2 == 0 ? White : TableStripe);
        for (std::size_t i = 0; i < rows[j].size(); ++i) {
            ui.text({rx + float(i) * cell + 8, y, cell - 16, 34}, rows[j][i], 12.5f, Ink, false, false, false, true);
        }
        ui.line(x, y + 34, x + w, y + 34, Border, 1);
        y += 35;
    }
    ui.clip = original;
    if (scrolling) {
        ui.button({x, y + 4, 28, 24}, L"‹", [owner = &app, scrollKey] {
            owner->state.choices[scrollKey] = std::max(0, owner->state.choices[scrollKey] - 160);
            owner->refresh();
        });
        ui.button({x + w - 28, y + 4, 28, 24}, L"›", [owner = &app, scrollKey, maximum] {
            owner->state.choices[scrollKey] = std::min(maximum, owner->state.choices[scrollKey] + 160);
            owner->refresh();
        });
        const Rect track{x + 36, y + 10, std::max(20.f, w - 72), 12};
        float thumb = std::max(30.f, track.w * w / total), travel = track.w - thumb;
        ui.fill(track, Soft, 5);
        ui.fill({track.x + travel * float(offset) / float(std::max(1, maximum)), track.y, thumb, track.h},
                BorderStrong, 5);
        ui.hits.push_back({track, original, [owner = &app, scrollKey, track, thumb, travel, maximum] {
                               if (travel > 0) {
                                   owner->state.choices[scrollKey] =
                                       int(std::clamp((owner->mouseX - track.x - thumb / 2) / travel, 0.f, 1.f) *
                                           maximum);
                               }
                               owner->refresh();
                           }});
        y += 34;
    }
    y += 8;
}

void Page::button(const std::wstring &title, std::function<void()> action) {
    ui.button({x, y, 154, 34}, title, std::move(action));
    y += 44;
}

void Page::chart(const std::vector<Point> &curve, const std::vector<Point> &points, Curve exact) {
    if (curve.empty()) {
        return;
    }
    heading(L"插值曲线 / 特性曲线");
    ui.fill({x, y, w, 310}, White, 10);
    ui.border({x, y, w, 310}, Border, 10);
    double minY = curve.front().y, maxY = minY;
    for (auto p : curve) {
        minY = std::min(minY, p.y);
        maxY = std::max(maxY, p.y);
    }
    Axis xa(L"x", curve.front().x, curve.back().x, 5, false), ya(L"y", minY, maxY, 4, true);
    Rect plot{x + 90, y + 24, std::max(40.f, w - 114), 235};
    for (auto tick : ya.ticks) {
        float py = plot.y + plot.h * (1 - float(ya.tickFraction(tick.value)));
        ui.line(plot.x, py, plot.x + plot.w, py, Border);
        ui.text({x + 4, py - 12, 78, 24}, tick.label, 11, Muted, false, true);
    }
    for (auto tick : xa.ticks) {
        float px = plot.x + plot.w * float(xa.tickFraction(tick.value));
        ui.line(px, plot.y, px, plot.y + plot.h, Border);
        ui.text({px - 34, plot.y + plot.h + 6, 68, 24}, tick.label, 11, Muted, false, true);
    }
    ui.text({x + 90, y + 2, std::max(40.f, w - 114), 22}, ya.caption, 11, Muted);
    ui.text({x + 90, y + 283, std::max(40.f, w - 114), 24}, xa.caption, 11, Muted, false, true);
    auto old = ui.clip;
    ui.clip = intersectRect(old, plot);
    auto at = [&](Point p) {
        return Point{plot.x + plot.w * xa.fraction(p.x), plot.y + plot.h * (1 - ya.fraction(p.y))};
    };
    for (std::size_t i = 1; i < curve.size(); ++i) {
        auto a = at(curve[i - 1]), b = at(curve[i]);
        ui.line(float(a.x), float(a.y), float(b.x), float(b.y), Blue, 2);
    }
    for (auto p : points) {
        auto q = at(p);
        ui.fill({float(q.x) - 3, float(q.y) - 3, 6, 6}, Red, 3);
    }
    app.chartArea = plot;
    if (plot.contains(app.mouseX, app.mouseY) && old.contains(app.mouseX, app.mouseY)) {
        double fraction = (app.mouseX - plot.x) / plot.w;
        double px = xa.origin + xa.factor * (xa.minimum + fraction * (xa.maximum - xa.minimum));
        auto right =
            std::lower_bound(curve.begin(), curve.end(), px, [](Point p, double x) { return p.x < x; });
        double py = curve.back().y;
        if (exact) {
            py = exact(px);
        } else if (right == curve.begin()) {
            py = right->y;
        } else if (right != curve.end()) {
            auto left = right - 1;
            py = left->y + (right->y - left->y) * (px - left->x) / (right->x - left->x);
        }
        auto point = at({px, py});
        ui.line(float(point.x), plot.y, float(point.x), plot.y + plot.h, Muted);
        ui.fill({float(point.x) - 4, float(point.y) - 4, 8, 8}, Blue, 4);
        ui.fill({plot.x + 8, plot.y + 8, std::min(300.f, plot.w - 16), 54}, Soft, 7);
        ui.border({plot.x + 8, plot.y + 8, std::min(300.f, plot.w - 16), 54}, Border, 7);
        ui.text({plot.x + 16, plot.y + 10, plot.w - 32, 24}, L"x = " + format(px), 12, Blue, false, false, false,
                true);
        ui.text({plot.x + 16, plot.y + 34, plot.w - 32, 24}, L"y = " + format(py), 12, Blue, false, false, false,
                true);
    }
    ui.clip = old;
    y += 324;
}
} // 命名空间 toolbox
