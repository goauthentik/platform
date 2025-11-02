#include "pch.h"
#include "Debug.h"
#include "authentik_sys_bridge/ffi.h"
#include <Windows.h>
#include <string>
#include "event.h"

#define BUFFER_SIZE 10000

std::mutex g_dbgMutex;
HANDLE g_evtSource;

void Debug(const char* data, bool bReset)
{
    g_dbgMutex.lock();
    if (g_evtSource == NULL) {
        g_evtSource = RegisterEventSourceW(L"", L"authentik WCP");
    }

    if (!ReportEvent(g_evtSource, EVENTLOG_ERROR_TYPE, 0, MSG_DEBUG, NULL, 1, 0, (LPCWSTR*)data, NULL))
    {
        wprintf(L"ReportEvent failed with 0x%x for event 0x%x.\n", GetLastError(), MSG_DEBUG);
    }
    g_dbgMutex.unlock();
}

void Debug16(const char16_t* data, bool bReset)
{
    char DataBuffer[BUFFER_SIZE] = { '\0' };
    size_t i = 0;
    for (i = 0; (i < (DWORD)std::char_traits<char16_t>::length(data)) && (i < BUFFER_SIZE); ++i)
    {
        DataBuffer[i] = (char)(data[i]);
    }
    Debug(DataBuffer, bReset);
}
