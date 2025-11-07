
#include <windows.h>
#include <ntsecapi.h>

// Custom authentication data structure
typedef struct _CUSTOM_LOGON_DATA {
    PUNICODE_STRING Domain;
    PUNICODE_STRING UserName;
    PUNICODE_STRING Password;
    ULONG LogonType;
} CUSTOM_LOGON_DATA, *PCUSTOM_LOGON_DATA;
