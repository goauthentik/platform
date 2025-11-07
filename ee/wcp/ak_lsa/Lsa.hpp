#pragma once

typedef long NTSTATUS, *PNTSTATUS;

#include <ntstatus.h>
#define WIN32_NO_STATUS
#ifndef SECURITY_WIN32
#define SECURITY_WIN32
#endif

#include <Windows.h>
#include <LsaLookup.h>
#include <NTSecAPI.h>
#include <sspi.h>
#include <NTSecPKG.h>
#include <string>

using std::wstring;

union LocalUniqId
{
	LUID Luid;
	__int64 RawId;
};

union int64u
{
	int64_t i64;
	uint64_t ui64;
	LARGE_INTEGER li;
	LUID id;
	FILETIME ft;
};

extern void LoadLsaString(const LSA_STRING *, wstring &);

// LSA Authentication Package Functions Begin
extern "C"
{
	extern NTSTATUS NTAPI LsaApInitializePackage(ULONG, LSA_DISPATCH_TABLE*, LSA_STRING*, LSA_STRING*, LSA_STRING**);

	extern NTSTATUS NTAPI LsaApCallPackage(PLSA_CLIENT_REQUEST, void*, void*, ULONG, void**, ULONG*, NTSTATUS*);
	extern NTSTATUS NTAPI LsaApCallPackagePassthrough(PLSA_CLIENT_REQUEST, void*, void*, ULONG, void**, ULONG*, NTSTATUS*);
	extern NTSTATUS NTAPI LsaApCallPackageUntrusted(PLSA_CLIENT_REQUEST, void*, void*, ULONG, void**, ULONG*, NTSTATUS*);

	extern NTSTATUS NTAPI LsaApLogonUser(PLSA_CLIENT_REQUEST, SECURITY_LOGON_TYPE, void *, void *, ULONG, void**, ULONG*, LUID*, NTSTATUS*, LSA_TOKEN_INFORMATION_TYPE*, void**, UNICODE_STRING**, UNICODE_STRING**);
	extern NTSTATUS NTAPI LsaApLogonUserEx(PLSA_CLIENT_REQUEST, SECURITY_LOGON_TYPE, void *, void *, ULONG, void**, ULONG*, LUID*, NTSTATUS*, LSA_TOKEN_INFORMATION_TYPE*, void**, UNICODE_STRING**, UNICODE_STRING**, UNICODE_STRING**);
	extern NTSTATUS NTAPI LsaApLogonUserEx2(PLSA_CLIENT_REQUEST, SECURITY_LOGON_TYPE, void *, void *, ULONG, void**, ULONG*, LUID*, NTSTATUS*, LSA_TOKEN_INFORMATION_TYPE*, void**, UNICODE_STRING**, UNICODE_STRING**, UNICODE_STRING**, SECPKG_PRIMARY_CRED*, SECPKG_SUPPLEMENTAL_CRED_ARRAY**);

	extern     void NTAPI   LsaApLogonTerminated(LUID*);
}
// LSA Authentication Package Functions End
