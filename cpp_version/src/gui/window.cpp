// Windows 7 基线窗口后端。较新 DPI API 一律动态查询，避免旧系统加载程序失败。
#include "toolbox/gui.hpp"
#include "toolbox/resource_ids.h"
#include <commctrl.h>
#include <cstring>
#include <memory>
#include <windowsx.h>
namespace toolbox {
static constexpr UINT rebuildMessage = WM_APP + 1;
static LRESULT CALLBACK editProcedure(HWND window, UINT msg, WPARAM w, LPARAM l, UINT_PTR, DWORD_PTR) {
    // 编辑框的客户区使用文字选择光标，与是否获得焦点无关。
    if (msg == WM_SETCURSOR && LOWORD(l) == HTCLIENT) {
        SetCursor(LoadCursorW(nullptr, IDC_IBEAM));
        return TRUE;
    }
    if (msg == WM_MOUSEWHEEL) {
        return SendMessageW(GetParent(window), msg, w, l);
    }
    if (msg == WM_NCDESTROY) {
        RemoveWindowSubclass(window, editProcedure, 1);
    }
    return DefSubclassProc(window, msg, w, l);
}
static LRESULT CALLBACK comboProcedure(HWND window, UINT msg, WPARAM w, LPARAM l, UINT_PTR, DWORD_PTR) {
    // 下拉框收起时，滚轮只滚动页面，避免误改分度号、单位等计算参数。
    // 展开列表后仍保留原生滚动、点击与键盘选择行为。
    if (msg == WM_MOUSEWHEEL && !SendMessageW(window, CB_GETDROPPEDSTATE, 0, 0)) {
        return SendMessageW(GetParent(window), msg, w, l);
    }
    if (msg == WM_NCDESTROY) {
        RemoveWindowSubclass(window, comboProcedure, 1);
    }
    return DefSubclassProc(window, msg, w, l);
}
template <class T> static void release(T *&object) {
    if (object) {
        object->Release();
        object = nullptr;
    }
}
static D2D1_RECT_F rectangle(Rect r) { return D2D1::RectF(r.x, r.y, r.x + r.w, r.y + r.h); }
App::~App() {
    // 窗口正常退出和初始化中途失败都由同一所有者回收资源。
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
    release(target_);
}
void App::refresh() {
    if (window_) {
        PostMessageW(window_, rebuildMessage, 0, 0);
    }
}
void App::open(int index) {
    if (index >= 0 && index != selected && !remembersInputs(tools().at(std::size_t(index)))) {
        clearToolInputs(tools().at(std::size_t(index)));
    }
    selected = index;
    scroll = 0;
    refresh();
}
void App::clearToolInputs(const Tool &tool) {
    const auto prefix = tool.id + ".";
    for (auto it = state.values.begin(); it != state.values.end();)
        if (it->first.compare(0, prefix.size(), prefix) == 0) it = state.values.erase(it);
        else ++it;
    for (auto it = state.choices.begin(); it != state.choices.end();)
        if (it->first.compare(0, prefix.size(), prefix) == 0) it = state.choices.erase(it);
        else ++it;
    clearGaugeCache(tool.id);
    SetFocus(window_);
}
void App::restoreExample() {
    if (selected < 0 || exportJob.active) return;
    const auto &tool = tools().at(std::size_t(selected));
    clearToolInputs(tool);
    inputHistory_.observe(tool, state);
    inputHistory_.flush(true);
    refresh();
}
int App::run(HINSTANCE instance, int show) {
    inputHistory_.load(state);
    // Win7 使用系统 DPI；Win10 及以后按每屏 DPI 缩放同一套逻辑坐标。
    auto user = GetModuleHandleW(L"user32.dll");
    using SetContext = BOOL(WINAPI *)(HANDLE);
    auto contextAddress = GetProcAddress(user, "SetProcessDpiAwarenessContext");
    SetContext setContext = nullptr;
    // Win32 的 FARPROC 仅保存地址；复制到真实签名，避免调用不兼容的函数指针类型。
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
    // 标题栏使用 ICO 中专门绘制的小尺寸图像，避免直接缩小大图导致细节模糊。
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
    editBrush_ = CreateSolidBrush(RGB(255, 255, 255));
    window_ = CreateWindowExW(0, wc.lpszClassName, L"工具箱 · 桌面版", WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
                              CW_USEDEFAULT, CW_USEDEFAULT, int(1220 * dpi_ / 96), int(820 * dpi_ / 96),
                              nullptr, nullptr, instance, this);
    if (!window_) {
        return 1;
    }
    // 许可文字与运行库一起嵌入 EXE，可从窗口系统菜单阅读，不需要随附文本文件。
    if (auto menu = GetSystemMenu(window_, FALSE)) {
        AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
        AppendMenuW(menu, MF_STRING, TOOLBOX_LICENSE_MENU, L"开源许可…");
    }
    ShowWindow(window_, show);
    UpdateWindow(window_);
    SetTimer(window_, 1, 200, nullptr);
    MSG msg{};
    while (GetMessageW(&msg, nullptr, 0, 0) > 0) {
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
        // 异常不能穿过系统窗口回调边界，计算错误通常已由工具页面就地显示。
        auto text = wide(error.what());
        MessageBoxW(window, text.c_str(), L"工具箱", MB_ICONERROR);
        return 0;
    }
}
LRESULT App::message(UINT msg, WPARAM w, LPARAM l) {
    switch (msg) {
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
        POINT p{GET_X_LPARAM(l), GET_Y_LPARAM(l)};
        ScreenToClient(window_, &p);
        float side = width >= 880 ? (collapsed ? 92.f : 245.f) : 0;
        float delta = -GET_WHEEL_DELTA_WPARAM(w) / 120.f * 64;
        for (auto it = scene.scrollRegions.rbegin(); it != scene.scrollRegions.rend(); ++it) {
            float px = p.x * 96.f / dpi_, py = p.y * 96.f / dpi_;
            if (it->rect.contains(px, py) && it->clip.contains(px, py) && it->scroll(delta)) {
                return 0;
            }
        }
        if (p.x * 96.f / dpi_ < side) {
            sideScroll = std::clamp(sideScroll + delta, 0.f, std::max(0.f, sideHeight_ - (height - 150)));
        } else {
            scroll = std::clamp(scroll + delta, 0.f, std::max(0.f, contentHeight_ - (height - 96)));
        }
        rebuild();
        return 0;
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
        // 子控件也会把此消息交给父窗口。返回 FALSE，让输入框、下拉框自行设置光标；
        // 否则父窗口的箭头/手形逻辑会覆盖编辑框原生的 I 形光标。
        if (reinterpret_cast<HWND>(w) != window_) {
            return FALSE;
        }
        if (LOWORD(l) == HTCLIENT) {
            POINT p;
            GetCursorPos(&p);
            ScreenToClient(window_, &p);
            float x = p.x * 96.f / dpi_, y = p.y * 96.f / dpi_;
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
                    refresh();
                } else if (HIWORD(w) == CBN_SELCHANGE) {
                    state.choices[it->second] = int(SendMessageW(child, CB_GETCURSEL, 0, 0));
                    refresh();
                }
                return 0;
            }
        }
        break;
    case WM_CTLCOLOREDIT:
    case WM_CTLCOLORLISTBOX: {
        auto dc = reinterpret_cast<HDC>(w);
        SetTextColor(dc, RGB(35, 48, 67));
        SetBkColor(dc, RGB(255, 255, 255));
        return reinterpret_cast<LRESULT>(editBrush_);
    }
    case WM_TIMER:
        inputHistory_.flush();
        if (exportJob.active) {
            refresh();
        }
        return 0;
    case WM_QUERYENDSESSION:
        if (selected >= 0) inputHistory_.observe(tools().at(std::size_t(selected)), state);
        inputHistory_.flush(true);
        return TRUE;
    case WM_CLOSE:
        if (selected >= 0) inputHistory_.observe(tools().at(std::size_t(selected)), state);
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
void App::synchronizeControls() {
    synchronizing_ = true;
    if (!font_) {
        font_ = CreateFontW(-int(14 * dpi_ / 96), 0, 0, 0, FW_NORMAL, FALSE, FALSE, FALSE, DEFAULT_CHARSET,
                            OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_PITCH,
                            L"Microsoft YaHei");
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
                                           (combo ? CBS_DROPDOWNLIST | WS_VSCROLL : ES_AUTOHSCROLL),
                                       0, 0, 0, 0, window_, nullptr, GetModuleHandleW(nullptr), nullptr);
            require(n.window != nullptr, "无法创建输入控件");
            nativeKeys_[n.window] = c.key;
            SetWindowSubclass(n.window, combo ? comboProcedure : editProcedure, 1, 0);
        }
        n.seen = true;
        SendMessageW(n.window, WM_SETFONT, reinterpret_cast<WPARAM>(font_), TRUE);
        if (combo) {
            int count = int(SendMessageW(n.window, CB_GETCOUNT, 0, 0));
            bool changed = count != int(c.options.size());
            if (!changed) {
                for (int i = 0; i < count; ++i) {
                    std::wstring text(std::size_t(SendMessageW(n.window, CB_GETLBTEXTLEN, i, 0)) + 1, L'\0');
                    SendMessageW(n.window, CB_GETLBTEXT, i, reinterpret_cast<LPARAM>(text.data()));
                    text.resize(text.size() - 1);
                    if (text != c.options[std::size_t(i)]) {
                        changed = true;
                        break;
                    }
                }
            }
            if (changed) {
                SendMessageW(n.window, CB_RESETCONTENT, 0, 0);
                for (auto &o : c.options) {
                    SendMessageW(n.window, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(o.c_str()));
                }
            }
            // 收起按钮可以很窄，展开列表须按当前字体测量最长选项，避免截断规格及单位备注。
            // 仅在选项或 DPI 改变时重新测宽，滚动页面时复用缓存。
            if (changed || n.optionDpi != dpi_) {
                n.optionTextWidth = 0;
                HDC dc = GetDC(n.window);
                if (dc) {
                    HGDIOBJ previousFont = SelectObject(dc, font_);
                    for (const auto &option : c.options) {
                        SIZE extent{};
                        if (GetTextExtentPoint32W(dc, option.c_str(), int(option.size()), &extent)) {
                            n.optionTextWidth = std::max(n.optionTextWidth, int(extent.cx));
                        }
                    }
                    SelectObject(dc, previousFont);
                    ReleaseDC(n.window, dc);
                } else {
                    for (const auto &option : c.options) {
                        n.optionTextWidth =
                            std::max(n.optionTextWidth, int(std::ceil(textWidth(option, 14) * dpi_ / 96)));
                    }
                }
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
            SendMessageW(n.window, CB_SETITEMHEIGHT, WPARAM(-1), LPARAM(28 * dpi_ / 96));
        } else if (GetFocus() != n.window) {
            int length = GetWindowTextLengthW(n.window);
            std::wstring current(std::size_t(length) + 1, L'\0');
            GetWindowTextW(n.window, current.data(), length + 1);
            current.resize(length);
            if (current != c.value) {
                SetWindowTextW(n.window, c.value.c_str());
            }
            SendMessageW(n.window, EM_SETMARGINS, EC_LEFTMARGIN | EC_RIGHTMARGIN, MAKELPARAM(8, 8));
            if (c.key == "home.search" || c.key == "navigation.search") {
                SendMessageW(n.window, EM_SETCUEBANNER, TRUE, reinterpret_cast<LPARAM>(L"搜索名称或功能"));
            }
        }
        float scale = dpi_ / 96;
        int px = int(c.rect.x * scale) + 1, py = int(c.rect.y * scale) + 2, pw = int(c.rect.w * scale) - 2,
            ph = int((combo ? 380 : c.rect.h - 4) * scale);
        SetWindowPos(n.window, HWND_TOP, px, py, pw, ph, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        // 裁去离开视口的部分；编辑框不截获滚轮，主窗口负责纵向滚动。
        int clipTop = std::max(0, int(top * scale) - py), clipBottom = std::min(ph, int(bottom * scale) - py);
        if (combo && top <= c.rect.y && bottom >= c.rect.y + c.rect.h) {
            SetWindowRgn(n.window, nullptr, TRUE);
        } else {
            SetWindowRgn(n.window, CreateRectRgn(0, clipTop, pw, std::max(clipTop, clipBottom)), TRUE);
        }
    }
    for (auto &entry : native_) {
        if (!entry.second.seen) {
            ShowWindow(entry.second.window, SW_HIDE);
        }
    }
    synchronizing_ = false;
}
void App::rebuild() {
    if (!window_ || width < 10 || height < 10) {
        return;
    }
    scene = Scene{};
    chartArea = {};
    scene.clip = {0, 0, width, height};
    scene.fill({0, 0, width, height}, Background);
    scene.fill({0, 0, width, 68}, White);
    scene.line(0, 68, width, 68, Border);
    scene.icon({23, 14, 42, 42});
    scene.text({74, 14, 240, 42}, L"工具箱", 19, Ink, true);
    scene.text({width - 182, 20, 165, 30}, std::to_wstring(tools().size()) + L" 项工具  ● 本地计算", 12,
               Green, false, true);
    navigation();
    float left = width >= 880 ? (collapsed ? 92.f : 245.f) : 0;
    float top = width >= 880 ? 82.f : 126.f;
    scene.clip = {left + 1, top, width - left - 2, height - top};
    if (selected < 0) {
        homepage();
    } else {
        const auto &tool = tools().at(std::size_t(selected));
        float x = left + 24,
              w = std::min(width - left - 48, tool.group == Group::Gauges ||
                                                      tool.kind == Kind::BSplineInterpolation ||
                                                      tool.kind == Kind::CalibrationCurveFit
                                                  ? 960.f
                                                  : 820.f),
              y = top + 10 - scroll;
        scene.button({x, y, 86, 28}, L"← 工具箱", [this] { open(-1); });
        scene.text({x + 96, y, w - 96, 28}, L"/ " + groupName(tool.group) + L" / " + tool.name, 12, Muted);
        y += 44;
        if (!tool.icon.empty()) {
            scene.fill({x, y, 48, 48}, Soft, 4);
            scene.text({x, y, 48, 48}, tool.icon, 18, Blue, false, true);
        }
        float offset = tool.icon.empty() ? 0 : 60;
        const float actionWidth = 104, actionGap = 16;
        const bool actionBelow = textWidth(tool.name, 26) + offset + actionWidth + actionGap > w;
        const float textSpace = w - offset - (!exportJob.active && !actionBelow ? actionWidth + actionGap : 0);
        scene.text({x + offset, y, textSpace, 32}, tool.name, 26, Ink, true);
        scene.text({x + offset, y + 34, textSpace, 44}, tool.description, 12, Muted);
        if (!exportJob.active && !actionBelow) {
            scene.button({x + w - actionWidth, y + 3, actionWidth, 40}, L"恢复示例", [this] { restoreExample(); });
        }
        y += 86;
        if (!exportJob.active && actionBelow) {
            scene.button({x + w - actionWidth, y, actionWidth, 40}, L"恢复示例", [this] { restoreExample(); });
            y += 52;
        }
        Page page(*this, scene, tool, x, y, w);
        try {
            renderPage(page);
            page.check();
        } catch (const std::exception &e) {
            page.note(page.inputError.empty() ? wide(e.what()) : page.inputError, Red);
        }
        page.y += 14;
        page.explain();
        contentHeight_ = page.y + scroll - top + 24;
        inputHistory_.observe(tool, state);
    }
    if (contentHeight_ > height - top) {
        float track = height - top - 12, thumb = std::max(28.f, track * (height - top) / contentHeight_);
        scene.fill({width - 7,
                    top + 6 + (track - thumb) * scroll / std::max(1.f, contentHeight_ - (height - top)), 4,
                    thumb},
                   Border, 2);
    }
    synchronizeControls();
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
        target_->Clear(D2D1::ColorF(1, 1, 1));
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
                } else {
                    IDWriteTextFormat *format = nullptr;
                    textFactory_->CreateTextFormat(
                        L"Microsoft YaHei", nullptr,
                        d.bold ? DWRITE_FONT_WEIGHT_SEMI_BOLD : DWRITE_FONT_WEIGHT_NORMAL,
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
} // namespace toolbox
