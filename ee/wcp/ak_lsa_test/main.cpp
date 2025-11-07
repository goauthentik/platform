#include "pch.h"
#include <windows.h>
#include <ntsecapi.h>
#include <iostream>
#include <SubAuth.h>
#include <tchar.h>
#include <string>
#include <ak_lsa/LogonData.h>

using std::to_wstring;

// Helper function to initialize LSA_STRING from char string
void InitLsaString(PLSA_STRING LsaString, LPSTR String)
{
    DWORD StringLength;

    if (String == NULL) {
        LsaString->Buffer = NULL;
        LsaString->Length = 0;
        LsaString->MaximumLength = 0;
        return;
    }

    StringLength = lstrlenA(String);
    LsaString->Buffer = String;
    LsaString->Length = (USHORT)StringLength;
    LsaString->MaximumLength = (USHORT)(StringLength + 1);
}
HRESULT UnicodeStringInitWithString(_In_ PWSTR pwz, _Out_ UNICODE_STRING *pus) {
  HRESULT hr;
  if (pwz) {
    size_t lenString = wcslen(pwz);
    USHORT usCharCount;
    hr = SizeTToUShort(lenString, &usCharCount);
    if (SUCCEEDED(hr)) {
      USHORT usSize;
      hr = SizeTToUShort(sizeof(wchar_t), &usSize);
      if (SUCCEEDED(hr)) {
        hr = UShortMult(
            usCharCount, usSize,
            &(pus->Length)); // Explicitly NOT including NULL terminator
        if (SUCCEEDED(hr)) {
          pus->MaximumLength = pus->Length;
          pus->Buffer = pwz;
          hr = S_OK;
        } else {
          hr = HRESULT_FROM_WIN32(ERROR_ARITHMETIC_OVERFLOW);
        }
      }
    }
  } else {
    hr = E_INVALIDARG;
  }
  return hr;
}

void Debug(std::wstring data) {
    MessageBoxW(0,data.c_str(),0,0);
}

int APIENTRY _tWinMain(HINSTANCE hInstance,
                       HINSTANCE hPrevInstance,
                       LPTSTR    lpCmdLine,
                       int       nCmdShow) {
    UNREFERENCED_PARAMETER(hPrevInstance);
    UNREFERENCED_PARAMETER(lpCmdLine);
    HANDLE lsaHandle = NULL;
    LSA_STRING packageName;
    ULONG packageId;
    NTSTATUS status;
    NTSTATUS substatus;
    LSA_OPERATIONAL_MODE securityMode;

    PCUSTOM_LOGON_DATA logonInfo;
    ULONG logonInfoSize;

    TOKEN_SOURCE sourceContext;
    PVOID profileBuffer = NULL;
    ULONG profileBufferLength;
    LUID logonId;
    HANDLE tokenHandle;
    QUOTA_LIMITS quotas;

    // Initialize LSA connection
    status = LsaRegisterLogonProcess(&packageName, &lsaHandle, &securityMode);
    if (!NT_SUCCESS(status)) {
        Debug(L"Failed to register with LSA: " +to_wstring((long) status));

        // Try untrusted connection
        status = LsaConnectUntrusted(&lsaHandle);
        if (!NT_SUCCESS(status)) {
            Debug(L"Failed to connect to LSA: " + to_wstring((long) status));
            return 1;
        }
    }

    // Look up authentication package
    InitLsaString(&packageName, "ak_lsa");
    status = LsaLookupAuthenticationPackage(lsaHandle, &packageName, &packageId);
    if (!NT_SUCCESS(status)) {
        Debug(L"Failed to lookup package: " + to_wstring((long) status));
        LsaDeregisterLogonProcess(lsaHandle);
        return 1;
    }

    wchar_t wcharDomain[11] = L"testdomain";
    PWSTR pwstrDomain = wcharDomain;
    wchar_t wcharUser[11] = L"testuser";
    PWSTR pwstrUser = wcharUser;
    wchar_t wcharPassword[18] = L"custompassword123";
    PWSTR pwstrPassword = wcharPassword;

    // Prepare logon information (simplified example)
    logonInfoSize = sizeof(MSV1_0_INTERACTIVE_LOGON) +
        (wcslen(pwstrDomain) * sizeof(WCHAR)) +
        (wcslen(pwstrUser) * sizeof(WCHAR)) +
        (wcslen(pwstrPassword) * sizeof(WCHAR)) +
        3 * sizeof(WCHAR); // null terminators

    logonInfo = (PCUSTOM_LOGON_DATA)LocalAlloc(LMEM_ZEROINIT, logonInfoSize);
    if (!logonInfo) {
        Debug(L"Failed to allocate logon info");
        LsaDeregisterLogonProcess(lsaHandle);
        return 1;
    }

    HRESULT hr = UnicodeStringInitWithString(pwstrDomain, logonInfo->Domain);
    if (!SUCCEEDED(hr)) {
        Debug(L"Failed to set domain");
        LsaDeregisterLogonProcess(lsaHandle);
        return 1;
    }
    hr = UnicodeStringInitWithString(pwstrUser, logonInfo->UserName);
    if (!SUCCEEDED(hr)) {
        Debug(L"Failed to set user");
        LsaDeregisterLogonProcess(lsaHandle);
        return 1;
    }
    hr = UnicodeStringInitWithString(pwstrPassword, logonInfo->Password);
    if (!SUCCEEDED(hr)) {
        Debug(L"Failed to set password");
        LsaDeregisterLogonProcess(lsaHandle);
        return 1;
    }

    // logonInfo->MessageType = MsV1_0InteractiveLogon;

    // Set up source context
    lstrcpynA(sourceContext.SourceName, "CustomApp", sizeof(sourceContext.SourceName));
    AllocateLocallyUniqueId(&sourceContext.SourceIdentifier);

    // Perform logon (this is simplified - you'd need to properly format your custom data)
    status = LsaLogonUser(
        lsaHandle,
        &packageName,
        Interactive,
        packageId,
        logonInfo,
        logonInfoSize,
        NULL, // No additional groups
        &sourceContext,
        &profileBuffer,
        &profileBufferLength,
        &logonId,
        &tokenHandle,
        &quotas,
        &substatus
    );

    if (NT_SUCCESS(status)) {
        Debug(L"Custom authentication successful!");
        CloseHandle(tokenHandle);
    } else {
        Debug(L"Authentication failed: " + to_wstring((long)status));
        Debug(L"Sub status: " + to_wstring((long)substatus));
        //  + to_wstring((long) status) <<
        //               L" (substatus: " << std::hex << substatus << L")");
    }

    // Cleanup
    if (profileBuffer) {
        LsaFreeReturnBuffer(profileBuffer);
    }
    if (logonInfo) {
        LocalFree(logonInfo);
    }
    LsaDeregisterLogonProcess(lsaHandle);

    return 0;
}
