#pragma once
// Win32 仅负责窗口与编辑控件；所有展示文字由 DirectWrite 排版并使用 ClearType。
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include "models.hpp"
#include "pdf_download.hpp"
#include <atomic>
#include <d2d1.h>
#include <dwrite.h>
#include <map>
#include <memory>
#include <mutex>
#include <set>
#include <thread>
#include <vector>
#include <windows.h>

namespace toolbox {
// 测量和绘制必须共用行距，窗口缩放后段落高度才不会偏移或截断。
constexpr float paragraphLineSpacing(float size) { return size * 1.65f; }
struct Rect {
    float x = 0, y = 0, w = 0, h = 0;
    bool contains(float px, float py) const;
};
struct Color {
    float r, g, b, a;
    // 显式构造避免旧版 i686 GCC 对聚合浮点默认成员的常量求值问题。
    Color(float red, float green, float blue, float alpha = 1.f) : r(red), g(green), b(blue), a(alpha) {}
};
struct GroupAccent {
    Color fg;
    Color bg;
};

// 动态主题色板（默认浅色，由 applyTheme 切换浅色/深色，与 Rust/Web 保持一致）
inline Color Ink{24 / 255.f, 34 / 255.f, 48 / 255.f}, Muted{104 / 255.f, 118 / 255.f, 138 / 255.f},
    Blue{37 / 255.f, 99 / 255.f, 235 / 255.f}, Soft{238 / 255.f, 244 / 255.f, 1.f},
    TableStripe{245 / 255.f, 249 / 255.f, 1.f}, Background{245 / 255.f, 247 / 255.f, 251 / 255.f},
    White{1, 1, 1}, Border{224 / 255.f, 231 / 255.f, 240 / 255.f}, Green{22 / 255.f, 163 / 255.f, 74 / 255.f},
    Red{205 / 255.f, 56 / 255.f, 56 / 255.f}, SurfaceSubtle{248 / 255.f, 250 / 255.f, 253 / 255.f},
    InputBg{252 / 255.f, 253 / 255.f, 1.f}, BorderStrong{168 / 255.f, 182 / 255.f, 200 / 255.f},
    DangerSoft{1.f, 244 / 255.f, 243 / 255.f}, DangerBorder{243 / 255.f, 200 / 255.f, 200 / 255.f},
    SuccessSoft{236 / 255.f, 253 / 255.f, 245 / 255.f}, AccentText{1.f, 1.f, 1.f},
    StarGold{245 / 255.f, 158 / 255.f, 11 / 255.f};

void applyTheme(bool dark);
GroupAccent groupAccent(Group group);
Rect intersectRect(Rect a, Rect b);

struct Draw {
    int kind = 0;
    Rect rect, clip;
    Color color = Ink;
    std::wstring text;
    std::wstring formula;
    float size = 13, radius = 0, stroke = 1;
    bool bold = false, center = false, paragraph = false, mono = false;
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
// 每个工具一个本地快照，停止输入后保存；快捷换算不进入持久化范围。
bool remembersInputs(const Tool &tool);
std::wstring calibrationPath();
std::wstring uiPrefsPath();
bool readCalibrationJson(const std::wstring &path, State &state);
bool writeCalibrationJson(const std::wstring &path, const State &state);
class InputHistory {
    std::wstring path_;
    State saved_;
    bool pending_ = false;
    ULONGLONG due_ = 0;

  public:
    explicit InputHistory(std::wstring path = {});
    void load(State &state);
    void observe(const Tool &tool, const State &state);
    void flush(bool force = false);
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
              bool center = false, bool paragraph = false, bool mono = false);
    void line(float x1, float y1, float x2, float y2, Color color, float thickness = 1);
    void icon(Rect r);
    void math(Rect r, const std::wstring &fallback, const std::wstring &formula);
    void button(Rect r, const std::wstring &text, std::function<void()> action, bool selected = false);
    void input(const std::string &key, Rect r, const std::wstring &value);
    void combo(const std::string &key, Rect r, const std::vector<std::wstring> &options, int selection);
};
class App;
class Page {
    struct SplitFrame {
        bool active = false;
        bool sideBySide = false;
        bool inRight = false;
        float outerX = 0, outerW = 0, topY = 0;
        float leftW = 0, rightX = 0, rightW = 0, gap = 16;
        float leftBottom = 0;
        std::size_t leftFillIndex = 0, rightFillIndex = 0;
        Rect savedClip;
    };
    SplitFrame split_;

  public:
    App &app;
    Scene &ui;
    State &state;
    const Tool &tool;
    float x, y, w;
    std::wstring inputError;
    std::vector<std::wstring> processSteps;
    Page(App &app, Scene &ui, const Tool &tool, float x, float y, float width);
    std::string key(const std::string &field) const;

    // 统一双栏工作台原语：宽屏（>=880）左输右显并严格裁剪防越界，窄屏自动上下单栏卡片
    void beginSplit(float leftRatio = 0.42f);
    void rightColumn();
    void endSplit();

    void heading(const std::wstring &title);
    void note(const std::wstring &text, Color color = Muted);
    void error(const std::wstring &text);
    void details(const std::string &field, const std::wstring &title, const std::vector<std::wstring> &lines);
    void explain();
    std::wstring processValue(double value);
    void showProcess();
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
                  const std::wstring &signal, Curve curve,
                  std::function<std::optional<double>(double)> slope = {});
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
struct PdfJob {
    std::atomic<bool> active{false}, done{false};
    std::mutex mutex;
    std::shared_ptr<pdf::Download> result;
    std::wstring error, message;
};
enum class PdfModal { None, Warning, Success };
class App {
    struct Native {
        HWND window = nullptr;
        bool combo = false, seen = false;
        std::string key;
        std::vector<std::wstring> options;
        std::vector<std::array<std::wstring, 3>> optionRows;
        int optionTextWidth = 0;
        std::array<int, 3> optionColumns{};
        float optionDpi = 0;
        Rect layoutRect, layoutClip;
        bool layoutValid = false;
    };
    HWND window_ = nullptr;
    float dpi_ = 96;
    HFONT font_ = nullptr;
    HBRUSH editBrush_ = nullptr;
    ID2D1Factory *factory_ = nullptr;
    IDWriteFactory *textFactory_ = nullptr;
    ID2D1HwndRenderTarget *target_ = nullptr;
    ID2D1Bitmap *iconBitmap_ = nullptr;
    std::map<std::wstring, ID2D1Bitmap *> dynamicMathBitmaps_;
    std::map<std::string, Native> native_;
    std::map<HWND, std::string> nativeKeys_;
    bool synchronizing_ = false;
    float contentHeight_ = 0, sideHeight_ = 0;
    InputHistory inputHistory_;
    void clearToolInputs(const Tool &tool);
    void restoreExample();
    void synchronizeControls(bool scrolling);
    void paint();
    void releaseTarget();
    void loadIconBitmap();
    ID2D1Bitmap *loadDynamicMathBitmap(const std::wstring &formula);
    void rebuild(bool scrolling = false);
    void updateEditBrush();
    void loadUiPrefs();
    void saveUiPrefs();
    void renderPalette();
    void renderToast();
    LRESULT message(UINT message, WPARAM w, LPARAM l);
    static LRESULT CALLBACK procedure(HWND, UINT, WPARAM, LPARAM);

  public:
    // 每个屏幕像素至少对应三个公式采样点，限制高 DPI 下的缓存尺寸。
    float mathRenderScale() const { return dpi_ < 96.f ? 3.f : dpi_ > 192.f ? 6.f : dpi_ / 32.f; }
    float mathPixelScale() const { return dpi_ / 96.f; }
    std::array<std::vector<int>, 2> uncertaintyColumns;
    State calibrationSnapshot;
    bool calibrationValid = false;
    std::map<std::string, std::wstring> calibrationSelection;
    void calibrationControls(Page &page);
    void captureCalibration(Page &page, std::size_t firstControl);
    void selectCalibration(int index);
    float mouseX = -1, mouseY = -1;
    Rect chartArea;
    State state;
    std::set<std::string> openDetails;
    Scene scene;
    int selected = -1, filter = -1;
    bool collapsed = false;
    std::array<bool, groupCount> expanded{};
    float scroll = 0, sideScroll = 0, width = 1280, height = 840;

    // 现代界面增强状态：双主题、多标签栏、收藏/最近、Ctrl+K 命令面板与 Toast 轻提示
    bool darkMode = false;
    std::vector<std::string> favorites;
    std::vector<std::string> recent;
    std::vector<int> openTabs;
    bool paletteOpen = false;
    int paletteHighlight = 0;
    std::wstring toastMessage;
    ULONGLONG toastExpire = 0;

    void toggleTheme();
    bool isFavorite(const std::string &id) const;
    void toggleFavorite(const std::string &id, const std::wstring &name);
    void closeTab(int toolIndex);
    void openPalette();
    void closePalette();
    void showToast(const std::wstring &text);
    void copyToClipboard(const std::wstring &text, const std::wstring &toastText);

    ExportJob exportJob;
    std::shared_ptr<PdfJob> pdfJob = std::make_shared<PdfJob>();
    PdfModal pdfModal = PdfModal::None;
    std::wstring pdfPendingUrl, pdfDialogText;
    bool pdfSavePending = false;
    int pdfDialogFocus = 0;
    void requestPdfDownload(const std::wstring &url);
    void startPdfDownload();
    void pollPdfDownload();
    void savePdf();
    void renderPdfModal();
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
    HWND handle() const { return window_; }
};
void renderPage(Page &page);
void renderSensors(Page &page);
void renderPressure(Page &page);
void renderHumidity(Page &page);
void renderErrors(Page &page);
void renderOther(Page &page);
void renderPdfDownload(Page &page);
void renderCurveFit(Page &page);
void renderGauges(Page &page);
void clearGaugeCache(const std::string &id);
void showLicenses(HWND owner, float dpi);
} // 命名空间 toolbox
