// dllmain.cpp : Defines the entry point for the DLL application.
#include "pch.h"
#include <string>

#include "Dll.h"

#include "authentik_sys_bridge/ffi.h"
#include "rust/cxx.h"

#include "ak_sentry.h"
#include "include/cef_command_line.h"
#include "include/cef_sandbox_win.h"
#include "spdlog/spdlog.h"

static LONG g_cRef = 0;   // global dll reference count
HINSTANCE g_hinst = NULL; // global dll hinstance

TCHAR g_path[MAX_PATH];

std::string g_strPath = "";

// Find the path of us (the dll) and the parent directory
void SetPaths() {
  GetModuleFileName(g_hinst, g_path, MAX_PATH);
  SIZE_T i = 0;
  while (i < MAX_PATH) {
    if (g_path[i] == NULL) {
      break;
    }
    g_strPath.append(1, g_path[i]);
    ++i;
  }
  while (i >= 0) {
    if (g_path[i] == '\\') {
      g_path[i] = NULL;
      break;
    }
    g_path[i] = NULL;
    --i;
  }
  g_strPath = g_strPath.substr(0, g_strPath.find_last_of("\\"));
}

STDAPI_(BOOL)
DllMain(__in HINSTANCE hinstDll, __in DWORD dwReason, __in LPVOID lpReserved) {
  g_hinst = hinstDll;

  switch (dwReason) {
  case DLL_PROCESS_ATTACH: {
    SetPaths();
    SetupLogs("ak_cred_provider");
    SentrySetup("ak_cred_provider");
    Debug("DllMain::DLL_PROCESS_ATTACH");
    try {
      std::string ping = std::string("");
      ak_sys_ping(ping);
      Debug(std::string("sysd version: ").append(ping).c_str());
    } catch (const rust::Error &ex) {
      Debug("Exception in ak_grpc_ping");
      Debug(ex.what());
    }

    DisableThreadLibraryCalls(hinstDll);
    Debug(std::string("DLL hInstance: " + std::to_string((size_t)hinstDll))
              .c_str());
    std::string strID =
        "DLL ProcessID: " + std::to_string(GetCurrentProcessId()) +
        ", ThreadID: " + std::to_string(GetCurrentThreadId());
    Debug(strID.c_str());
  } break;
  case DLL_THREAD_ATTACH:
  case DLL_THREAD_DETACH:
    break;
  case DLL_PROCESS_DETACH:
    Debug("DllMain::DLL_PROCESS_DETACH");
    SentryShutdown();
    spdlog::shutdown();
    break;
  }
  return TRUE;
}

STDAPI DllGetClassObject(__in REFCLSID rclsid, __in REFIID riid,
                         __deref_out void **ppv) {
  *ppv = NULL;
  HRESULT hr;
  if (rclsid == CLSID_CredentialProvider) {
    ClassFactory *pcf = new ClassFactory();
    if (pcf) {
      hr = pcf->QueryInterface(riid, ppv);
      pcf->Release();
    } else {
      hr = E_OUTOFMEMORY;
    }
  } else {
    hr = CLASS_E_CLASSNOTAVAILABLE;
  }
  return hr;
}

void DllAddRef() { InterlockedIncrement(&g_cRef); }

void DllRelease() { InterlockedDecrement(&g_cRef); }

STDAPI DllCanUnloadNow() {
  if (g_cRef > 0) {
    return S_FALSE;
  }
  return S_OK;
}
