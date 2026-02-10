#include "PrepareToken.hpp"
#include <Lm.h>
#include "Utils.hpp"
#include "spdlog/spdlog.h"

#pragma comment(lib, "Netapi32.lib")

static bool NameToSid(const wchar_t* username, PSID* userSid) {
  DWORD lengthSid = 0;
  SID_NAME_USE Use = {};
  DWORD referencedDomainNameLen = 0;
  BOOL res = LookupAccountNameW(nullptr, username, nullptr, &lengthSid, nullptr,
                                &referencedDomainNameLen, &Use);

  *userSid = (PSID)FunctionTable.AllocateLsaHeap(lengthSid);
  wchar_t* referencedDomainName = (wchar_t*)FunctionTable.AllocateLsaHeap(
      sizeof(wchar_t) * referencedDomainNameLen);  // throwaway string
  res = LookupAccountNameW(nullptr, username, *userSid, &lengthSid, referencedDomainName,
                           &referencedDomainNameLen, &Use);
  if (!res) {
    DWORD err = GetLastError();
    spdlog::debug("  LookupAccountNameW failed (err %u)", err);
    return false;
  }

  FunctionTable.FreeLsaHeap(referencedDomainName);
  return true;
}

static void GetPrimaryGroupSidFromUserSid(PSID userSID, PSID* primaryGroupSID) {
  // duplicate the user sid
  *primaryGroupSID = (PSID)FunctionTable.AllocateLsaHeap(GetLengthSid(userSID));
  CopySid(GetLengthSid(userSID), *primaryGroupSID, userSID);

  // replace the last subauthority by DOMAIN_GROUP_RID_USERS
  // https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/understand-security-identifiers
  // (last SubAuthority = RID
  // https://learn.microsoft.com/nb-no/windows/win32/secauthz/well-known-sids
  UCHAR SubAuthorityCount = *GetSidSubAuthorityCount(*primaryGroupSID);
  *GetSidSubAuthority(*primaryGroupSID, SubAuthorityCount - 1) = DOMAIN_GROUP_RID_USERS;
}

static bool GetGroups(const wchar_t* UserName, GROUP_USERS_INFO_1** lpGroupInfo,
                      DWORD* pTotalEntries) {
  DWORD NumberOfEntries = 0;
  DWORD status = NetUserGetGroups(NULL, UserName, 1, (BYTE**)lpGroupInfo, MAX_PREFERRED_LENGTH,
                                  &NumberOfEntries, pTotalEntries);
  if (status != NERR_Success) {
    spdlog::debug("ERROR: NetUserGetGroups failed with error %u", status);
    return false;
  }
  return true;
}

static bool GetLocalGroups(const wchar_t* UserName, GROUP_USERS_INFO_0** lpGroupInfo,
                           DWORD* pTotalEntries) {
  DWORD NumberOfEntries = 0;
  DWORD status = NetUserGetLocalGroups(NULL, UserName, 0, 0, (BYTE**)lpGroupInfo,
                                       MAX_PREFERRED_LENGTH, &NumberOfEntries, pTotalEntries);
  if (status != NERR_Success) {
    spdlog::debug("ERROR: NetUserGetLocalGroups failed with error %u", status);
    return false;
  }
  return true;
}

NTSTATUS UserNameToToken(__in LSA_UNICODE_STRING* AccountName,
                         __out LSA_TOKEN_INFORMATION_V1** Token, __out PNTSTATUS SubStatus) {
  const LARGE_INTEGER Forever{
      .LowPart = 0xFFFFFFFF,   // unsigned
      .HighPart = 0x7FFFFFFF,  // signed
  };

  // convert username to zero-terminated string
  std::wstring username = ToWstring(*AccountName);

  auto* token =
      (LSA_TOKEN_INFORMATION_V1*)FunctionTable.AllocateLsaHeap(sizeof(LSA_TOKEN_INFORMATION_V1));

  token->ExpirationTime = Forever;

  PSID userSid = nullptr;
  const wchar_t* user = username.c_str();
  {
    // configure "User"
    if (!NameToSid(username.c_str(), &userSid)) return STATUS_FAIL_FAST_EXCEPTION;
    spdlog::debug("  User.User: {}", utf8_encode(username));
    token->User.User = {
        .Sid = userSid,
        .Attributes = 0,
    };
  }

  {
    // configure "Groups"
    DWORD NumberOfGroups = 0;
    GROUP_USERS_INFO_1* pGroupInfo = nullptr;
    if (!GetGroups(user, &pGroupInfo, &NumberOfGroups)) {
      return STATUS_FAIL_FAST_EXCEPTION;
    }
    spdlog::debug("  NumberOfGroups: %u", NumberOfGroups);

    DWORD NumberOfLocalGroups = 0;
    GROUP_USERS_INFO_0* pLocalGroupInfo = nullptr;
    if (!GetLocalGroups(user, &pLocalGroupInfo, &NumberOfLocalGroups)) {
      return STATUS_FAIL_FAST_EXCEPTION;
    }
    spdlog::debug("  NumberOfLocalGroups: %u", NumberOfLocalGroups);

    TOKEN_GROUPS* tokenGroups = (TOKEN_GROUPS*)FunctionTable.AllocateLsaHeap(
        FIELD_OFFSET(TOKEN_GROUPS, Groups[NumberOfGroups + NumberOfLocalGroups]));
    tokenGroups->GroupCount = NumberOfGroups + NumberOfLocalGroups;
    for (size_t i = 0; i < NumberOfGroups; i++) {
      NameToSid(pGroupInfo[i].grui1_name, &tokenGroups->Groups[i].Sid);

      tokenGroups->Groups[i].Attributes = pGroupInfo[i].grui1_attributes;
    }
    for (size_t i = 0; i < NumberOfLocalGroups; i++) {
      NameToSid(pLocalGroupInfo[i].grui0_name, &tokenGroups->Groups[NumberOfGroups + i].Sid);

      // get the attributes of group since pLocalGroupInfo doesn't contain attributes
      if (*GetSidSubAuthority(tokenGroups->Groups[NumberOfGroups + i].Sid, 0) !=
          SECURITY_BUILTIN_DOMAIN_RID)
        tokenGroups->Groups[NumberOfGroups + i].Attributes =
            SE_GROUP_ENABLED | SE_GROUP_ENABLED_BY_DEFAULT;
      else
        tokenGroups->Groups[NumberOfGroups + i].Attributes = 0;
    }

    token->Groups = tokenGroups;
  }

  GetPrimaryGroupSidFromUserSid(userSid, &token->PrimaryGroup.PrimaryGroup);

  // TOKEN_PRIVILEGES Privileges not currently configured
  token->Privileges = nullptr;

  // PSID Owner not currently configured
  token->Owner.Owner = (PSID) nullptr;

  // PACL DefaultDacl not currently configured
  token->DefaultDacl.DefaultDacl = nullptr;

  // assign outputs
  *Token = token;
  *SubStatus = STATUS_SUCCESS;
  return STATUS_SUCCESS;
}
