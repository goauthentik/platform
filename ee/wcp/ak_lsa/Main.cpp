#include "PrepareToken.hpp"
#include "PrepareProfile.hpp"
#include "Utils.hpp"

// exported symbols
#pragma comment(linker, "/export:SpLsaModeInitialize")

LSA_SECPKG_FUNCTION_TABLE FunctionTable;


NTSTATUS NTAPI SpInitialize(_In_ ULONG_PTR PackageId, _In_ SECPKG_PARAMETERS* Parameters, _In_ LSA_SECPKG_FUNCTION_TABLE* functionTable) {
    LogMessage("SpInitialize");

    LogMessage("  PackageId: %u", PackageId);
    LogMessage("  Version: %u", Parameters->Version);
    {
        ULONG state = Parameters->MachineState;
        LogMessage("  MachineState:");
        if (state & SECPKG_STATE_ENCRYPTION_PERMITTED) {
            state &= ~SECPKG_STATE_ENCRYPTION_PERMITTED;
            LogMessage("  - ENCRYPTION_PERMITTED");
        }
        if (state & SECPKG_STATE_STRONG_ENCRYPTION_PERMITTED) {
            state &= ~SECPKG_STATE_STRONG_ENCRYPTION_PERMITTED;
            LogMessage("  - STRONG_ENCRYPTION_PERMITTED");
        }
        if (state & SECPKG_STATE_DOMAIN_CONTROLLER) {
            state &= ~SECPKG_STATE_DOMAIN_CONTROLLER;
            LogMessage("  - DOMAIN_CONTROLLER");
        }
        if (state & SECPKG_STATE_WORKSTATION) {
            state &= ~SECPKG_STATE_WORKSTATION;
            LogMessage("  - WORKSTATION");
        }
        if (state & SECPKG_STATE_STANDALONE) {
            state &= ~SECPKG_STATE_STANDALONE;
            LogMessage("  - STANDALONE");
        }
        if (state) {
            // print resudual flags not already covered
            LogMessage("  * Unknown flags: 0x%X", state);
        }
    }
    LogMessage("  SetupMode: %u", Parameters->SetupMode);
    // parameters not logged
    Parameters->DomainSid;
    Parameters->DomainName;
    Parameters->DnsDomainName;
    Parameters->DomainGuid;

    FunctionTable = *functionTable; // copy function pointer table

    LogMessage("  return STATUS_SUCCESS");
    return STATUS_SUCCESS;
}

NTSTATUS NTAPI SpShutDown() {
    LogMessage("SpShutDown");
    LogMessage("  return STATUS_SUCCESS");
    return STATUS_SUCCESS;
}

NTSTATUS NTAPI SpGetInfo(_Out_ SecPkgInfoW* PackageInfo) {
    LogMessage("SpGetInfo");

    // return security package metadata
    PackageInfo->fCapabilities = SECPKG_FLAG_LOGON //  supports LsaLogonUser
                               | SECPKG_FLAG_CLIENT_ONLY; // no server auth support
    PackageInfo->wVersion = SECURITY_SUPPORT_PROVIDER_INTERFACE_VERSION;
    PackageInfo->wRPCID = SECPKG_ID_NONE; // no DCE/RPC support
    PackageInfo->cbMaxToken = 0;
    PackageInfo->Name = (wchar_t*)L"ak_lsa";
    PackageInfo->Comment = (wchar_t*)L"authentik Token Authentication";

    LogMessage("  return STATUS_SUCCESS");
    return STATUS_SUCCESS;
}


/* Authenticate a user logon attempt.
   Returns STATUS_SUCCESS if the login attempt succeeded. */
NTSTATUS LsaApLogonUser (
    _In_ PLSA_CLIENT_REQUEST ClientRequest,
    _In_ SECURITY_LOGON_TYPE LogonType,
    _In_reads_bytes_(SubmitBufferSize) VOID* ProtocolSubmitBuffer,
    _In_ VOID* ClientBufferBase,
    _In_ ULONG SubmitBufferSize,
    _Outptr_result_bytebuffer_(*ProfileBufferSize) VOID** ProfileBuffer,
    _Out_ ULONG* ProfileBufferSize,
    _Out_ LUID* LogonId,
    _Out_ NTSTATUS* SubStatus,
    _Out_ LSA_TOKEN_INFORMATION_TYPE* TokenInformationType,
    _Outptr_ VOID** TokenInformation,
    _Out_ LSA_UNICODE_STRING** AccountName,
    _Out_ LSA_UNICODE_STRING** AuthenticatingAuthority
) {
    LogMessage("LsaApLogonUser");

    {
        // clear output arguments first in case of failure
        *ProfileBuffer = nullptr;
        *ProfileBufferSize = 0;
        *LogonId = {};
        *SubStatus = 0;
        *TokenInformationType = {};
        *TokenInformation = nullptr;
        *AccountName = nullptr;
        if (AuthenticatingAuthority)
            *AuthenticatingAuthority = nullptr;
    }

    // input arguments
    LogMessage("  LogonType: %i", LogonType); // Interactive=2, RemoteInteractive=10
    ClientBufferBase;
    LogMessage("  ProtocolSubmitBuffer size: %i", SubmitBufferSize);

    // deliberately restrict supported logontypes
    if ((LogonType != Interactive) && (LogonType != RemoteInteractive)) {
        LogMessage("  return STATUS_NOT_IMPLEMENTED (unsupported LogonType)");
        return STATUS_NOT_IMPLEMENTED;
    }

    // authentication credentials passed by client
    auto* logonInfo = (MSV1_0_INTERACTIVE_LOGON*)ProtocolSubmitBuffer;
    {
        if (SubmitBufferSize < sizeof(MSV1_0_INTERACTIVE_LOGON)) {
            LogMessage("  ERROR: SubmitBufferSize too small");
            return STATUS_INVALID_PARAMETER;
        }

        // make relative pointers absolute to ease later access
        logonInfo->LogonDomainName.Buffer = (wchar_t*)((BYTE*)logonInfo + (size_t)logonInfo->LogonDomainName.Buffer);
        logonInfo->UserName.Buffer = (wchar_t*)((BYTE*)logonInfo + (size_t)logonInfo->UserName.Buffer);
        logonInfo->Password.Buffer = (wchar_t*)((BYTE*)logonInfo + (size_t)logonInfo->Password.Buffer);
    }

    // assign output arguments

    {
        wchar_t computerName[MAX_COMPUTERNAME_LENGTH + 1] = {};
        DWORD computerNameSize = ARRAYSIZE(computerName);
        if (!GetComputerNameW(computerName, &computerNameSize)) {
            LogMessage("  return STATUS_INTERNAL_ERROR (GetComputerNameW failed)");
            return STATUS_INTERNAL_ERROR;
        }

        // assign "ProfileBuffer" output argument
        *ProfileBufferSize = GetProfileBufferSize(computerName, *logonInfo);
        FunctionTable.AllocateClientBuffer(ClientRequest, *ProfileBufferSize, ProfileBuffer); // will update *ProfileBuffer

        std::vector<BYTE> profileBuffer = PrepareProfileBuffer(computerName, *logonInfo, (BYTE*)*ProfileBuffer);
        FunctionTable.CopyToClientBuffer(ClientRequest, (ULONG)profileBuffer.size(), *ProfileBuffer, profileBuffer.data()); // copy to caller process
    }

    {
        // assign "LogonId" output argument
        if (!AllocateLocallyUniqueId(LogonId)) {
            LogMessage("  ERROR: AllocateLocallyUniqueId failed");
            return STATUS_FAIL_FAST_EXCEPTION;
        }
        NTSTATUS status = FunctionTable.CreateLogonSession(LogonId);
        if (status != STATUS_SUCCESS) {
            LogMessage("  ERROR: CreateLogonSession failed with err: 0x%x", status);
            return status;
        }

        LogMessage("  LogonId: High=0x%x , Low=0x%x", LogonId->HighPart, LogonId->LowPart);
    }

    *SubStatus = STATUS_SUCCESS; // reason for error

    {
        // Assign "TokenInformation" output argument
        LSA_TOKEN_INFORMATION_V2* tokenInfo = nullptr;
        NTSTATUS subStatus = 0;
        NTSTATUS status = UserNameToToken(&logonInfo->UserName, &tokenInfo, &subStatus);
        if (status != STATUS_SUCCESS) {
            LogMessage("ERROR: UserNameToToken failed with err: 0x%x", status);
            *SubStatus = subStatus;
            return status;
        }

        *TokenInformationType = LsaTokenInformationV1;
        *TokenInformation = tokenInfo;
    }

    {
        // assign "AccountName" output argument
        LogMessage("  AccountName: %ls", ToWstring(logonInfo->UserName).c_str());
        *AccountName = CreateLsaUnicodeString(logonInfo->UserName.Buffer, logonInfo->UserName.Length); // mandatory
    }

    if (AuthenticatingAuthority) {
        // assign "AuthenticatingAuthority" output argument
        *AuthenticatingAuthority = (LSA_UNICODE_STRING*)FunctionTable.AllocateLsaHeap(sizeof(LSA_UNICODE_STRING));

        if (logonInfo->LogonDomainName.Length > 0) {
            LogMessage("  AuthenticatingAuthority: %ls", ToWstring(logonInfo->LogonDomainName).c_str());
            *AuthenticatingAuthority = CreateLsaUnicodeString(logonInfo->LogonDomainName.Buffer, logonInfo->LogonDomainName.Length);
        } else {
            LogMessage("  AuthenticatingAuthority: <empty>");
            **AuthenticatingAuthority = {
                .Length = 0,
                .MaximumLength = 0,
                .Buffer = nullptr,
            };
        }
    }

    LogMessage("  return STATUS_SUCCESS");
    return STATUS_SUCCESS;
}

void LsaApLogonTerminated(_In_ LUID* LogonId) {
    LogMessage("LsaApLogonTerminated");
    LogMessage("  LogonId: High=0x%x , Low=0x%x", LogonId->HighPart, LogonId->LowPart);
    LogMessage("  return");
}

SECPKG_FUNCTION_TABLE SecurityPackageFunctionTable = {
    .InitializePackage = nullptr,
    .LogonUser = LsaApLogonUser,
    .CallPackage = nullptr,
    .LogonTerminated = LsaApLogonTerminated,
    .CallPackageUntrusted = nullptr,
    .CallPackagePassthrough = nullptr,
    .LogonUserEx = nullptr,
    .LogonUserEx2 = nullptr,
    .Initialize = SpInitialize,
    .Shutdown = SpShutDown,
    .GetInfo = SpGetInfo,
    .AcceptCredentials = nullptr,
    .AcquireCredentialsHandle = nullptr,
    .QueryCredentialsAttributes = nullptr,
    .FreeCredentialsHandle = nullptr,
    .SaveCredentials = nullptr,
    .GetCredentials = nullptr,
    .DeleteCredentials = nullptr,
    .InitLsaModeContext = nullptr,
    .AcceptLsaModeContext = nullptr,
    .DeleteContext = nullptr,
    .ApplyControlToken = nullptr,
    .GetUserInfo = nullptr,
    .GetExtendedInformation = nullptr,
    .QueryContextAttributes = nullptr,
    .AddCredentialsW = nullptr,
    .SetExtendedInformation = nullptr,
    .SetContextAttributes = nullptr,
    .SetCredentialsAttributes = nullptr,
    .ChangeAccountPassword = nullptr,
    .QueryMetaData = nullptr,
    .ExchangeMetaData = nullptr,
    .GetCredUIContext = nullptr,
    .UpdateCredentials = nullptr,
    .ValidateTargetInfo = nullptr,
    .PostLogonUser = nullptr,
    .GetRemoteCredGuardLogonBuffer = nullptr,
    .GetRemoteCredGuardSupplementalCreds = nullptr,
    .GetTbalSupplementalCreds = nullptr,
    .LogonUserEx3 = nullptr,
    .PreLogonUserSurrogate = nullptr,
    .PostLogonUserSurrogate = nullptr,
    .ExtractTargetInfo = nullptr,
};

/** LSA calls SpLsaModeInitialize() when loading SSP/AP DLLs. */
extern "C"
NTSTATUS NTAPI SpLsaModeInitialize(
    _In_ ULONG LsaVersion,
    _Out_ ULONG* PackageVersion,
    _Out_ SECPKG_FUNCTION_TABLE** ppTables,
    _Out_ ULONG* pcTables
) {
    LogMessage("SpLsaModeInitialize");
    LogMessage("  LsaVersion %u", LsaVersion);

    *PackageVersion = SECPKG_INTERFACE_VERSION;
    *ppTables = &SecurityPackageFunctionTable;
    *pcTables = 1;

    LogMessage("  return STATUS_SUCCESS");
    return STATUS_SUCCESS;
}
