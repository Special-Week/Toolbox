#pragma once

#include <exception>
#include <string>
#include <vector>
#include <utility>

namespace toolbox::pdf {

struct Failure : std::exception {
    explicit Failure(std::wstring message) : message(std::move(message)) {}
    std::wstring message;
};

struct Download {
    std::wstring code;
    std::wstring name;
    std::vector<unsigned char> data;
    size_t pages = 0;
};

std::wstring standard_number(const std::wstring& url);
std::wstring standard_name(const std::string& html);
std::wstring default_filename(const std::wstring& code, const std::wstring& name);
size_t validate_pdf(const std::vector<unsigned char>& data);
Download download_pdf(const std::wstring& url);
void save_atomic(const std::wstring& path, const std::vector<unsigned char>& data);

} // namespace toolbox::pdf
