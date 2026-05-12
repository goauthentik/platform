#pragma once
#include <cassert>
#include <fstream>
#include <wincred.h>
#include "authentik_sys_bridge/ffi.h"
#include "rust/cxx.h"
#include "ak_common/include/ak_log.h"
#include "spdlog/spdlog.h"

extern LSA_SECPKG_FUNCTION_TABLE FunctionTable;

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

  spdlog::debug("  decryptPassword: Fixing pointers...");
  PWSTR pszCredentials = (PWSTR)FunctionTable.AllocateLsaHeap(Length + sizeof(WCHAR));
  memcpy(pszCredentials, pkil->Password.Buffer, pkil->Password.MaximumLength);

  spdlog::debug("  decryptPassword: Checking if password is encrypted...");
  if (!CredIsProtectedW(pszCredentials, &ProtectionType)) {
    spdlog::debug("  decryptPassword: Password is not encrypted");
    return pszCredentials;
  }

  ULONG cchToken = 0;
  PWSTR pszToken = 0;
  ULONG cchCredentials = Length / sizeof(WCHAR);

  HRESULT status;

  if (ProtectionType != CredUnprotected) {
    spdlog::debug("  decryptPassword: Password is protected");
    while (true) {
      spdlog::debug("  decryptPassword: CredUnprotectW call");
      if (CredUnprotectW(FALSE, pszCredentials, cchCredentials, pszToken, &cchToken)) {
        break;
      }
      if (pszToken) {
        break;
      }
      auto err = GetLastError();
      if (err == ERROR_INSUFFICIENT_BUFFER) {
        spdlog::debug("  decryptPassword: ERROR_INSUFFICIENT_BUFFER, {}", cchToken);
        pszToken = (PWSTR)FunctionTable.AllocatePrivateHeap(cchToken * sizeof(WCHAR));
        // pszToken = (PWSTR)alloca(cchToken * sizeof(WCHAR));
      }
    }
  } else {
    spdlog::debug("  decryptPassword: PW was not encrypted");
    pszToken = pszCredentials;
    cchToken = cchCredentials;
  }
  return pszToken;
}

inline bool ValidateToken(MSV1_0_INTERACTIVE_LOGON* pkil) {
  try {
    spdlog::debug("  ak_sys_auth_token_validate: Decrypting password");
    std::wstring pwW = decryptPassword(pkil);
    std::string pw = utf8_encode(pwW);
    TokenResponse validatedToken;
    spdlog::debug("  ak_sys_auth_token_validate: {:d}, {}", pw.length(), pw);
    if (ak_sys_auth_token_validate(pw, validatedToken)) {
      spdlog::debug("  ak_sys_auth_token_validate Succeeded");
      return true;
    }
  } catch (const rust::Error& ex) {
    spdlog::debug("  ak_sys_auth_token_validate Error: {}", ex.what());
    return false;
  }
  return false;
}
