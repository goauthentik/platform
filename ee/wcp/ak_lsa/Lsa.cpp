#include "Lsa.hpp"
#include <intrin.h>
#include <ak_cred_provider/include/Debug.h>

// CustomAuthPackage.cpp - Corrected version
// Example LSA Authentication Package for custom password authentication

#include <windows.h>
#include <ntsecapi.h>
#include <subauth.h>
#include <lm.h>
#include "LogonData.h"

// Global variables
static PLSA_DISPATCH_TABLE g_LsaDispatchTable = NULL;
static ULONG g_AuthenticationPackageId = 0;

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
    ULONG AuthenticationPackageId,
    PLSA_DISPATCH_TABLE LsaDispatchTable,
    PLSA_STRING Database,
    PLSA_STRING Confidentiality,
    PLSA_STRING *AuthenticationPackageName
)
{
    PLSA_STRING packageName;
    const char* packageNameStr = "ak_lsa";
    SIZE_T nameLength = strlen(packageNameStr);

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

    CopyMemory(packageName->Buffer, packageNameStr, nameLength);
    packageName->Buffer[nameLength] = '\0';
    packageName->Length = (USHORT)nameLength;
    packageName->MaximumLength = (USHORT)(nameLength + 1);

    *AuthenticationPackageName = packageName;

    return STATUS_SUCCESS;
}


// Custom password validation function
BOOLEAN ValidateCustomPassword(
    PUNICODE_STRING Domain,
    PUNICODE_STRING UserName,
    PUNICODE_STRING Password
)
{
    // Implement your custom password validation logic here
    // This is where you would:
    // - Connect to your custom authentication database
    // - Validate the credentials against your system
    // - Perform any additional security checks

    // Example: Simple validation (NOT for production use)
    if (Password->Length < 8 * sizeof(WCHAR)) {
        return FALSE; // Password too short
    }

    // Add your custom validation logic here
    // For demonstration, we'll accept any password with "custom" in it
    if (wcsstr(Password->Buffer, L"custom") != NULL) {
        return TRUE;
    }

    return FALSE;
}


// Main logon function - corrected version
NTSTATUS NTAPI LsaApLogonUserEx2(
    PLSA_CLIENT_REQUEST ClientRequest,
    SECURITY_LOGON_TYPE LogonType,
    PVOID AuthenticationInformation,
    PVOID ClientAuthenticationBase,
    ULONG AuthenticationInformationLength,
    PVOID *ProfileBuffer,
    PULONG ProfileBufferLength,
    PLUID LogonId,
    PNTSTATUS SubStatus,
    PLSA_TOKEN_INFORMATION_TYPE TokenInformationType,
    PVOID *TokenInformation,
    PUNICODE_STRING *AccountName,
    PUNICODE_STRING *AuthenticatingAuthority,
    PUNICODE_STRING *MachineName,
    PSECPKG_PRIMARY_CRED PrimaryCredentials,
    PSECPKG_SUPPLEMENTAL_CRED_ARRAY *SupplementalCredentials
)
{
    PCUSTOM_LOGON_DATA logonData;
    PMSV1_0_INTERACTIVE_PROFILE profile;
    PLSA_TOKEN_INFORMATION_V1 tokenInfo;
    PUNICODE_STRING accountName;
    PUNICODE_STRING authAuthority;
    PUNICODE_STRING machineName;
    NTSTATUS status;

    *SubStatus = STATUS_SUCCESS;

    // Validate input parameters
    if (!AuthenticationInformation ||
        AuthenticationInformationLength < sizeof(CUSTOM_LOGON_DATA)) {
        return STATUS_INVALID_PARAMETER;
    }

    logonData = (PCUSTOM_LOGON_DATA)AuthenticationInformation;

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
    *ProfileBufferLength = sizeof(MSV1_0_INTERACTIVE_PROFILE);

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


NTSTATUS NTAPI LsaApCallPackage(
    PLSA_CLIENT_REQUEST ClientRequest,
    PVOID ProtocolSubmitBuffer,
    PVOID ClientBufferBase,
    ULONG SubmitBufferLength,
    PVOID* ProtocolReturnBuffer,
    PULONG ReturnBufferLength,
    PNTSTATUS ProtocolStatus
)
{
    *ProtocolReturnBuffer = NULL;
    *ReturnBufferLength = 0;
    *ProtocolStatus = STATUS_SUCCESS;
    return STATUS_SUCCESS;
}

NTSTATUS NTAPI LsaApCallPackageUntrusted(
    PLSA_CLIENT_REQUEST ClientRequest,
    PVOID ProtocolSubmitBuffer,
    PVOID ClientBufferBase,
    ULONG SubmitBufferLength,
    PVOID* ProtocolReturnBuffer,
    PULONG ReturnBufferLength,
    PNTSTATUS ProtocolStatus
)
{
    return LsaApCallPackage(ClientRequest, ProtocolSubmitBuffer,
                           ClientBufferBase, SubmitBufferLength,
                           ProtocolReturnBuffer, ReturnBufferLength,
                           ProtocolStatus);
}

VOID NTAPI LsaApLogonTerminated(PLUID LogonId)
{
    // Cleanup when logon session ends
}
