#include "pch.h"
#include <windows.h>
#include <ntsecapi.h>
#include <iostream>
#include <SubAuth.h>
#include <tchar.h>
#include <string>
#include <ak_lsa/include/LogonData.h>

#include <Windows.h>
#include <string>
#include <mutex>
#include <fstream>
#include <iostream>

using std::string;
using std::ofstream;

void LOG(const char* data) {
    std::fstream fs("c:\\ak_lsa.txt", std::ios_base::app);

    if(fs) {
        fs << data << std::endl;
        fs.flush();
        fs.close();
    }
}

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
}// CustomAuthClient.cpp - Updated for CUSTOM_LOGON_DATA with pointers

// Helper function to pack strings into a buffer with pointers
NTSTATUS PackCustomLogonData(
    _In_ LPCWSTR domain,
    _In_ LPCWSTR username,
    _In_ LPCWSTR password,
    _In_ ULONG logonType,
    _Out_ PVOID* Buffer,
    _Out_ PULONG BufferSize
)
{
    ULONG domainLen = domain ? (lstrlenW(domain) * sizeof(WCHAR)) : 0;
    ULONG usernameLen = username ? (lstrlenW(username) * sizeof(WCHAR)) : 0;
    ULONG passwordLen = password ? (lstrlenW(password) * sizeof(WCHAR)) : 0;

    ULONG totalSize = sizeof(CUSTOM_LOGON_DATA) +
                     sizeof(UNICODE_STRING) * 3 +  // Three UNICODE_STRING structures
                     domainLen + sizeof(WCHAR) +    // Domain string + null terminator
                     usernameLen + sizeof(WCHAR) +  // Username string + null terminator
                     passwordLen + sizeof(WCHAR);   // Password string + null terminator

    PBYTE buffer = (PBYTE)LocalAlloc(LMEM_ZEROINIT, totalSize);
    if (!buffer) {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    PCUSTOM_LOGON_DATA logonData = (PCUSTOM_LOGON_DATA)buffer;
    PUNICODE_STRING domainStr = (PUNICODE_STRING)(buffer + sizeof(CUSTOM_LOGON_DATA));
    PUNICODE_STRING usernameStr = domainStr + 1;
    PUNICODE_STRING passwordStr = usernameStr + 1;
    PWSTR stringData = (PWSTR)(passwordStr + 1);

    // Set up the main structure
    logonData->Domain = domainStr;
    logonData->UserName = usernameStr;
    logonData->Password = passwordStr;
    logonData->LogonType = logonType;

    // Pack domain string
    if (domain && domainLen > 0) {
        domainStr->Buffer = stringData;
        domainStr->Length = (USHORT)domainLen;
        domainStr->MaximumLength = (USHORT)(domainLen + sizeof(WCHAR));
        CopyMemory(stringData, domain, domainLen);
        stringData = (PWSTR)((PBYTE)stringData + domainLen + sizeof(WCHAR));
    } else {
        domainStr->Buffer = NULL;
        domainStr->Length = 0;
        domainStr->MaximumLength = 0;
    }

    // Pack username string
    if (username && usernameLen > 0) {
        usernameStr->Buffer = stringData;
        usernameStr->Length = (USHORT)usernameLen;
        usernameStr->MaximumLength = (USHORT)(usernameLen + sizeof(WCHAR));
        CopyMemory(stringData, username, usernameLen);
        stringData = (PWSTR)((PBYTE)stringData + usernameLen + sizeof(WCHAR));
    } else {
        usernameStr->Buffer = NULL;
        usernameStr->Length = 0;
        usernameStr->MaximumLength = 0;
    }

    // Pack password string
    if (password && passwordLen > 0) {
        passwordStr->Buffer = stringData;
        passwordStr->Length = (USHORT)passwordLen;
        passwordStr->MaximumLength = (USHORT)(passwordLen + sizeof(WCHAR));
        CopyMemory(stringData, password, passwordLen);
    } else {
        passwordStr->Buffer = NULL;
        passwordStr->Length = 0;
        passwordStr->MaximumLength = 0;
    }

    *Buffer = buffer;
    *BufferSize = totalSize;

    return STATUS_SUCCESS;
}

void CreateConsole() {
    bool attached = AttachConsole(ATTACH_PARENT_PROCESS) != 0;

    // if failed create a new console
    if(!attached) {
        attached = AllocConsole() != 0;
    }

    if (!attached) {
        // Add some error handling here.
        // You can call GetLastError() to get more info about the error.
        return;
    }

    // std::cout, std::clog, std::cerr, std::cin
    FILE* fDummy;
    freopen_s(&fDummy, "CONOUT$", "w", stdout);
    freopen_s(&fDummy, "CONOUT$", "w", stderr);
    freopen_s(&fDummy, "CONIN$", "r", stdin);
    std::cout.clear();
    std::clog.clear();
    std::cerr.clear();
    std::cin.clear();

    // std::wcout, std::wclog, std::wcerr, std::wcin
    HANDLE hConOut = CreateFile(_T("CONOUT$"), GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    HANDLE hConIn = CreateFile(_T("CONIN$"), GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    SetStdHandle(STD_OUTPUT_HANDLE, hConOut);
    SetStdHandle(STD_ERROR_HANDLE, hConOut);
    SetStdHandle(STD_INPUT_HANDLE, hConIn);
    std::wcout.clear();
    std::wclog.clear();
    std::wcerr.clear();
    std::wcin.clear();
}


int APIENTRY _tWinMain(HINSTANCE hInstance,
                       HINSTANCE hPrevInstance,
                       LPTSTR    lpCmdLine,
                       int       nCmdShow) {
    UNREFERENCED_PARAMETER(hPrevInstance);
    UNREFERENCED_PARAMETER(lpCmdLine);

    CreateConsole();

    HANDLE lsaHandle = NULL;
    LSA_STRING packageName;
    ULONG packageId;
    NTSTATUS status;
    NTSTATUS substatus;
    LSA_OPERATIONAL_MODE securityMode;
    PVOID profileBuffer = NULL;
    ULONG profileBufferLength;

    TOKEN_SOURCE sourceContext;
    PVOID logonBuffer = NULL;
    ULONG logonBufferSize;
    LUID logonId;
    HANDLE tokenHandle;
    QUOTA_LIMITS quotas;

    // Initialize LSA connection
    status = LsaRegisterLogonProcess(&packageName, &lsaHandle, &securityMode);
    if (!NT_SUCCESS(status)) {
        std::wcout << L"Failed to connect to LSA (trusted): 0x" << std::hex << status << std::endl;

        // Try untrusted connection
        status = LsaConnectUntrusted(&lsaHandle);
        if (!NT_SUCCESS(status)) {
            std::wcout << L"Failed to connect to LSA: 0x" << std::hex << status << std::endl;
            return 1;
        }
        std::wcout << L"Connected to LSA: 0x" << std::hex << status << std::endl;
    }

    // Look up authentication package
    InitLsaString(&packageName, "ak_lsa");
    status = LsaLookupAuthenticationPackage(lsaHandle, &packageName, &packageId);
    if (!NT_SUCCESS(status)) {
        std::wcout << L"Failed to lookup package: 0x" << std::hex << status << std::endl;
        LsaDeregisterLogonProcess(lsaHandle);
        return 1;
    }
    std::wcout << L"Found authentication package with ID: " << packageId << std::endl;

    wchar_t wcharDomain[11] = L"testdomain";
    PWSTR pwstrDomain = wcharDomain;
    wchar_t wcharUser[11] = L"testuser";
    PWSTR pwstrUser = wcharUser;
    wchar_t wcharPassword[18] = L"custompassword123";
    PWSTR pwstrPassword = wcharPassword;

    // Prepare custom logon information
    status = PackCustomLogonData(
        L"MYDOMAIN",           // Domain
        L"testuser",           // Username
        L"custompassword123",  // Password (contains "custom" so will be accepted)
        Interactive,           // Logon type
        &logonBuffer,
        &logonBufferSize
    );

    if (!NT_SUCCESS(status)) {
        std::wcout << L"Failed to pack logon data: 0x" << std::hex << status << std::endl;
        LsaDeregisterLogonProcess(lsaHandle);
        return 1;
    }

    // logonInfo->MessageType = MsV1_0InteractiveLogon;

    // Set up source context
    lstrcpynA(sourceContext.SourceName, "CustomApp", sizeof(sourceContext.SourceName));
    AllocateLocallyUniqueId(&sourceContext.SourceIdentifier);

    LSA_STRING origin;
    InitLsaString(&origin, (LPSTR)"lsatest");

    // Perform logon (this is simplified - you'd need to properly format your custom data)
    status = LsaLogonUser(
        lsaHandle,
        &origin,
        Interactive,
        packageId,
        logonBuffer,
        logonBufferSize,
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
        std::wcout << L"Custom authentication successful!" << std::endl;
        std::wcout << L"Profile buffer size: " << profileBufferLength << L" bytes" << std::endl;

        // You can cast profileBuffer to PCUSTOM_PROFILE_DATA to access the data
        if (profileBuffer && profileBufferLength >= sizeof(CUSTOM_PROFILE_DATA)) {
            PCUSTOM_PROFILE_DATA profile = (PCUSTOM_PROFILE_DATA)profileBuffer;
            std::wcout << L"Message Type: " << profile->MessageType << std::endl;
            std::wcout << L"User Flags: 0x" << std::hex << profile->UserFlags << std::endl;
        }

        CloseHandle(tokenHandle);
    } else {
        std::wcout << L"Authentication failed: 0x" << std::hex << status <<
                      L" (substatus: 0x" << std::hex << substatus << L")" << std::endl;
    }

    // Cleanup
    if (profileBuffer) {
        LsaFreeReturnBuffer(profileBuffer);
    }
    if (logonBuffer) {
        LocalFree(logonBuffer);
    }
    LsaDeregisterLogonProcess(lsaHandle);
    // _getch();
    return 0;
}
