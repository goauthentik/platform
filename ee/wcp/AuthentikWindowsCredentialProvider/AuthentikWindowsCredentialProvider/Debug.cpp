#include "pch.h"
#include "Debug.h"
#include "authentik_sys_bridge/ffi.h"
#include <Windows.h>
#include <string>

#define BUFFER_SIZE 10000

std::mutex g_dbgMutex;
bool g_hasInit;

void Debug(const char* data, bool bReset)
{
    g_dbgMutex.lock();
    if (!g_hasInit) {
        ak_log_init("authentik WCP");
        g_hasInit = true;
    }

    ak_log_msg(data);
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
