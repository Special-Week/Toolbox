// Windows 7 基线窗口后端。支持浅色/深色双主题、Ctrl+K 快搜命令面板、多工具标签栏与 Toast 轻提示。
#include "toolbox/gui.hpp"
#include "toolbox/resource_ids.h"
#include <algorithm>
#include <commctrl.h>
#include <cstring>
#include <memory>
#include <windowsx.h>
namespace toolbox {
static constexpr UINT rebuildMessage = WM_APP + 1;
static bool containsFold(const std::wstring &text, const std::wstring &query) {
    if (query.empty())
        return true;
    const auto fold = [](wchar_t v) { return v >= L'A' && v <= L'Z' ? wchar_t(v + (L'a' - L'A')) : v; };
    return std::search(text.begin(), text.end(), query.begin(), query.end(),
                       [&](wchar_t a, wchar_t b) { return fold(a) == fold(b); }) != text.end();
}
static LRESULT CALLBACK editProcedure(HWND window, UINT msg, WPARAM w, LPARAM l, UINT_PTR, DWORD_PTR) {
    if (msg == WM_SETCURSOR && LOWORD(l) == HTCLIENT) {
        SetCursor(LoadCursorW(nullptr, IDC_IBEAM));
        return TRUE;
    }
    if (msg == WM_MOUSEWHEEL) {
        return SendMessageW(GetParent(window), msg, w, l);
    }
    if (msg == WM_KEYDOWN && (w == VK_ESCAPE || w == VK_RETURN || w == VK_UP || w == VK_DOWN)) {
        if (SendMessageW(GetParent(window), WM_APP + 2, w, reinterpret_cast<LPARAM>(window)))
            return 0;
    }
    if (msg == WM_NCDESTROY) {
        RemoveWindowSubclass(window, editProcedure, 1);
    }
    return DefSubclassProc(window, msg, w, l);
}
static LRESULT CALLBACK comboProcedure(HWND window, UINT msg, WPARAM w, LPARAM l, UINT_PTR, DWORD_PTR data) {
    HWND face = reinterpret_cast<HWND>(data);
    if (msg == WM_MOUSEWHEEL && !SendMessageW(window, CB_GETDROPPEDSTATE, 0, 0)) {
        return SendMessageW(GetParent(window), msg, w, l);
    }
    if (face && msg == WM_PAINT) {
        RECT rect{};
        GetClientRect(window, &rect);
        SetWindowPos(face, nullptr, 0, 0, rect.right, rect.bottom, SWP_NOZORDER | SWP_NOACTIVATE);
        SendMessageW(face, WM_SETFONT, SendMessageW(window, WM_GETFONT, 0, 0), FALSE);
        SendMessageW(face, CB_SETITEMHEIGHT, WPARAM(-1),
                     SendMessageW(window, CB_GETITEMHEIGHT, WPARAM(-1), 0));
        EnableWindow(face, IsWindowEnabled(window));
        const int selection = int(SendMessageW(window, CB_GETCURSEL, 0, 0));
        const int length = selection < 0 ? CB_ERR : int(SendMessageW(window, CB_GETLBTEXTLEN, selection, 0));
        std::wstring text(length < 0 ? 0 : std::size_t(length) + 1, L'\0');
        if (length >= 0) {
            SendMessageW(window, CB_GETLBTEXT, selection, reinterpret_cast<LPARAM>(text.data()));
            text.resize(length);
        }
        PAINTSTRUCT paint{};
        HDC dc = BeginPaint(window, &paint);
        SendMessageW(face, WM_PRINT, reinterpret_cast<WPARAM>(dc),
                     PRF_CLIENT | PRF_NONCLIENT | PRF_ERASEBKGND | PRF_CHILDREN);
        COMBOBOXINFO info{};
        info.cbSize = sizeof(info);
        if (GetComboBoxInfo(window, &info)) {
            RECT label = info.rcItem;
            label.left += 3;
            const int previousMode = SetBkMode(dc, TRANSPARENT);
            const COLORREF previousColor =
                SetTextColor(dc, GetSysColor(IsWindowEnabled(window) ? COLOR_WINDOWTEXT : COLOR_GRAYTEXT));
            HGDIOBJ previousFont =
                SelectObject(dc, reinterpret_cast<HFONT>(SendMessageW(window, WM_GETFONT, 0, 0)));
            DrawTextW(dc, text.c_str(), -1, &label,
                      DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_END_ELLIPSIS);
            if (previousFont)
                SelectObject(dc, previousFont);
            SetTextColor(dc, previousColor);
            SetBkMode(dc, previousMode);
            if (GetFocus() == window && !(SendMessageW(window, WM_QUERYUISTATE, 0, 0) & UISF_HIDEFOCUS))
                DrawFocusRect(dc, &info.rcItem);
        }
        EndPaint(window, &paint);
        return 0;
    }
    if (msg == WM_NCDESTROY) {
        RemoveWindowSubclass(window, comboProcedure, 1);
        if (face && IsWindow(face))
            DestroyWindow(face);
    }
    const LRESULT result = DefSubclassProc(window, msg, w, l);
    if (face &&
        (msg == WM_SETFOCUS || msg == WM_KILLFOCUS || msg == WM_ENABLE || msg == WM_LBUTTONDOWN ||
         msg == WM_LBUTTONUP || msg == CB_SETCURSEL || msg == WM_THEMECHANGED || msg == WM_SYSCOLORCHANGE)) {
        InvalidateRect(window, nullptr, FALSE);
    }
    return result;
}
template <class T> static void release(T *&object) {
    if (object) {
        object->Release();
        object = nullptr;
    }
}
static D2D1_RECT_F rectangle(Rect r) { return D2D1::RectF(r.x, r.y, r.x + r.w, r.y + r.h); }
static bool sameRect(Rect a, Rect b) { return a.x == b.x && a.y == b.y && a.w == b.w && a.h == b.h; }
static std::array<std::wstring, 3> specColumns(const std::wstring &text) {
    const auto first = text.find(L" ("), second = text.find(L" / ", first);
    if (first == std::wstring::npos || second == std::wstring::npos || text.empty() || text.back() != L')')
        return {text, L"", L""};
    return {text.substr(0, first), text.substr(first + 2, second - first - 2),
            text.substr(second + 3, text.size() - second - 4)};
}

App::~App() {
    exportJob.cancel = true;
    if (exportJob.worker.joinable()) {
        exportJob.worker.join();
    }
    if (window_ && IsWindow(window_)) {
        DestroyWindow(window_);
    }
    releaseTarget();
    release(textFactory_);
    release(factory_);
    if (font_) {
        DeleteObject(font_);
    }
    if (editBrush_) {
        DeleteObject(editBrush_);
    }
}
void App::releaseTarget() {
    release(iconBitmap_);
    for (auto &entry : dynamicMathBitmaps_)
        release(entry.second);
    dynamicMathBitmaps_.clear();
    release(target_);
}
void App::updateEditBrush() {
    if (editBrush_) {
        DeleteObject(editBrush_);
        editBrush_ = nullptr;
    }
    editBrush_ = CreateSolidBrush(darkMode ? RGB(19, 28, 43) : RGB(252, 253, 255));
}
void App::loadUiPrefs() {
    State prefs;
    if (!readCalibrationJson(uiPrefsPath(), prefs))
        return;
    darkMode = prefs.choices["theme.dark"] != 0;
    applyTheme(darkMode);
    favorites.clear();
    for (int i = 0; i < 64; ++i) {
        auto it = prefs.values.find("fav." + std::to_string(i));
        if (it == prefs.values.end())
            break;
        favorites.push_back(utf8(it->second));
    }
    recent.clear();
    for (int i = 0; i < 16; ++i) {
        auto it = prefs.values.find("recent." + std::to_string(i));
        if (it == prefs.values.end())
            break;
        recent.push_back(utf8(it->second));
    }
}
void App::saveUiPrefs() {
    State prefs;
    prefs.choices["theme.dark"] = darkMode ? 1 : 0;
    for (std::size_t i = 0; i < favorites.size(); ++i)
        prefs.values["fav." + std::to_string(i)] = wide(favorites[i]);
    for (std::size_t i = 0; i < recent.size(); ++i)
        prefs.values["recent." + std::to_string(i)] = wide(recent[i]);
    writeCalibrationJson(uiPrefsPath(), prefs);
}
void App::toggleTheme() {
    darkMode = !darkMode;
    applyTheme(darkMode);
    updateEditBrush();
    saveUiPrefs();
    refresh();
    if (window_)
        RedrawWindow(window_, nullptr, nullptr, RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_UPDATENOW);
}
bool App::isFavorite(const std::string &id) const {
    return std::find(favorites.begin(), favorites.end(), id) != favorites.end();
}
void App::toggleFavorite(const std::string &id, const std::wstring &name) {
    auto it = std::find(favorites.begin(), favorites.end(), id);
    if (it != favorites.end()) {
        favorites.erase(it);
        showToast(L"已取消收藏：" + name);
    } else {
        favorites.insert(favorites.begin(), id);
        showToast(L"已收藏：" + name);
    }
    saveUiPrefs();
    refresh();
}
void App::closeTab(int toolIndex) {
    auto it = std::find(openTabs.begin(), openTabs.end(), toolIndex);
    if (it == openTabs.end())
        return;
    std::size_t pos = std::size_t(it - openTabs.begin());
    openTabs.erase(it);
    if (selected == toolIndex) {
        if (openTabs.empty()) {
            open(-1);
        } else {
            open(openTabs[std::min(pos, openTabs.size() - 1)]);
        }
    } else {
        refresh();
    }
}
void App::openPalette() {
    paletteOpen = true;
    paletteHighlight = 0;
    state.values["__palette.query"] = L"";
    refresh();
}
void App::closePalette() {
    paletteOpen = false;
    SetFocus(window_);
    refresh();
}
void App::showToast(const std::wstring &text) {
    toastMessage = text;
    toastExpire = GetTickCount64() + 2400;
    refresh();
}
void App::copyToClipboard(const std::wstring &text, const std::wstring &toastText) {
    struct Allocation {
        HGLOBAL handle = nullptr;
        ~Allocation() {
            if (handle)
                GlobalFree(handle);
        }
    } allocation;
    const auto bytes = (text.size() + 1) * sizeof(wchar_t);
    allocation.handle = GlobalAlloc(GMEM_MOVEABLE, bytes);
    if (!allocation.handle)
        return;
    auto buffer = GlobalLock(allocation.handle);
    if (!buffer)
        return;
    std::memcpy(buffer, text.c_str(), bytes);
    GlobalUnlock(allocation.handle);
    if (!OpenClipboard(window_))
        return;
    struct Guard {
        ~Guard() { CloseClipboard(); }
    } guard;
    if (EmptyClipboard() && SetClipboardData(CF_UNICODETEXT, allocation.handle)) {
        allocation.handle = nullptr;
        showToast(toastText.empty() ? L"已复制到剪贴板" : toastText);
    }
}
void App::refresh() {
    if (window_) {
        PostMessageW(window_, rebuildMessage, 0, 0);
    }
}
void App::open(int index) {
    if (index >= 0 && index != selected && tools().at(std::size_t(index)).kind != Kind::PdfDownload &&
        !remembersInputs(tools().at(std::size_t(index)))) {
        clearToolInputs(tools().at(std::size_t(index)));
    }
    selected = index;
    scroll = 0;
    if (index >= 0) {
        const auto &tool = tools().at(std::size_t(index));
        expanded[int(tool.group)] = true;
        if (std::find(openTabs.begin(), openTabs.end(), index) == openTabs.end()) {
            openTabs.push_back(index);
        }
        recent.erase(std::remove(recent.begin(), recent.end(), tool.id), recent.end());
        recent.insert(recent.begin(), tool.id);
        if (recent.size() > 8)
            recent.resize(8);
        saveUiPrefs();
    }
    refresh();
}
void App::clearToolInputs(const Tool &tool) {
    const auto prefix = tool.id + ".";
    for (auto it = state.values.begin(); it != state.values.end();)
        if (it->first.compare(0, prefix.size(), prefix) == 0)
            it = state.values.erase(it);
        else
            ++it;
    for (auto it = state.choices.begin(); it != state.choices.end();)
        if (it->first.compare(0, prefix.size(), prefix) == 0)
            it = state.choices.erase(it);
        else
            ++it;
    for (auto it = openDetails.begin(); it != openDetails.end();)
        if (it->compare(0, prefix.size(), prefix) == 0)
            it = openDetails.erase(it);
        else
            ++it;
    clearGaugeCache(tool.id);
    SetFocus(window_);
}
void App::restoreExample() {
    if (selected < 0 || exportJob.active)
        return;
    const auto &tool = tools().at(std::size_t(selected));
    if (tool.kind == Kind::PdfDownload) {
        if (pdfJob->active)
            return;
        std::lock_guard<std::mutex> lock(pdfJob->mutex);
        pdfJob->result.reset();
        pdfJob->error.clear();
        pdfJob->message.clear();
        pdfSavePending = false;
    }
    clearToolInputs(tool);
    if (tool.kind == Kind::Uncertainty)
        uncertaintyColumns = {};
    calibrationSelection.erase(tool.id);
    inputHistory_.observe(tool, state);
    inputHistory_.flush(true);
    showToast(tool.kind == Kind::PdfDownload ? L"已清空检索条件" : L"已恢复示例输入");
    refresh();
}
int App::run(HINSTANCE instance, int show) {
    applyTheme(false);
    loadUiPrefs();
    inputHistory_.load(state);
    auto user = GetModuleHandleW(L"user32.dll");
    using SetContext = BOOL(WINAPI *)(HANDLE);
    auto contextAddress = GetProcAddress(user, "SetProcessDpiAwarenessContext");
    SetContext setContext = nullptr;
    static_assert(sizeof(setContext) == sizeof(contextAddress), "Win32 函数指针大小不一致");
    std::memcpy(&setContext, &contextAddress, sizeof(setContext));
    if (!setContext || !setContext(reinterpret_cast<HANDLE>(-4))) {
        SetProcessDPIAware();
    }
    INITCOMMONCONTROLSEX common{sizeof(common), ICC_STANDARD_CLASSES};
    InitCommonControlsEx(&common);
    WNDCLASSEXW wc{};
    wc.cbSize = sizeof(wc);
    wc.lpfnWndProc = procedure;
    wc.hInstance = instance;
    wc.lpszClassName = L"ToolboxCppWindow";
    wc.hCursor = LoadCursorW(nullptr, IDC_ARROW);
    wc.hIcon = LoadIconW(instance, MAKEINTRESOURCEW(TOOLBOX_ICON));
    wc.hIconSm = static_cast<HICON>(LoadImageW(instance, MAKEINTRESOURCEW(TOOLBOX_ICON), IMAGE_ICON,
                                               GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON),
                                               LR_SHARED));
    RegisterClassExW(&wc);
    HRESULT result = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, &factory_);
    if (FAILED(result) || FAILED(DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, __uuidof(IDWriteFactory),
                                                     reinterpret_cast<IUnknown **>(&textFactory_)))) {
        MessageBoxW(nullptr, L"无法初始化 Direct2D / DirectWrite。Windows 7 请安装 SP1 及系统图形组件更新。",
                    L"工具箱", MB_ICONERROR);
        return 1;
    }
    HDC dc = GetDC(nullptr);
    dpi_ = float(GetDeviceCaps(dc, LOGPIXELSX));
    ReleaseDC(nullptr, dc);
    updateEditBrush();
    window_ = CreateWindowExW(0, wc.lpszClassName, L"工具箱 · 桌面版 v2026.11.20", WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
                              CW_USEDEFAULT, CW_USEDEFAULT, int(1280 * dpi_ / 96), int(840 * dpi_ / 96),
                              nullptr, nullptr, instance, this);
    if (!window_) {
        return 1;
    }
    if (auto menu = GetSystemMenu(window_, FALSE)) {
        AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
        AppendMenuW(menu, MF_STRING, TOOLBOX_LICENSE_MENU, L"开源许可…");
    }
    ShowWindow(window_, show);
    UpdateWindow(window_);
    SetTimer(window_, 1, 200, nullptr);
    MSG msg{};
    while (GetMessageW(&msg, nullptr, 0, 0) > 0) {
        if (msg.message == WM_KEYDOWN && msg.wParam == 'K' && (GetKeyState(VK_CONTROL) & 0x8000)) {
            if (paletteOpen)
                closePalette();
            else
                openPalette();
            continue;
        }
        if (msg.message == WM_KEYDOWN && msg.wParam == VK_TAB && IsDialogMessageW(window_, &msg)) {
            continue;
        }
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    return int(msg.wParam);
}
LRESULT CALLBACK App::procedure(HWND window, UINT msg, WPARAM w, LPARAM l) {
    App *app = reinterpret_cast<App *>(GetWindowLongPtrW(window, GWLP_USERDATA));
    if (msg == WM_NCCREATE) {
        app = static_cast<App *>(reinterpret_cast<CREATESTRUCTW *>(l)->lpCreateParams);
        app->window_ = window;
        SetWindowLongPtrW(window, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(app));
    }
    try {
        return app ? app->message(msg, w, l) : DefWindowProcW(window, msg, w, l);
    } catch (const std::exception &error) {
        auto text = wide(error.what());
        MessageBoxW(window, text.c_str(), L"工具箱", MB_ICONERROR);
        return 0;
    }
}
LRESULT App::message(UINT msg, WPARAM w, LPARAM l) {
    switch (msg) {
    case WM_APP + 2: {
        // 命令面板输入框按键转发：支持 Esc/上下键/回车直达
        if (!paletteOpen)
            return 0;
        HWND child = reinterpret_cast<HWND>(l);
        auto it = nativeKeys_.find(child);
        if (it == nativeKeys_.end() || it->second != "__palette.query")
            return 0;
        if (w == VK_ESCAPE) {
            closePalette();
            return 1;
        }
        std::wstring q = state.values["__palette.query"];
        std::vector<int> matches;
        for (std::size_t i = 0; i < tools().size(); ++i) {
            const auto &t = tools()[i];
            if (containsFold(t.name, q) || containsFold(groupName(t.group), q) ||
                containsFold(t.description, q) || containsFold(wide(t.id), q)) {
                matches.push_back(int(i));
                if (matches.size() >= 10)
                    break;
            }
        }
        if (w == VK_DOWN && !matches.empty()) {
            paletteHighlight = (paletteHighlight + 1) % int(matches.size());
            refresh();
            return 1;
        }
        if (w == VK_UP && !matches.empty()) {
            paletteHighlight = (paletteHighlight - 1 + int(matches.size())) % int(matches.size());
            refresh();
            return 1;
        }
        if (w == VK_RETURN && !matches.empty()) {
            int chosen = matches[std::clamp(paletteHighlight, 0, int(matches.size()) - 1)];
            closePalette();
            open(chosen);
            return 1;
        }
        return 0;
    }
    case WM_SYSCOMMAND:
        if ((w & 0xFFF0) == TOOLBOX_LICENSE_MENU) {
            showLicenses(window_, dpi_);
            return 0;
        }
        break;
    case WM_CREATE:
        refresh();
        return 0;
    case WM_GETMINMAXINFO: {
        auto m = reinterpret_cast<MINMAXINFO *>(l);
        m->ptMinTrackSize = {LONG(620 * dpi_ / 96), LONG(520 * dpi_ / 96)};
        return 0;
    }
    case WM_SIZE: {
        width = LOWORD(l) * 96.f / dpi_;
        height = HIWORD(l) * 96.f / dpi_;
        if (target_) {
            target_->Resize(D2D1::SizeU(LOWORD(l), HIWORD(l)));
        }
        rebuild();
        return 0;
    }
    case WM_DPICHANGED: {
        dpi_ = float(HIWORD(w));
        auto rect = reinterpret_cast<RECT *>(l);
        SetWindowPos(window_, nullptr, rect->left, rect->top, rect->right - rect->left,
                     rect->bottom - rect->top, SWP_NOZORDER | SWP_NOACTIVATE);
        releaseTarget();
        if (font_) {
            DeleteObject(font_);
            font_ = nullptr;
        }
        rebuild();
        return 0;
    }
    case rebuildMessage:
        rebuild();
        return 0;
    case WM_PAINT:
        paint();
        return 0;
    case WM_ERASEBKGND:
        return 1;
    case WM_MOUSEMOVE: {
        bool wasInChart = chartArea.contains(mouseX, mouseY);
        mouseX = GET_X_LPARAM(l) * 96.f / dpi_;
        mouseY = GET_Y_LPARAM(l) * 96.f / dpi_;
        if (wasInChart || chartArea.contains(mouseX, mouseY)) {
            refresh();
        }
        TRACKMOUSEEVENT tracking{sizeof(tracking), TME_LEAVE, window_, 0};
        TrackMouseEvent(&tracking);
        return 0;
    }
    case WM_MOUSELEAVE:
        mouseX = mouseY = -1;
        refresh();
        return 0;
    case WM_MOUSEWHEEL: {
        if (pdfModal != PdfModal::None || paletteOpen)
            return 0;
        POINT p{GET_X_LPARAM(l), GET_Y_LPARAM(l)};
        ScreenToClient(window_, &p);
        float side = width >= 880 ? (collapsed ? 76.f : 248.f) : 0;
        float delta = -GET_WHEEL_DELTA_WPARAM(w) / 120.f * 64;
        for (auto it = scene.scrollRegions.rbegin(); it != scene.scrollRegions.rend(); ++it) {
            float px = p.x * 96.f / dpi_, py = p.y * 96.f / dpi_;
            if (it->rect.contains(px, py) && it->clip.contains(px, py) && it->scroll(delta)) {
                rebuild(true);
                RedrawWindow(window_, nullptr, nullptr, RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_UPDATENOW);
                return 0;
            }
        }
        if (p.x * 96.f / dpi_ < side) {
            sideScroll = std::clamp(sideScroll + delta, 0.f, std::max(0.f, sideHeight_ - (height - 150)));
        } else {
            scroll = std::clamp(scroll + delta, 0.f, std::max(0.f, contentHeight_ - (height - 96)));
        }
        rebuild(true);
        RedrawWindow(window_, nullptr, nullptr, RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_UPDATENOW);
        return 0;
    }
    case WM_MEASUREITEM: {
        auto item = reinterpret_cast<MEASUREITEMSTRUCT *>(l);
        if (item->CtlType == ODT_COMBOBOX) {
            item->itemHeight = UINT(std::ceil(32 * dpi_ / 96));
            return TRUE;
        }
        break;
    }
    case WM_DRAWITEM: {
        auto item = reinterpret_cast<DRAWITEMSTRUCT *>(l);
        if (item->CtlType != ODT_COMBOBOX)
            break;
        const auto key = nativeKeys_.find(item->hwndItem);
        if (key == nativeKeys_.end() || (key->second != "gauge_unified.gauge.size.suggestion" &&
                                         key->second != "gauge_unified.gauge.tpi.suggestion"))
            break;
        const auto native = native_.find(key->second);
        if (native == native_.end())
            break;
        const bool selectedItem = (item->itemState & ODS_SELECTED) != 0;
        const bool closed = (item->itemState & ODS_COMBOBOXEDIT) != 0;
        FillRect(item->hDC, &item->rcItem,
                 GetSysColorBrush(selectedItem ? COLOR_HIGHLIGHT
                                  : closed     ? COLOR_BTNFACE
                                               : COLOR_WINDOW));
        const int previousMode = SetBkMode(item->hDC, TRANSPARENT);
        const COLORREF previousColor =
            SetTextColor(item->hDC, GetSysColor((item->itemState & ODS_DISABLED) ? COLOR_GRAYTEXT
                                                : selectedItem                   ? COLOR_HIGHLIGHTTEXT
                                                                                 : COLOR_WINDOWTEXT));
        HGDIOBJ previous = SelectObject(
            item->hDC, font_ ? font_ : reinterpret_cast<HFONT>(GetStockObject(DEFAULT_GUI_FONT)));
        if (item->itemID < native->second.options.size() && item->itemID < native->second.optionRows.size()) {
            const auto &text = native->second.options[item->itemID];
            const auto &columns = native->second.optionRows[item->itemID];
            RECT rect = item->rcItem;
            rect.left += int(std::ceil(6 * dpi_ / 96));
            if (closed || columns[1].empty()) {
                DrawTextW(item->hDC, text.c_str(), -1, &rect,
                          DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_END_ELLIPSIS);
            } else {
                const int gap = int(std::ceil(16 * dpi_ / 96));
                for (std::size_t i = 0; i < columns.size(); ++i) {
                    RECT cell = rect;
                    if (i < 2)
                        cell.right = std::min(cell.right, cell.left + native->second.optionColumns[i]);
                    DrawTextW(item->hDC, columns[i].c_str(), -1, &cell,
                              DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_END_ELLIPSIS);
                    if (i < 2)
                        rect.left += native->second.optionColumns[i] + gap;
                }
            }
        }
        SelectObject(item->hDC, previous);
        SetBkMode(item->hDC, previousMode);
        SetTextColor(item->hDC, previousColor);
        if (item->itemState & ODS_FOCUS)
            DrawFocusRect(item->hDC, &item->rcItem);
        return TRUE;
    }
    case WM_LBUTTONDOWN: {
        const float px = GET_X_LPARAM(l) * 96.f / dpi_, py = GET_Y_LPARAM(l) * 96.f / dpi_;
        for (auto it = scene.controls.rbegin(); it != scene.controls.rend(); ++it) {
            if (!it->options.empty() || !it->rect.contains(px, py) || !it->clip.contains(px, py))
                continue;
            auto native = native_.find(it->key);
            if (native == native_.end() || !IsWindowVisible(native->second.window))
                continue;
            HWND edit = native->second.window;
            POINT point{GET_X_LPARAM(l), GET_Y_LPARAM(l)};
            MapWindowPoints(window_, edit, &point, 1);
            RECT client{};
            GetClientRect(edit, &client);
            point.x = std::clamp(point.x, 0L, std::max(0L, client.right - 1));
            point.y = std::clamp(point.y, 0L, std::max(0L, client.bottom - 1));
            SetFocus(edit);
            SendMessageW(edit, WM_LBUTTONDOWN, w, MAKELPARAM(point.x, point.y));
            return 0;
        }
        break;
    }
    case WM_LBUTTONUP: {
        float px = GET_X_LPARAM(l) * 96.f / dpi_, py = GET_Y_LPARAM(l) * 96.f / dpi_;
        for (auto it = scene.hits.rbegin(); it != scene.hits.rend(); ++it) {
            if (it->rect.contains(px, py) && it->clip.contains(px, py)) {
                auto action = it->action;
                action();
                break;
            }
        }
        return 0;
    }
    case WM_SETCURSOR:
        if (reinterpret_cast<HWND>(w) != window_) {
            return FALSE;
        }
        if (LOWORD(l) == HTCLIENT) {
            POINT p;
            GetCursorPos(&p);
            ScreenToClient(window_, &p);
            float x = p.x * 96.f / dpi_, y = p.y * 96.f / dpi_;
            for (const auto &control : scene.controls) {
                if (control.options.empty() && control.rect.contains(x, y) && control.clip.contains(x, y)) {
                    SetCursor(LoadCursorW(nullptr, IDC_IBEAM));
                    return TRUE;
                }
            }
            bool clickable = false;
            for (auto &hit : scene.hits) {
                if (hit.rect.contains(x, y) && hit.clip.contains(x, y)) {
                    clickable = true;
                    break;
                }
            }
            SetCursor(LoadCursorW(nullptr, clickable ? IDC_HAND : IDC_ARROW));
            return TRUE;
        }
        break;
    case WM_COMMAND:
        if (!synchronizing_) {
            HWND child = reinterpret_cast<HWND>(l);
            auto it = nativeKeys_.find(child);
            if (it != nativeKeys_.end()) {
                if (HIWORD(w) == EN_CHANGE) {
                    int length = GetWindowTextLengthW(child);
                    std::wstring text(std::size_t(length) + 1, L'\0');
                    GetWindowTextW(child, text.data(), length + 1);
                    text.resize(length);
                    state.values[it->second] = text;
                    if (it->second == "__palette.query")
                        paletteHighlight = 0;
                    refresh();
                } else {
                    const bool suggestion = it->second == "gauge_unified.gauge.size.suggestion" ||
                                            it->second == "gauge_unified.gauge.tpi.suggestion";
                    const auto notification = HIWORD(w);
                    if (notification == CBN_SELCHANGE || (suggestion && notification == CBN_SELENDOK)) {
                        if (suggestion && notification == CBN_SELCHANGE &&
                            SendMessageW(child, CB_GETDROPPEDSTATE, 0, 0))
                            return 0;
                        const int selection = int(SendMessageW(child, CB_GETCURSEL, 0, 0));
                        if (it->second == "__calibration.selection") {
                            selectCalibration(selection);
                            return 0;
                        }
                        if (!suggestion || (selection >= 0 && state.choices[it->second] != selection)) {
                            state.choices[it->second] = selection;
                            refresh();
                        }
                    }
                }
                return 0;
            }
        }
        break;
    case WM_CTLCOLOREDIT:
    case WM_CTLCOLORLISTBOX: {
        auto dc = reinterpret_cast<HDC>(w);
        SetTextColor(dc, darkMode ? RGB(236, 242, 249) : RGB(24, 34, 48));
        SetBkColor(dc, darkMode ? RGB(19, 28, 43) : RGB(252, 253, 255));
        return reinterpret_cast<LRESULT>(editBrush_);
    }
    case WM_KEYDOWN:
        if (paletteOpen && w == VK_ESCAPE) {
            closePalette();
            return 0;
        }
        if (pdfModal != PdfModal::None) {
            if (w == VK_ESCAPE) {
                pdfModal = PdfModal::None;
                refresh();
            } else if (w == VK_TAB) {
                pdfDialogFocus = 1 - pdfDialogFocus;
                refresh();
            } else if (w == VK_RETURN || w == VK_SPACE) {
                if (pdfModal == PdfModal::Warning && pdfDialogFocus == 0)
                    startPdfDownload();
                else {
                    pdfModal = PdfModal::None;
                    refresh();
                }
            }
            return 0;
        }
        break;
    case WM_TIMER:
        inputHistory_.flush();
        pollPdfDownload();
        if (!toastMessage.empty() && GetTickCount64() >= toastExpire) {
            toastMessage.clear();
            refresh();
        }
        if (exportJob.active) {
            refresh();
        }
        return 0;
    case WM_QUERYENDSESSION:
        if (selected >= 0)
            inputHistory_.observe(tools().at(std::size_t(selected)), state);
        inputHistory_.flush(true);
        return TRUE;
    case WM_CLOSE:
        if (selected >= 0)
            inputHistory_.observe(tools().at(std::size_t(selected)), state);
        inputHistory_.flush(true);
        exportJob.cancel = true;
        DestroyWindow(window_);
        return 0;
    case WM_DESTROY:
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcW(window_, msg, w, l);
}

void App::synchronizeControls(bool scrolling) {
    synchronizing_ = true;
    if (!font_) {
        font_ = CreateFontW(-int(14 * dpi_ / 96), 0, 0, 0, FW_NORMAL, FALSE, FALSE, FALSE, DEFAULT_CHARSET,
                            OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_PITCH,
                            L"Microsoft YaHei");
    }
    int editHeight = int(std::ceil(20 * dpi_ / 96));
    if (HDC dc = GetDC(window_)) {
        HGDIOBJ previousFont = SelectObject(dc, font_);
        TEXTMETRICW metrics{};
        if (GetTextMetricsW(dc, &metrics))
            editHeight = metrics.tmHeight + int(std::ceil(2 * dpi_ / 96));
        SelectObject(dc, previousFont);
        ReleaseDC(window_, dc);
    }
    for (auto &entry : native_) {
        entry.second.seen = false;
    }
    for (auto &c : scene.controls) {
        float top = std::max(c.rect.y, c.clip.y), bottom = std::min(c.rect.y + c.rect.h, c.clip.y + c.clip.h);
        if (bottom <= top || c.rect.x + c.rect.w <= c.clip.x || c.rect.x >= c.clip.x + c.clip.w) {
            continue;
        }
        auto &n = native_[c.key];
        bool combo = !c.options.empty();
        const bool alignedSpecs = c.key == "gauge_unified.gauge.size.suggestion";
        const bool ownerDraw = alignedSpecs || c.key == "gauge_unified.gauge.tpi.suggestion";
        if (n.window && n.combo != combo) {
            nativeKeys_.erase(n.window);
            DestroyWindow(n.window);
            n.window = nullptr;
        }
        bool created = !n.window;
        if (created) {
            n.combo = combo;
            n.key = c.key;
            n.window = CreateWindowExW(0, combo ? L"COMBOBOX" : L"EDIT", L"",
                                       WS_CHILD | WS_TABSTOP |
                                           (combo ? CBS_DROPDOWNLIST | WS_VSCROLL |
                                                        (ownerDraw ? CBS_OWNERDRAWFIXED | CBS_HASSTRINGS : 0)
                                                  : ES_AUTOHSCROLL),
                                       0, 0, 0, 0, window_, nullptr, GetModuleHandleW(nullptr), nullptr);
            require(n.window != nullptr, "无法创建输入控件");
            nativeKeys_[n.window] = c.key;
            HWND face =
                ownerDraw ? CreateWindowExW(0, L"COMBOBOX", L"", WS_CHILD | CBS_DROPDOWNLIST | WS_VSCROLL, 0,
                                            0, 0, 0, window_, nullptr, GetModuleHandleW(nullptr), nullptr)
                          : nullptr;
            SetWindowSubclass(n.window, combo ? comboProcedure : editProcedure, 1,
                              reinterpret_cast<DWORD_PTR>(face));
        }
        n.seen = true;
        const bool changed = combo && (created || n.options != c.options);
        if (ownerDraw && !changed && n.layoutValid && n.optionDpi == dpi_ && sameRect(n.layoutRect, c.rect) &&
            sameRect(n.layoutClip, c.clip) && SendMessageW(n.window, CB_GETDROPPEDSTATE, 0, 0)) {
            continue;
        }
        SendMessageW(n.window, WM_SETFONT, reinterpret_cast<WPARAM>(font_), TRUE);
        if (combo) {
            if (changed) {
                n.options = c.options;
                n.optionRows.clear();
                if (ownerDraw) {
                    n.optionRows.reserve(n.options.size());
                    for (const auto &option : n.options)
                        n.optionRows.push_back(specColumns(option));
                }
                SendMessageW(n.window, CB_RESETCONTENT, 0, 0);
                for (auto &o : c.options) {
                    SendMessageW(n.window, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(o.c_str()));
                }
            }
            if (changed || n.optionDpi != dpi_) {
                n.optionTextWidth = 0;
                n.optionColumns = {};
                HDC dc = GetDC(n.window);
                if (dc) {
                    HGDIOBJ previousFont = SelectObject(dc, font_);
                    for (std::size_t index = 0; index < n.options.size(); ++index) {
                        const auto &option = n.options[index];
                        SIZE extent{};
                        if (GetTextExtentPoint32W(dc, option.c_str(), int(option.size()), &extent)) {
                            n.optionTextWidth = std::max(n.optionTextWidth, int(extent.cx));
                        }
                        if (alignedSpecs) {
                            const auto &columns = n.optionRows[index];
                            for (std::size_t i = 0; i < columns.size(); ++i) {
                                if (GetTextExtentPoint32W(dc, columns[i].c_str(), int(columns[i].size()),
                                                          &extent))
                                    n.optionColumns[i] = std::max(n.optionColumns[i], int(extent.cx));
                            }
                        }
                    }
                    SelectObject(dc, previousFont);
                    ReleaseDC(n.window, dc);
                } else {
                    for (std::size_t index = 0; index < n.options.size(); ++index) {
                        const auto &option = n.options[index];
                        n.optionTextWidth =
                            std::max(n.optionTextWidth, int(std::ceil(textWidth(option, 14) * dpi_ / 96)));
                        if (alignedSpecs) {
                            const auto &columns = n.optionRows[index];
                            for (std::size_t i = 0; i < columns.size(); ++i)
                                n.optionColumns[i] =
                                    std::max(n.optionColumns[i],
                                             int(std::ceil(textWidth(columns[i], 14) * dpi_ / 96)));
                        }
                    }
                }
                if (alignedSpecs)
                    n.optionTextWidth =
                        std::max(n.optionTextWidth, n.optionColumns[0] + n.optionColumns[1] +
                                                        n.optionColumns[2] + int(std::ceil(32 * dpi_ / 96)));
                n.optionDpi = dpi_;
            }
            const float comboScale = dpi_ / 96;
            MONITORINFO monitor{};
            monitor.cbSize = sizeof(monitor);
            int availableWidth = GetSystemMetrics(SM_CXSCREEN);
            if (GetMonitorInfoW(MonitorFromWindow(window_, MONITOR_DEFAULTTONEAREST), &monitor)) {
                availableWidth = monitor.rcWork.right - monitor.rcWork.left;
            }
            const int padding = int(std::ceil(44 * comboScale));
            const int popupWidth = std::max(int(c.rect.w * comboScale), n.optionTextWidth + padding);
            const int screenLimit = std::max(1, availableWidth - int(std::ceil(16 * comboScale)));
            SendMessageW(n.window, CB_SETDROPPEDWIDTH, WPARAM(std::min(popupWidth, screenLimit)), 0);
            SendMessageW(n.window, CB_SETITEMHEIGHT, 0, LPARAM(std::ceil(32 * comboScale)));
            SendMessageW(n.window, CB_SETCURSEL, c.selection, 0);
            LPARAM selectionHeight = LPARAM(28 * dpi_ / 96);
            DWORD_PTR faceData = 0;
            if (ownerDraw && GetWindowSubclass(n.window, comboProcedure, 1, &faceData) && faceData) {
                HWND face = reinterpret_cast<HWND>(faceData);
                SendMessageW(face, WM_SETFONT, reinterpret_cast<WPARAM>(font_), FALSE);
                SetWindowPos(face, nullptr, 0, 0, int(c.rect.w * comboScale), int(380 * comboScale),
                             SWP_NOZORDER | SWP_NOACTIVATE);
                const LRESULT height = SendMessageW(face, CB_GETITEMHEIGHT, WPARAM(-1), 0);
                if (height != CB_ERR)
                    selectionHeight = height;
            }
            SendMessageW(n.window, CB_SETITEMHEIGHT, WPARAM(-1), selectionHeight);
        } else if (GetFocus() != n.window) {
            int length = GetWindowTextLengthW(n.window);
            std::wstring current(std::size_t(length) + 1, L'\0');
            GetWindowTextW(n.window, current.data(), length + 1);
            current.resize(length);
            if (current != c.value) {
                SetWindowTextW(n.window, c.value.c_str());
            }
            SendMessageW(n.window, EM_SETMARGINS, EC_LEFTMARGIN | EC_RIGHTMARGIN, MAKELPARAM(8, 8));
        }
        float scale = dpi_ / 96;
        const int boxHeight = int(std::round(c.rect.h * scale));
        const int ph = combo ? int(380 * scale) : std::min(std::max(1, boxHeight - 4), editHeight);
        int px = int(c.rect.x * scale) + 1,
            py = int(std::round(c.rect.y * scale)) + (combo ? int(2 * scale) : (boxHeight - ph) / 2),
            pw = int(c.rect.w * scale) - 2;
        SetWindowPos(n.window, HWND_TOP, px, py, pw, ph,
                     SWP_NOACTIVATE | SWP_SHOWWINDOW | (scrolling ? SWP_NOCOPYBITS : 0));
        int clipTop = std::max(0, int(top * scale) - py), clipBottom = std::min(ph, int(bottom * scale) - py);
        if (combo && top <= c.rect.y && bottom >= c.rect.y + c.rect.h) {
            SetWindowRgn(n.window, nullptr, TRUE);
        } else {
            SetWindowRgn(n.window, CreateRectRgn(0, clipTop, pw, std::max(clipTop, clipBottom)), TRUE);
        }
        n.layoutRect = c.rect;
        n.layoutClip = c.clip;
        n.layoutValid = true;
    }
    for (auto &entry : native_) {
        if (!entry.second.seen) {
            ShowWindow(entry.second.window, SW_HIDE);
        }
    }
    if (paletteOpen) {
        auto it = native_.find("__palette.query");
        if (it != native_.end() && IsWindowVisible(it->second.window) && GetFocus() != it->second.window) {
            SetFocus(it->second.window);
        }
    }
    synchronizing_ = false;
}

void App::renderPalette() {
    if (!paletteOpen)
        return;
    scene.clip = {0, 0, width, height};
    scene.controls.clear();
    scene.hits.clear();
    scene.scrollRegions.clear();

    // 遮罩点击关闭
    scene.fill({0, 0, width, height}, Color(0, 0, 0, darkMode ? .58f : .36f));
    scene.hits.push_back({{0, 0, width, height}, scene.clip, [this] { closePalette(); }});

    const float dlgW = std::min(560.f, width - 36.f);
    const float dlgX = (width - dlgW) / 2.f;
    const float dlgY = std::max(60.f, height * 0.12f);

    std::wstring q = state.value("__palette.query", L"");
    std::vector<int> matches;
    for (std::size_t i = 0; i < tools().size(); ++i) {
        const auto &t = tools()[i];
        if (containsFold(t.name, q) || containsFold(groupName(t.group), q) || containsFold(t.description, q) ||
            containsFold(wide(t.id), q)) {
            matches.push_back(int(i));
            if (matches.size() >= 10)
                break;
        }
    }
    if (!matches.empty())
        paletteHighlight = std::clamp(paletteHighlight, 0, int(matches.size()) - 1);

    const float listH = matches.empty() ? 56.f : float(matches.size()) * 46.f + 12.f;
    const float dlgH = std::min(height - dlgY - 32.f, 64.f + listH);
    Rect dlg{dlgX, dlgY, dlgW, dlgH};
    scene.fill({dlgX + 2, dlgY + 8, dlgW, dlgH}, Color(0, 0, 0, .14f), 12);
    scene.fill(dlg, White, 12);
    scene.border(dlg, Border, 12);
    // 阻断遮罩点击穿透
    scene.hits.push_back({dlg, scene.clip, [] {}});

    scene.text({dlgX + 16, dlgY + 14, 24, 34}, L"⌕", 16, Muted, false, true);
    scene.input("__palette.query", {dlgX + 44, dlgY + 14, dlgW - 110, 34}, q);
    Rect escRect{dlgX + dlgW - 58, dlgY + 18, 42, 26};
    scene.fill(escRect, SurfaceSubtle, 6);
    scene.border(escRect, Border, 6);
    scene.text(escRect, L"ESC", 10.5f, Muted, true, true, false, true);
    scene.hits.push_back({escRect, scene.clip, [this] { closePalette(); }});
    scene.line(dlgX, dlgY + 58, dlgX + dlgW, dlgY + 58, Border);

    float iy = dlgY + 64.f;
    if (matches.empty()) {
        scene.text({dlgX + 16, iy + 8, dlgW - 32, 34}, L"未找到匹配的工具", 13, Muted, false, true);
    } else {
        for (std::size_t idx = 0; idx < matches.size(); ++idx) {
            if (iy + 44.f > dlgY + dlgH - 4.f)
                break;
            const int toolIdx = matches[idx];
            const auto &t = tools()[std::size_t(toolIdx)];
            const auto accent = groupAccent(t.group);
            Rect row{dlgX + 10, iy, dlgW - 20, 42};
            if (int(idx) == paletteHighlight) {
                scene.fill(row, Soft, 8);
            }
            scene.fill({row.x + 8, row.y + 6, 30, 30}, accent.bg, 7);
            scene.text({row.x + 8, row.y + 6, 30, 30}, t.icon.empty() ? L"工" : t.icon, 13, accent.fg, true, true);
            scene.text({row.x + 48, row.y + 3, row.w - 134, 20}, t.name, 13, Ink, true);
            scene.text({row.x + 48, row.y + 22, row.w - 134, 17}, t.description, 11, Muted);
            Rect tag{row.x + row.w - 76, row.y + 10, 68, 22};
            scene.fill(tag, accent.bg, 11);
            scene.text(tag, shortGroupName(t.group), 10.5f, accent.fg, true, true);
            scene.hits.push_back({row, scene.clip, [this, toolIdx] {
                                      closePalette();
                                      open(toolIdx);
                                  }});
            iy += 46.f;
        }
    }
}

void App::renderToast() {
    if (toastMessage.empty() || GetTickCount64() >= toastExpire)
        return;
    scene.clip = {0, 0, width, height};
    const float tw = std::min(width - 40.f, std::max(180.f, textWidth(toastMessage, 12.5f) + 42.f));
    const float th = 38.f;
    const float tx = width - tw - 20.f;
    const float ty = height - th - 20.f;
    scene.fill({tx, ty, tw, th}, Ink, 9);
    scene.text({tx + 14, ty, 18, th}, L"✓", 13, Green, true, true);
    scene.text({tx + 34, ty, tw - 44, th}, toastMessage, 12.5f, White);
}

void App::rebuild(bool scrolling) {
    if (!window_ || width < 10 || height < 10) {
        return;
    }
    calibrationValid = false;
    scene = Scene{};
    chartArea = {};
    scene.clip = {0, 0, width, height};
    scene.fill({0, 0, width, height}, Background);

    // 现代紧凑 52px 顶栏
    const float headerH = 52.f;
    scene.fill({0, 0, width, headerH}, White);
    scene.line(0, headerH, width, headerH, Border);

    // 左侧品牌（点击回首页）
    Rect brandRect{14, 7, 196, 38};
    scene.icon({16, 10, 32, 32});
    scene.text({56, 8, 120, 22}, L"工具箱", 16, Ink, true);
    scene.text({56, 28, 160, 16}, L"计量工程工作台 · " + std::to_wstring(tools().size()) + L" 项", 10.5f, Muted);
    scene.hits.push_back({brandRect, scene.clip, [this] { open(-1); }});

    // 右侧深浅主题切换 + 状态徽章
    Rect themeBtn{width - 96.f, 10.f, 82.f, 32.f};
    scene.fill(themeBtn, SurfaceSubtle, 8);
    scene.border(themeBtn, Border, 8);
    scene.text(themeBtn, darkMode ? L"☀ 浅色" : L"☾ 深色", 12, Ink, false, true);
    scene.hits.push_back({themeBtn, scene.clip, [this] { toggleTheme(); }});

    float rightEdge = themeBtn.x - 10.f;
    if (width >= 900) {
        Rect badge{rightEdge - 86.f, 13.f, 86.f, 26.f};
        scene.fill(badge, SuccessSoft, 13);
        scene.text(badge, L"● 本地计算", 11, Green, true, true);
        rightEdge = badge.x - 12.f;
    }

    // 中间快速搜索胶囊（Ctrl+K）
    if (rightEdge - 224.f >= 160.f) {
        const float sw = std::min(340.f, rightEdge - 236.f);
        const float sx = std::max(226.f, (width - sw) / 2.f);
        Rect searchPill{sx, 10.f, sw, 32.f};
        scene.fill(searchPill, SurfaceSubtle, 9);
        scene.border(searchPill, Border, 9);
        scene.text({sx + 12, 10.f, sw - 74, 32.f}, L"⌕  快速搜索任意工具…", 12, Muted);
        scene.fill({sx + sw - 54, 15.f, 44, 22.f}, White, 5);
        scene.border({sx + sw - 54, 15.f, 44, 22.f}, Border, 5);
        scene.text({sx + sw - 54, 15.f, 44, 22.f}, L"Ctrl K", 10, Muted, true, true, false, true);
        scene.hits.push_back({searchPill, scene.clip, [this] { openPalette(); }});
    }

    navigation();

    float left = width >= 880 ? (collapsed ? 76.f : 248.f) : 0;
    float baseTop = width >= 880 ? headerH : headerH + 48.f;

    // 多工具标签栏
    float tabBarH = 0.f;
    if (!openTabs.empty()) {
        tabBarH = 38.f;
        scene.clip = {left + 1, baseTop, width - left - 2, tabBarH};
        scene.fill({left + 1, baseTop, width - left - 2, tabBarH}, SurfaceSubtle);
        scene.line(left + 1, baseTop + tabBarH - 1, width, baseTop + tabBarH - 1, Border);
        float tx = left + 16.f;
        // 首页标签
        {
            Rect homeTab{tx, baseTop + 5, 58, 28};
            const bool active = selected < 0;
            scene.fill(homeTab, active ? Soft : White, 7);
            scene.border(homeTab, active ? Blue : Border, 7);
            scene.text(homeTab, L"首页", 12, active ? Blue : Muted, active, true);
            scene.hits.push_back({homeTab, scene.clip, [this] { open(-1); }});
            tx += 64.f;
        }
        for (int tabIdx : openTabs) {
            if (tx > width - 60.f)
                break;
            const auto &t = tools().at(std::size_t(tabIdx));
            const auto accent = groupAccent(t.group);
            const bool active = selected == tabIdx;
            float tw = std::min(196.f, textWidth(t.name, 12) + 46.f);
            Rect pill{tx, baseTop + 5, tw, 28};
            scene.fill(pill, active ? accent.bg : White, 7);
            scene.border(pill, active ? accent.fg : Border, 7);
            scene.fill({tx + 9, baseTop + 16, 6, 6}, accent.fg, 3);
            scene.text({tx + 20, baseTop + 5, tw - 44, 28}, t.name, 12, active ? accent.fg : Ink, active);
            scene.hits.push_back({{tx, baseTop + 5, tw - 22, 28}, scene.clip, [this, tabIdx] { open(tabIdx); }});
            Rect closeRect{tx + tw - 22, baseTop + 7, 18, 24};
            scene.text(closeRect, L"×", 13, Muted, false, true);
            scene.hits.push_back({closeRect, scene.clip, [this, tabIdx] { closeTab(tabIdx); }});
            tx += tw + 6.f;
        }
    }

    float top = baseTop + tabBarH;
    scene.clip = {left + 1, top, width - left - 2, height - top};
    if (selected < 0) {
        homepage();
    } else {
        const auto &tool = tools().at(std::size_t(selected));
        const auto accent = groupAccent(tool.group);
        float w = std::min(width - left - 48.f, 1240.f);
        float x = left + std::max(24.f, (width - left - w) / 2.f);
        float y = top + 14.f - scroll;

        // 面包屑
        scene.button({x, y, 82, 28}, L"← 首页", [this] { open(-1); });
        scene.text({x + 92, y, w - 92, 28}, L"/  " + groupName(tool.group) + L"  /  " + tool.name, 12, Muted);
        y += 38.f;

        // 工具头部（带分类色图标 + 收藏星标 + 恢复示例按钮）
        const float iconSize = 44.f;
        scene.fill({x, y, iconSize, iconSize}, accent.bg, 10);
        scene.text({x, y, iconSize, iconSize}, tool.icon.empty() ? L"工" : tool.icon, 18, accent.fg, true, true);
        float offset = 56.f;
        const float actionWidth = 100.f, starWidth = 34.f, actionGap = 8.f;
        const float totalRight = (!exportJob.active ? actionWidth + actionGap : 0.f) + starWidth + 12.f;
        const float textSpace = std::max(140.f, w - offset - totalRight);
        scene.text({x + offset, y - 2, textSpace, 28}, tool.name, 22, Ink, true);
        scene.text({x + offset, y + 26, textSpace, 22}, tool.description, 12, Muted);

        // 收藏星标按钮
        const bool fav = isFavorite(tool.id);
        float rightCursor = x + w;
        if (!exportJob.active) {
            rightCursor -= actionWidth;
            scene.button({rightCursor, y + 4, actionWidth, 34},
                         tool.kind == Kind::PdfDownload ? L"清空" : L"恢复示例",
                         [this] { restoreExample(); });
            rightCursor -= actionGap;
        }
        rightCursor -= starWidth;
        Rect starBtn{rightCursor, y + 4, starWidth, 34};
        scene.fill(starBtn, SurfaceSubtle, 8);
        scene.border(starBtn, Border, 8);
        scene.text(starBtn, fav ? L"★" : L"☆", 15, fav ? StarGold : Muted, false, true);
        scene.hits.push_back({starBtn, scene.clip, [this, id = tool.id, name = tool.name] {
                                  toggleFavorite(id, name);
                              }});

        y += 58.f;

        Page page(*this, scene, tool, x, y, w);
        try {
            renderPage(page);
            page.check();
        } catch (const std::exception &e) {
            page.endSplit();
            page.processSteps.clear();
            page.error(page.inputError.empty() ? wide(e.what()) : page.inputError);
        }
        page.endSplit();
        page.y += 10;
        page.showProcess();
        page.explain();
        contentHeight_ = page.y + scroll - top + 52;
        inputHistory_.observe(tool, state);
    }
    if (contentHeight_ > height - top) {
        float track = height - top - 12, thumb = std::max(28.f, track * (height - top) / contentHeight_);
        scene.fill({width - 7,
                    top + 6 + (track - thumb) * scroll / std::max(1.f, contentHeight_ - (height - top)), 4,
                    thumb},
                   BorderStrong, 2);
    }
    renderPdfModal();
    renderPalette();
    renderToast();
    synchronizeControls(scrolling);
    InvalidateRect(window_, nullptr, FALSE);
}

void App::paint() {
    PAINTSTRUCT ps;
    BeginPaint(window_, &ps);
    if (!target_) {
        RECT client;
        GetClientRect(window_, &client);
        auto properties = D2D1::RenderTargetProperties();
        properties.dpiX = dpi_;
        properties.dpiY = dpi_;
        HRESULT hr = factory_->CreateHwndRenderTarget(
            properties, D2D1::HwndRenderTargetProperties(window_, D2D1::SizeU(client.right, client.bottom)),
            &target_);
        if (FAILED(hr)) {
            properties.type = D2D1_RENDER_TARGET_TYPE_SOFTWARE;
            factory_->CreateHwndRenderTarget(
                properties,
                D2D1::HwndRenderTargetProperties(window_, D2D1::SizeU(client.right, client.bottom)),
                &target_);
        }
    }
    if (target_) {
        loadIconBitmap();
        target_->BeginDraw();
        target_->SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE);
        target_->Clear(D2D1::ColorF(Background.r, Background.g, Background.b));
        ID2D1SolidColorBrush *brush = nullptr;
        target_->CreateSolidColorBrush(D2D1::ColorF(0, 0, 0), &brush);
        if (brush) {
            for (const auto &d : scene.draws) {
                if (d.clip.h <= 0 || d.clip.w <= 0) {
                    continue;
                }
                if (d.kind != 3 && (d.rect.y + d.rect.h < d.clip.y || d.rect.y > d.clip.y + d.clip.h ||
                                    d.rect.x + d.rect.w < d.clip.x || d.rect.x > d.clip.x + d.clip.w)) {
                    continue;
                }
                target_->PushAxisAlignedClip(rectangle(d.clip), D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
                brush->SetColor(D2D1::ColorF(d.color.r, d.color.g, d.color.b, d.color.a));
                auto r = rectangle(d.rect);
                if (d.kind == 0) {
                    target_->FillRoundedRectangle(D2D1::RoundedRect(r, d.radius, d.radius), brush);
                } else if (d.kind == 1) {
                    target_->DrawRoundedRectangle(D2D1::RoundedRect(r, d.radius, d.radius), brush, d.stroke);
                } else if (d.kind == 3) {
                    target_->DrawLine(D2D1::Point2F(d.rect.x, d.rect.y),
                                      D2D1::Point2F(d.rect.x + d.rect.w, d.rect.y + d.rect.h), brush,
                                      d.stroke);
                } else if (d.kind == 4) {
                    if (iconBitmap_) {
                        target_->DrawBitmap(iconBitmap_, r, 1.f, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR);
                    }
                } else if (d.kind == 5) {
                    const float deviceScale = dpi_ / 96.f;
                    const UINT pixelWidth = std::max(1u, UINT(std::lround(d.rect.w * deviceScale)));
                    const UINT pixelHeight = std::max(1u, UINT(std::lround(d.rect.h * deviceScale)));
                    auto bitmap = loadDynamicMathBitmap(d.formula);
                    if (bitmap) {
                        r.left = std::round(d.rect.x * deviceScale) / deviceScale;
                        r.top = std::round(d.rect.y * deviceScale) / deviceScale;
                        r.right = r.left + pixelWidth / deviceScale;
                        r.bottom = r.top + pixelHeight / deviceScale;
                        target_->DrawBitmap(bitmap, r, 1.f, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR);
                    } else {
                        IDWriteTextFormat *format = nullptr;
                        const float fallbackSize =
                            std::min(13.f, 13.f * d.rect.w / std::max(1.f, textWidth(d.text, 13)));
                        if (SUCCEEDED(textFactory_->CreateTextFormat(
                                L"Microsoft YaHei", nullptr, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL,
                                DWRITE_FONT_STRETCH_NORMAL, fallbackSize, L"zh-cn", &format))) {
                            format->SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
                            target_->DrawTextW(d.text.c_str(), UINT32(d.text.size()), format, r, brush,
                                               D2D1_DRAW_TEXT_OPTIONS_CLIP);
                            format->Release();
                        }
                    }
                } else {
                    IDWriteTextFormat *format = nullptr;
                    const wchar_t *family = d.mono ? L"Consolas" : L"Microsoft YaHei";
                    textFactory_->CreateTextFormat(
                        family, nullptr, d.bold ? DWRITE_FONT_WEIGHT_SEMI_BOLD : DWRITE_FONT_WEIGHT_NORMAL,
                        DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, d.size, L"zh-cn", &format);
                    if (format) {
                        format->SetParagraphAlignment(d.paragraph ? DWRITE_PARAGRAPH_ALIGNMENT_NEAR
                                                                  : DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
                        if (d.paragraph) {
                            format->SetLineSpacing(DWRITE_LINE_SPACING_METHOD_UNIFORM,
                                                   paragraphLineSpacing(d.size), d.size * 1.25f);
                        }
                        format->SetTextAlignment(d.center ? DWRITE_TEXT_ALIGNMENT_CENTER
                                                          : DWRITE_TEXT_ALIGNMENT_LEADING);
                        target_->DrawTextW(d.text.c_str(), UINT32(d.text.size()), format, r, brush,
                                           D2D1_DRAW_TEXT_OPTIONS_CLIP);
                        format->Release();
                    }
                }
                target_->PopAxisAlignedClip();
            }
        }
        release(brush);
        if (target_->EndDraw() == static_cast<HRESULT>(D2DERR_RECREATE_TARGET)) {
            releaseTarget();
        }
    }
    EndPaint(window_, &ps);
}
} // 命名空间 toolbox
