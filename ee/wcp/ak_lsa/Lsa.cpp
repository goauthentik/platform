#include "Lsa.hpp"
#include <intrin.h>
#include "include/Debug.h"

#include <windows.h>
#include <ntsecapi.h>
#include <subauth.h>
#include <lm.h>
#include "include/LogonData.h"
#include "include/ak.h"

// Global variables
static PLSA_DISPATCH_TABLE g_LsaDispatchTable = NULL;
static ULONG g_AuthenticationPackageId = 0;

const char* PACKAGE_NAME = "ak_lsa";

// Helper function to allocate and copy a Unicode string in LSA heap
NTSTATUS AllocateAndCopyUnicodeString(
    PUNICODE_STRING Destination,
    PUNICODE_STRING Source,
    PLSA_DISPATCH_TABLE LsaDispatchTable
)
{
    if (!Source || !Source->Buffer || Source->Length == 0) {
        Destination->Buffer = NULL;
        Destination->Length = 0;
        Destination->MaximumLength = 0;
        return STATUS_SUCCESS;
    }

    Destination->MaximumLength = Source->Length + sizeof(WCHAR);
    Destination->Buffer = (PWSTR)LsaDispatchTable->AllocateLsaHeap(
        Destination->MaximumLength
    );

    if (!Destination->Buffer) {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    CopyMemory(Destination->Buffer, Source->Buffer, Source->Length);
    Destination->Buffer[Source->Length / sizeof(WCHAR)] = L'\0';
    Destination->Length = Source->Length;

    return STATUS_SUCCESS;
}

// Helper function to create a Unicode string from a wide string
NTSTATUS CreateUnicodeStringFromWideString(
    PUNICODE_STRING Destination,
    PCWSTR Source,
    PLSA_DISPATCH_TABLE LsaDispatchTable
)
{
    SIZE_T length;

    if (!Source) {
        Destination->Buffer = NULL;
        Destination->Length = 0;
        Destination->MaximumLength = 0;
        return STATUS_SUCCESS;
    }

    length = wcslen(Source) * sizeof(WCHAR);
    if (length > USHRT_MAX - sizeof(WCHAR)) {
        return STATUS_INVALID_PARAMETER;
    }

    Destination->Length = (USHORT)length;
    Destination->MaximumLength = Destination->Length + sizeof(WCHAR);
    Destination->Buffer = (PWSTR)LsaDispatchTable->AllocateLsaHeap(
        Destination->MaximumLength
    );

    if (!Destination->Buffer) {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    CopyMemory(Destination->Buffer, Source, Destination->Length);
    Destination->Buffer[Destination->Length / sizeof(WCHAR)] = L'\0';

    return STATUS_SUCCESS;
}

// Initialize the authentication package
NTSTATUS NTAPI LsaApInitializePackage(
    _In_ ULONG AuthenticationPackageId,
    _In_ PLSA_DISPATCH_TABLE LsaDispatchTable,
    _In_opt_ PLSA_STRING Database,
    _In_opt_ PLSA_STRING Confidentiality,
    _Out_ PLSA_STRING *AuthenticationPackageName
) {
    // Debug("LsaApInitializePackage: " + AuthenticationPackageId);
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

// Main logon function - corrected version
NTSTATUS NTAPI LsaApLogonUserEx2(
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
    // Debug("LsaApLogonUserEx2");
    PCUSTOM_LOGON_DATA logonData;
    PMSV1_0_INTERACTIVE_PROFILE profile;
    PLSA_TOKEN_INFORMATION_V1 tokenInfo;
    PUNICODE_STRING accountName;
    PUNICODE_STRING authAuthority;
    PUNICODE_STRING machineName;
    NTSTATUS status;

    *SubStatus = STATUS_SUCCESS;

    // Validate input parameters
    if (!ProtocolSubmitBuffer ||
        SubmitBufferSize < sizeof(CUSTOM_LOGON_DATA)) {
        return STATUS_INVALID_PARAMETER;
    }

    logonData = (PCUSTOM_LOGON_DATA)ProtocolSubmitBuffer;

    // Perform custom password validation
    if (!ValidateCustomPassword(logonData->Domain,
                               logonData->UserName,
                               logonData->Password)) {
        *SubStatus = STATUS_LOGON_FAILURE;
        return STATUS_LOGON_FAILURE;
    }

    // Create profile buffer
    profile = (PMSV1_0_INTERACTIVE_PROFILE)g_LsaDispatchTable->AllocateLsaHeap(
        sizeof(MSV1_0_INTERACTIVE_PROFILE)
    );
    if (!profile) {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    ZeroMemory(profile, sizeof(MSV1_0_INTERACTIVE_PROFILE));
    profile->MessageType = MsV1_0InteractiveProfile;

    *ProfileBuffer = profile;
    *ProfileBufferSize = sizeof(MSV1_0_INTERACTIVE_PROFILE);

    // Create token information
    tokenInfo = (PLSA_TOKEN_INFORMATION_V1)g_LsaDispatchTable->AllocateLsaHeap(
        sizeof(LSA_TOKEN_INFORMATION_V1)
    );
    if (!tokenInfo) {
        g_LsaDispatchTable->FreeLsaHeap(profile);
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    ZeroMemory(tokenInfo, sizeof(LSA_TOKEN_INFORMATION_V1));
    tokenInfo->ExpirationTime.QuadPart = 0x7FFFFFFFFFFFFFFF; // Never expire

    *TokenInformationType = LsaTokenInformationV1;
    *TokenInformation = tokenInfo;

    // Set account name - corrected allocation
    accountName = (PUNICODE_STRING)g_LsaDispatchTable->AllocateLsaHeap(
        sizeof(UNICODE_STRING)
    );
    if (!accountName) {
        status = STATUS_INSUFFICIENT_RESOURCES;
        goto cleanup;
    }

    status = AllocateAndCopyUnicodeString(accountName, logonData->UserName,
                                         g_LsaDispatchTable);
    if (!NT_SUCCESS(status)) {
        goto cleanup;
    }
    *AccountName = accountName;

    // Set authenticating authority - corrected allocation
    authAuthority = (PUNICODE_STRING)g_LsaDispatchTable->AllocateLsaHeap(
        sizeof(UNICODE_STRING)
    );
    if (!authAuthority) {
        status = STATUS_INSUFFICIENT_RESOURCES;
        goto cleanup;
    }

    status = CreateUnicodeStringFromWideString(authAuthority, L"ak_lsa",
                                              g_LsaDispatchTable);
    if (!NT_SUCCESS(status)) {
        goto cleanup;
    }
    *AuthenticatingAuthority = authAuthority;

    // Set machine name (optional)
    machineName = (PUNICODE_STRING)g_LsaDispatchTable->AllocateLsaHeap(
        sizeof(UNICODE_STRING)
    );
    if (machineName) {
        CreateUnicodeStringFromWideString(machineName, L"CUSTOM-AUTH-MACHINE",
                                         g_LsaDispatchTable);
        *MachineName = machineName;
    } else {
        *MachineName = NULL;
    }

    return STATUS_SUCCESS;

cleanup:
    // Cleanup on failure
    if (profile) {
        g_LsaDispatchTable->FreeLsaHeap(profile);
    }
    if (tokenInfo) {
        g_LsaDispatchTable->FreeLsaHeap(tokenInfo);
    }
    if (accountName) {
        if (accountName->Buffer) {
            g_LsaDispatchTable->FreeLsaHeap(accountName->Buffer);
        }
        g_LsaDispatchTable->FreeLsaHeap(accountName);
    }
    if (authAuthority) {
        if (authAuthority->Buffer) {
            g_LsaDispatchTable->FreeLsaHeap(authAuthority->Buffer);
        }
        g_LsaDispatchTable->FreeLsaHeap(authAuthority);
    }

    return status;
}

// Called by the Local Security Authority (LSA) when a logon application with a trusted connection to the LSA calls the LsaCallAuthenticationPackage function and specifies the authentication package's identifier.
NTSTATUS NTAPI LsaApCallPackage(
    _In_ PLSA_CLIENT_REQUEST ClientRequest,
    _In_ PVOID ProtocolSubmitBuffer,
    _In_ PVOID ClientBufferBase,
    _In_ ULONG SubmitBufferLength,
    _Out_ PVOID* ProtocolReturnBuffer,
    _Out_ PULONG ReturnBufferLength,
    _Out_ PNTSTATUS ProtocolStatus
) {
    // Debug("LsaApCallPackage");
    *ProtocolReturnBuffer = NULL;
    *ReturnBufferLength = 0;
    *ProtocolStatus = STATUS_SUCCESS;
    return STATUS_SUCCESS;
}

// Called by the Local Security Authority (LSA) when an application with an untrusted connection to the LSA calls the LsaCallAuthenticationPackage function and specifies the authentication package's identifier.
NTSTATUS NTAPI LsaApCallPackageUntrusted(
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
    // Debug("LsaApLogonTerminated");
}

// The dispatch function for pass-through logon requests sent to the LsaCallAuthenticationPackage function.
NTSTATUS NTAPI LsaApCallPackagePassthrough(
  _In_ PLSA_CLIENT_REQUEST ClientRequest,
  _In_ PVOID ProtocolAuthenticationInformation,
  _In_ PVOID ClientBufferBase,
  _In_ ULONG AuthenticationInformationLength,
  _Out_ PVOID* ProtocolReturnBuffer,
  _Out_ PULONG ReturnBufferLength,
  _Out_ PNTSTATUS ProtocolStatus
)
{
    // Debug("LsaApCallPackagePassthrough");
	return STATUS_NOT_IMPLEMENTED;
}
