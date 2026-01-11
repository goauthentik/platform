#include <string>
#include <locale>
#include <codecvt>

std::wstring utf8_decode(const std::string& str);
std::string utf8_encode(const std::wstring& str);
