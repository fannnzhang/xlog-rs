#include <string>
#include <vector>

#include "mars/comm/strutil.h"

namespace strutil {

bool StartsWith(const std::string& str, const std::string& substr) {
    if (substr.size() > str.size()) {
        return false;
    }
    return str.compare(0, substr.size(), substr) == 0;
}

bool EndsWith(const std::string& str, const std::string& substr) {
    if (substr.size() > str.size()) {
        return false;
    }
    return str.compare(str.size() - substr.size(), substr.size(), substr) == 0;
}

bool StartsWith(const std::wstring& str, const std::wstring& substr) {
    if (substr.size() > str.size()) {
        return false;
    }
    return str.compare(0, substr.size(), substr) == 0;
}

bool EndsWith(const std::wstring& str, const std::wstring& substr) {
    if (substr.size() > str.size()) {
        return false;
    }
    return str.compare(str.size() - substr.size(), substr.size(), substr) == 0;
}

std::vector<std::string>& SplitToken(const std::string& str,
                                     const std::string& delimiters,
                                     std::vector<std::string>& out) {
    out.clear();
    std::string::size_type last = 0;
    auto pos = str.find_first_of(delimiters, last);
    while (pos != std::string::npos) {
        if (pos > last) {
            out.push_back(str.substr(last, pos - last));
        }
        last = pos + 1;
        pos = str.find_first_of(delimiters, last);
    }
    if (last < str.size()) {
        out.push_back(str.substr(last));
    }
    return out;
}

std::vector<std::wstring>& SplitToken(const std::wstring& str,
                                      const std::wstring& delimiters,
                                      std::vector<std::wstring>& out) {
    out.clear();
    std::wstring::size_type last = 0;
    auto pos = str.find_first_of(delimiters, last);
    while (pos != std::wstring::npos) {
        if (pos > last) {
            out.push_back(str.substr(last, pos - last));
        }
        last = pos + 1;
        pos = str.find_first_of(delimiters, last);
    }
    if (last < str.size()) {
        out.push_back(str.substr(last));
    }
    return out;
}

}  // namespace strutil
