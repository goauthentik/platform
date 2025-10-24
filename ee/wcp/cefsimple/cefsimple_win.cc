// Copyright (c) 2013 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#include <windows.h>

#include "include/cef_command_line.h"
#include "include/cef_sandbox_win.h"
#include "cefsimple/simple_app.h"
#include "AuthentikWindowsCredentialProvider/AuthentikWindowsCredentialProvider/include/Debug.h"
#include "crypt.h"
#include "Credential.h"

extern std::string g_strPath;

// When generating projects with CMake the CEF_USE_SANDBOX value will be defined
// automatically if using the required compiler version. Pass -DUSE_SANDBOX=OFF
// to the CMake command-line to disable use of the sandbox.
// Uncomment this line to manually enable sandbox support.
// #define CEF_USE_SANDBOX 1

#if defined(CEF_USE_SANDBOX)
// The cef_sandbox.lib static library may not link successfully with all VS
// versions.
#pragma comment(lib, "cef_sandbox.lib")
#endif

// Entry point function for all processes.
// int APIENTRY wWinMain(HINSTANCE hInstance,
//                       HINSTANCE hPrevInstance,
//                       LPTSTR lpCmdLine,
//                       int nCmdShow) {
//   UNREFERENCED_PARAMETER(hPrevInstance);
//   UNREFERENCED_PARAMETER(lpCmdLine);
// int APIENTRY LaunchCEF(HINSTANCE hInstance,
int CEFLaunch(sHookData* pData,
  HWND hWnd,
  int nCmdShow) {

  int exit_code;

  Debug("CEFLaunch");
  std::string str = "CEFLaunch ProcessID: " + std::to_string(GetCurrentProcessId()) + ", ThreadID: " + std::to_string(GetCurrentThreadId());
  Debug(str.c_str());

#if defined(ARCH_CPU_32_BITS)
  // Run the main thread on 32-bit Windows using a fiber with the preferred 4MiB
  // stack size. This function must be called at the top of the executable entry
  // point function (`main()` or `wWinMain()`). It is used in combination with
  // the initial stack size of 0.5MiB configured via the `/STACK:0x80000` linker
  // flag on executable targets. This saves significant memory on threads (like
  // those in the Windows thread pool, and others) whose stack size can only be
  // controlled via the linker flag.
  exit_code = CefRunWinMainWithPreferredStackSize(wWinMain, pData->hInstance,
                                                  lpCmdLine, nCmdShow);
  // if (exit_code >= 0) {
  //   // The fiber has completed so return here.
  //   return exit_code;
  // }
#endif

  void* sandbox_info = nullptr;

#if defined(CEF_USE_SANDBOX)
  // Manage the life span of the sandbox information object. This is necessary
  // for sandbox support on Windows. See cef_sandbox_win.h for complete details.
  CefScopedSandboxInfo scoped_sandbox;
  sandbox_info = scoped_sandbox.sandbox_info();
#endif
  Debug("CefScopedSandboxInfo");
  // Provide CEF with command-line arguments.
  CefMainArgs main_args(pData->hInstance);

  exit_code = 0;

  Debug("CefMainArgs");
  // CEF applications have multiple sub-processes (render, GPU, etc) that share
  // the same executable. This function checks the command-line and, if this is
  // a sub-process, executes the appropriate logic.

  //exit_code = CefExecuteProcess(main_args, nullptr, sandbox_info);
  //Debug("CefExecuteProcess");
  //if (exit_code >= 0) {
  //    Debug("Cef: exit_code");
  //  // The sub-process has completed so return here.
  //  return exit_code;
  //}

  Debug("CefCommandLine::CreateCommandLine");
  // Parse command-line arguments for use in this method.
  CefRefPtr<CefCommandLine> command_line = CefCommandLine::CreateCommandLine();
  command_line->InitFromString(::GetCommandLineW());

  // Specify CEF global settings here.
  CefSettings settings;

  // Specify the path for the sub-process executable.
  std::string strPath = g_strPath + "\\cefexe.exe";
  CefString(&settings.browser_subprocess_path).FromASCII(strPath.c_str());
  // std::string strRPath = g_strPath + "\\" + GetRandomStr(5);
  // Do not set cache_path to launch CEF in private/ incognito mode
  // CefString(&settings.root_cache_path).FromASCII(strRPath.c_str());
  // CefString(&settings.cache_path).FromASCII(std::string(strRPath + "\\CPath" + GetRandomStr(5)).c_str());

  strPath = g_strPath + "\\..\\..\\ceflog.txt";
  CefString(&settings.log_file).FromASCII(strPath.c_str());
  settings.log_severity = LOGSEVERITY_INFO;

#if !defined(CEF_USE_SANDBOX)
  settings.no_sandbox = true;
#endif

  //settings.multi_threaded_message_loop = true;
  // settings.chrome_app_icon_id = IDB_TILE_IMAGE;


  Debug("CefSettings");
  // SimpleApp implements application-level callbacks for the browser process.
  // It will create the first browser instance in OnContextInitialized() after
  // CEF has initialized.
  CefRefPtr<SimpleApp> app(new SimpleApp(pData, hWnd));
  Debug("Cef: new SimpleApp");

  Debug(std::string("app.get:::" + std::to_string((size_t)(app.get()))).c_str());
  // Initialize the CEF browser process. May return false if initialization
  // fails or if early exit is desired (for example, due to process singleton
  // relaunch behavior).
  if (!CefInitialize(main_args, settings, app.get(), sandbox_info)) {
      Debug("CefGetExitCode");
    return CefGetExitCode();
  }
  Debug("CefInitialize");
  // Run the CEF message loop. This will block until CefQuitMessageLoop() is
  // called.
  CefRunMessageLoop();

  Debug("CefRunMessageLoop");

  // Shut down CEF.
  CefShutdown();
  Debug("CefShutdown");

  return 0;
}
