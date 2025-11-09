#include <intrin.h>
#include <windows.h>
#include <ntsecapi.h>
#include <subauth.h>

BOOLEAN ValidateCustomPassword(
    PUNICODE_STRING Domain,
    PUNICODE_STRING UserName,
    PUNICODE_STRING Password
);
