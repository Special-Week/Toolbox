// 内嵌量规表的只读加载器。资源随 EXE 打包，运行时无需 Rust、Excel 或外部 JSON 文件。
#include "toolbox/gauges.hpp"
#include "toolbox/resource_ids.h"
#include <cctype>
#include <windows.h>

namespace toolbox::gauges {
const Data &Data::operator[](const std::string &key) const {
    static const Data empty;
    auto found = object.find(key);
    return found == object.end() ? empty : found->second;
}
const Data &Data::operator[](std::size_t index) const {
    static const Data empty;
    return index < array.size() ? array[index] : empty;
}
bool Data::has(const std::string &key) const { return object.find(key) != object.end(); }

namespace {
class Reader {
    const std::string &source;
    std::size_t position = 0;
    void skip() {
        while (position < source.size() && std::isspace(static_cast<unsigned char>(source[position]))) {
            ++position;
        }
    }
    char take() {
        require(position < source.size(), "内置量规数据意外结束");
        return source[position++];
    }
    bool accept(char c) {
        skip();
        if (position < source.size() && source[position] == c) {
            ++position;
            return true;
        }
        return false;
    }
    void expect(char c) { require(accept(c), "内置量规 JSON 结构无效"); }
    unsigned hex() {
        unsigned v = 0;
        for (int i = 0; i < 4; ++i) {
            char c = take();
            unsigned digit = c >= '0' && c <= '9'   ? unsigned(c - '0')
                             : c >= 'a' && c <= 'f' ? unsigned(c - 'a' + 10)
                             : c >= 'A' && c <= 'F' ? unsigned(c - 'A' + 10)
                                                    : 16;
            require(digit < 16, "内置量规 Unicode 转义无效");
            v = v * 16 + digit;
        }
        return v;
    }
    std::string string() {
        expect('"');
        std::string out;
        while (true) {
            char c = take();
            if (c == '"') {
                return out;
            }
            if (c != '\\') {
                require(static_cast<unsigned char>(c) >= 32, "内置量规字符串无效");
                out += c;
                continue;
            }
            c = take();
            switch (c) {
            case '"':
            case '\\':
            case '/':
                out += c;
                break;
            case 'n':
                out += '\n';
                break;
            case 'r':
                out += '\r';
                break;
            case 't':
                out += '\t';
                break;
            case 'b':
                out += '\b';
                break;
            case 'f':
                out += '\f';
                break;
            case 'u': {
                unsigned cp = hex();
                if (cp >= 0xd800 && cp <= 0xdbff) {
                    require(take() == '\\' && take() == 'u', "内置量规代理对无效");
                    unsigned lo = hex();
                    require(lo >= 0xdc00 && lo <= 0xdfff, "内置量规代理对无效");
                    cp = 0x10000 + (cp - 0xd800) * 1024 + lo - 0xdc00;
                }
                require(!(cp >= 0xdc00 && cp <= 0xdfff), "内置量规 Unicode 无效");
                if (cp < 0x80) {
                    out += char(cp);
                } else if (cp < 0x800) {
                    out += char(0xc0 | (cp >> 6));
                    out += char(0x80 | (cp & 63));
                } else if (cp < 0x10000) {
                    out += char(0xe0 | (cp >> 12));
                    out += char(0x80 | ((cp >> 6) & 63));
                    out += char(0x80 | (cp & 63));
                } else {
                    out += char(0xf0 | (cp >> 18));
                    out += char(0x80 | ((cp >> 12) & 63));
                    out += char(0x80 | ((cp >> 6) & 63));
                    out += char(0x80 | (cp & 63));
                }
                break;
            }
            default:
                require(false, "内置量规转义无效");
            }
        }
    }
    Data value(unsigned depth) {
        require(depth < 128, "内置量规数据嵌套过深");
        skip();
        require(position < source.size(), "内置量规数据为空");
        Data result;
        if (source[position] == '"') {
            result.tag = Data::Tag::String;
            result.string = string();
            return result;
        }
        if (accept('[')) {
            result.tag = Data::Tag::Array;
            if (!accept(']')) {
                do {
                    result.array.push_back(value(depth + 1));
                } while (accept(','));
                expect(']');
            }
            return result;
        }
        if (accept('{')) {
            result.tag = Data::Tag::Object;
            if (!accept('}')) {
                do {
                    auto key = string();
                    expect(':');
                    result.object.emplace(std::move(key), value(depth + 1));
                } while (accept(','));
                expect('}');
            }
            return result;
        }
        if (source.compare(position, 4, "null") == 0) {
            position += 4;
            return result;
        }
        if (source.compare(position, 4, "true") == 0) {
            position += 4;
            result.tag = Data::Tag::Number;
            result.number = 1;
            return result;
        }
        if (source.compare(position, 5, "false") == 0) {
            position += 5;
            result.tag = Data::Tag::Number;
            result.number = 0;
            return result;
        }
        auto start = position;
        while (position < source.size() &&
               std::string("-+0123456789.eE").find(source[position]) != std::string::npos) {
            ++position;
        }
        require(position > start, "内置量规 JSON 数值无效");
        result.tag = Data::Tag::Number;
        result.number = toolbox::parse(source.substr(start, position - start));
        return result;
    }

  public:
    explicit Reader(const std::string &text) : source(text) {}
    Data read() {
        auto result = value(0);
        skip();
        require(position == source.size(), "内置量规数据尾部无效");
        return result;
    }
};
std::string resource(unsigned id) {
    HMODULE module = GetModuleHandleW(nullptr);
    auto item = FindResourceW(module, MAKEINTRESOURCEW(id), RT_RCDATA);
    require(item != nullptr, "缺少内置量规资源，请使用 build.cmd 构建");
    auto loaded = LoadResource(module, item);
    require(loaded != nullptr, "无法加载内置量规数据");
    auto bytes = static_cast<const char *>(LockResource(loaded));
    require(bytes != nullptr, "无法读取内置量规数据");
    return {bytes, SizeofResource(module, item)};
}
} // namespace
Data Data::parse(const std::string &text) { return Reader(text).read(); }
const Data &database() {
    static const Data data = [] {
        auto result = Data::parse(resource(TOOLBOX_GAUGE_TABLES));
        // 与 Rust 相同的四项已确认录入修正；原始资源保留，便于追溯。
        for (auto &r : result.object["DATA"].object["smooth"].object["ebH"].array) {
            const double upper = n(r, "基本尺寸≤");
            const char *key = nullptr;
            double corrected = 0;
            if (upper == 3) {
                key = "F";
                corrected = 6;
            } else if (upper == 30) {
                key = "D";
                corrected = 65;
            } else if (upper == 80) {
                key = "D";
                corrected = 100;
            } else if (upper == 120) {
                key = "D";
                corrected = 120;
            }
            if (key) {
                auto &cell = r.object[key];
                cell.tag = Data::Tag::Number;
                cell.number = corrected;
            }
        }
        return result;
    }();
    return data;
}
const Data &unifiedData() {
    static const Data data = Data::parse(resource(TOOLBOX_GAUGE_UNIFIED));
    return data;
}
double number(const Data &v) {
    if (v.tag == Data::Tag::Number) {
        return v.number;
    }
    if (v.tag == Data::Tag::String) {
        try {
            return toolbox::parse(v.string);
        } catch (const std::exception &) {
        }
    }
    return missing;
}
double n(const Data &r, const std::string &key) { return number(r[key]); }
double first(const Data &r, std::initializer_list<const char *> keys) {
    for (auto key : keys) {
        if (r[key].tag != Data::Tag::Null) {
            return n(r, key);
        }
    }
    return missing;
}
bool valid(double v) { return std::isfinite(v) && v != 99999. && v != -100000.; }
bool same(double a, double b) { return std::abs(a - b) < 1e-9; }
} // namespace toolbox::gauges
