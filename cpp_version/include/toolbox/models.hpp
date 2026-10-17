#pragma once
#include "core.hpp"
#include <memory>
#include <optional>

namespace toolbox {
// 参考曲线分段与其一阶导数共用同一套系数，避免分度表和反算口径不同。
struct ThermoSegment {
    double minimum, maximum;
    Values coefficients;
    std::optional<std::array<double, 3>> gaussian;
    std::pair<double, double> evaluate(double t) const;
};
struct ExtraReference {
    std::string kind;
    double minimum, maximum, join, scale;
    std::vector<Values> coefficients;
};
const std::string &nistSource(const std::string &kind);
const std::vector<ExtraReference> &extraReferences();
const std::vector<ThermoSegment> &thermoSegments(const std::string &kind);
std::pair<double, double> thermoBounds(const std::string &kind);
double thermoEmf(const std::string &kind, double t);
double thermoSlope(const std::string &kind, double t);
double thermoTemperature(const std::string &kind, double emf);
struct Tolerance {
    std::wstring name, formula;
    double minimum, maximum;
    std::optional<double> delta;
};
std::vector<Tolerance> thermoTolerances(const std::string &kind, double t);

enum class RtdCurve { Cvd, Pt3916, Cu428, Cu427, Ni617, Ni618, Ni672, Ni1000 };
enum class RtdTolerance { Platinum, PlatinumReference, Copper, None };
struct RtdDefinition {
    std::string key;
    std::wstring name;
    double r0, minimum, maximum;
    RtdCurve curve;
    Values coefficients;
    RtdTolerance tolerance;
};
const std::vector<RtdDefinition> &rtdDefinitions();
struct RtdModel {
    RtdDefinition definition;
    double r0;
    Values coefficients;
    RtdModel(std::size_t index, double zero, const Values &custom = {});
    std::pair<double, double> ratio(double temperature) const;
    double resistance(double temperature) const;
    double temperature(double resistance) const;
    double slope(double temperature) const;
};
std::vector<Tolerance> rtdTolerances(RtdTolerance kind, double t, bool film);

struct ThermoZone {
    double minimum, maximum;
    Values coefficients;
};
std::vector<ThermoZone> standardZones(const std::string &kind);
struct StandardCouple {
    std::string kind = "S";
    std::vector<std::pair<double, double>> points;
    std::optional<ThermoZone> zone;
    double emf(double temperature) const;
    double temperature(double emf) const;
    static void fit(ThermoZone &zone, const std::vector<std::pair<double, double>> &points);
};
const Values &sprtA();
const Values &sprtB();
const Values &sprtC();
const Values &sprtD();
double sprtReference(double t);
struct Sprt {
    double rtp = 25.12461, a = -0.000272072, b = 0.0000178924, c = -0.0000112269;
    double a4 = 0, b4 = 0, d = 0, w660 = 3.376008599409339;
    bool lowQuadratic = false;
    void validate() const;
    double temperatureBranch(double resistance, bool high) const;
    double temperature(double resistance) const;
    double resistanceBranch(double temperature, bool high) const;
    double resistance(double temperature) const;
    void fit(const std::vector<std::pair<double, double>> &high,
             const std::vector<std::pair<double, double>> &low, int terms);
    void fitSilver(double t, double resistance);
};
struct BetaThermistor {
    double r0, t0, beta;
    BetaThermistor(double zero, double referenceC, double coefficient);
    double temperature(double resistance) const;
    double resistance(double temperature) const;
};
struct PolynomialThermistor {
    Values a, b;
    double temperature(double resistance) const;
    double resistance(double temperature) const;
    double slope(double temperature) const;
    static PolynomialThermistor fit(const std::vector<std::pair<double, double>> &points, int order, bool sh);
};

// 所有范围的上下限必须同单位，准确度按百分数输入（0.05 即 0.05%）。
struct Range {
    double low, high, span;
    Range(double lower, double upper);
    bool covers(const Range &other) const;
};
struct Selection {
    double dutTolerance, referenceTolerance, ratio;
    bool coverage, meets;
};
Selection selectStandard(Range dut, Range reference, double da, double ra, double required,
                         bool piston = false);
struct ElectricalSelection {
    Selection pressure;
    double expanded, allowance;
    bool meets;
};
ElectricalSelection selectTransmitter(Range dut, Range reference, Range output, double da, double ra,
                                      double reading, double fixed, double required);
const std::vector<std::pair<double, double>> &commonRanges();
const std::vector<std::pair<double, double>> &extendedRanges();
double hydrostatic(double height, double density, double gravity);
double gasDifference(double height, double normalDensity, double gravity, double temperature, double pressure,
                     double ambient, bool gauge);
std::pair<double, double> transmitterForward(Range process, Range output, bool root, double value);
std::pair<double, double> transmitterInverse(Range process, Range output, bool root, double value);

struct Humidity {
    double rh, saturated, vapor, volumeFraction, ppm, absolute, mass, specific, mixing, normal;
    // 湿球可能低于饱和压模型下限；单项无解不应抹去其余有效参数。
    std::optional<double> dew, frost, wet;
};
double saturation(double t);
Humidity humidity(double t, double p, int known, double value);
double effectiveWavelength(int response, double wavelength, double t);
double radiationError(double emissivity, double wavelength, double t, double background);
double radiationCorrection(double instrument, double source, double wavelength, double t, double background);
double earBlackbody(double t, double emissivity, double probe, double probeEmissivity,
                    bool singleReflection = false);
class Spectrum {
    std::vector<std::pair<double, double>> nodes_;
    double logRadiance(double kelvin) const;
    double invert(double logarithm) const;
    double transform(double t, double background, double weight) const;

  public:
    Spectrum(double wavelength, double upper = 0);
    double apparent(double t, double background, double emissivity) const;
    double corrected(double indicated, double background, double instrument, double source) const;
};

// 分量树先计算各子项贡献，再合成并应用父项灵敏系数，防止重复乘系数。
struct UncertaintyDetail {
    std::wstring name;
    double standard, sensitivity, contribution;
    std::vector<UncertaintyDetail> children;
};
struct Component {
    std::wstring name = L"分量";
    double sensitivity = 1, halfWidth = 0.02;
    int kind = 1, distribution = 0;
    bool useRange = false, maximum = false;
    Values values;
    std::vector<Component> children;
    UncertaintyDetail calculate() const;
};
double distributionDivisor(int distribution);
double combineUncertainty(const Values &values, bool maximum);
double roundedUncertainty(double value, int places, bool carry);

struct Point {
    double x = 0, y = 0;
};
class Spline {
    struct Cubic {
        double a, b, c, d;
        double value(double u) const;
        double derivative(double u) const;
        double minimumDerivative() const;
    };
    struct Segment {
        Cubic x, y;
    };
    std::vector<Point> points_;
    std::vector<Segment> segments_;
    Point origin_, scale_;
    Segment endpoint(bool right) const;
    static Cubic coefficients(const std::array<double, 4> &points, double value);
    static std::pair<double, double> externalBracket(Cubic p, double target, bool right);
    static double invert(Cubic p, double target, double low, double high);

  public:
    explicit Spline(const std::vector<Point> &points);
    double y(double x) const;
};
std::size_t sampleCount(double minimum, double maximum, double step);
double sampleX(double minimum, double maximum, double step, std::size_t count, std::size_t index);
Values divisionPoints(double start, double end, double step);
} // namespace toolbox
