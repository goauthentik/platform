// BISMILLAAHIRRAHMAANIRRAHEEM

#include "pch.h"
#include "Debug.h"
#include <Windows.h>
#include <string>



#define BUFFER_SIZE 10000
extern HINSTANCE g_hinst;
extern std::string g_strPath;

// void GetPath(wchar_t* path)
// {
//     HMODULE hm = NULL;

//     if (GetModuleHandleEx(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS |
//         GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
//         (LPCWSTR)&Debug, &hm) == 0)
//     {
//         int ret = GetLastError();
//         fprintf(stderr, "GetModuleHandle failed, error = %d\n", ret);
//         // Return or however you want to handle an error.
//     }
//     if (GetModuleFileName(hm, path, sizeof(path)) == 0)
//     {
//         int ret = GetLastError();
//         fprintf(stderr, "GetModuleFileName failed, error = %d\n", ret);
//         // Return or however you want to handle an error.
//     }
// }

// void ErrorExit()
// {
//     // Retrieve the system error message for the last-error code

//     LPVOID lpMsgBuf;
//     DWORD dw = GetLastError();

//     if (FormatMessage(
//         FORMAT_MESSAGE_ALLOCATE_BUFFER |
//         FORMAT_MESSAGE_FROM_SYSTEM |
//         FORMAT_MESSAGE_IGNORE_INSERTS,
//         NULL,
//         dw,
//         MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
//         (LPTSTR) &lpMsgBuf,
//         0, NULL) == 0) {
//         MessageBox(NULL, TEXT("FormatMessage failed"), TEXT("Error"), MB_OK);
//         ExitProcess(dw);
//     }

//     MessageBox(NULL, (LPCTSTR)lpMsgBuf, TEXT("Error"), MB_OK);

//     LocalFree(lpMsgBuf);
//     // ExitProcess(dw); //-- caution!
// }
std::mutex g_dbgMutex;
void Debug(const char* data, bool bReset)
{
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

    // TCHAR path[MAX_PATH];
    // GetModuleFileName(g_hinst, path, MAX_PATH);
    // DWORD dwPathSize = (DWORD)wcslen(path);
    // path[dwPathSize] = '.';
    // ++dwPathSize;
    // path[dwPathSize] = 't';
    // ++dwPathSize;
    // path[dwPathSize] = 'x';
    // ++dwPathSize;
    // path[dwPathSize] = 't';
    // ++dwPathSize;
    std::string strPath = g_strPath + "\\..\\..\\file.txt";

    if (bReset)
    {
        // hFile = CreateFileW(L"C:\\Users\\mbkha\\source\\repos\\AuthentikCEFWCP\\file.txt",                // name of the write
        // hFile = CreateFileW(path,                // name of the write
        hFile = CreateFileW(std::wstring(strPath.begin(), strPath.end()).c_str(),                // name of the write
            GENERIC_WRITE,          // open for writing
            0,                      // do not share
            NULL,                   // default security
            CREATE_ALWAYS,             // create new file only
            FILE_ATTRIBUTE_NORMAL,  // normal file
            NULL);                  // no attr. template
    }
    else
    {
        // hFile = CreateFileW(L"C:\\Users\\mbkha\\source\\repos\\AuthentikCEFWCP\\file.txt",                // name of the write
        hFile = CreateFileW(std::wstring(strPath.begin(), strPath.end()).c_str(),                // name of the write
                FILE_APPEND_DATA,          // open for writing
            0,                      // do not share
            NULL,                   // default security
            OPEN_ALWAYS,             // create new file only
            FILE_ATTRIBUTE_NORMAL,  // normal file
            NULL);                  // no attr. template
    }

    if (hFile == INVALID_HANDLE_VALUE)
    {

        // MessageBox(
        //     NULL,
        //     (LPCWSTR)L"Invalid handle",
        //     (LPCWSTR)L"Error",
        //     MB_OK
        // );
    }
    else
    {
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