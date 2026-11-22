#include "toolbox/curve_fit.hpp"
#include "toolbox/gauges.hpp"
#include "toolbox/models.hpp"
#include <fstream>
#include <iostream>
#include <variant>

namespace {
using namespace toolbox;
using gauges::Data;
using Outcome = std::variant<std::monostate, double, std::string>;

Values numbers(const Data &array) {
    Values result;
    for (const auto &value : array.array) {
        result.push_back(gauges::number(value));
    }
    return result;
}

Outcome evaluate(const Data &item) {
    const auto &input = item["input"];
    const auto &tool = item["tool"].string;
    auto n = [&](const std::string &key) { return gauges::n(input, key); };
    auto orNumber = [&](const std::string &key, double fallback) {
        return input.has(key) ? n(key) : fallback;
    };
    const TemperatureUnit unit{int(orNumber("unit", 0))};
    auto temperature = [&](std::optional<double> value) -> Outcome {
        return value ? Outcome(unit.display(*value)) : Outcome(std::monostate{});
    };
    if (tool == "pressure_reference_selection") {
        const auto result = selectStandard({orNumber("low", 0), n("high")}, {orNumber("refLow", 0), n("refHigh")},
                                           n("accuracy"), n("refAccuracy"), n("ratio"));
        return std::string(result.meets ? "符合" : "不符合");
    }
    if (tool == "pressure_transmitter_reference_selection") {
        const auto result = selectTransmitter({n("low"), n("high")}, {n("refLow"), n("refHigh")},
            {n("signalLow"), n("signalHigh")}, n("accuracy"), n("refAccuracy"), n("reading"),
            orNumber("fixedError", 0), n("ratio"));
        return item["metric"].string == "电信号允许限值" ? result.allowance : result.expanded;
    }
    if (tool == "standard_thermocouple") {
        StandardCouple model;
        model.kind = orNumber("type", 0) == 0 ? "S" : "B";
        if (n("source") == 1) {
            model.zone = standardZones(model.kind).at(std::size_t(orNumber("zone", 0)));
            if (input.has("coefficients")) model.zone->coefficients = numbers(input["coefficients"]);
            const auto bounds = model.bounds();
            const double cold = unit.celsiusInBounds(orNumber("cold", unit.display(0)), bounds);
            return model.emf(unit.celsiusInBounds(n("value"), bounds)) - model.referenceEndEmf(cold);
        }
        const Values temperatures = model.kind == "S" ? Values{419.527, 660.323, 1084.62}
                                                      : Values{1100, 1200, 1300, 1400, 1500};
        const auto fixed = numbers(input["fixed"]);
        require(fixed.size() == temperatures.size(), "invalid fixture fixed point count");
        for (std::size_t i = 0; i < fixed.size(); ++i) {
            model.points.emplace_back(temperatures[i], fixed[i]);
        }
        if (n("direction") == 1) {
            return unit.display(model.temperature(n("value")));
        }
        const auto slope = model.slope(unit.celsiusInBounds(n("value"), model.bounds()));
        return slope ? Outcome((*slope)[2] / unit.span()) : Outcome(std::monostate{});
    }
    if (tool == "industrial_thermal_resistance") {
        const auto index = std::size_t(orNumber("graduation", 0));
        const auto &def = rtdDefinitions().at(index);
        const RtdModel model(index, orNumber("r0", def.r0));
        if (orNumber("direction", 0) == 1) {
            return unit.display(model.temperature(n("value")));
        }
        const double t = unit.celsiusInBounds(n("value"), {def.minimum, def.maximum});
        return model.resistance(t);
    }
    if (tool == "temperature_humidity_conversion") {
        const int known = int(orNumber("known", 0));
        const double value = known == 1 || known == 2 ? unit.celsius(n("value")) : n("value");
        const auto result = humidity(unit.celsius(n("temperature")), orNumber("pressure", 101.325) * 1000,
                                     known, value);
        const auto &metric = item["metric"].string;
        if (metric == "露点") return temperature(result.dew);
        if (metric == "湿球温度") return temperature(result.wet);
        if (metric == "相对湿度") return result.rh;
        if (metric == "水气分压") return result.vapor;
        throw std::logic_error("unknown humidity metric " + metric);
    }
    if (tool == "altitude_pressure_converter")
        return atmosphereKineticTemperature(n("value")) - 273.15;
    if (tool == "transmitter") {
        const Range process{n("low"), n("high")};
        const Range signal{n("signalLow"), n("signalHigh")};
        const bool root = n("characteristic") == 1;
        return (n("direction") == 1 ? transmitterInverse(process, signal, root, n("value"))
                                    : transmitterForward(process, signal, root, n("value"))).first;
    }
    if (tool == "calibration_curve_fit") {
        std::vector<std::pair<double, double>> points;
        for (const auto &point : input["points"].array) {
            points.emplace_back(point[0].number, point[1].number);
        }
        return CurveFit(points, static_cast<CurveFitKind>(int(n("model")))).coefficients.at(1);
    }
    if (tool == "bspline_interpolation") {
        std::vector<Point> points;
        for (const auto &point : input["points"].array) {
            points.push_back({point[0].number, point[1].number});
        }
        const Spline model(points);
        return double(sampleCount(n("start"), n("end"), n("step")));
    }
    if (tool == "blackbody_radiation_source") {
        return Spectrum(n("wavelength")).apparent(n("temperature"), n("ambient"), n("emissivity"));
    }
    if (tool == "radiation_thermometer") {
        const auto response = orNumber("response", 0);
        const auto spectrum = response == 1 ? Spectrum(8., 14.) : response == 2 ? Spectrum(7., 18.) : Spectrum(n("wavelength"));
        return spectrum.corrected(n("temperature"), n("ambient"), n("instrumentEmissivity"), n("emissivity"));
    }
    if (tool == "working_thermocouple") {
        const std::string kind = workingThermocoupleKinds.at(std::size_t(orNumber("type", 3)));
        const auto bounds = thermoBounds(kind);
        const double cold = unit.celsiusInBounds(orNumber("cold", unit.display(0)), bounds);
        const double coldEmf = thermoEmf(kind, cold);
        const double correction = orNumber("correction", 0);
        const bool inverse = n("direction") == 1;
        const double value = orNumber("value", inverse ? 4.096230218723254 : unit.display(100));
        const double signal = inverse ? unit.display(thermoTemperatureFromMeasured(kind, value, coldEmf, correction))
                                      : thermoEmf(kind, unit.celsiusInBounds(value, bounds)) - coldEmf - correction;
        if (input["division"].number == 1) {
            const double start = n("tableStart"), end = n("tableEnd");
            require(start >= unit.display(bounds.first) && end <= unit.display(bounds.second), "分度表超出所选温区");
            return double(divisionPoints(start, end, n("tableStep")).size());
        }
        return signal;
    }
    if (tool == "thermistor") {
        const PolynomialThermistor model{input.has("aCoefficients") ? numbers(input["aCoefficients"]) : Values{},
                                         numbers(input["bCoefficients"])};
        const double minimum = orNumber("min", -20), maximum = orNumber("max", 100);
        double t = 0, r = 0;
        if (orNumber("direction", 0) == 1) {
            t = model.temperatureInBounds(n("value"), minimum, maximum);
            require(t >= minimum && t <= maximum, "温度须处于有效校准温区内");
            r = n("value");
        } else {
            t = unit.celsiusInBounds(n("value"), {minimum, maximum});
            require(t >= minimum && t <= maximum, "温度须处于有效校准温区内");
            r = model.resistance(t);
        }
        const auto &metric = item.has("metric") ? item["metric"].string : std::string{};
        if (metric == "电阻变化率") return model.slope(t) / unit.span();
        if (metric == "温度") return unit.display(t);
        return r;
    }
    if (tool == "standard_platinum_resistor_sprt") {
        const int highRange = int(orNumber("highRange", 1));
        const int lowRange = int(orNumber("lowRange", 0));
        const bool highEnabled = highRange != 6;
        const bool lowEnabled = lowRange != 2;
        Sprt model;
        model.rtp = orNumber("rtp", 25.12461);
        model.a = highEnabled ? orNumber("a", -0.000272072) : 0.0;
        model.b = (highEnabled && highRange <= 3) ? orNumber("b", 0.0000178924) : 0.0;
        model.c = (highEnabled && highRange <= 1) ? orNumber("c", -0.0000112269) : 0.0;
        model.a4 = lowEnabled ? orNumber("a4", 0.0) : 0.0;
        model.b4 = lowEnabled ? orNumber("b4", 0.0) : 0.0;
        model.lowQuadratic = lowRange == 1;
        model.d = highRange == 0 ? orNumber("d", 0.0) : 0.0;
        model.w660 = orNumber("w660", 3.376008599409339);
        if (highRange == 0) require(model.w660 > 1.0, "W660 须大于 1");
        require(highEnabled || lowEnabled, "至少启用一个温区");
        if (orNumber("direction", 0) == 1) {
            const double r = n("value");
            const bool high = highEnabled && (!lowEnabled || r >= model.rtp);
            return unit.display(model.temperatureBranch(r, high));
        }
        const double t = unit.celsius(n("value"));
        const bool high = highEnabled && (!lowEnabled || t >= 0.01);
        return model.resistanceBranch(t, high);
    }
    if (tool == "uncertainty_calculation") {
        Values contributions;
        std::vector<std::pair<double, DegreesOfFreedom>> dofTerms;
        for (const auto &node : input["components"].array) {
            Component c;
            c.kind = node.has("kind") ? int(gauges::number(node["kind"])) : 1;
            c.sensitivity = node.has("sensitivity") ? gauges::number(node["sensitivity"]) : 1.0;
            if (c.kind == 0) {
                c.useRange = node.has("method") && int(gauges::number(node["method"])) == 1;
                c.values = numbers(node["values"]);
            } else {
                c.halfWidth = gauges::n(node, "halfWidth");
                c.distribution = node.has("distribution") ? int(gauges::number(node["distribution"])) : 0;
                if (node.has("dof") && node["dof"].tag != Data::Tag::Null) {
                    c.dof = node["dof"].tag == Data::Tag::String
                                ? parseDegreesOfFreedom(wide(node["dof"].string))
                                : parseDegreesOfFreedom(wide(rawNumber(gauges::number(node["dof"]))));
                }
            }
            const auto d = c.calculate();
            contributions.push_back(d.contribution);
            dofTerms.push_back({d.contribution, d.dof});
        }
        const bool maximum = orNumber("combine", 0) == 1;
        const double uc = combineUncertainty(contributions, maximum);
        const auto eff = combineDegreesOfFreedom(dofTerms, maximum);
        const double k = coverageFactor(int(orNumber("coverageMode", 0)), orNumber("coverage", 2.0), eff);
        const auto &metric = item.has("metric") ? item["metric"].string : std::string{};
        if (metric == "有效自由度 ν_eff") {
            if (eff.kind == DofKind::Finite) return eff.value;
            return std::string(eff.kind == DofKind::Infinite ? "∞" : "不适用");
        }
        if (metric == "包含因子 k") return k;
        if (metric == "修约后 U") {
            return roundedCombinedUncertainty(uc, k, int(orNumber("digits", 1)) + 1,
                                              orNumber("roundingV2", 1) == 1).second;
        }
        return uc;
    }
    if (tool == "repeatability_calculation") {
        const auto stats = statistics(numbers(input["values"]));
        return item["metric"].string == "平均值" ? stats.mean : stats.sd;
    }
    throw std::logic_error("unknown fixture tool " + tool);
}

bool check(const Data &item) {
    if (item.has("expected") == item.has("error")) {
        std::cerr << "fixture requires exactly one of expected or error\n";
        return false;
    }
    const auto &expectedError = item["error"].string;
    if (item.has("error") && expectedError.empty()) {
        std::cerr << "fixture error must be a nonempty string\n";
        return false;
    }
    Outcome actual;
    try {
        actual = evaluate(item);
    } catch (const std::exception &error) {
        if (!expectedError.empty() && std::string(error.what()).find(expectedError) != std::string::npos) {
            return true;
        }
        std::cerr << error.what() << '\n';
        return false;
    }
    if (!expectedError.empty()) {
        return false;
    }
    const auto &expected = item["expected"];
    if (const auto number = std::get_if<double>(&actual)) {
        const double tolerance = item.has("tolerance") ? item["tolerance"].number : 0.;
        const bool ok = expected.tag == Data::Tag::Number && std::isfinite(*number) &&
                        std::abs(*number - expected.number) <= tolerance;
        if (!ok) {
            std::cerr.precision(17);
            std::cerr << "  got " << *number << ", expected " << expected.number << '\n';
        }
        return ok;
    }
    if (const auto text = std::get_if<std::string>(&actual)) {
        return expected.tag == Data::Tag::String && *text == expected.string;
    }
    return expected.tag == Data::Tag::Null;
}
}

int main(int argc, char **argv) {
    if (argc != 2) {
        return 2;
    }
    std::ifstream file(argv[1], std::ios::binary);
    const std::string json((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    const auto cases = Data::parse(json);
    if (cases.array.empty()) {
        return 2;
    }
    bool passed = true;
    for (const auto &item : cases.array) {
        if (!check(item)) {
            std::cerr << "boundary regression failed: " << item["id"].string << '\n';
            passed = false;
        }
    }
    if (passed) {
        std::cout << cases.array.size() << " boundary regressions passed\n";
    }
    return passed ? 0 : 1;
}
