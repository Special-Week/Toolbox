#pragma once
// Win32 仅负责窗口与编辑控件；所有展示文字由 DirectWrite 排版并使用 ClearType。
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include "models.hpp"
#include <atomic>
#include <d2d1.h>
#include <dwrite.h>
#include <map>
#include <mutex>
#include <thread>
#include <windows.h>

namespace toolbox {
// 测量和绘制必须共用行距，窗口缩放后段落高度才不会偏移或截断。
constexpr float paragraphLineSpacing(float size) {
    return size * 1.65f;
}
struct Rect {
    float x = 0, y = 0, w = 0, h = 0;
    bool contains(float px, float py) const;
};
struct Color {
    float r, g, b, a;
    // 显式构造避免旧版 i686 GCC 对聚合浮点默认成员的常量求值问题。
    Color(float red, float green, float blue, float alpha = 1.f) : r(red), g(green), b(blue), a(alpha) {}
};
inline const Color Ink{35 / 255.f, 48 / 255.f, 67 / 255.f}, Muted{111 / 255.f, 126 / 255.f, 146 / 255.f},
    Blue{39 / 255.f, 105 / 255.f, 220 / 255.f}, Soft{234 / 255.f, 242 / 255.f, 1},
    Background{246 / 255.f, 248 / 255.f, 251 / 255.f}, White{1, 1, 1}, Border{.84f, .88f, .94f},
    Green{.114f, .545f, .38f}, Red{.72f, .28f, .27f};
struct Draw {
    int kind = 0;
    Rect rect, clip;
    Color color = Ink;
    std::wstring text;
    float size = 13, radius = 0, stroke = 1;
    bool bold = false, center = false, paragraph = false;
};
struct Control {
    std::string key;
    Rect rect, clip;
    std::wstring value;
    std::vector<std::wstring> options;
    int selection = 0;
};
struct Hit {
    Rect rect, clip;
    std::function<void()> action;
};
struct ScrollRegion {
    Rect rect, clip;
    std::function<bool(float)> scroll;
};
struct State {
    std::map<std::string, std::wstring> values;
    std::map<std::string, int> choices;
    std::wstring &value(const std::string &key, const std::wstring &initial);
    int &choice(const std::string &key, int initial);
};
struct Scene {
    Rect clip;
    std::vector<Draw> draws;
    std::vector<Control> controls;
    std::vector<Hit> hits;
    std::vector<ScrollRegion> scrollRegions;
    void fill(Rect r, Color color, float radius = 0);
    void border(Rect r, Color color = Border, float radius = 6);
    void text(Rect r, const std::wstring &text, float size = 13, Color color = Ink, bool bold = false,
              bool center = false, bool paragraph = false);
    void line(float x1, float y1, float x2, float y2, Color color, float thickness = 1);
    void icon(Rect r);
    void button(Rect r, const std::wstring &text, std::function<void()> action, bool selected = false);
    void input(const std::string &key, Rect r, const std::wstring &value);
    void combo(const std::string &key, Rect r, const std::vector<std::wstring> &options, int selection);
};
class App;
class Page {
  public:
    App &app;
    Scene &ui;
    State &state;
    const Tool &tool;
    float x, y, w;
    std::wstring inputError;
    Page(App &app, Scene &ui, const Tool &tool, float x, float y, float width);
    std::string key(const std::string &field) const;
    void heading(const std::wstring &title);
    void note(const std::wstring &text, Color color = Muted);
    void explain();
    double number(const std::string &field, const std::wstring &label, double initial);
    std::wstring textInput(const std::string &field, const std::wstring &label, const std::wstring &initial);
    int select(const std::string &field, const std::wstring &label, const std::vector<std::wstring> &options,
               int initial = 0);
    int count(const std::string &field, const std::wstring &label, int initial, int minimum, int maximum);
    bool collapse(const std::string &field, const std::wstring &title, bool initial = false);
    Values values(const std::string &field, const std::wstring &label, int count, const Values &initial);
    std::vector<std::pair<double, double>> pairs(const std::string &field, const std::wstring &label,
                                                 int count,
                                                 const std::vector<std::pair<double, double>> &initial,
                                                 float maximumHeight = 0);
    void metrics(const std::wstring &title, const Metrics &metrics);
    void table(const std::vector<std::wstring> &headings, const std::vector<std::vector<std::wstring>> &rows);
    void chart(const std::vector<Point> &curve, const std::vector<Point> &points = {}, Curve exact = {});
    void button(const std::wstring &title, std::function<void()> action);
    void check() const;
    void division(const std::string &field, std::pair<double, double> bounds, TemperatureUnit unit,
                  const std::wstring &signal, Curve curve);
};
// 导出线程只读取输入快照，通过原子进度和消息通知主线程，禁止直接访问 HWND 控件。
struct ExportJob {
    std::thread worker;
    std::atomic<bool> cancel{false}, active{false};
    std::atomic<std::size_t> progress{0}, total{0};
    std::mutex mutex;
    std::wstring message;
    ~ExportJob();
};
class App {
    struct Native {
        HWND window = nullptr;
        bool combo = false, seen = false;
        std::string key;
        int optionTextWidth = 0;
        float optionDpi = 0;
    };
    HWND window_ = nullptr;
    float dpi_ = 96;
    HFONT font_ = nullptr;
    HBRUSH editBrush_ = nullptr;
    ID2D1Factory *factory_ = nullptr;
    IDWriteFactory *textFactory_ = nullptr;
    ID2D1HwndRenderTarget *target_ = nullptr;
    ID2D1Bitmap *iconBitmap_ = nullptr;
    std::map<std::string, Native> native_;
    std::map<HWND, std::string> nativeKeys_;
    bool synchronizing_ = false;
    float contentHeight_ = 0, sideHeight_ = 0;
    void synchronizeControls();
    void paint();
    void releaseTarget();
    void loadIconBitmap();
    void rebuild();
    LRESULT message(UINT message, WPARAM w, LPARAM l);
    static LRESULT CALLBACK procedure(HWND, UINT, WPARAM, LPARAM);

  public:
    float mouseX = -1, mouseY = -1;
    Rect chartArea;
    State state;
    Scene scene;
    int selected = -1, filter = -1;
    bool collapsed = false;
    std::array<bool, groupCount> expanded{};
    float scroll = 0, sideScroll = 0, width = 1220, height = 820;
    ExportJob exportJob;
    ~App();
    int run(HINSTANCE instance, int show);
    void refresh();
    void open(int index);
    void homepage();
    void navigation();
    float paragraphHeight(const std::wstring &text, float width, float size = 13) const;
    float textWidth(const std::wstring &text, float size = 12) const;
    void exportCsv(const std::wstring &defaultName, const std::string &header, std::size_t count,
                   std::function<std::string(std::size_t)> row);
    HWND handle() const {
        return window_;
    }
};
void renderPage(Page &page);
void renderSensors(Page &page);
void renderPressure(Page &page);
void renderHumidity(Page &page);
void renderErrors(Page &page);
void renderOther(Page &page);
void renderCurveFit(Page &page);
void renderGauges(Page &page);
void showLicenses(HWND owner, float dpi);
} // namespace toolbox
