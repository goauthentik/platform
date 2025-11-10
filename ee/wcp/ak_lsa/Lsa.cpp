#include "Lsa.hpp"
#include <intrin.h>
#include "spdlog/spdlog.h"

#include <windows.h>
#include <ntsecapi.h>
#include <subauth.h>
#include <lm.h>
#include "include/LogonData.h"
#include "include/ak.h"
#include "include/ustring.h"
#include "authentik_sys_bridge/ffi.h"

// Global variables
static PLSA_DISPATCH_TABLE g_LsaDispatchTable = NULL;
static ULONG g_AuthenticationPackageId = 0;

const char* PACKAGE_NAME = "ak_lsa";

extern "C" void * SpAlloc(size_t s)
{
	return (*g_LsaDispatchTable->AllocateLsaHeap)(static_cast<ULONG>(s));
}

extern "C" void SpFree(void * p)
{
	(*g_LsaDispatchTable->FreeLsaHeap)(p);
}

// Initialize the authentication package
extern "C" NTSTATUS NTAPI LsaApInitializePackage(
    _In_ ULONG AuthenticationPackageId,
    _In_ PLSA_DISPATCH_TABLE LsaDispatchTable,
    _In_opt_ PLSA_STRING Database,
    _In_opt_ PLSA_STRING Confidentiality,
    _Out_ PLSA_STRING *AuthenticationPackageName
) {
    SPDLOG_DEBUG(std::string("LsaApInitializePackage: ").append(std::to_string(AuthenticationPackageId)).c_str());
    PLSA_STRING packageName;
    SIZE_T nameLength = strlen(PACKAGE_NAME);

    // Store the dispatch table and package ID
    g_LsaDispatchTable = LsaDispatchTable;
    g_AuthenticationPackageId = AuthenticationPackageId;

    // Allocate memory for package name
    packageName = (PLSA_STRING)g_LsaDispatchTable->AllocateLsaHeap(sizeof(LSA_STRING));
    if (!packageName) {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    // Set up package name
    packageName->Buffer = (PCHAR)g_LsaDispatchTable->AllocateLsaHeap(nameLength + 1);
    if (!packageName->Buffer) {
        g_LsaDispatchTable->FreeLsaHeap(packageName);
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    CopyMemory(packageName->Buffer, PACKAGE_NAME, nameLength);
    packageName->Buffer[nameLength] = '\0';
    packageName->Length = (USHORT)nameLength;
    packageName->MaximumLength = (USHORT)(nameLength + 1);

    *AuthenticationPackageName = packageName;

    return STATUS_SUCCESS;
}

extern "C" NTSTATUS NTAPI LsaApLogonUserEx2(
    _In_ PLSA_CLIENT_REQUEST ClientRequest,
    _In_ SECURITY_LOGON_TYPE LogonType,
    _In_ PVOID ProtocolSubmitBuffer,
    _In_ PVOID ClientBufferBase,
    _In_ ULONG SubmitBufferSize,
    _Out_ PVOID *ProfileBuffer,
    _Out_ PULONG ProfileBufferSize,
    _Out_ PLUID LogonId,
    _Out_ PNTSTATUS SubStatus,
    _Out_ PLSA_TOKEN_INFORMATION_TYPE TokenInformationType,
    _Out_ PVOID *TokenInformation,
    _Out_ PUNICODE_STRING *AccountName,
    _Out_ PUNICODE_STRING *AuthenticatingAuthority,
    _Out_ PUNICODE_STRING *MachineName,
    _Out_ PSECPKG_PRIMARY_CRED PrimaryCredentials,
    _Out_ PSECPKG_SUPPLEMENTAL_CRED_ARRAY *SupplementalCredentials
) {
    SPDLOG_DEBUG("LsaApLogonUserEx2");
    PCUSTOM_LOGON_DATA logonData;
    PMSV1_0_INTERACTIVE_PROFILE profile;
    PLSA_TOKEN_INFORMATION_V1 tokenInfo;
    NTSTATUS status;

    try {
        std::string ping = std::string("");
        ak_sys_grpc_ping(ping);
        SPDLOG_DEBUG(std::string("sysd version: ").append(ping).c_str());
    } catch (const std::exception &ex) {
        SPDLOG_DEBUG("Exception in ak_grpc_ping");
        SPDLOG_DEBUG(ex.what());
    }

    *SubStatus = STATUS_SUCCESS;

    SPDLOG_DEBUG("validate");
    // Validate input parameters
    if (!ProtocolSubmitBuffer ||
        SubmitBufferSize < sizeof(CUSTOM_LOGON_DATA)) {
        return STATUS_INVALID_PARAMETER;
    }

    logonData = (PCUSTOM_LOGON_DATA)ProtocolSubmitBuffer;

    // SPDLOG_DEBUG("validate password");
    // Perform custom password validation
    // if (!ValidateCustomPassword(logonData->Domain,
    //                            logonData->UserName,
    //                            logonData->Password)) {
    //     *SubStatus = STATUS_LOGON_FAILURE;
    //     return STATUS_LOGON_FAILURE;
    // }

    SPDLOG_DEBUG("profile buffer");
    // Create profile buffer
    profile = (PMSV1_0_INTERACTIVE_PROFILE)g_LsaDispatchTable->AllocateLsaHeap(
        sizeof(MSV1_0_INTERACTIVE_PROFILE)
    );
    if (!profile) {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    SPDLOG_DEBUG("prepare profile");
    ZeroMemory(profile, sizeof(MSV1_0_INTERACTIVE_PROFILE));
    profile->MessageType = MsV1_0InteractiveProfile;

    *ProfileBuffer = profile;
    *ProfileBufferSize = sizeof(MSV1_0_INTERACTIVE_PROFILE);

    SPDLOG_DEBUG("create token info");
    // Create token information
    tokenInfo = (PLSA_TOKEN_INFORMATION_V1)g_LsaDispatchTable->AllocateLsaHeap(
        sizeof(LSA_TOKEN_INFORMATION_V1)
    );
    if (!tokenInfo) {
        g_LsaDispatchTable->FreeLsaHeap(profile);
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    SPDLOG_DEBUG("prepare token info");
    ZeroMemory(tokenInfo, sizeof(LSA_TOKEN_INFORMATION_V1));
    tokenInfo->ExpirationTime.QuadPart = 0x7FFFFFFFFFFFFFFF; // Never expire

    *TokenInformationType = LsaTokenInformationV1;
    *TokenInformation = tokenInfo;

    SPDLOG_DEBUG("set account name");
    // Set account name - corrected allocation
    ustring account_name(L"Administrator");
    account_name.set_allocater(SpAlloc, SpFree);
    *AccountName = account_name.to_unicode_string();

    // SPDLOG_DEBUG("set authority");
    // // Set authenticating authority - corrected allocation
    // ustring authority (L"WORKGROUP");
    // account_name.set_allocater(SpAlloc, SpFree);
    // *AuthenticatingAuthority = authority.to_unicode_string();

    // SPDLOG_DEBUG("set machine name");
    // // Set machine name (optional)
    // machineName = (PUNICODE_STRING)g_LsaDispatchTable->AllocateLsaHeap(
    //     sizeof(UNICODE_STRING)
    // );
    // if (machineName) {
    //     CreateUnicodeStringFromWideString(machineName, L"CUSTOM-AUTH-MACHINE",
    //                                      g_LsaDispatchTable);
    //     *MachineName = machineName;
    // } else {
    //     *MachineName = NULL;
    // }

    SPDLOG_DEBUG("LogonId");
    {
        int64u luid;
        luid.ui64 = __rdtsc();
        //GetSystemTimePreciseAsFileTime(&luid.ft);
        *LogonId = luid.id;
	}

    return STATUS_SUCCESS;
}

// Called by the Local Security Authority (LSA) when a logon application with a trusted connection to the LSA calls the LsaCallAuthenticationPackage function and specifies the authentication package's identifier.
extern "C" NTSTATUS NTAPI LsaApCallPackage(
    _In_ PLSA_CLIENT_REQUEST ClientRequest,
    _In_ PVOID ProtocolSubmitBuffer,
    _In_ PVOID ClientBufferBase,
    _In_ ULONG SubmitBufferLength,
    _Out_ PVOID* ProtocolReturnBuffer,
    _Out_ PULONG ReturnBufferLength,
    _Out_ PNTSTATUS ProtocolStatus
) {
    // Debug("LsaApCallPackage");
    // *ProtocolReturnBuffer = NULL;
    // *ReturnBufferLength = 0;
    // *ProtocolStatus = STATUS_SUCCESS;
    return STATUS_NOT_IMPLEMENTED;
}

// Called by the Local Security Authority (LSA) when an application with an untrusted connection to the LSA calls the LsaCallAuthenticationPackage function and specifies the authentication package's identifier.
extern "C" NTSTATUS NTAPI LsaApCallPackageUntrusted(
    _In_ PLSA_CLIENT_REQUEST ClientRequest,
    _In_ PVOID ProtocolSubmitBuffer,
    _In_ PVOID ClientBufferBase,
    _In_ ULONG SubmitBufferLength,
    _Out_ PVOID* ProtocolReturnBuffer,
    _Out_ PULONG ReturnBufferLength,
    _Out_ PNTSTATUS ProtocolStatus
) {
    // Debug("LsaApCallPackageUntrusted");
    return LsaApCallPackage(ClientRequest, ProtocolSubmitBuffer,
                           ClientBufferBase, SubmitBufferLength,
                           ProtocolReturnBuffer, ReturnBufferLength,
                           ProtocolStatus);
}

// Used to notify an authentication package when a logon session terminates. A logon session terminates when the last token referencing the logon session is deleted.
VOID NTAPI LsaApLogonTerminated(_In_ PLUID LogonId) {
    SPDLOG_DEBUG("LsaApLogonTerminated");
}

// The dispatch function for pass-through logon requests sent to the LsaCallAuthenticationPackage function.
extern "C" NTSTATUS NTAPI LsaApCallPackagePassthrough(
  _In_ PLSA_CLIENT_REQUEST ClientRequest,
  _In_ PVOID ProtocolAuthenticationInformation,
  _In_ PVOID ClientBufferBase,
  _In_ ULONG AuthenticationInformationLength,
  _Out_ PVOID* ProtocolReturnBuffer,
  _Out_ PULONG ReturnBufferLength,
  _Out_ PNTSTATUS ProtocolStatus
)
{
    SPDLOG_DEBUG("LsaApCallPackagePassthrough");
	return STATUS_NOT_IMPLEMENTED;
}
