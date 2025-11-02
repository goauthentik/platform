// dllmain.cpp : Defines the entry point for the DLL application.
#include "pch.h"
#include "Dll.h"

#include <string>

#include "include/cef_command_line.h"
#include "include/cef_sandbox_win.h"

static LONG g_cRef = 0;   // global dll reference count
HINSTANCE g_hinst = NULL; // global dll hinstance

TCHAR g_path[MAX_PATH];

std::string g_strPath = "";


void SetPaths()
{
    GetModuleFileName(g_hinst, g_path, MAX_PATH);
    SIZE_T i = 0;
    while (i < MAX_PATH)
    {
        if (g_path[i] == NULL)
        {
            break;
        }
        g_strPath.append(1, g_path[i]);
        ++i;
    }
    while (i >= 0)
    {
        if (g_path[i] == '\\')
        {
            g_path[i] = NULL;
            break;
        }
        g_path[i] = NULL;
        --i;
    }
    g_strPath = g_strPath.substr(0, g_strPath.find_last_of("\\"));
}

STDAPI_(BOOL) DllMain(  __in HINSTANCE hinstDll,
                        __in DWORD dwReason,
                        __in LPVOID lpReserved
                     )
{
    sentry_options_t *options = sentry_options_new();
    sentry_options_set_dsn(options, "https://c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688.ingest.us.sentry.io/4509208005312512");
    sentry_options_set_release(options, "ak-platform-wcp@" + AK_WCP_VERSION);
    sentry_init(options);
    g_hinst = hinstDll;
    switch (dwReason)
    {
    case DLL_PROCESS_ATTACH:
    {
        SetPaths();
        Debug("BISMILLAAHIRRAHMAANIRRAHEEM");
        DisableThreadLibraryCalls(hinstDll);
        Debug(std::string("DLL hInstance: " + std::to_string((size_t)hinstDll)).c_str());
        std::string strID = "DLL ProcessID: " + std::to_string(GetCurrentProcessId()) + ", ThreadID: " + std::to_string(GetCurrentThreadId());
        Debug(strID.c_str());
    }
        break;
    case DLL_THREAD_ATTACH:
    case DLL_THREAD_DETACH:
        break;
    case DLL_PROCESS_DETACH:
    // {
    //     std::string strID = "Detach DLL ProcessID: " + std::to_string(GetCurrentProcessId()) + ", ThreadID: " + std::to_string(GetCurrentThreadId());
    //     Debug(strID.c_str());
    // }
        break;
    }
    return TRUE;
}

STDAPI DllGetClassObject(__in REFCLSID rclsid, __in REFIID riid, __deref_out void** ppv)
{
    *ppv = NULL;
    HRESULT hr;
    if (rclsid == CLSID_CredentialProvider)
    {
        ClassFactory* pcf = new ClassFactory();
        if (pcf)
        {
            hr = pcf->QueryInterface(riid, ppv);
            pcf->Release();
        }
        else
        {
            hr = E_OUTOFMEMORY;
        }
    }
    else
    {
        hr = CLASS_E_CLASSNOTAVAILABLE;
    }
    return hr;
}

void DllAddRef()
{
    InterlockedIncrement(&g_cRef);
}

void DllRelease()
{
    InterlockedDecrement(&g_cRef);
}

STDAPI DllCanUnloadNow()
{
    if (g_cRef > 0) {
        return S_FALSE;
    }
    sentry_shutdown();
    return S_OK;
}
