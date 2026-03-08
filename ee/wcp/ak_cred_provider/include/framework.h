#pragma once

// Enable, if required, to avoid conflicting declarations for Winsock
// #define WIN32_LEAN_AND_MEAN             // Exclude rarely-used stuff from Windows headers
// Windows Header Files
#define WIN32_NO_STATUS
#include <windows.h>
#undef WIN32_NO_STATUS
#include <shlwapi.h>
#include <shlguid.h>  // CPFG_CREDENTIAL_PROVIDER_LOGO etc
#include <ShellAPI.h>
#include <LM.h>  // Password reset

// GUID
#include <initguid.h>

// COM
#include <unknwn.h>

// Credential provider
#include <credentialprovider.h>
#pragma warning(disable : 4005)
#include <ntstatus.h>
#pragma warning(default : 4005)
#include <propkey.h>

// std
#include <new>  // std::nothrow
#include <thread>
#include <vector>
#include <mutex>

// Helpers
#include <intsafe.h>
#include <ntsecapi.h>
#include <wincred.h>
#include <strsafe.h>
