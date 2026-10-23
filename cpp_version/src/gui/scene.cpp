// 声明式界面场景。控件键值稳定，计算结果刷新不会重建正在输入的编辑框。
#include "toolbox/gui.hpp"
namespace toolbox {
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
void Scene::text(Rect r, const std::wstring &t, float size, Color c, bool bold, bool center, bool paragraph) {
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
    fill(r, selected ? Blue : Soft, 5);
    text(r, t, 13, selected ? White : Blue, false, true);
    hits.push_back({r, clip, std::move(action)});
}
void Scene::icon(Rect r) {
    Draw d;
    d.kind = 4;
    d.rect = r;
    d.clip = clip;
    draws.push_back(d);
}
void Scene::input(const std::string &key, Rect r, const std::wstring &value) {
    fill(r, White, 4);
    border(r);
    controls.push_back({key, r, clip, value, {}, 0});
}
void Scene::combo(const std::string &key, Rect r, const std::vector<std::wstring> &options, int selection) {
    controls.push_back({key, r, clip, L"", options, selection});
}
Page::Page(App &owner, Scene &scene, const Tool &definition, float left, float top, float width)
    : app(owner), ui(scene), state(owner.state), tool(definition), x(left), y(top), w(width) {}
std::string Page::key(const std::string &f) const { return tool.id + "." + f; }
void Page::heading(const std::wstring &t) {
    y += 12;
    ui.text({x, y, w, 26}, t, 15, Ink, true);
    y += 34;
}
void Page::note(const std::wstring &t, Color c) {
    float h = std::max(22.f, app.paragraphHeight(t, w));
    ui.text({x, y, w, h}, t, 13, c, false, false, true);
    y += h + 6;
}
void Page::explain() {
    // 说明只读取当前选择，公式默认折叠，不改变计算参数。
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
    const float padding = 16, innerWidth = w - 2 * padding;
    const float textHeight = app.paragraphHeight(sections.front().second, innerWidth);
    const float height = 2 * padding + 32 + textHeight;
    ui.fill({x, y, w, height}, White, 8);
    ui.border({x, y, w, height}, Border, 8);
    ui.fill({x, y + 16, 3, 22}, Blue, 1);
    ui.text({x + padding, y + padding, innerWidth, 24}, L"计算说明", 15, Ink, true);
    ui.text({x + padding, y + padding + 32, innerWidth, textHeight}, sections.front().second, 13, Ink, false,
            false, true);
    y += height + 8;
    if (sections.size() > 1 &&
        collapse("calculation.formula." + std::to_string(variant), L"详细公式与取值")) {
        for (std::size_t i = 1; i < sections.size(); ++i) {
            note(sections[i].second, Ink);
        }
    }
}
std::wstring Page::textInput(const std::string &f, const std::wstring &label, const std::wstring &initial) {
    auto &value = state.value(key(f), initial);
    bool inlineLabel = w >= 720;
    float labelWidth = 186;
    if (inlineLabel) {
        ui.text({x, y, labelWidth - 12, 36}, label, 13, Muted);
        ui.input(key(f), {x + labelWidth, y, w - labelWidth, 36}, value);
        y += 44;
    } else {
        ui.text({x, y, w, 24}, label, 13, Muted);
        y += 26;
        ui.input(key(f), {x, y, w, 36}, value);
        y += 46;
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
    bool inlineLabel = w >= 720;
    if (inlineLabel) {
        ui.text({x, y, 174, 36}, label, 13, Muted);
        ui.combo(key(f), {x + 186, y, std::min(270.f, w - 186), 36}, options, choice);
        y += 44;
    } else {
        ui.text({x, y, w, 24}, label, 13, Muted);
        y += 26;
        ui.combo(key(f), {x, y, w, 36}, options, choice);
        y += 46;
    }
    return choice;
}
int Page::count(const std::string &f, const std::wstring &label, int initial, int minimum, int maximum) {
    auto k = key(f);
    int &n = state.choice(k, initial);
    n = std::clamp(n, minimum, maximum);
    ui.text({x, y, std::max(120.f, w - 210), 34}, label + L" · " + std::to_wstring(n) + L" 项", 14, Ink,
            true);
    ui.button({x + w - 202, y, 96, 34}, L"− 减少一项", [thisApp = &app, k, minimum] {
        auto &n = thisApp->state.choices[k];
        n = std::max(minimum, n - 1);
        thisApp->refresh();
    });
    ui.button({x + w - 100, y, 100, 34}, L"＋ 添加一项", [thisApp = &app, k, maximum] {
        auto &n = thisApp->state.choices[k];
        n = std::min(maximum, n + 1);
        thisApp->refresh();
    });
    y += 44;
    return n;
}
bool Page::collapse(const std::string &f, const std::wstring &t, bool initial) {
    auto k = key(f);
    bool open = state.choice(k, initial ? 1 : 0) != 0;
    ui.button({x, y, w, 36}, (open ? L"⌄  " : L"›  ") + t, [owner = &app, k] {
        owner->state.choices[k] = !owner->state.choices[k];
        owner->refresh();
    });
    y += 44;
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
    bool scrolling = maximumHeight > 0 && n * 44 > maximumHeight;
    if (scrolling) {
        std::string scrollKey = key(f + ".scroll");
        int limit = int(n * 44 - maximumHeight);
        int &offset = state.choice(scrollKey, 0);
        offset = std::clamp(offset, 0, limit);
        Rect viewport{x, top, w, maximumHeight};
        ui.scrollRegions.push_back({viewport, previousClip, [owner = &app, scrollKey, limit](float delta) {
                                        int old = owner->state.choices[scrollKey];
                                        int value = std::clamp(old + int(delta), 0, limit);
                                        if (value == old) {
                                            return false; // 已到边界时让外层页面继续滚动。
                                        }
                                        owner->state.choices[scrollKey] = value;
                                        owner->refresh();
                                        return true;
                                    }});
        float clipTop = std::max(top, previousClip.y);
        float clipBottom = std::min(top + maximumHeight, previousClip.y + previousClip.h);
        ui.clip = {previousClip.x, clipTop, previousClip.w, std::max(0.f, clipBottom - clipTop)};
        y -= offset;
    }
    float cell = (w - 64) / 2;
    for (int i = 0; i < n; ++i) {
        auto p = i < int(initial.size()) ? initial[i] : std::make_pair(0., 0.);
        ui.text({x, y, 54, 36}, label + std::to_wstring(i + 1), 13, Blue);
        double rowY = y;
        float oldX = x, oldW = w;
        x += 60;
        w = cell;
        auto read = [&](const std::string &field, double def) {
            auto &raw = state.value(key(field), inputNumber(def));
            ui.input(key(field), {x, y, w, 36}, raw);
            try {
                return parse(utf8(raw));
            } catch (const std::exception &) {
                inputError = L"请填写完整的校准点坐标";
                return std::numeric_limits<double>::quiet_NaN();
            }
        };
        double a = read(f + "." + std::to_string(i) + ".x", p.first);
        x += cell + 4;
        double b = read(f + "." + std::to_string(i) + ".y", p.second);
        x = oldX;
        w = oldW;
        y = float(rowY) + 44;
        values.push_back({a, b});
    }
    if (scrolling) {
        ui.clip = previousClip;
        y = top + maximumHeight + 8;
        ui.border({x, top, w, maximumHeight});
    }
    return values;
}
void Page::check() const { require(inputError.empty(), utf8(inputError).c_str()); }
void Page::metrics(const std::wstring &title, const Metrics &entries) {
    check();
    heading(title);
    int columns = w > 830 ? 3 : (w > 510 ? 2 : 1);
    float cell = (w - 8 * (columns - 1)) / columns;
    int i = 0;
    for (const auto &e : entries) {
        float cx = x + (i % columns) * (cell + 8), cy = y + (i / columns) * 88;
        ui.fill({cx, cy, cell, 80}, White, 6);
        ui.border({cx, cy, cell, 80});
        ui.text({cx + 10, cy + 7, cell - 20, 24}, e.first, 12, Muted);
        ui.text({cx + 10, cy + 32, cell - 20, 39}, e.second, 16, Blue);
        ++i;
    }
    y += float((int(entries.size()) + columns - 1) / columns) * 88;
}
void Page::table(const std::vector<std::wstring> &headings,
                 const std::vector<std::vector<std::wstring>> &rows) {
    float cell = w / float(headings.size());
    ui.fill({x, y, w, 32}, Soft, 3);
    for (std::size_t i = 0; i < headings.size(); ++i) {
        ui.text({x + float(i) * cell + 8, y, cell - 16, 32}, headings[i], 13, Ink, true);
    }
    y += 34;
    for (std::size_t j = 0; j < rows.size(); ++j) {
        if (j % 2 == 0) {
            ui.fill({x, y, w, 34}, White);
        }
        for (std::size_t i = 0; i < rows[j].size(); ++i) {
            ui.text({x + float(i) * cell + 8, y, cell - 16, 34}, rows[j][i], 13);
        }
        y += 36;
    }
    y += 8;
}
void Page::button(const std::wstring &title, std::function<void()> action) {
    ui.button({x, y, 154, 36}, title, std::move(action));
    y += 46;
}
void Page::chart(const std::vector<Point> &curve, const std::vector<Point> &points, Curve exact) {
    if (curve.empty()) {
        return;
    }
    heading(L"插值曲线 / 特性曲线");
    ui.fill({x, y, w, 310}, White, 6);
    ui.border({x, y, w, 310});
    double minY = curve.front().y, maxY = minY;
    for (auto p : curve) {
        minY = std::min(minY, p.y);
        maxY = std::max(maxY, p.y);
    }
    Axis xa(L"x", curve.front().x, curve.back().x, 5, false), ya(L"y", minY, maxY, 4, true);
    Rect plot{x + 90, y + 24, w - 114, 235};
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
    ui.text({x + 90, y + 2, w - 114, 22}, ya.caption, 11, Muted);
    ui.text({x + 90, y + 283, w - 114, 24}, xa.caption, 11, Muted, false, true);
    auto old = ui.clip;
    ui.clip = {std::max(old.x, plot.x), std::max(old.y, plot.y),
               std::max(0.f, std::min(old.x + old.w, plot.x + plot.w) - std::max(old.x, plot.x)),
               std::max(0.f, std::min(old.y + old.h, plot.y + plot.h) - std::max(old.y, plot.y))};
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
        ui.fill({plot.x + 8, plot.y + 8, std::min(300.f, plot.w - 16), 54}, Soft, 5);
        ui.text({plot.x + 16, plot.y + 10, plot.w - 32, 24}, L"x = " + format(px), 12, Blue);
        ui.text({plot.x + 16, plot.y + 34, plot.w - 32, 24}, L"y = " + format(py), 12, Blue);
    }
    ui.clip = old;
    y += 324;
}
} // namespace toolbox
