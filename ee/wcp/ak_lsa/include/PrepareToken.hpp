#pragma once
#pragma warning(disable : 4005)
#include <ntstatus.h>
#pragma warning(default : 4005)
#include <windows.h>
#include <Lmcons.h>
#include <sspi.h>
#include <NTSecAPI.h>  // for LSA_STRING
#include <ntsecpkg.h>  // for LSA_DISPATCH_TABLE

NTSTATUS UserNameToToken(__in LSA_UNICODE_STRING* AccountName,
                         __out LSA_TOKEN_INFORMATION_V1** Token, __out PNTSTATUS SubStatus);
