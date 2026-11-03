#include "toolbox/pdf_download.hpp"

#include <windows.h>
#include <winhttp.h>
#include <bcrypt.h>
#include "pdfio.h"

#include <algorithm>
#include <array>
#include <cctype>
#include <cwctype>
#include <limits>
#include <regex>
#include <sstream>
#include <utility>

namespace toolbox::pdf {
namespace {
constexpr size_t max_pdf_bytes = 50 * 1024 * 1024;
const wchar_t* host = L"jjg.spc.org.cn";
const wchar_t* agent = L"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                       L"(KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0";
const char* error_pdf_hash = "f487c0adbe09950cd6effdbca2af0b9d59b4c53ed7adbf5e58cfff694b3e2ed8";

struct InternetHandle {
    HINTERNET value = nullptr;
    explicit InternetHandle(HINTERNET handle) : value(handle) {}
    ~InternetHandle() { if (value) WinHttpCloseHandle(value); }
    InternetHandle(const InternetHandle&) = delete;
    InternetHandle& operator=(const InternetHandle&) = delete;
};

std::wstring win_error(const wchar_t* action) {
    return std::wstring(action) + L"（错误代码 " + std::to_wstring(GetLastError()) + L"）";
}

std::wstring trim(std::wstring text) {
    const auto start = text.find_first_not_of(L" \t\r\n");
    if (start == std::wstring::npos) return L"";
    const auto end = text.find_last_not_of(L" \t\r\n");
    return text.substr(start, end - start + 1);
}

std::wstring from_utf8(const std::string& text) {
    if (text.empty()) return L"";
    int n = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(),
                                static_cast<int>(text.size()), nullptr, 0);
    if (!n) throw Failure(L"网站返回了无效的 UTF-8 文本。 ");
    std::wstring result(static_cast<size_t>(n), L'\0');
    MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(),
                        static_cast<int>(text.size()), &result[0], n);
    return result;
}

std::string to_utf8(const std::wstring& text) {
    if (text.empty()) return "";
    int n = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text.data(),
                                static_cast<int>(text.size()), nullptr, 0, nullptr, nullptr);
    if (!n) throw Failure(L"文本编码转换失败。 ");
    std::string result(static_cast<size_t>(n), '\0');
    WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text.data(),
                        static_cast<int>(text.size()), &result[0], n, nullptr, nullptr);
    return result;
}

std::string url_encode(const std::string& text) {
    const char* digits = "0123456789ABCDEF";
    std::string result;
    for (unsigned char c : text) {
        if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') ||
            (c >= '0' && c <= '9') || c == '-' || c == '_' || c == '.' || c == '~') {
            result += static_cast<char>(c);
        } else {
            result += '%'; result += digits[c >> 4]; result += digits[c & 15];
        }
    }
    return result;
}

std::wstring decode_parameter(const std::wstring& text) {
    std::string bytes;
    for (size_t i = 0; i < text.size(); ++i) {
        wchar_t c = text[i];
        if (c == L'+') bytes += ' ';
        else if (c == L'%') {
            if (i + 2 >= text.size() || !std::iswxdigit(text[i + 1]) || !std::iswxdigit(text[i + 2]))
                throw Failure(L"规范编号的 URL 编码无效。 ");
            unsigned long n = std::stoul(text.substr(i + 1, 2), nullptr, 16);
            bytes += static_cast<char>(n); i += 2;
        } else if (c <= 127) bytes += static_cast<char>(c);
        else throw Failure(L"规范编号中包含无效字符。 ");
    }
    return from_utf8(bytes);
}

std::wstring unescape_html(const std::wstring& input) {
    std::wstring result;
    for (size_t i = 0; i < input.size(); ++i) {
        if (input[i] != L'&') { result += input[i]; continue; }
        size_t end = input.find(L';', i + 1);
        if (end == std::wstring::npos || end - i > 12) { result += input[i]; continue; }
        std::wstring entity = input.substr(i + 1, end - i - 1);
        unsigned long value = 0;
        if (entity == L"amp") value = L'&';
        else if (entity == L"lt") value = L'<';
        else if (entity == L"gt") value = L'>';
        else if (entity == L"quot") value = L'"';
        else if (entity == L"apos") value = L'\'';
        else if (!entity.empty() && entity[0] == L'#') {
            try {
                size_t consumed = 0;
                bool hex = entity.size() > 2 && (entity[1] == L'x' || entity[1] == L'X');
                std::wstring digits = entity.substr(hex ? 2 : 1);
                value = std::stoul(digits, &consumed, hex ? 16 : 10);
                if (consumed != digits.size()) value = 0;
            } catch (...) { value = 0; }
        }
        if (!value || value > 0x10ffff || (value >= 0xd800 && value <= 0xdfff)) {
            result += input[i]; continue;
        }
        if (value < 0x10000) result += static_cast<wchar_t>(value);
        else {
            value -= 0x10000;
            result += static_cast<wchar_t>(0xd800 + (value >> 10));
            result += static_cast<wchar_t>(0xdc00 + (value & 0x3ff));
        }
        i = end;
    }
    return result;
}

std::string sha256(const std::vector<unsigned char>& data) {
    BCRYPT_ALG_HANDLE algorithm = nullptr;
    BCRYPT_HASH_HANDLE hash = nullptr;
    DWORD object_size = 0, returned = 0;
    if (BCryptOpenAlgorithmProvider(&algorithm, BCRYPT_SHA256_ALGORITHM, nullptr, 0) < 0)
        throw Failure(L"无法初始化文件校验。 ");
    NTSTATUS result = BCryptGetProperty(algorithm, BCRYPT_OBJECT_LENGTH,
                                        reinterpret_cast<PUCHAR>(&object_size), sizeof(object_size),
                                        &returned, 0);
    std::vector<unsigned char> object(object_size);
    std::array<unsigned char, 32> digest{};
    if (result >= 0) result = BCryptCreateHash(algorithm, &hash, object.data(), object_size, nullptr, 0, 0);
    if (result >= 0) result = BCryptHashData(hash, const_cast<PUCHAR>(data.data()),
                                             static_cast<ULONG>(data.size()), 0);
    if (result >= 0) result = BCryptFinishHash(hash, digest.data(), static_cast<ULONG>(digest.size()), 0);
    if (hash) BCryptDestroyHash(hash);
    BCryptCloseAlgorithmProvider(algorithm, 0);
    if (result < 0) throw Failure(L"文件校验失败。 ");
    const char* hex = "0123456789abcdef";
    std::string output;
    for (unsigned char c : digest) { output += hex[c >> 4]; output += hex[c & 15]; }
    return output;
}

struct TempFile {
    std::wstring path;
    explicit TempFile(const std::wstring& directory) {
        wchar_t buffer[MAX_PATH] = {};
        if (!GetTempFileNameW(directory.c_str(), L"jjf", 0, buffer)) throw Failure(win_error(L"无法创建临时文件"));
        path = buffer;
    }
    ~TempFile() { if (!path.empty()) DeleteFileW(path.c_str()); }
};

void write_file(const std::wstring& path, const std::vector<unsigned char>& data) {
    HANDLE file = CreateFileW(path.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) throw Failure(win_error(L"无法写入文件"));
    DWORD written = 0;
    bool ok = WriteFile(file, data.data(), static_cast<DWORD>(data.size()), &written, nullptr) && written == data.size();
    DWORD error = GetLastError();
    if (!CloseHandle(file)) { ok = false; error = GetLastError(); }
    if (!ok) { SetLastError(error); throw Failure(win_error(L"写入文件失败")); }
}

bool pdf_error(pdfio_file_t*, const char*, void*) { return false; }

class Client {
    InternetHandle session_;
    InternetHandle connection_;
public:
    Client() : session_(WinHttpOpen(agent, WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, WINHTTP_NO_PROXY_NAME,
                                     WINHTTP_NO_PROXY_BYPASS, 0)),
               connection_(session_.value ? WinHttpConnect(session_.value, host, INTERNET_DEFAULT_HTTPS_PORT, 0) : nullptr) {
        if (!session_.value || !connection_.value) throw Failure(win_error(L"无法初始化网络连接"));
        WinHttpSetTimeouts(session_.value, 15000, 15000, 30000, 30000);
        DWORD protocols = WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_2;
        if (!WinHttpSetOption(session_.value, WINHTTP_OPTION_SECURE_PROTOCOLS, &protocols, sizeof(protocols)))
            throw Failure(win_error(L"无法启用 TLS 1.2"));
    }
    std::vector<unsigned char> request(const wchar_t* method, const std::wstring& path,
                                       const std::wstring& headers, const std::string& body,
                                       size_t limit) {
        InternetHandle req(WinHttpOpenRequest(connection_.value, method, path.c_str(), nullptr,
                                              WINHTTP_NO_REFERER, WINHTTP_DEFAULT_ACCEPT_TYPES,
                                              WINHTTP_FLAG_SECURE));
        if (!req.value) throw Failure(win_error(L"无法创建网络请求"));
        // 不跟随跳转，避免将预览参数或请求头发送到其他站点。
        DWORD redirects = WINHTTP_OPTION_REDIRECT_POLICY_NEVER;
        if (!WinHttpSetOption(req.value, WINHTTP_OPTION_REDIRECT_POLICY, &redirects, sizeof(redirects)))
            throw Failure(win_error(L"无法设置网络请求"));
        if (!WinHttpSendRequest(req.value, headers.empty() ? WINHTTP_NO_ADDITIONAL_HEADERS : headers.c_str(),
                                headers.empty() ? 0 : static_cast<DWORD>(headers.size()),
                                body.empty() ? WINHTTP_NO_REQUEST_DATA : const_cast<char*>(body.data()),
                                static_cast<DWORD>(body.size()), static_cast<DWORD>(body.size()), 0) ||
            !WinHttpReceiveResponse(req.value, nullptr)) throw Failure(win_error(L"网络请求失败"));
        DWORD status = 0, status_len = sizeof(status);
        if (!WinHttpQueryHeaders(req.value, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                                 WINHTTP_HEADER_NAME_BY_INDEX, &status, &status_len, WINHTTP_NO_HEADER_INDEX))
            throw Failure(win_error(L"无法读取网站响应"));
        if (status == 429) throw Failure(L"???????HTTP 429????????");
        if (status < 200 || status >= 300) throw Failure(L"网站返回 HTTP " + std::to_wstring(status));
        std::vector<unsigned char> data;
        for (;;) {
            DWORD available = 0;
            if (!WinHttpQueryDataAvailable(req.value, &available)) throw Failure(win_error(L"读取响应失败"));
            if (!available) break;
            if (available > limit - data.size()) throw Failure(L"网站返回的数据超过大小限制。 ");
            size_t old_size = data.size(); data.resize(old_size + available);
            DWORD received = 0;
            if (!WinHttpReadData(req.value, data.data() + old_size, available, &received))
                throw Failure(win_error(L"读取响应失败"));
            if (!received) { data.resize(old_size); break; }
            data.resize(old_size + received);
        }
        return data;
    }
};

std::string script_value(const std::string& page, const std::string& name) {
    // 网站当前返回的 token 和 enc 是 Base64 字符串；格式变化时明确报错。
    std::regex pattern("\\bvar\\s+" + name + "\\s*=\\s*\"([A-Za-z0-9+/=]+)\"");
    std::smatch match;
    if (!std::regex_search(page, match, pattern)) throw Failure(L"预览页面缺少必要参数，网站可能已更新。 ");
    return match[1].str();
}

std::string text_of(const std::vector<unsigned char>& data) {
    return std::string(data.begin(), data.end());
}
} // namespace

std::wstring standard_number(const std::wstring& input) {
    std::wstring url = trim(input);
    for (wchar_t c : url) if (c < 0x20) throw Failure(L"链接中包含无效字符。 ");
    URL_COMPONENTS parts{}; parts.dwStructSize = sizeof(parts);
    parts.dwHostNameLength = parts.dwUrlPathLength = parts.dwExtraInfoLength = static_cast<DWORD>(-1);
    parts.dwUserNameLength = parts.dwPasswordLength = static_cast<DWORD>(-1);
    if (!WinHttpCrackUrl(url.c_str(), static_cast<DWORD>(url.size()), 0, &parts))
        throw Failure(L"请输入有效的规范详情页链接。 ");
    std::wstring hostname(parts.lpszHostName, parts.dwHostNameLength);
    std::transform(hostname.begin(), hostname.end(), hostname.begin(), towlower);
    if (parts.nScheme != INTERNET_SCHEME_HTTPS || hostname != host || parts.nPort != 443 ||
        parts.dwUserNameLength || parts.dwPasswordLength ||
        std::wstring(parts.lpszUrlPath, parts.dwUrlPathLength) != L"/resmea/standard/detail.html")
        throw Failure(L"请输入该网站的规范详情页 HTTPS 链接。 ");
    std::wstring query = parts.dwExtraInfoLength ? std::wstring(parts.lpszExtraInfo, parts.dwExtraInfoLength) : L"";
    size_t fragment = query.find(L'#'); if (fragment != std::wstring::npos) query.resize(fragment);
    std::wstring code; int matches = 0;
    size_t begin = !query.empty() && query[0] == L'?' ? 1 : 0;
    while (begin <= query.size()) {
        size_t end = query.find(L'&', begin);
        std::wstring param = query.substr(begin, end == std::wstring::npos ? end : end - begin);
        size_t eq = param.find(L'=');
        if (eq != std::wstring::npos && param.substr(0, eq) == L"standno") {
            const auto decoded = decode_parameter(param.substr(eq + 1));
            for (wchar_t c : decoded) if (c < 0x20) throw Failure(L"规范编号中包含无效字符。 ");
            code = trim(decoded); ++matches;
        }
        if (end == std::wstring::npos) break;
        begin = end + 1;
    }
    if (matches != 1 || code.empty() || code.size() > 100) throw Failure(L"链接中没有有效的规范编号。 ");
    for (wchar_t c : code)
        if (!((c >= L'A' && c <= L'Z') || (c >= L'a' && c <= L'z') ||
              (c >= L'0' && c <= L'9') || c == L'/' || c == L' ' || c == L'.' || c == L'_' || c == L'-'))
            throw Failure(L"规范编号中包含无效字符。 ");
    return code;
}

std::wstring standard_name(const std::string& page) {
    size_t start = page.find("<title");
    if (start == std::string::npos) return L"";
    start = page.find('>', start);
    if (start == std::string::npos) return L"";
    size_t end = page.find("</title>", ++start);
    if (end == std::string::npos) return L"";
    std::wstring title = trim(unescape_html(from_utf8(page.substr(start, end - start))));
    const std::wstring suffix = L" - 国家计量技术规范全文公开系统";
    if (title.size() < suffix.size() || title.compare(title.size() - suffix.size(), suffix.size(), suffix)) return L"";
    return trim(title.substr(0, title.size() - suffix.size()));
}

std::wstring default_filename(const std::wstring& code, const std::wstring& name) {
    std::wstring label = trim(code + (name.empty() ? L"" : L" " + name));
    for (wchar_t& c : label) if (c < 32 || std::wstring(L"\\/:*?\"<>|").find(c) != std::wstring::npos) c = L'_';
    if (label.size() > 180) {
        label.resize(180);
        // UTF-16 代理对不能截断在中间。
        if (label.back() >= 0xd800 && label.back() <= 0xdbff) label.pop_back();
    }
    while (!label.empty() && (label.back() == L' ' || label.back() == L'.')) label.pop_back();
    auto base = label.substr(0, label.find(L'.'));
    std::transform(base.begin(), base.end(), base.begin(), towupper);
    if (base == L"CON" || base == L"PRN" || base == L"AUX" || base == L"NUL" ||
        (base.size() == 4 && (base.compare(0, 3, L"COM") == 0 || base.compare(0, 3, L"LPT") == 0) && base[3] >= L'1' && base[3] <= L'9'))
        label.insert(0, L"_");
    return label + L".pdf";
}

size_t validate_pdf(const std::vector<unsigned char>& data) {
    if (data.size() < 5 || std::string(data.begin(), data.begin() + 5) != "%PDF-")
        throw Failure(L"网站没有返回有效的 PDF。 ");
    if (sha256(data) == error_pdf_hash) throw Failure(L"网站返回了在线阅读加载失败的提示文件。 ");
    wchar_t temp_dir[MAX_PATH] = {};
    DWORD len = GetTempPathW(MAX_PATH, temp_dir);
    if (!len || len >= MAX_PATH) throw Failure(win_error(L"无法获取临时目录"));
    TempFile temp(temp_dir);
    write_file(temp.path, data);
    std::string path = to_utf8(temp.path);
    pdfio_file_t* pdf = pdfioFileOpen(path.c_str(), nullptr, nullptr, pdf_error, nullptr);
    if (!pdf) throw Failure(L"PDF 无法解析或已加密。 ");
    // PDFio 能打开的空密码 PDF 可以直接检查，不必因为有加密标记而拒绝。
    size_t pages = pdfioFileGetNumPages(pdf);
    pdfioFileClose(pdf);
    if (!pages) throw Failure(L"PDF 没有页面。 ");
    return pages;
}

Download download_pdf(const std::wstring& url) {
    Download out; out.code = standard_number(url);
    const std::string encoded = url_encode(to_utf8(out.code));
    const std::wstring detail_path = L"/resmea/standard/detail.html?standno=" + from_utf8(encoded);
    try {
        Client client;
        auto page = client.request(L"GET", detail_path, L"Referer: https://jjg.spc.org.cn/\r\n", "", 1024 * 1024);
        out.name = standard_name(text_of(page));
    } catch (const Failure& error) {
        if (error.message.find(L"429") != std::wstring::npos) throw;
        /* 名称不可用时仍允许用户手动填写。 */
    }
    std::wstring last_error;
    for (int attempt = 0; attempt < 2; ++attempt) {
        try {
            Client client;
            std::string body = "a100=" + encoded + "&ismobile=1";
            std::wstring headers = L"Content-Type: application/x-www-form-urlencoded\r\nReferer: https://jjg.spc.org.cn" + detail_path + L"\r\n";
            auto preview = client.request(L"POST", L"/resmea/view/stdonline", headers, body, 1024 * 1024);
            std::string page = text_of(preview);
            std::string token = script_value(page, "token"), enc = script_value(page, "enc");
            std::wstring path = L"/resmea/view/onlinereading?token=" + from_utf8(url_encode(token));
            headers = L"Referer: https://jjg.spc.org.cn/resmea/view/stdonline\r\nmyfoxit: " + from_utf8(enc) + L"\r\nX-Requested-With: XMLHttpRequest\r\nAccept: */*\r\n";
            out.data = client.request(L"GET", path, headers, "", max_pdf_bytes);
            out.pages = validate_pdf(out.data);
            return out;
        } catch (const Failure& e) { if (e.message.find(L"429") != std::wstring::npos) throw; last_error = e.message; }
    }
    throw Failure(L"获取失败：" + last_error);
}

void save_atomic(const std::wstring& path, const std::vector<unsigned char>& data) {
    size_t separator = path.find_last_of(L"\\/");
    if (separator == std::wstring::npos) throw Failure(L"请选择有效的保存路径。 ");
    TempFile temp(path.substr(0, separator + 1));
    write_file(temp.path, data);
    if (!MoveFileExW(temp.path.c_str(), path.c_str(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH))
        throw Failure(win_error(L"保存文件失败"));
}

} // namespace toolbox::pdf
