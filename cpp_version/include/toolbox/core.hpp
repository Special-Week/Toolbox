#pragma once
// 公共数值接口。模型一律使用 double；温度模型内部使用摄氏度，界面负责显示温标转换。
#include <algorithm>
#include <array>
#include <cmath>
#include <functional>
#include <initializer_list>
#include <limits>
#include <numeric>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace toolbox {
using Values = std::vector<double>;
using Metrics = std::vector<std::pair<std::wstring, std::wstring>>;
using Curve = std::function<double(double)>;
void require(bool condition, const char *message);
void finite(std::initializer_list<double> values);
void finite(const Values &values);
double parse(const std::string &text);
std::wstring format(double value);
std::wstring formatUnit(double value);
// 输入和导出保留原数值的最短普通十进制，不暴露二进制展开产生的无意义尾数。
std::string rawNumber(double value);
std::wstring inputNumber(double value);
std::wstring wide(const std::string &text);
std::string utf8(const std::wstring &text);
double polynomial(const Values &coefficients, double x);
std::pair<double, double> polynomialSlope(const Values &coefficients, double x);
double inverse(const Curve &curve, double target, double low, double high);
Values leastSquares(const std::vector<Values> &rows, const Values &target);
double rss(const Values &values);
struct Statistics {
    double mean, sd, minimum, maximum, standardError;
};
Statistics statistics(const Values &values);
double rangeCoefficient(std::size_t count);
double cvd(double t, double r0, double a, double b, double c);
std::array<double, 3> fitCvd(double r0, const std::array<std::pair<double, double>, 3> &points);
double atmosphereTemperature(double height);
double atmospherePressure(double height);
double atmosphereAltitude(double pressure);

struct TemperatureUnit {
    int index = 0; // 0=°C，1=K，2=°F，与 Rust 页面一致。
    std::wstring symbol() const;
    double celsius(double display) const;
    double display(double celsius) const;
    double span() const;
};
enum class Group { Units, Its90, Pressure, Humidity, Errors, Other };
enum class Dimension {
    Pressure,
    Temperature,
    MassFlow,
    VolumeFlow,
    Mass,
    Length,
    Area,
    Volume,
    Density,
    Velocity,
    Force,
    Energy,
    Power,
    HeatTransferCoefficient,
    ThermalConductivity,
    DynamicViscosity,
    KinematicViscosity,
    DiffusionCoefficient,
    SurfaceTension,
    HeatTransferRate,
    Torque,
    Concentration
};
enum class Kind {
    Unit,
    WorkingThermocouple,
    IndustrialResistance,
    StandardThermocouple,
    StandardPlatinumResistor,
    Thermistor,
    PressureReference,
    TransmitterReference,
    HeightCorrection,
    AltitudePressure,
    HumidityConversion,
    EarBlackbody,
    BlackbodyError,
    RadiationCorrection,
    Repeatability,
    RangeMethod,
    ResultantError,
    ErrorAnalysis,
    Uncertainty,
    Transmitter,
    BSplineInterpolation
};
struct Tool {
    std::string id;
    std::wstring name;
    Group group;
    Kind kind;
    std::wstring icon, description;
    Dimension dimension = Dimension::Pressure;
};
struct Unit {
    unsigned id;
    std::wstring name, code;
    double factor, offset;
};
const std::vector<Tool> &tools();
const std::vector<Unit> &units(Dimension dimension);
std::wstring groupName(Group group);
std::wstring shortGroupName(Group group);
// 说明按小节组织；variant 区分同一工具中的计算模型，避免展示不适用的公式。
using CalculationNotes = std::vector<std::pair<std::wstring, std::wstring>>;
std::wstring unitNotes(Dimension dimension);
CalculationNotes calculationNotes(const Tool &tool, int variant = 0);
double convert(Dimension dimension, double value, std::size_t from, std::size_t to);

// 坐标轴保留原值与显示坐标的变换，解决大基准、小变化及极小量的标签拥挤。
struct Tick {
    double value;
    std::wstring label;
};
struct Axis {
    double minimum, maximum, origin, factor;
    std::vector<Tick> ticks;
    std::wstring caption;
    Axis(const std::wstring &name, double low, double high, int intervals, bool roundBounds);
    double fraction(double value) const;
    double tickFraction(double value) const;
};
} // namespace toolbox
