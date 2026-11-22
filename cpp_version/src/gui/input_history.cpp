// 输入以 UTF-8 JSON 保存在当前用户目录，完整写入临时文件后再替换正式文件。
#include "toolbox/gui.hpp"
#include <utility>

namespace toolbox {
namespace {
constexpr DWORD maximumBytes = 8 * 1024 * 1024;

struct File {
    HANDLE value = INVALID_HANDLE_VALUE;
    ~File() { close(); }
    void close() {
        if (value != INVALID_HANDLE_VALUE) {
            CloseHandle(value);
            value = INVALID_HANDLE_VALUE;
        }
    }
};

std::wstring environment(const wchar_t *name) {
    DWORD size = GetEnvironmentVariableW(name, nullptr, 0);
    if (!size)
        return {};
    std::wstring value(size, L'\0');
    DWORD length = GetEnvironmentVariableW(name, value.data(), size);
    if (!length || length >= size)
        return {};
    value.resize(length);
    return value;
}

std::wstring defaultPath() {
    auto base = environment(L"LOCALAPPDATA");
    if (base.empty())
        base = environment(L"APPDATA");
    return base.empty() ? L"" : base + L"\\Toolbox\\cpp\\inputs-v1.json";
}

bool ensureDirectory(const std::wstring &path) {
    const DWORD attributes = GetFileAttributesW(path.c_str());
    if (attributes != INVALID_FILE_ATTRIBUTES)
        return (attributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
    const auto separator = path.find_last_of(L"\\/");
    if (separator == std::wstring::npos || !separator || !ensureDirectory(path.substr(0, separator)))
        return false;
    if (CreateDirectoryW(path.c_str(), nullptr))
        return true;
    const DWORD current = GetFileAttributesW(path.c_str());
    return current != INVALID_FILE_ATTRIBUTES && (current & FILE_ATTRIBUTE_DIRECTORY) != 0;
}

bool startsWith(const std::string &key, const std::string &prefix) {
    return key.compare(0, prefix.size(), prefix) == 0;
}

bool rememberedField(const std::string &key) {
    const auto separator = key.find('.');
    if (separator == std::string::npos || separator + 1 == key.size())
        return false;
    const auto id = key.substr(0, separator);
    for (const auto &tool : tools())
        if (tool.id == id)
            return remembersInputs(tool);
    return false;
}

State toolInputs(const Tool &tool, const State &state) {
    State result;
    const auto prefix = tool.id + ".";
    for (auto it = state.values.lower_bound(prefix);
         it != state.values.end() && startsWith(it->first, prefix); ++it)
        result.values.insert(*it);
    for (auto it = state.choices.lower_bound(prefix);
         it != state.choices.end() && startsWith(it->first, prefix); ++it)
        result.choices.insert(*it);
    return result;
}

void replaceTool(const Tool &tool, const State &source, State &target) {
    const auto prefix = tool.id + ".";
    for (auto it = target.values.lower_bound(prefix);
         it != target.values.end() && startsWith(it->first, prefix);)
        it = target.values.erase(it);
    for (auto it = target.choices.lower_bound(prefix);
         it != target.choices.end() && startsWith(it->first, prefix);)
        it = target.choices.erase(it);
    target.values.insert(source.values.begin(), source.values.end());
    target.choices.insert(source.choices.begin(), source.choices.end());
}

std::string quotedJson(const std::string &text) {
    constexpr char hex[] = "0123456789abcdef";
    std::string result = "\"";
    for (unsigned char c : text) {
        switch (c) {
        case '"':
            result += "\\\"";
            break;
        case '\\':
            result += "\\\\";
            break;
        case '\b':
            result += "\\b";
            break;
        case '\f':
            result += "\\f";
            break;
        case '\n':
            result += "\\n";
            break;
        case '\r':
            result += "\\r";
            break;
        case '\t':
            result += "\\t";
            break;
        default:
            if (c < 0x20) {
                result += "\\u00";
                result += hex[c >> 4];
                result += hex[c & 15];
            } else
                result += char(c);
        }
    }
    return result + '"';
}

std::string encode(const State &state) {
    std::string text = "{\n  \"version\": 1,\n  \"values\": {";
    bool first = true;
    for (const auto &entry : state.values) {
        if (!first)
            text += ',';
        text += "\n    " + quotedJson(entry.first) + ": " + quotedJson(utf8(entry.second));
        first = false;
    }
    text += "\n  },\n  \"choices\": {";
    first = true;
    for (const auto &entry : state.choices) {
        if (!first)
            text += ',';
        text += "\n    " + quotedJson(entry.first) + ": " + std::to_string(entry.second);
        first = false;
    }
    return text + "\n  }\n}\n";
}

// 只接受本文件写出的固定结构；限制文件大小并完整校验后才恢复任何字段。
class Reader {
    const std::string &text_;
    std::size_t position_ = 0;
    void skip() {
        while (position_ < text_.size() && (text_[position_] == ' ' || text_[position_] == '\t' ||
                                            text_[position_] == '\r' || text_[position_] == '\n'))
            ++position_;
    }
    char take() {
        require(position_ < text_.size(), "输入记录不完整");
        return text_[position_++];
    }
    bool accept(char c) {
        skip();
        if (position_ == text_.size() || text_[position_] != c)
            return false;
        ++position_;
        return true;
    }
    void expect(char c) { require(accept(c), "输入记录 JSON 格式无效"); }
    unsigned hex() {
        unsigned result = 0;
        for (int i = 0; i < 4; ++i) {
            const char c = take();
            const unsigned digit = c >= '0' && c <= '9'   ? unsigned(c - '0')
                                   : c >= 'a' && c <= 'f' ? unsigned(c - 'a' + 10)
                                   : c >= 'A' && c <= 'F' ? unsigned(c - 'A' + 10)
                                                          : 16;
            require(digit < 16, "输入记录 Unicode 转义无效");
            result = result * 16 + digit;
        }
        return result;
    }
    std::string string() {
        expect('"');
        std::string result;
        while (true) {
            const char c = take();
            if (c == '"')
                return result;
            require(static_cast<unsigned char>(c) >= 0x20, "输入记录字符串无效");
            if (c != '\\') {
                result += c;
                continue;
            }
            switch (take()) {
            case '"':
                result += '"';
                break;
            case '\\':
                result += '\\';
                break;
            case '/':
                result += '/';
                break;
            case 'b':
                result += '\b';
                break;
            case 'f':
                result += '\f';
                break;
            case 'n':
                result += '\n';
                break;
            case 'r':
                result += '\r';
                break;
            case 't':
                result += '\t';
                break;
            case 'u': {
                const unsigned high = hex();
                std::wstring codepoints(1, wchar_t(high));
                if (high >= 0xd800 && high <= 0xdbff) {
                    require(take() == '\\' && take() == 'u', "输入记录 Unicode 代理对无效");
                    const unsigned low = hex();
                    require(low >= 0xdc00 && low <= 0xdfff, "输入记录 Unicode 代理对无效");
                    codepoints += wchar_t(low);
                } else
                    require(high < 0xdc00 || high > 0xdfff, "输入记录 Unicode 代理对无效");
                result += utf8(codepoints);
                break;
            }
            default:
                require(false, "输入记录字符串转义无效");
            }
        }
    }
    int integer() {
        skip();
        bool negative = false;
        if (position_ < text_.size() && text_[position_] == '-') {
            negative = true;
            ++position_;
        }
        // '-' 后不能出现空白；数字只接受 JSON 的十进制整数形式。
        require(position_ < text_.size() && text_[position_] >= '0' && text_[position_] <= '9',
                "输入记录整数无效");
        const bool zero = text_[position_] == '0';
        unsigned long long result = unsigned(take() - '0');
        const auto limit = static_cast<unsigned long long>(std::numeric_limits<int>::max()) + negative;
        while (position_ < text_.size() && text_[position_] >= '0' && text_[position_] <= '9') {
            require(!zero, "输入记录整数不能包含前导零");
            result = result * 10 + unsigned(take() - '0');
            require(result <= limit, "输入记录整数超出范围");
        }
        if (negative && result == limit)
            return std::numeric_limits<int>::min();
        return negative ? -int(result) : int(result);
    }
    template <class Value> void object(Value value) {
        expect('{');
        if (accept('}'))
            return;
        do {
            const auto key = string();
            expect(':');
            value(key);
        } while (accept(','));
        expect('}');
    }

  public:
    explicit Reader(const std::string &text) : text_(text) {}
    State read(bool filter = true) {
        State state;
        bool version = false, values = false, choices = false;
        object([&](const std::string &key) {
            if (key == "version" && !version) {
                require(integer() == 1, "输入记录版本不支持");
                version = true;
            } else if (key == "values" && !values) {
                object([&](const std::string &field) {
                    auto value = wide(string());
                    require(state.values.emplace(field, std::move(value)).second, "输入记录字段重复");
                });
                values = true;
            } else if (key == "choices" && !choices) {
                object([&](const std::string &field) {
                    require(state.choices.emplace(field, integer()).second, "输入记录字段重复");
                });
                choices = true;
            } else
                require(false, "输入记录字段无效或重复");
        });
        skip();
        require(version && values && choices && position_ == text_.size(), "输入记录结构不完整");
        if (filter) {
            for (auto it = state.values.begin(); it != state.values.end();)
                if (!rememberedField(it->first))
                    it = state.values.erase(it);
                else
                    ++it;
            for (auto it = state.choices.begin(); it != state.choices.end();)
                if (!rememberedField(it->first))
                    it = state.choices.erase(it);
                else
                    ++it;
        }
        return state;
    }
};

bool readJson(const std::wstring &path, State &state, bool filter = true, bool *retry = nullptr) {
    File file;
    file.value = CreateFileW(path.c_str(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_DELETE, nullptr,
                             OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file.value == INVALID_HANDLE_VALUE) {
        const DWORD error = GetLastError();
        if (retry) *retry = error != ERROR_FILE_NOT_FOUND && error != ERROR_PATH_NOT_FOUND;
        return false;
    }
    LARGE_INTEGER size{};
    if (retry) *retry = true;
    if (!GetFileSizeEx(file.value, &size) || size.QuadPart > maximumBytes)
        return false;
    std::string text(std::size_t(size.QuadPart), '\0');
    DWORD read = 0;
    if (!ReadFile(file.value, text.data(), DWORD(text.size()), &read, nullptr) || read != text.size())
        return false;
    if (retry) *retry = false;
    // 接受常见编辑器生成的 UTF-8 BOM，但不接受损坏的 UTF-8 字节。
    if (text.compare(0, 3, "\xef\xbb\xbf") == 0)
        text.erase(0, 3);
    if (text.empty() ||
        !MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(), int(text.size()), nullptr, 0))
        return false;
    try {
        state = Reader(text).read(filter);
        return true;
    } catch (const std::exception &) {
        return false;
    }
}

bool writeJson(const std::wstring &path, const State &state) {
    if (path.empty())
        return false;
    const auto text = encode(state);
    if (text.size() > maximumBytes)
        return false;
    const auto separator = path.find_last_of(L"\\/");
    if (separator == std::wstring::npos || !ensureDirectory(path.substr(0, separator)))
        return false;
    File file;
    std::wstring temporary;
    for (unsigned attempt = 0; attempt < 32; ++attempt) {
        temporary = path + L"." + std::to_wstring(GetCurrentProcessId()) + L"." +
                    std::to_wstring(GetTickCount64()) + L"." + std::to_wstring(attempt) + L".tmp";
        file.value = CreateFileW(temporary.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                                 FILE_ATTRIBUTE_NORMAL, nullptr);
        if (file.value != INVALID_HANDLE_VALUE)
            break;
        if (GetLastError() != ERROR_FILE_EXISTS && GetLastError() != ERROR_ALREADY_EXISTS)
            return false;
    }
    if (file.value == INVALID_HANDLE_VALUE)
        return false;
    DWORD written = 0;
    const bool complete = WriteFile(file.value, text.data(), DWORD(text.size()), &written, nullptr) &&
                          written == text.size() && FlushFileBuffers(file.value);
    file.close();
    if (complete &&
        MoveFileExW(temporary.c_str(), path.c_str(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH))
        return true;
    DeleteFileW(temporary.c_str());
    return false;
}
} // 命名空间

std::wstring calibrationPath() {
    auto base = environment(L"LOCALAPPDATA");
    if (base.empty())
        base = environment(L"APPDATA");
    return base.empty() ? L"" : base + L"\\Toolbox\\cpp\\calibrations-v1.json";
}
std::wstring uiPrefsPath() {
    auto base = environment(L"LOCALAPPDATA");
    if (base.empty())
        base = environment(L"APPDATA");
    return base.empty() ? L"" : base + L"\\Toolbox\\cpp\\ui-prefs-v1.json";
}
bool readCalibrationJson(const std::wstring &path, State &state) {
    if (path.empty())
        return false;
    const DWORD attributes = GetFileAttributesW(path.c_str());
    if (attributes == INVALID_FILE_ATTRIBUTES) {
        const DWORD error = GetLastError();
        if (error == ERROR_FILE_NOT_FOUND || error == ERROR_PATH_NOT_FOUND) {
            state = {};
            return true;
        }
        return false;
    }
    return !(attributes & FILE_ATTRIBUTE_DIRECTORY) && readJson(path, state, false);
}
bool writeCalibrationJson(const std::wstring &path, const State &state) {
    try {
        return writeJson(path, state);
    } catch (const std::exception &) {
        return false;
    }
}

bool remembersInputs(const Tool &tool) {
    switch (tool.kind) {
    case Kind::StandardThermocouple:
    case Kind::StandardPlatinumResistor:
    case Kind::Thermistor:
    case Kind::PressureReference:
    case Kind::TransmitterReference:
    case Kind::Repeatability:
    case Kind::RangeMethod:
    case Kind::ResultantError:
    case Kind::Uncertainty:
    case Kind::BSplineInterpolation:
    case Kind::CalibrationCurveFit:
    case Kind::GaugeMetric:
    case Kind::GaugeTrapezoidal:
    case Kind::GaugeUnified:
    case Kind::GaugePipe:
    case Kind::GaugeRebar:
    case Kind::GaugeSmooth:
        return true;
    default:
        return false;
    }
}

InputHistory::InputHistory(std::wstring path) : path_(path.empty() ? defaultPath() : std::move(path)) {}

void InputHistory::load(State &state) {
    State loaded;
    if (!readJson(path_, loaded))
        return;
    saved_ = loaded;
    state.values.insert(loaded.values.begin(), loaded.values.end());
    state.choices.insert(loaded.choices.begin(), loaded.choices.end());
}

bool InputHistory::reloadTool(const Tool &tool, State &state) {
    if (path_.empty() || pending(tool))
        return false;
    State loaded;
    bool retry = false;
    readJson(path_, loaded, true, &retry);
    if (retry)
        return false;
    const auto current = toolInputs(tool, loaded);
    replaceTool(tool, current, state);
    replaceTool(tool, current, saved_);
    return true;
}

void InputHistory::observe(const Tool &tool, const State &state, bool reset) {
    if (!remembersInputs(tool))
        return;
    const auto current = toolInputs(tool, state), previous = toolInputs(tool, saved_);
    if (!reset && current.values == previous.values && current.choices == previous.choices)
        return;
    const auto markChanges = [](const auto &before, const auto &after, auto &fields) {
        for (const auto &[key, value] : before)
            if (after.find(key) == after.end())
                fields.insert(key);
        for (const auto &[key, value] : after) {
            const auto old = before.find(key);
            if (old == before.end() || old->second != value)
                fields.insert(key);
        }
    };
    if (reset)
        resets_.insert(tool.id);
    markChanges(previous.values, current.values, changedValues_);
    markChanges(previous.choices, current.choices, changedChoices_);
    replaceTool(tool, current, saved_);
    dirty_.insert(tool.id);
    due_ = GetTickCount64() + 700;
}

void InputHistory::snapshot(const Tool &tool, const State &state) {
    if (remembersInputs(tool))
        replaceTool(tool, toolInputs(tool, state), saved_);
}

void InputHistory::flush(bool force) {
    if (dirty_.empty() || (!force && GetTickCount64() < due_))
        return;
    due_ = GetTickCount64() + 5000;
    try {
        const auto separator = path_.find_last_of(L"\\/");
        if (separator == std::wstring::npos || !ensureDirectory(path_.substr(0, separator)))
            return;
        File lock;
        lock.value = CreateFileW((path_ + L".lock").c_str(), GENERIC_READ | GENERIC_WRITE,
                                 FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr, OPEN_ALWAYS,
                                 FILE_ATTRIBUTE_NORMAL, nullptr);
        OVERLAPPED range{};
        if (lock.value == INVALID_HANDLE_VALUE || !LockFileEx(lock.value, LOCKFILE_EXCLUSIVE_LOCK, 0, 1, 0, &range))
            return;
        State merged;
        bool retry = false;
        readJson(path_, merged, true, &retry);
        if (retry)
            return;
        for (const auto &tool : tools())
            if (resets_.count(tool.id))
                replaceTool(tool, toolInputs(tool, saved_), merged);
        auto valueFields = changedValues_, choiceFields = changedChoices_;
        for (const auto &tool : tools()) {
            if (!pending(tool))
                continue;
            const auto prefix = tool.id + ".";
            if (tool.kind == Kind::PressureReference || tool.kind == Kind::TransmitterReference) {
                auto unit = prefix + "selection.unit";
                const auto &pressureUnits = units(Dimension::Pressure);
                const auto mpa = std::find_if(pressureUnits.begin(), pressureUnits.end(),
                    [](const auto &entry) { return entry.code == L"MPa"; });
                const int defaultUnit = mpa == pressureUnits.end() ? 0 : int(mpa - pressureUnits.begin());
                const auto getUnit = [&](const State &state) {
                    auto it = state.choices.find(unit);
                    return it == state.choices.end() ? defaultUnit : it->second;
                };
                const std::vector<std::string> fields{
                    prefix + "dut_low", prefix + "dut_high", prefix + "ref_low", prefix + "ref_high",
                };
                if (getUnit(saved_) != getUnit(merged) &&
                    (choiceFields.count(unit) || std::any_of(fields.begin(), fields.end(),
                        [&](const auto &key) { return valueFields.count(key) != 0; }))) {
                    valueFields.insert(fields.begin(), fields.end());
                    choiceFields.insert(unit);
                    choiceFields.insert(prefix + "previous.unit");
                }
            } else if (tool.kind == Kind::GaugeUnified) {
                const std::vector<std::string> fields{
                    prefix + "gauge.input.size", prefix + "gauge.input.tpi",
                    prefix + "gauge.input.series", prefix + "gauge.input._lastSeries",
                    prefix + "gauge.input._threadChange",
                    prefix + "gauge.size", prefix + "gauge.tpi",
                };
                if (std::any_of(fields.begin(), fields.end(),
                        [&](const auto &key) { return valueFields.count(key) != 0; }) ||
                    choiceFields.count(prefix + "gauge.series.select")) {
                    valueFields.insert(fields.begin(), fields.end());
                    choiceFields.insert(prefix + "gauge.series.select");
                }
            }
        }
        const auto applyChanges = [](const auto &fields, const auto &source, auto &destination) {
            for (const auto &key : fields) {
                const auto value = source.find(key);
                if (value == source.end())
                    destination.erase(key);
                else
                    destination.insert_or_assign(key, value->second);
            }
        };
        applyChanges(valueFields, saved_.values, merged.values);
        applyChanges(choiceFields, saved_.choices, merged.choices);
        if (writeJson(path_, merged)) {
            dirty_.clear();
            resets_.clear();
            changedValues_.clear();
            changedChoices_.clear();
        }
    } catch (const std::exception &) {
        // 保存失败保留待存输入，稍后重试，不中断当前计算。
    }
}
} // 命名空间 toolbox
