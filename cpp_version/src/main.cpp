// C++ 入口：清除工作目录依赖，所有静态系数随可执行文件编译。
#include "toolbox/gui.hpp"
#include <objbase.h>
int WINAPI wWinMain(HINSTANCE instance, HINSTANCE, PWSTR, int show) {
    HRESULT com = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    int result = 1;
    try {
        toolbox::App app;
        result = app.run(instance, show);
    } catch (const std::exception &error) {
        auto text = toolbox::wide(error.what());
        MessageBoxW(nullptr, text.c_str(), L"工具箱启动失败", MB_ICONERROR);
    }
    // S_OK/S_FALSE 都需配对释放；初始化失败时不能减少别人的 COM 引用计数。
    if (SUCCEEDED(com)) {
        CoUninitialize();
    }
    return result;
}
