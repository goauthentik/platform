#pragma once

#define SECURITY_WIN32
#include <security.h>

//makes a copy of a field descriptor using CoTaskMemAlloc
HRESULT FieldDescriptorCoAllocCopy(
    _In_ const CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR& rcpfd,
    _Outptr_result_nullonfailure_ CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR** ppcpfd
);

//makes a copy of a field descriptor on the normal heap
HRESULT FieldDescriptorCopy(
    _In_ const CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR& rcpfd,
    _Out_ CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR* pcpfd
);

//creates a UNICODE_STRING from a NULL-terminated string
HRESULT UnicodeStringInitWithString(
    _In_ PWSTR pwz,
    _Out_ UNICODE_STRING* pus
);

//initializes a KERB_INTERACTIVE_UNLOCK_LOGON with weak references to the provided credentials
HRESULT KerbInteractiveUnlockLogonInit(
    _In_ PWSTR pwzDomain,
    _In_ PWSTR pwzUsername,
    _In_ PWSTR pwzPassword,
    _In_ CREDENTIAL_PROVIDER_USAGE_SCENARIO cpus,
    _Out_ KERB_INTERACTIVE_UNLOCK_LOGON* pkiul
);

//packages the credentials into the buffer that the system expects
HRESULT KerbInteractiveUnlockLogonPack(
    _In_ const KERB_INTERACTIVE_UNLOCK_LOGON& rkiulIn,
    _Outptr_result_bytebuffer_(*pcb) BYTE** prgb,
    _Out_ DWORD* pcb
);

//get the authentication package that will be used for our logon attempt
HRESULT RetrieveNegotiateAuthPackage(
    _Out_ ULONG* pulAuthPackage
);

//encrypt a password (if necessary) and copy it; if not, just copy it
HRESULT ProtectIfNecessaryAndCopyPassword(
    _In_ PCWSTR pwzPassword,
    _In_ CREDENTIAL_PROVIDER_USAGE_SCENARIO cpus,
    _Outptr_result_nullonfailure_ PWSTR* ppwzProtectedPassword
);

HRESULT KerbInteractiveUnlockLogonRepackNative(
    _In_reads_bytes_(cbWow) BYTE* rgbWow,
    _In_ DWORD cbWow,
    _Outptr_result_bytebuffer_(*pcbNative) BYTE** prgbNative,
    _Out_ DWORD* pcbNative
);

void KerbInteractiveUnlockLogonUnpackInPlace(
    _Inout_updates_bytes_(cb) KERB_INTERACTIVE_UNLOCK_LOGON* pkiul,
    DWORD cb
);

HRESULT DomainUsernameStringAlloc(
    _In_ PCWSTR pwszDomain,
    _In_ PCWSTR pwszUsername,
    _Outptr_result_nullonfailure_ PWSTR* ppwszDomainUsername
);

HRESULT SplitDomainAndUsername(_In_ PCWSTR pszQualifiedUserName, _Outptr_result_nullonfailure_ PWSTR* ppszDomain, _Outptr_result_nullonfailure_ PWSTR* ppszUsername);