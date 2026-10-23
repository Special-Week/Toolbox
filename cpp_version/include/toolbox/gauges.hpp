#pragma once
// 量规模块的输入、表格与显示分离；计算始终使用 double，格式化不会写回算法。
#include "core.hpp"
#include <map>
#include <optional>

namespace toolbox::gauges {
constexpr double missing = std::numeric_limits<double>::quiet_NaN();
constexpr double inch = 25.4;
enum class Type { Metric, Trapezoidal, Unified, Pipe, Rebar, Smooth };

// 仅承载内嵌、受信数据表，不处理网络或用户 JSON。缺失字段返回空值。
struct Data {
    enum class Tag { Null, Number, String, Array, Object } tag = Tag::Null;
    double number = missing;
    std::string string;
    std::vector<Data> array;
    std::map<std::string, Data> object;
    const Data &operator[](const std::string &key) const;
    const Data &operator[](std::size_t index) const;
    bool has(const std::string &key) const;
    static Data parse(const std::string &text);
};
const Data &database();
const Data &unifiedData();
double number(const Data &value);
double n(const Data &row, const std::string &key);
double first(const Data &row, std::initializer_list<const char *> keys);
bool valid(double value);
bool same(double a, double b);
std::string fixed(double value);

struct Cell {
    std::string display, raw;
    Cell(const char *text) : display(text), raw(text) {}
    Cell(std::string text) : display(text), raw(std::move(text)) {}
    Cell(double value);
    static Cell parameter(double value);
    static Cell limit(double value, const std::string &sign);
    static Cell pair(double a, double b, const std::string &separator = " / ");
    static Cell value(const Data &value);
};
struct Table {
    std::string title;
    std::vector<std::string> headings;
    std::vector<std::vector<Cell>> rows;
};
struct Report {
    std::string title;
    std::vector<std::pair<std::string, Cell>> metrics;
    std::vector<Table> tables;
    std::vector<std::string> notes;
    explicit Report(std::string name) : title(std::move(name)) {}
    void metric(const std::string &name, Cell value);
    void parameter(const std::string &name, double value);
    void note(const std::string &text);
    void checked() const;
    void showInches();
    std::string text(Type type) const;
};
void geometry(Report &out, double d, double p, double d2, double d1, double angle, int lines);
void positiveGeometry(double d, double p, double d2, double d1);
double pitch(const Data &row);
const Data *closestPitch(const Data &table, double p);
std::pair<double, double> tolerance(const Data &table, double d, double p, const std::string &grade,
                                    std::vector<std::string> &notes);
double deviation(const Data &table, double p, const std::string &key);
std::optional<double> minorTolerance(const Data &table, double p, const std::string &grade);
std::optional<double> trapezoidalMinorTolerance(double d, double p, const std::string &grade,
                                                const std::string &letter, std::vector<std::string> &notes);
struct Factors {
    double tr, tpl, tcp, m, zr, zpl, rgo, rng, sgo, sng;
};
Factors factors(bool trap, double tolerance, bool negativeEs);
std::pair<double, double> lengths(bool trap, bool ring, double d, double p, std::vector<std::string> &notes);
struct MetricLengths {
    double go, noGo, total;
    std::string handle;
};
MetricLengths metricLengths(double d, double p, std::vector<std::string> &notes);

struct Gauge {
    std::string code, name;
    std::array<double, 2> pd, major{missing, missing}, minor{missing, missing};
    double center, majorMin = missing, minorMax = missing, wear = missing, length = missing;
    Gauge(std::string code, std::string name, double center, double tolerance);
};
std::vector<Gauge> plugSet(double major, double low, double high, double minor, Factors f, bool trap,
                           double truncation);
std::pair<std::vector<Gauge>, std::vector<Gauge>> ringSet(double d, double d2, double d1, double es,
                                                          double td, Factors f, double truncation);
Table gaugeTable(const std::string &title, const std::vector<Gauge> &gauges, bool includeWear = true);
void referenceTable(Report &out, const std::string &name, double base, double low, double high,
                    std::optional<double> tolerance);
void factorMetrics(Report &out, Factors f, bool ring);
std::vector<std::string> grades(bool trap, double d, double p);
Report thread(bool trap, double d, double p, const std::string &grade, int lines);
Report rebar(int angle, std::size_t index, int lines);
Report unified(const std::string &size, double tpi, const std::string &grade);
Report smooth(double d, int mode, const std::string &letter, int grade);
Report pipe(bool npt, const std::string &code);
double fraction(const std::string &value);
double unDiameter(const std::string &size);
std::vector<std::pair<std::string, double>> unVariants(const std::string &size);
std::string unSeries(const std::string &size, double tpi);
std::vector<std::string> pipeSizes(bool npt);
std::vector<std::string> letters(bool hole);

struct Option {
    std::string value, label;
};
struct Field {
    std::string key, label;
    bool select = false;
    std::vector<Option> options;
};
using State = std::map<std::string, std::string>;
struct PageData {
    State state;
    std::vector<Field> fields;
    std::optional<Report> report;
    std::string error;
};
// 输入尚未完成时也返回完整字段，不抹除空串、负号或用户的分数写法。
PageData page(Type type, State state, const std::string &changed);
std::string explanation(Type type);
std::string convention(Type type);
} // namespace toolbox::gauges
