#pragma once
#include <cassert>
#include <fstream>

extern LSA_SECPKG_FUNCTION_TABLE FunctionTable;


inline void LogMessage(const char* message, ...) {
    // append to log file
    FILE* file = nullptr;
    fopen_s(&file, "C:\\ak_lsa.txt", "a+");
    if (!file)
        return;
    {
        // print variadic message
        va_list args;
        va_start(args, message);
        _vfprintf_l(file, message, NULL, args);
        va_end(args);
    }
    fprintf(file, "\n");
    fclose(file);
}

/** Allocate and create a new LSA_STRING object.
    Assumes that "FunctionTable" is initialized. */
inline LSA_STRING* CreateLsaString(const std::string& msg) {
    auto msg_len = (USHORT)msg.size(); // exclude null-termination

    assert(FunctionTable.AllocateLsaHeap);
    auto* obj = (LSA_STRING*)FunctionTable.AllocateLsaHeap(sizeof(LSA_STRING));
    obj->Buffer = (char*)FunctionTable.AllocateLsaHeap(msg_len);
    memcpy(/*dst*/obj->Buffer, /*src*/msg.c_str(), msg_len);
    obj->Length = msg_len;
    obj->MaximumLength = msg_len;
    return obj;
}

/** Allocate and create a new LSA_UNICODE_STRING object.
    Assumes that "FunctionTable" is initialized. */
inline LSA_UNICODE_STRING* CreateLsaUnicodeString(const wchar_t* msg, USHORT msg_len_bytes) {
    assert(FunctionTable.AllocateLsaHeap);
    auto* obj = (LSA_UNICODE_STRING*)FunctionTable.AllocateLsaHeap(sizeof(LSA_UNICODE_STRING));
    obj->Buffer = (wchar_t*)FunctionTable.AllocateLsaHeap(msg_len_bytes);
    memcpy(/*dst*/obj->Buffer, /*src*/msg, msg_len_bytes);
    obj->Length = msg_len_bytes;
    obj->MaximumLength = msg_len_bytes;
    return obj;
}

inline LSA_UNICODE_STRING* CreateLsaUnicodeString(const std::wstring& msg) {
    return CreateLsaUnicodeString(msg.c_str(), (USHORT)msg.size()*sizeof(wchar_t));
}

inline std::wstring ToWstring(LSA_UNICODE_STRING& lsa_str) {
    if (lsa_str.Length == 0)
        return L"<empty>";
    return std::wstring(lsa_str.Buffer, lsa_str.Length/2);
}

inline void AssignLsaUnicodeString(const LSA_UNICODE_STRING& source, LSA_UNICODE_STRING& dest) {
    assert(FunctionTable.AllocateLsaHeap);
    if (dest.Buffer)
        FunctionTable.FreeLsaHeap(dest.Buffer);

    dest.Buffer = (wchar_t*)FunctionTable.AllocateLsaHeap(source.Length);
    memcpy(/*dst*/dest.Buffer, /*src*/source.Buffer, source.Length);
    dest.Length = source.Length;
    dest.MaximumLength = source.Length;
}
