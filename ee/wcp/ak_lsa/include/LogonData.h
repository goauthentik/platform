
#include <windows.h>
#include <ntsecapi.h>

// Custom authentication data structure
typedef struct _CUSTOM_LOGON_DATA {
    PUNICODE_STRING Domain;
    PUNICODE_STRING UserName;
    PUNICODE_STRING Password;
    ULONG LogonType;
} CUSTOM_LOGON_DATA, *PCUSTOM_LOGON_DATA;

// Custom profile structure that will be returned to the client
typedef struct _CUSTOM_PROFILE_DATA {
    MSV1_0_PROFILE_BUFFER_TYPE MessageType;
    LARGE_INTEGER LogonTime;
    LARGE_INTEGER LogoffTime;
    LARGE_INTEGER KickOffTime;
    LARGE_INTEGER PasswordLastSet;
    LARGE_INTEGER PasswordCanChange;
    LARGE_INTEGER PasswordMustChange;
    UNICODE_STRING LogonScript;
    UNICODE_STRING HomeDirectory;
    UNICODE_STRING FullName;
    UNICODE_STRING ProfilePath;
    UNICODE_STRING HomeDirectoryDrive;
    UNICODE_STRING LogonServer;
    ULONG UserFlags;
} CUSTOM_PROFILE_DATA, *PCUSTOM_PROFILE_DATA;
