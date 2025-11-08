#include "include/Debug.h"
#include <Windows.h>
#include <string>
#include <mutex>

#define BUFFER_SIZE 10000
std::mutex g_dbgMutex;

void Debug(const char* data) {
    g_dbgMutex.lock();
    HANDLE hFile;
    char DataBuffer[BUFFER_SIZE] = { '\0' };
    size_t i = 0;
    for (i = 0; (i < (DWORD)strlen(data)) && (i < BUFFER_SIZE); ++i)
    {
        DataBuffer[i] = data[i];
    }
    DataBuffer[i] = '\n';
    DWORD dwBytesToWrite = (DWORD)strlen(DataBuffer);
    DWORD dwBytesWritten = 0;
    BOOL bErrorFlag = FALSE;

    std::string strPath = "C:\\ak_lsa.txt";

    hFile = CreateFileW(
        std::wstring(strPath.begin(), strPath.end()).c_str(),                // name of the write
        FILE_APPEND_DATA,          // open for writing
        0,                      // do not share
        NULL,                   // default security
        OPEN_ALWAYS,             // create new file only
        FILE_ATTRIBUTE_NORMAL,  // normal file
        NULL);                  // no attr. template

    if (hFile != INVALID_HANDLE_VALUE) {
        bErrorFlag = WriteFile(
            hFile,           // open file handle
            DataBuffer,      // start of data to write
            dwBytesToWrite,  // number of bytes to write
            &dwBytesWritten, // number of bytes that were written
            NULL);            // no overlapped structure

        if (FALSE == bErrorFlag)
        {
            /*MessageBox(
                NULL,
                (LPCWSTR)L"Unable to write to file",
                (LPCWSTR)L"Error",
                MB_OK
            );*/
        }
        else
        {
            if (dwBytesWritten != dwBytesToWrite)
            {
                // This is an error because a synchronous write that results in
                // success (WriteFile returns TRUE) should write all data as
                // requested. This would not necessarily be the case for
                // asynchronous writes.
                /*MessageBox(
                    NULL,
                    (LPCWSTR)L"dwBytesWritten != dwBytesToWrite",
                    (LPCWSTR)L"Error",
                    MB_OK
                );*/
            }
            else
            {
                /*MessageBox(
                    NULL,
                    (LPCWSTR)L"ALHAMDULILLAAH, write successful.",
                    (LPCWSTR)L"ALHAMDULILLAAH",
                    MB_OK
                );*/
            }
        }

        CloseHandle(hFile);
    }

    g_dbgMutex.unlock();
}
