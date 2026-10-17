// CSV 在同目录临时文件中写完、刷新后再原子替换；取消或失败保留已有目标文件。
#include "toolbox/gui.hpp"
#include <commdlg.h>
namespace toolbox {
ExportJob::~ExportJob() {
    cancel = true;
    if (worker.joinable()) {
        worker.join();
    }
}
void App::exportCsv(const std::wstring &name, const std::string &header, std::size_t count,
                    std::function<std::string(std::size_t)> row) {
    if (exportJob.active) {
        MessageBoxW(window_, L"已有导出任务正在进行。", L"工具箱", MB_OK);
        return;
    }
    std::vector<wchar_t> file(32768);
    std::copy(name.begin(), name.end(), file.begin());
    OPENFILENAMEW dialog{};
    dialog.lStructSize = sizeof(dialog);
    dialog.hwndOwner = window_;
    dialog.lpstrFilter = L"CSV 文件\0*.csv\0\0";
    dialog.lpstrFile = file.data();
    dialog.nMaxFile = DWORD(file.size());
    dialog.lpstrTitle = L"导出完整计算结果";
    dialog.lpstrDefExt = L"csv";
    dialog.Flags = OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
    if (!GetSaveFileNameW(&dialog)) {
        return;
    }
    if (exportJob.worker.joinable()) {
        exportJob.worker.join();
    }
    exportJob.cancel = false;
    exportJob.active = true;
    exportJob.progress = 0;
    exportJob.total = count;
    {
        std::lock_guard<std::mutex> lock(exportJob.mutex);
        exportJob.message = L"正在导出…";
    }
    std::wstring path = file.data();
    exportJob.worker = std::thread([this, path, header, count, row] {
        std::wstring temporary = path + L".toolbox-" + std::to_wstring(GetCurrentProcessId()) + L"-" +
                                 std::to_wstring(GetTickCount()) + L".tmp";
        HANDLE output = INVALID_HANDLE_VALUE;
        std::wstring message;
        try {
            output = CreateFileW(temporary.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                                 FILE_ATTRIBUTE_NORMAL, nullptr);
            require(output != INVALID_HANDLE_VALUE, "无法创建导出暂存文件");
            auto write = [&](const std::string &data) {
                DWORD n = 0;
                require(WriteFile(output, data.data(), DWORD(data.size()), &n, nullptr) && n == data.size(),
                        "CSV 写入失败");
            };
            std::string buffer = header;
            buffer.reserve(256 * 1024);
            for (std::size_t i = 0; i < count; ++i) {
                if (i % 1024 == 0) {
                    require(!exportJob.cancel, "导出已取消");
                    exportJob.progress = i;
                }
                buffer += row(i);
                if (buffer.size() >= 240 * 1024) {
                    write(buffer);
                    buffer.clear();
                }
            }
            require(!exportJob.cancel, "导出已取消");
            if (!buffer.empty()) {
                write(buffer);
            }
            require(FlushFileBuffers(output), "无法刷新导出文件");
            CloseHandle(output);
            output = INVALID_HANDLE_VALUE;
            require(MoveFileExW(temporary.c_str(), path.c_str(),
                                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH),
                    "无法替换目标文件，请确认未被其他程序占用");
            exportJob.progress = count;
            message = L"已导出：" + path;
        } catch (const std::exception &e) {
            if (output != INVALID_HANDLE_VALUE) {
                CloseHandle(output);
            }
            DeleteFileW(temporary.c_str());
            message = wide(e.what());
        }
        {
            std::lock_guard<std::mutex> lock(exportJob.mutex);
            exportJob.message = message;
        }
        exportJob.active = false;
        refresh();
    });
    refresh();
}
void Page::division(const std::string &field, std::pair<double, double> bounds, TemperatureUnit unit,
                    const std::wstring &signal, Curve curve) {
    if (!collapse("table." + field, L"分度表与特性曲线")) {
        return;
    }
    double lo = unit.display(bounds.first), hi = unit.display(bounds.second);
    std::string k = "table." + field + "." + std::to_string(unit.index);
    double start = number(k + ".start", L"起始温度 (" + unit.symbol() + L")", lo),
           end = number(k + ".end", L"终止温度 (" + unit.symbol() + L")", hi),
           step = number(k + ".step", L"温度步长 (" + unit.symbol() + L")", unit.span());
    check();
    require(start >= lo && end <= hi, "分度表范围超出所选曲线或校准分区");
    auto samples = divisionPoints(start, end, step);
    auto value = [curve, bounds, unit](double x) {
        // 分度表已核验显示温区；温标往返的末位舍入不能让端点落到有效区间之外。
        double t = std::clamp(unit.celsius(x), bounds.first, bounds.second),
               delta = std::max(std::abs(t) * std::numeric_limits<double>::epsilon() * 8, .01),
               lo = std::max(t - delta, bounds.first), hi = std::min(t + delta, bounds.second);
        double signal = curve(t), slope = (curve(hi) - curve(lo)) / (hi - lo) / unit.span();
        finite({signal, slope});
        return std::make_pair(signal, slope);
    };
    button(L"导出完整 CSV", [owner = &app, samples, value, unit, signal, name = tool.id] {
        owner->exportCsv(wide(name) + L"_division.csv",
                         "\xEF\xBB\xBF" + utf8(L"温度 (" + unit.symbol() + L"),信号 (" + signal +
                                               L"),变化率 (" + signal + L"/" + unit.symbol() + L")\r\n"),
                         samples.size(), [samples, value](std::size_t i) {
                             auto v = value(samples[i]);
                             return rawNumber(samples[i]) + "," + rawNumber(v.first) + "," +
                                    rawNumber(v.second) + "\r\n";
                         });
    });
    // 预览仅在本次页面构建内读取；导出闭包已按值持有快照，无需额外共享所有权。
    int pages = int((samples.size() + 9) / 10), page = state.choice(key(k + ".page"), 0);
    page = std::clamp(page, 0, pages - 1);
    state.choices[key(k + ".page")] = page;
    ui.button({x, y, 90, 32}, L"上一页", [owner = &app, key = key(k + ".page")] {
        owner->state.choices[key] = std::max(0, owner->state.choices[key] - 1);
        owner->refresh();
    });
    ui.button({x + 100, y, 90, 32}, L"下一页", [owner = &app, key = key(k + ".page"), pages] {
        owner->state.choices[key] = std::min(pages - 1, owner->state.choices[key] + 1);
        owner->refresh();
    });
    ui.text({x + 204, y, w - 204, 32},
            L"第 " + std::to_wstring(page + 1) + L" / " + std::to_wstring(pages) + L" 页 · " +
                std::to_wstring(samples.size()) + L" 行",
            12, Muted);
    y += 42;
    std::vector<std::vector<std::wstring>> rows;
    for (std::size_t i = page * 10; i < std::min(samples.size(), std::size_t(page * 10 + 10)); ++i) {
        auto v = value(samples[i]);
        rows.push_back({format(samples[i]), format(v.first), format(v.second)});
    }
    table({L"温度 (" + unit.symbol() + L")", L"信号 (" + signal + L")", L"变化率"}, rows);
    std::vector<Point> plot;
    for (int i = 0; i <= 100; ++i) {
        double x = start + (end - start) * i / 100;
        plot.push_back({x, value(x).first});
    }
    chart(plot, {}, [value](double x) { return value(x).first; });
}
} // namespace toolbox
