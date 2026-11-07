#include "toolbox/gui.hpp"
#include <commdlg.h>
#include <shellapi.h>

namespace toolbox {
namespace {
void button(Scene &ui, Rect rect, const std::wstring &label, std::function<void()> action, bool primary) {
    ui.fill(rect, primary ? Blue : White, 6);
    if (!primary) ui.border(rect, Border, 6);
    ui.text(rect, label, 14, primary ? White : Ink, false, true);
    ui.hits.push_back({rect, ui.clip, std::move(action)});
}
}

void App::requestPdfDownload(const std::wstring &url) {
    if (pdfJob->active) return;
    try {
        pdf::standard_number(url);
        pdfPendingUrl = url;
        pdfModal = PdfModal::Warning;
        pdfDialogFocus = 1; // 默认停在取消，确认后才发起网络请求。
        SetFocus(handle());
    } catch (const pdf::Failure &error) {
        std::lock_guard<std::mutex> lock(pdfJob->mutex);
        pdfJob->error = error.message;
    }
    refresh();
}

void App::startPdfDownload() {
    if (pdfJob->active) return;
    pdfModal = PdfModal::None;
    auto job = pdfJob;
    {
        std::lock_guard<std::mutex> lock(job->mutex);
        job->result.reset();
        job->error.clear();
        job->message.clear();
    }
    job->active = true;
    job->done = false;
    pdfSavePending = false;
    try {
        // 线程只持有任务状态，不引用 App 或 HWND；关闭窗口后不会访问已释放的界面。
        std::thread([job, url = pdfPendingUrl] {
            try {
                auto result = std::make_shared<pdf::Download>(pdf::download_pdf(url));
                std::lock_guard<std::mutex> lock(job->mutex);
                job->result = std::move(result);
            } catch (const pdf::Failure &error) {
                std::lock_guard<std::mutex> lock(job->mutex);
                job->error = error.message;
            } catch (const std::exception &) {
                std::lock_guard<std::mutex> lock(job->mutex);
                job->error = L"下载失败，请稍后重试。";
            }
            job->active = false;
            job->done = true;
        }).detach();
    } catch (const std::exception &) {
        job->active = false;
        std::lock_guard<std::mutex> lock(job->mutex);
        job->error = L"无法启动下载线程。";
    }
    refresh();
}

void App::pollPdfDownload() {
    if (pdfJob->done.exchange(false)) {
        std::lock_guard<std::mutex> lock(pdfJob->mutex);
        pdfSavePending = bool(pdfJob->result);
        refresh();
    }
    if (pdfSavePending && selected >= 0 && tools().at(std::size_t(selected)).kind == Kind::PdfDownload) {
        pdfSavePending = false;
        savePdf();
    }
}

void App::savePdf() {
    std::shared_ptr<pdf::Download> result;
    {
        std::lock_guard<std::mutex> lock(pdfJob->mutex);
        result = pdfJob->result;
    }
    if (!result) return;
    std::vector<wchar_t> path(32768);
    const auto name = pdf::default_filename(result->code, result->name);
    std::copy(name.begin(), name.end(), path.begin());
    OPENFILENAMEW dialog{};
    dialog.lStructSize = sizeof(dialog);
    dialog.hwndOwner = handle();
    dialog.lpstrTitle = L"保存计量规范 PDF";
    dialog.lpstrFilter = L"PDF 文件\0*.pdf\0\0";
    dialog.lpstrDefExt = L"pdf";
    dialog.lpstrFile = path.data();
    dialog.nMaxFile = DWORD(path.size());
    dialog.Flags = OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
    if (!GetSaveFileNameW(&dialog)) {
        std::lock_guard<std::mutex> lock(pdfJob->mutex);
        pdfJob->message = CommDlgExtendedError() ? L"无法打开保存窗口，请重试。" : L"已取消保存，可以点击“保存 PDF”重新选择位置。";
    } else {
        try {
            pdf::save_atomic(path.data(), result->data);
            std::lock_guard<std::mutex> lock(pdfJob->mutex);
            pdfJob->error.clear();
            pdfJob->message = L"已保存：" + std::wstring(path.data());
            pdfDialogText = result->code + L" " + result->name + L"\nPDF 页数：" + std::to_wstring(result->pages) + L" 页\n\n" + pdfJob->message;
            if (result->pages == 1) pdfDialogText += L"\n\nPDF 只有一页，请打开文件确认是否为规范正文。";
            pdfModal = PdfModal::Success;
            pdfDialogFocus = 0;
            SetFocus(handle());
        } catch (const pdf::Failure &error) {
            std::lock_guard<std::mutex> lock(pdfJob->mutex);
            pdfJob->error = error.message;
        }
    }
    refresh();
}

void renderPdfDownload(Page &p) {
    const float x = p.x, y = p.y, w = p.w, pad = 18;
    const auto hint = L"输入名称关键词或规范编号筛选，在结果中选择需要下载的规范。";
    const float hintHeight = p.app.paragraphHeight(hint, w - 2 * pad, 12);
    const auto source = L"数据来源：https://jjg.spc.org.cn/（截至 2026-11 的规范目录）";
    const float sourceHeight = p.app.paragraphHeight(source, w - 2 * pad, 12);
    const float inputY = y + pad + 30 + hintHeight + 10, sourceY = inputY + 54, buttonsY = sourceY + sourceHeight + 6;
    p.ui.fill({x, y, w, buttonsY + 40 + pad - y}, White, 8);
    p.ui.border({x, y, w, buttonsY + 40 + pad - y}, Border, 8);
    p.ui.text({x + pad, y + pad, w - 2 * pad, 26}, L"规范名称或编号", 15, Ink, true);
    p.ui.text({x + pad, y + pad + 30, w - 2 * pad, hintHeight}, hint, 12, Muted, false, false, true);
    p.ui.text({x + pad, sourceY, w - 2 * pad, sourceHeight}, source, 12, Muted, false, false, true);
    const auto query = p.state.value(p.key("query"), L"");
    if (!p.app.pdfJob->active) {
        p.ui.input(p.key("query"), {x + pad, inputY, w - 2 * pad, 42}, query);
    } else {
        p.ui.fill({x + pad, inputY, w - 2 * pad, 42}, Background, 6);
        p.ui.border({x + pad, inputY, w - 2 * pad, 42});
        p.ui.text({x + pad + 12, inputY, w - 2 * pad - 24, 42}, query, 14, Muted);
        p.ui.text({x + pad, buttonsY, w - 2 * pad, 40}, L"正在获取规范名称并检查 PDF…", 14, Blue);
    }
    if (!p.app.pdfJob->active) {
        button(p.ui, {x + pad, buttonsY, 132, 40}, L"打开规范网站", [owner = p.app.handle()] { ShellExecuteW(owner, L"open", L"https://jjg.spc.org.cn/resmea/", nullptr, nullptr, SW_SHOWNORMAL); }, false);
    }
    p.y = buttonsY + 40 + pad + 12;
    auto& lastQuery = p.state.value(p.key("last_query"), L"");
    auto& page = p.state.choice(p.key("page"), 0);
    if (lastQuery != query) { page = 0; lastQuery = query; }
    if (query.find_first_not_of(L" \t\r\n　") != std::wstring::npos) {
        const auto rows = pdf::search_standards(query);
        const int pages = std::max(1, int((rows.size() + 9) / 10));
        page = std::clamp(page, 0, pages - 1);
        p.heading(L"找到 " + std::to_wstring(rows.size()) + L" 条规范");
        if (rows.empty()) p.note(L"未找到匹配的规范，请尝试其他名称或编号。");
        struct ResultLayout {
            const pdf::Standard* standard;
            std::wstring type, details;
            float nameHeight, typeHeight, detailsHeight;
        };
        const float textWidth = std::max(120.f, w - 2 * pad - 138);
        std::vector<ResultLayout> layouts;
        float rowHeight = 0;
        for (size_t i = size_t(page) * 10; i < std::min(rows.size(), size_t(page + 1) * 10); ++i) {
            const auto* standard = rows[i];
            ResultLayout item{standard, standard->category,
                L"状态：" + standard->status + L"　发布：" + standard->published + L"　实施：" + (standard->effective.empty() ? L"未提供" : standard->effective),
                p.app.paragraphHeight(standard->name, textWidth, 15), 0, 0};
            item.typeHeight = p.app.paragraphHeight(item.type, textWidth, 12);
            item.detailsHeight = p.app.paragraphHeight(item.details, textWidth, 12);
            rowHeight = std::max(rowHeight, 2 * pad + 28 + item.nameHeight + 6 + item.typeHeight + 4 + item.detailsHeight);
            layouts.push_back(std::move(item));
        }
        for (const auto& item : layouts) {
            const auto* standard = item.standard;
            p.ui.fill({x, p.y, w, rowHeight}, White, 8);
            p.ui.border({x, p.y, w, rowHeight}, Border, 8);
            p.ui.text({x + pad, p.y + pad, textWidth, 24}, standard->code, 14, Blue, true);
            p.ui.text({x + pad, p.y + pad + 28, textWidth, item.nameHeight}, standard->name, 15, Ink, false, false, true);
            p.ui.text({x + pad, p.y + pad + 34 + item.nameHeight, textWidth, item.typeHeight}, item.type, 12, Muted, false, false, true);
            p.ui.text({x + pad, p.y + pad + 38 + item.nameHeight + item.typeHeight, textWidth, item.detailsHeight}, item.details, 12,
                standard->status == L"被代替" || standard->status == L"废止" ? Red : Muted, false, false, true);
            if (!p.app.pdfJob->active) button(p.ui, {x + w - pad - 112, p.y + (rowHeight - 40) / 2, 112, 40}, L"下载 PDF",
                [owner = &p.app, standard] { owner->requestPdfDownload(standard->url); }, true);
            p.y += rowHeight + 10;
        }
        if (!rows.empty()) {
            if (!p.app.pdfJob->active && page > 0) button(p.ui, {x, p.y, 112, 40}, L"上一页", [owner = &p.app, key = p.key("page")] {
                --owner->state.choices[key]; owner->scroll = 0; owner->refresh();
            }, false);
            p.ui.text({x + 124, p.y, w - 248, 40}, L"第 " + std::to_wstring(page + 1) + L" / " + std::to_wstring(pages) + L" 页", 13, Muted, false, true);
            if (!p.app.pdfJob->active && page + 1 < pages) button(p.ui, {x + w - 112, p.y, 112, 40}, L"下一页", [owner = &p.app, key = p.key("page")] {
                ++owner->state.choices[key]; owner->scroll = 0; owner->refresh();
            }, false);
            p.y += 50;
        }
        p.note(L"规范状态与实施日期请以规范网站为准。");
    }
    std::shared_ptr<pdf::Download> result;
    std::wstring error, message;
    {
        std::lock_guard<std::mutex> lock(p.app.pdfJob->mutex);
        result = p.app.pdfJob->result;
        error = p.app.pdfJob->error;
        message = p.app.pdfJob->message;
    }
    if (result) {
        p.metrics(L"下载结果", {{L"规范编号", result->code}, {L"规范名称", result->name.empty() ? L"未获取，请在保存时补充" : result->name}, {L"PDF 页数", std::to_wstring(result->pages) + L" 页"}});
        if (result->pages == 1) p.note(L"PDF 只有一页，请打开文件确认是否为规范正文。", Red);
        button(p.ui, {p.x, p.y, 112, 40}, L"保存 PDF…", [owner = &p.app] { owner->savePdf(); }, false);
        p.y += 52;
    }
    if (!error.empty()) p.note(error, Red);
    if (!message.empty()) p.note(message);
    const std::vector<std::wstring> lines{
        L"1. 按名称或编号搜索内置目录，选择规范后使用对应详情页链接获取 PDF。",
        L"2. 请求在线预览页面，提交 a100（规范编号）和 ismobile=1，取得 token 与 enc 参数。",
        L"POST https://jjg.spc.org.cn/resmea/view/stdonline",
        L"3. 沿用同一会话的 Cookie，以 token 作为查询参数、enc 作为 myfoxit 请求头，获取预览 PDF。",
        L"GET https://jjg.spc.org.cn/resmea/view/onlinereading?token=…",
        L"4. 检查 PDF 格式、已知错误提示文件及实际页数，再按所选位置保存网站返回的原始 PDF。",
        L"以上为网站在线预览使用的接口，网站调整或限流时可能无法获取。仅支持本站 HTTPS 详情页，单个 PDF 上限 100 MB；请按网站的电子文本使用说明使用文件。"
    };
    float height = 2 * pad + 30;
    for (const auto &line : lines) height += p.app.paragraphHeight(line, w - 2 * pad, 13) + 8;
    p.ui.fill({x, p.y, w, height}, Soft, 8);
    p.ui.text({x + pad, p.y + pad, w - 2 * pad, 24}, L"获取流程与接口说明", 14, Blue, true);
    p.y += pad + 30;
    for (const auto &line : lines) {
        const float h = p.app.paragraphHeight(line, w - 2 * pad, 13);
        p.ui.text({x + pad, p.y, w - 2 * pad, h}, line, 13, Ink, false, false, true);
        p.y += h + 8;
    }
    p.y += pad;
}

void App::renderPdfModal() {
    if (pdfModal == PdfModal::None) return;
    scene.clip = {0, 0, width, height};
    scene.controls.clear();
    scene.hits.clear();
    scene.scrollRegions.clear();
    scene.fill({0, 0, width, height}, Color(0, 0, 0, .38f));
    const bool warning = pdfModal == PdfModal::Warning;
    std::wstring text = warning ? L"短时间内频繁下载可能触发网站限流（HTTP 429 Too Many Requests），请适当控制下载频率。\n\n请按网站的电子文本使用说明使用下载文件。" : pdfDialogText;
    if (warning) for (const auto& standard : pdf::standard_catalog()) if (standard.url == pdfPendingUrl) {
        text = standard.code + L" " + standard.name + L"\n" + standard.status + L" · 实施日期：" +
            (standard.effective.empty() ? L"未提供" : standard.effective) + L"\n\n" + text;
        break;
    }
    const float w = std::min(488.f, width - 48), inner = w - 48;
    const float textHeight = paragraphHeight(text, inner, 14), h = textHeight + 144;
    const float x = (width - w) / 2, y = std::max(12.f, (height - h) / 2), buttonY = y + h - 64;
    scene.fill({x + 2, y + 6, w, h}, Color(0, 0, 0, .10f), 10);
    scene.fill({x, y, w, h}, White, 10);
    scene.border({x, y, w, h}, Border, 10);
    scene.text({x + 24, y + 24, inner, 30}, warning ? L"下载提示" : L"下载成功", 19, Ink, true);
    scene.text({x + 24, y + 66, inner, textHeight}, text, 14, Ink, false, false, true);
    const float primaryWidth = warning ? 168.f : 112.f;
    button(scene, {x + w - 24 - primaryWidth, buttonY, primaryWidth, 40}, warning ? L"知道了，继续下载" : L"确定", [this, warning] {
        if (warning) startPdfDownload(); else { pdfModal = PdfModal::None; refresh(); }
    }, true);
    if (warning) button(scene, {x + w - 24 - primaryWidth - 124, buttonY, 112, 40}, L"取消", [this] { pdfModal = PdfModal::None; refresh(); }, false);
    const Rect focused = warning && pdfDialogFocus == 1 ? Rect{x + w - 24 - primaryWidth - 124, buttonY, 112, 40} : Rect{x + w - 24 - primaryWidth, buttonY, primaryWidth, 40};
    scene.border({focused.x - 3, focused.y - 3, focused.w + 6, focused.h + 6}, Blue, 8);
}
} // namespace toolbox
