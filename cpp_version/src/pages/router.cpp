// 页面仅组织输入输出，数值算法集中在 models 与 core 中。
#include "toolbox/gui.hpp"
namespace toolbox {
void renderPage(Page &p) {
    if (p.tool.kind == Kind::Unit) {
        p.heading(L"换算参数");
        double value = p.number("value", L"输入数值", 1);
        const auto &list = units(p.tool.dimension);
        std::vector<std::wstring> labels;
        for (auto &u : list) {
            labels.push_back(u.name + L"  " + u.code);
        }
        int input = p.select("unit", L"输入单位", labels);
        p.check();
        p.heading(format(value) + L" " + list[input].code + L"  换算结果");
        for (std::size_t i = 0; i < list.size(); ++i) {
            double result = convert(p.tool.dimension, value, input, i);
            p.ui.text({p.x + 8, p.y, p.w * .56f - 16, 50}, formatUnit(result), 16, Blue);
            p.ui.text({p.x + p.w * .58f, p.y, p.w * .42f, 24}, list[i].name, 13);
            p.ui.text({p.x + p.w * .58f, p.y + 24, p.w * .42f, 26}, list[i].code, 13, Blue);
            p.y += 58;
            p.ui.line(p.x, p.y, p.x + p.w, p.y, Border);
            p.y += 4;
        }
        return;
    }
    switch (p.tool.group) {
    case Group::Its90:
        renderSensors(p);
        break;
    case Group::Pressure:
        renderPressure(p);
        break;
    case Group::Humidity:
        renderHumidity(p);
        break;
    case Group::Errors:
        renderErrors(p);
        break;
    case Group::Other:
        renderOther(p);
        break;
    default:
        break;
    }
    if (p.app.exportJob.active) {
        auto progress = p.app.exportJob.progress.load(), total = p.app.exportJob.total.load();
        p.note(L"导出中 " + std::to_wstring(total ? progress * 100 / total : 0) + L"%");
        p.button(L"取消导出", [owner = &p.app] {
            owner->exportJob.cancel = true;
            owner->refresh();
        });
    } else {
        std::lock_guard<std::mutex> lock(p.app.exportJob.mutex);
        if (!p.app.exportJob.message.empty()) {
            p.note(p.app.exportJob.message);
        }
    }
}
} // namespace toolbox
