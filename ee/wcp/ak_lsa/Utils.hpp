#pragma once
#include <cassert>
#include <fstream>
#include <wincred.h>
#include "authentik_sys_bridge/ffi.h"
#include "rust/cxx.h"

extern LSA_SECPKG_FUNCTION_TABLE FunctionTable;

inline void LogMessage(const char* message, ...) {
  // append to log file
  FILE* file = nullptr;
  fopen_s(&file, "C:\\ak_lsa.txt", "a+");
  if (!file) return;
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
  auto msg_len = (USHORT)msg.size();  // exclude null-termination

  assert(FunctionTable.AllocateLsaHeap);
  auto* obj = (LSA_STRING*)FunctionTable.AllocateLsaHeap(sizeof(LSA_STRING));
  obj->Buffer = (char*)FunctionTable.AllocateLsaHeap(msg_len);
  memcpy(/*dst*/ obj->Buffer, /*src*/ msg.c_str(), msg_len);
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
  memcpy(/*dst*/ obj->Buffer, /*src*/ msg, msg_len_bytes);
  obj->Length = msg_len_bytes;
  obj->MaximumLength = msg_len_bytes;
  return obj;
}

inline LSA_UNICODE_STRING* CreateLsaUnicodeString(const std::wstring& msg) {
  return CreateLsaUnicodeString(msg.c_str(), (USHORT)msg.size() * sizeof(wchar_t));
}

inline std::wstring ToWstring(LSA_UNICODE_STRING& lsa_str) {
  if (lsa_str.Length == 0) return L"<empty>";
  return std::wstring(lsa_str.Buffer, lsa_str.Length / 2);
}

inline void AssignLsaUnicodeString(const LSA_UNICODE_STRING& source, LSA_UNICODE_STRING& dest) {
  assert(FunctionTable.AllocateLsaHeap);
  if (dest.Buffer) FunctionTable.FreeLsaHeap(dest.Buffer);

  dest.Buffer = (wchar_t*)FunctionTable.AllocateLsaHeap(source.Length);
  memcpy(/*dst*/ dest.Buffer, /*src*/ source.Buffer, source.Length);
  dest.Length = source.Length;
  dest.MaximumLength = source.Length;
}

// Convert a wide Unicode string to an UTF8 string
inline std::string utf8_encode(const std::wstring& wstr) {
  if (wstr.empty()) return std::string();
  int size_needed =
      WideCharToMultiByte(CP_UTF8, 0, &wstr[0], (int)wstr.size(), NULL, 0, NULL, NULL);
  std::string strTo(size_needed, 0);
  WideCharToMultiByte(CP_UTF8, 0, &wstr[0], (int)wstr.size(), &strTo[0], size_needed, NULL, NULL);
  return strTo;
}

inline PWSTR decryptPassword(MSV1_0_INTERACTIVE_LOGON* pkil) {
  CRED_PROTECTION_TYPE ProtectionType;
  ULONG Length = pkil->Password.Length;

  LogMessage("  decryptPassword: Fixing pointers...");
  PWSTR pszCredentials = (PWSTR)FunctionTable.AllocateLsaHeap(Length + sizeof(WCHAR));
  memcpy(pszCredentials, pkil->Password.Buffer, pkil->Password.MaximumLength);

  LogMessage("  decryptPassword: Checking if password is encrypted...");
  if (!CredIsProtectedW(pszCredentials, &ProtectionType)) {
    LogMessage("  decryptPassword: Password is not encrypted");
    return pszCredentials;
  }

  ULONG cchPin = 0;
  PWSTR pszPin = 0;
  ULONG cchCredentials = Length / sizeof(WCHAR);

  HRESULT status;

  if (ProtectionType != CredUnprotected) {
    LogMessage("  decryptPassword: Password is protected");
    while (true) {
      LogMessage("  decryptPassword: CredUnprotectW call");
      if (CredUnprotectW(FALSE, pszCredentials, cchCredentials, pszPin, &cchPin)) {
        break;
      }
      if (pszPin) {
        break;
      }
      auto err = GetLastError();
      if (err == ERROR_INSUFFICIENT_BUFFER) {
        LogMessage("  decryptPassword: ERROR_INSUFFICIENT_BUFFER, %d", cchPin);
        // pszPin = (PWSTR)FunctionTable.AllocatePrivateHeap(cchPin * sizeof(WCHAR));
        pszPin = (PWSTR)alloca(cchPin * sizeof(WCHAR));
      }
    }
  } else {
    LogMessage("  decryptPassword: PW was not encrypted");
    pszPin = pszCredentials;
    cchPin = cchCredentials;
  }
  LogMessage("  decryptPassword: result: %ls", pszPin);
  return pszPin;
}

inline bool ValidateToken(MSV1_0_INTERACTIVE_LOGON* pkil) {
  try {
    LogMessage("  ak_sys_auth_token_validate: Decrypting password");
    auto pw = decryptPassword(pkil);
    TokenResponse validatedToken;
    LogMessage("  ak_sys_auth_token_validate Token: '%ls'", pw);
    if (ak_sys_auth_token_validate(utf8_encode(pw), validatedToken)) {
      LogMessage("  ak_sys_auth_token_validate Succeeded");
      return true;
    }
  } catch (const rust::Error& ex) {
    LogMessage("  ak_sys_auth_token_validate Error: %s", ex.what());
    return false;
  }
  return false;
}
