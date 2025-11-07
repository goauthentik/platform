#include <Windows.h>
#include <string>
// #include <spdlog/spdlog.h>
// #include "include/Debug.h"

// using std::wstring;

// static LONG g_cRef = 0;   // global dll reference count
// HINSTANCE g_hinst = NULL; // global dll hinstance

// TCHAR g_path[MAX_PATH];

// std::string g_strPath = "";

// // Find the path of us (the dll) and the parent directory
// void SetPaths() {
//   GetModuleFileName(g_hinst, g_path, MAX_PATH);
//   SIZE_T i = 0;
//   while (i < MAX_PATH) {
//     if (g_path[i] == NULL) {
//       break;
//     }
//     g_strPath.append(1, g_path[i]);
//     ++i;
//   }
//   while (i >= 0) {
//     if (g_path[i] == '\\') {
//       g_path[i] = NULL;
//       break;
//     }
//     g_path[i] = NULL;
//     --i;
//   }
//   g_strPath = g_strPath.substr(0, g_strPath.find_last_of("\\"));
// }

// namespace Lsa {
// HANDLE Heap;
// }

STDAPI_(BOOL)
DllMain(__in HINSTANCE hinstDll, __in DWORD dwReason, __in LPVOID lpReserved) {
//   g_hinst = hinstDll;

  // switch (dwReason) {
  // case DLL_PROCESS_ATTACH:
  //   // SetupLogsPath("C:\\", "lsa");
  //   break;
  // case DLL_PROCESS_DETACH:
  //   // Debug("DllMain::DLL_PROCESS_DETACH");
  //   // spdlog::shutdown();
  //   break;
  // }
  return TRUE;
}
//     SetPaths();
//     SetupLogs("ak_lsa");
//     // SentrySetup("ak_cred_provider");
//     Debug("DllMain::DLL_PROCESS_ATTACH");
//     Lsa::Heap = HeapCreate(0, 65536, 0);
//     if (!Lsa::Heap) {
//       Debug("HeapCreate failed.");
//       return false;
//     }

//     DisableThreadLibraryCalls(hinstDll);
//     Debug(std::string("DLL hInstance: " + std::to_string((size_t)hinstDll))
//               .c_str());
//     std::string strID =
//         "DLL ProcessID: " + std::to_string(GetCurrentProcessId()) +
//         ", ThreadID: " + std::to_string(GetCurrentThreadId());
//     Debug(strID.c_str());
//     // database = new wstring;
//     // confidentiality = new wstring;
//   } break;
//   case DLL_THREAD_ATTACH:
//   case DLL_THREAD_DETACH:
//     break;
//   case DLL_PROCESS_DETACH:
//     // delete confidentiality;
//     // delete database;
//     HeapDestroy(Lsa::Heap);
//     Debug("DllMain::DLL_PROCESS_DETACH");
//     // SentryShutdown();
    // spdlog::shutdown();
//     break;
//   }
//   return TRUE;
// }
