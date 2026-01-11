#include <string>
#include <locale>
#include <codecvt>

std::wstring utf8_decode(const std::string& str) {
  std::wstring_convert<std::codecvt_utf8<wchar_t>> myconv;
  return myconv.from_bytes(str);
}

std::string utf8_encode(const std::wstring& str) {
  std::wstring_convert<std::codecvt_utf8<wchar_t>> myconv;
  return myconv.to_bytes(str);
}
