#include "utf.h"

#include <cstdint>

using namespace std;
using namespace tex;

string tex::wide2utf8(const std::wstring& src) {
  const wchar_t* in = src.c_str();
  string out;
  unsigned int codepoint = 0;
  for (; *in != 0; ++in) {
    if (*in >= 0xd800 && *in <= 0xdbff) {
      codepoint = ((*in - 0xd800) << 10) + 0x10000;
    } else {
      if (*in >= 0xdc00 && *in <= 0xdfff) {
        codepoint |= *in - 0xdc00;
      } else {
        codepoint = *in;
      }

      if (codepoint <= 0x7f) {
        out.append(1, static_cast<char>(codepoint));
      } else if (codepoint <= 0x7ff) {
        out.append(1, static_cast<char>(0xc0 | ((codepoint >> 6) & 0x1f)));
        out.append(1, static_cast<char>(0x80 | (codepoint & 0x3f)));
      } else if (codepoint <= 0xffff) {
        out.append(1, static_cast<char>(0xe0 | ((codepoint >> 12) & 0x0f)));
        out.append(1, static_cast<char>(0x80 | ((codepoint >> 6) & 0x3f)));
        out.append(1, static_cast<char>(0x80 | (codepoint & 0x3f)));
      } else {
        out.append(1, static_cast<char>(0xf0 | ((codepoint >> 18) & 0x07)));
        out.append(1, static_cast<char>(0x80 | ((codepoint >> 12) & 0x3f)));
        out.append(1, static_cast<char>(0x80 | ((codepoint >> 6) & 0x3f)));
        out.append(1, static_cast<char>(0x80 | (codepoint & 0x3f)));
      }
      codepoint = 0;
    }
  }
  return out;
}

wstring tex::utf82wide(const std::string& src) {
  wstring out;
  auto appendCodePoint = [&out](std::uint32_t codepoint) {
    if constexpr (sizeof(wchar_t) == 2) {
      if (codepoint > 0xffff) {
        codepoint -= 0x10000;
        out.push_back(static_cast<wchar_t>(0xd800 + (codepoint >> 10)));
        out.push_back(static_cast<wchar_t>(0xdc00 + (codepoint & 0x03ff)));
      } else {
        out.push_back(static_cast<wchar_t>(codepoint));
      }
    } else {
      out.push_back(static_cast<wchar_t>(codepoint));
    }
  };

  for (size_t i = 0; i < src.size();) {
    const auto first = static_cast<unsigned char>(src[i]);
    std::uint32_t codepoint;
    std::uint32_t minimum;
    size_t length;

    if (first <= 0x7f) {
      codepoint = first;
      minimum = 0;
      length = 1;
    } else if (first >= 0xc2 && first <= 0xdf) {
      codepoint = first & 0x1f;
      minimum = 0x80;
      length = 2;
    } else if (first >= 0xe0 && first <= 0xef) {
      codepoint = first & 0x0f;
      minimum = 0x800;
      length = 3;
    } else if (first >= 0xf0 && first <= 0xf4) {
      codepoint = first & 0x07;
      minimum = 0x10000;
      length = 4;
    } else {
      appendCodePoint(0xfffd);
      ++i;
      continue;
    }

    bool valid = i + length <= src.size();
    for (size_t j = 1; valid && j < length; ++j) {
      const auto continuation = static_cast<unsigned char>(src[i + j]);
      if ((continuation & 0xc0) != 0x80) {
        valid = false;
      } else {
        codepoint = (codepoint << 6) | (continuation & 0x3f);
      }
    }

    if (!valid || codepoint < minimum || codepoint > 0x10ffff ||
        (codepoint >= 0xd800 && codepoint <= 0xdfff)) {
      appendCodePoint(0xfffd);
      ++i;
      continue;
    }

    appendCodePoint(codepoint);
    i += length;
  }
  return out;
}
