// 系数档案与输入历史分开保存；所有数值保留用户输入的原始文本。
#include "toolbox/gui.hpp"
#include "toolbox/resource_ids.h"
#include <cwctype>

namespace toolbox {
namespace {
struct Archive { std::string tool; std::wstring name; State fields; };
constexpr const char *couple = "standard_thermocouple";
constexpr const char *platinum = "standard_platinum_resistor_sprt";

bool valid(const Archive &a) {
    try {
        if ((a.tool != couple && a.tool != platinum) || a.name.empty() || a.name.size() > 200) return false;
        std::set<std::string> values, choices;
        auto v = [&](const std::string &key) { values.insert(a.tool + "." + key); };
        auto c = [&](const std::string &key, int lo, int hi) {
            auto full = a.tool + "." + key; choices.insert(full);
            auto it = a.fields.choices.find(full);
            require(it != a.fields.choices.end() && it->second >= lo && it->second <= hi, "invalid choice");
            return it->second;
        };
        if (a.tool == couple) {
            int type = c("type", 0, 1), mode = c("calibration", 0, 1);
            const std::string kind = type == 0 ? "S" : "B";
            if (!mode) {
                for (int i = 0; i < (type == 0 ? 3 : 5); ++i) v(kind + ".cal." + std::to_string(i));
            } else {
                auto zones = standardZones(kind);
                int zone = c(kind + ".zone", 0, int(zones.size()) - 1);
                int source = c("source", 0, 1);
                auto key = kind + "." + std::to_string(zone);
                if (!source) {
                    for (std::size_t i = 0; i < zones[zone].coefficients.size(); ++i) v(key + ".c." + std::to_string(i));
                } else {
                    int count = c("fit.count", 2, 30);
                    for (int i = 0; i < count; ++i) {
                        auto point = "fit." + key + "." + std::to_string(i);
                        v(point + ".x"); v(point + ".y");
                    }
                }
            }
        } else if (c("model", 0, 1) == 1) {
            v("cvd.r0");
            for (int i = 0; i < 3; ++i) {
                auto point = "cvd.point." + std::to_string(i);
                v(point + ".x"); v(point + ".y");
            }
        } else {
            int hr = c("hr", 0, 6), lr = c("lr", 0, 2);
            require(hr != 6 || lr != 2, "invalid range");
            bool full = c("deviation", 0, 1) == 0;
            int source = c("source", 0, 1);
            v("rtp");
            if (!source) {
                if (hr != 6) {
                    v("coefficient.a");
                    if (!full || hr <= 3) v("coefficient.b");
                    if (!full || hr <= 1) v("coefficient.c");
                }
                if (lr != 2) {
                    auto suffix = lr == 0 ? "4" : "5";
                    v(std::string("coefficient.a") + suffix); v(std::string("coefficient.b") + suffix);
                }
                if (full && hr == 0) { v("coefficient.d"); v("coefficient.w660"); }
            } else {
                const std::vector<Values> high{{231.928, 419.527, 660.323}, {231.928, 419.527, 660.323},
                    {231.928, 419.527}, {156.599, 231.928}, {156.599}, {29.765}, {}};
                const Values low = lr == 0 ? Values{-189.344, -38.834} : (lr == 1 ? Values{-38.834, 29.765} : Values{});
                for (double t : high[hr]) v("fixed." + rawNumber(t));
                for (double t : low) v("fixed." + rawNumber(t));
                if (full && hr == 0) v("fixed.961.78");
            }
        }
        if (a.fields.values.size() != values.size() || a.fields.choices.size() != choices.size()) return false;
        for (const auto &key : values) {
            auto it = a.fields.values.find(key);
            if (it == a.fields.values.end()) return false;
            parse(utf8(it->second));
        }
        return true;
    } catch (const std::exception &) { return false; }
}

bool load(std::vector<Archive> &archives) {
    State data;
    if (!readCalibrationJson(calibrationPath(), data)) return false;
    std::map<std::string, Archive> rows;
    for (const auto &entry : data.values) {
        auto dot = entry.first.find('.');
        if (dot == std::string::npos || dot == 0 || dot > 8 ||
            entry.first.substr(0, dot).find_first_not_of("0123456789") != std::string::npos) return false;
        auto &row = rows[entry.first.substr(0, dot)];
        auto field = entry.first.substr(dot + 1);
        if (field == "name") row.name = entry.second;
        else if (field == "tool") row.tool = utf8(entry.second);
        else if (field.compare(0, 2, "v.") == 0) row.fields.values.emplace(field.substr(2), entry.second);
        else return false;
    }
    for (const auto &entry : data.choices) {
        auto dot = entry.first.find('.');
        if (dot == std::string::npos || dot == 0 || dot > 8 ||
            entry.first.substr(0, dot).find_first_not_of("0123456789") != std::string::npos ||
            entry.first.compare(dot + 1, 2, "c.") != 0) return false;
        rows[entry.first.substr(0, dot)].fields.choices.emplace(entry.first.substr(dot + 3), entry.second);
    }
    std::set<std::pair<std::string, std::wstring>> names;
    for (auto &row : rows) {
        if (!valid(row.second) || !names.emplace(row.second.tool, row.second.name).second) return false;
        archives.push_back(std::move(row.second));
    }
    return true;
}
bool save(const std::vector<Archive> &archives) {
    State data;
    for (std::size_t i = 0; i < archives.size(); ++i) {
        const auto prefix = std::to_string(i) + ".";
        data.values[prefix + "name"] = archives[i].name;
        data.values[prefix + "tool"] = wide(archives[i].tool);
        for (const auto &value : archives[i].fields.values) data.values[prefix + "v." + value.first] = value.second;
        for (const auto &choice : archives[i].fields.choices) data.choices[prefix + "c." + choice.first] = choice.second;
    }
    return writeCalibrationJson(calibrationPath(), data);
}
void alert(HWND owner, const wchar_t *message) { MessageBoxW(owner, message, L"系数档案", MB_OK | MB_ICONWARNING); }
struct NameDialog { std::wstring name; };
INT_PTR CALLBACK nameProcedure(HWND dialog, UINT message, WPARAM w, LPARAM l) {
    auto *data = reinterpret_cast<NameDialog *>(GetWindowLongPtrW(dialog, DWLP_USER));
    if (message == WM_INITDIALOG) {
        data = reinterpret_cast<NameDialog *>(l);
        SetWindowLongPtrW(dialog, DWLP_USER, reinterpret_cast<LONG_PTR>(data));
        SendDlgItemMessageW(dialog, TOOLBOX_CALIBRATION_NAME, EM_SETLIMITTEXT, 200, 0);
        SetDlgItemTextW(dialog, TOOLBOX_CALIBRATION_NAME, data->name.c_str());
        return TRUE;
    }
    if (message == WM_COMMAND && LOWORD(w) == IDCANCEL) { EndDialog(dialog, IDCANCEL); return TRUE; }
    if (message == WM_COMMAND && LOWORD(w) == IDOK && data) {
        wchar_t buffer[201]{};
        GetDlgItemTextW(dialog, TOOLBOX_CALIBRATION_NAME, buffer, 201);
        std::wstring name(buffer);
        auto first = name.find_first_not_of(L" \t\r\n");
        if (first == std::wstring::npos) { alert(dialog, L"请输入档案名称。"); return TRUE; }
        name = name.substr(first, name.find_last_not_of(L" \t\r\n") - first + 1);
        std::size_t characters = 0;
        for (wchar_t ch : name) if (ch < 0xdc00 || ch > 0xdfff) ++characters;
        if (characters > 100) { alert(dialog, L"档案名称不能超过 100 个字。"); return TRUE; }
        data->name = name;
        EndDialog(dialog, IDOK);
        return TRUE;
    }
    return FALSE;
}
} // namespace

void App::captureCalibration(Page &page, std::size_t first) {
    State snapshot;
    for (std::size_t i = first; i < scene.controls.size(); ++i) {
        const auto &control = scene.controls[i];
        if (control.options.empty()) snapshot.values[control.key] = state.values.at(control.key);
        else snapshot.choices[control.key] = state.choices.at(control.key);
    }
    const auto count = page.key("fit.count");
    if (page.tool.kind == Kind::StandardThermocouple && state.choices[page.key("calibration")] == 1 &&
        state.choices[page.key("source")] == 1) snapshot.choices[count] = state.choices.at(count);
    Archive archive{page.tool.id, L"snapshot", snapshot};
    if (valid(archive)) { calibrationSnapshot = std::move(snapshot); calibrationValid = true; }
}

void App::selectCalibration(int index) {
    if (selected < 0) return;
    const auto &tool = tools().at(std::size_t(selected));
    std::vector<Archive> all;
    if (!load(all)) { alert(window_, L"系数档案无法读取或格式无效，原文件未被覆盖。"); refresh(); return; }
    calibrationSelection.erase(tool.id);
    int current = 0;
    for (const auto &item : all) if (item.tool == tool.id && ++current == index) {
        for (const auto &value : item.fields.values) state.values[value.first] = value.second;
        for (const auto &choice : item.fields.choices) state.choices[choice.first] = choice.second;
        calibrationSelection[tool.id] = item.name;
        break;
    }
    refresh();
}

void App::calibrationControls(Page &p) {
    std::vector<Archive> all;
    const bool readable = load(all);
    const auto id = p.tool.id;
    std::vector<std::wstring> names{L"选择已保存的档案"};
    int selectedIndex = 0;
    for (const auto &item : all) if (item.tool == id) {
        names.push_back(item.name);
        if (calibrationSelection[id] == item.name) selectedIndex = int(names.size()) - 1;
    }
    if (!selectedIndex) calibrationSelection.erase(id);
    const float y = p.y;
    p.ui.text({p.x, y, 76, 36}, L"系数档案", 13, Muted);
    float comboWidth = std::min(230.f, std::max(100.f, p.w - 76.f));
    p.ui.combo("__calibration.selection", {p.x + 76, y, comboWidth, 36}, names, selectedIndex);
    float buttonX = p.x + 76 + comboWidth + 8;
    float buttonY = y;
    if (buttonX + (selectedIndex ? 312.f : 104.f) > p.x + p.w) { buttonX = p.x; buttonY += 44; }
    auto action = [this, id](int operation) {
        std::vector<Archive> entries;
        if (!load(entries)) { alert(window_, L"系数档案无法读取或格式无效，原文件未被覆盖。"); return; }
        auto chosen = calibrationSelection.find(id);
        std::wstring target = chosen == calibrationSelection.end() ? L"" : chosen->second;
        if (operation == 0) {
            NameDialog dialog;
            if (DialogBoxParamW(GetModuleHandleW(nullptr), MAKEINTRESOURCEW(TOOLBOX_CALIBRATION_DIALOG), window_, nameProcedure,
                                reinterpret_cast<LPARAM>(&dialog)) != IDOK) return;
            target = dialog.name;
        }
        auto find = [&] { return std::find_if(entries.begin(), entries.end(), [&](const Archive &a) { return a.tool == id && a.name == target; }); };
        auto existing = find();
        if (operation != 0 && existing == entries.end()) { alert(window_, L"所选档案不存在，请重新选择。"); refresh(); return; }
        if (operation == 2) {
            if (MessageBoxW(window_, (L"确定删除档案“" + target + L"”吗？").c_str(), L"删除系数档案", MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2) != IDYES) return;
            entries.erase(existing);
        } else {
            if (operation == 1 || existing != entries.end()) {
                auto text = operation == 1 ? L"确定用当前证书参数更新档案“" + target + L"”吗？" : L"档案“" + target + L"”已存在，确定用当前证书参数覆盖吗？";
                if (MessageBoxW(window_, text.c_str(), operation == 1 ? L"更新系数档案" : L"覆盖系数档案", MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2) != IDYES) return;
            }
            rebuild();
            if (!calibrationValid) { alert(window_, L"请检查并填写完整有效的证书参数。"); return; }
            Archive archive{id, target, calibrationSnapshot};
            if (existing == entries.end()) entries.push_back(std::move(archive)); else *existing = std::move(archive);
        }
        if (!save(entries)) { alert(window_, L"无法保存系数档案，请检查本地目录权限和可用空间。"); return; }
        if (operation == 2) calibrationSelection.erase(id); else calibrationSelection[id] = target;
        refresh();
    };
    p.ui.button({buttonX, buttonY, 96, 36}, L"保存当前…", [action] { action(0); }, true);
    if (selectedIndex) {
        p.ui.button({buttonX + 104, buttonY, 96, 36}, L"更新选中…", [action] { action(1); });
        p.ui.button({buttonX + 208, buttonY, 96, 36}, L"删除…", [action] { action(2); });
    }
    p.y = buttonY + 44;
    p.note(readable ? L"保存当前模型、温区及对应证书参数，不保存待换算数值。调用档案会替换当前证书参数；其他温区需另存。" : L"系数档案无法读取或格式无效，原文件未被覆盖。");
}
} // namespace toolbox
