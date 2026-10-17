// 单文件发布保留第三方许可证：从 EXE 资源读取，在原生只读窗口中展示。
#include "toolbox/gui.hpp"
#include "toolbox/resource_ids.h"

namespace toolbox {
namespace {
struct LicenseView {
    std::wstring text;
    float scale = 1;
    HWND edit = nullptr, close = nullptr;
    HFONT font = nullptr;
};

void layout(HWND dialog, const LicenseView &view) {
    RECT client{};
    GetClientRect(dialog, &client);
    int margin = int(12 * view.scale), buttonWidth = int(84 * view.scale);
    int buttonHeight = int(32 * view.scale), buttonY = client.bottom - margin - buttonHeight;
    MoveWindow(view.edit, margin, margin, std::max(1, int(client.right) - 2 * margin),
               std::max(1, buttonY - 2 * margin), TRUE);
    MoveWindow(view.close, client.right - margin - buttonWidth, buttonY, buttonWidth, buttonHeight, TRUE);
}

INT_PTR CALLBACK licenseProcedure(HWND dialog, UINT message, WPARAM w, LPARAM l) {
    auto view = reinterpret_cast<LicenseView *>(GetWindowLongPtrW(dialog, DWLP_USER));
    if (message == WM_INITDIALOG) {
        view = reinterpret_cast<LicenseView *>(l);
        SetWindowLongPtrW(dialog, DWLP_USER, reinterpret_cast<LONG_PTR>(view));
        view->font = CreateFontW(-int(14 * view->scale), 0, 0, 0, FW_NORMAL, FALSE, FALSE, FALSE,
                                 DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY,
                                 DEFAULT_PITCH, L"Microsoft YaHei");
        view->edit = CreateWindowExW(
            WS_EX_CLIENTEDGE, L"EDIT", L"",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | ES_MULTILINE | ES_AUTOVSCROLL | ES_READONLY, 0,
            0, 0, 0, dialog, reinterpret_cast<HMENU>(100), GetModuleHandleW(nullptr), nullptr);
        SendMessageW(view->edit, WM_SETFONT, reinterpret_cast<WPARAM>(view->font), TRUE);
        SendMessageW(view->edit, EM_SETLIMITTEXT, view->text.size() + 1, 0);
        SetWindowTextW(view->edit, view->text.c_str());
        view->close = CreateWindowExW(
            0, L"BUTTON", L"关闭", WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_DEFPUSHBUTTON, 0, 0, 0, 0, dialog,
            reinterpret_cast<HMENU>(IDOK), GetModuleHandleW(nullptr), nullptr);
        SendMessageW(view->close, WM_SETFONT, reinterpret_cast<WPARAM>(view->font), TRUE);
        layout(dialog, *view);
        return TRUE;
    }
    if (message == WM_SIZE && view) {
        layout(dialog, *view);
        return TRUE;
    }
    if (message == WM_GETMINMAXINFO && view) {
        auto size = reinterpret_cast<MINMAXINFO *>(l);
        size->ptMinTrackSize = {LONG(450 * view->scale), LONG(300 * view->scale)};
        return TRUE;
    }
    if (message == WM_CLOSE || (message == WM_COMMAND && (LOWORD(w) == IDOK || LOWORD(w) == IDCANCEL))) {
        EndDialog(dialog, IDOK);
        return TRUE;
    }
    return FALSE;
}
} // namespace

void showLicenses(HWND owner, float dpi) {
    auto instance = GetModuleHandleW(nullptr);
    auto resource = FindResourceW(instance, MAKEINTRESOURCEW(TOOLBOX_LICENSE_TEXT), RT_RCDATA);
    require(resource != nullptr, "无法读取内置许可证资源");
    auto bytes = LoadResource(instance, resource);
    auto data = static_cast<const char *>(LockResource(bytes));
    auto size = SizeofResource(instance, resource);
    require(data != nullptr && size > 0, "内置许可证资源为空");
    LicenseView view;
    view.scale = dpi / 96;
    // 原生多行编辑框采用 CRLF；保持各许可证内容完整，仅统一换行方式。
    auto text = wide(std::string(data, size));
    for (std::size_t i = 0; i < text.size(); ++i) {
        if (text[i] == L'\n' && (i == 0 || text[i - 1] != L'\r')) {
            view.text += L'\r';
        }
        view.text += text[i];
    }
    // 无控件模板：菜单、窗口类为默认值，正文与关闭按钮在 WM_INITDIALOG 创建。
    struct alignas(DWORD) Template {
        DLGTEMPLATE dialog{};
        WORD menu = 0, windowClass = 0;
        wchar_t title[8] = L"开源许可";
    } dialog;
    dialog.dialog.style = WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | DS_MODALFRAME | DS_CENTER;
    dialog.dialog.dwExtendedStyle = WS_EX_DLGMODALFRAME;
    dialog.dialog.cx = 520;
    dialog.dialog.cy = 320;
    auto result = DialogBoxIndirectParamW(instance, &dialog.dialog, owner, licenseProcedure,
                                          reinterpret_cast<LPARAM>(&view));
    if (view.font) {
        DeleteObject(view.font);
    }
    require(result != -1, "无法打开许可证窗口");
}
} // namespace toolbox
