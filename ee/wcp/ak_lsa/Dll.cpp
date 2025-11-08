#include <Windows.h>
#include <string>
#include "include/Debug.h"

STDAPI_(BOOL)
DllMain(__in HINSTANCE hinstDll, __in DWORD dwReason, __in LPVOID lpReserved) {
//   switch (dwReason) {
//   case DLL_PROCESS_ATTACH:
//     Debug("DllMain::DLL_PROCESS_ATTACH");
//     break;
//   case DLL_PROCESS_DETACH:
//     Debug("DllMain::DLL_PROCESS_DETACH");
//     break;
//   }
  return TRUE;
}
