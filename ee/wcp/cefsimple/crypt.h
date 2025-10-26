// Class for cryptographic support functions

#include <string>

bool Hash_SHA256(const std::string& strData, std::string& strHash);
size_t GetRandomInt(const size_t nExclusiveUpperBound);
std::string GetRandomStr(const size_t nLength);
std::wstring GetRandomWStr(const size_t nLength);
