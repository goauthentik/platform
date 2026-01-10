// Copyright (c) 2013 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#include <windows.h>

#include "include/cef_command_line.h"
#include "include/cef_sandbox_win.h"
// #include "cefsimple/simple_app.h"
#include "cefsimple/simple_handler.h"
#include "cefsimple/cefsimple_win.h"
#include "ak_cred_provider/include/ak_log.h"
#include "crypt.h"
#include "Credential.h"
#include <ak_sentry.h>

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

int CEFLaunch(sHookData* pData, CefRefPtr<SimpleApp> pCefApp) {
  SPDLOG_DEBUG("CEFLaunch(...)");
  SPDLOG_DEBUG(std::string("CEFLaunch(...):  " + std::to_string((size_t)(Credential::m_oCefAppData.IsInit()))).c_str());
  MSG msg;
  // wait for CefApp initialization (call to CefBrowserProcessHandler::OnContextInitialized())
  while (! (Credential::m_oCefAppData.IsInit()))
  {
    if (pData->pqcws->QueryContinue() != S_OK) // Cancel button clicked
    {
      return -1; // no browser launch
    }
    if (PeekMessage(&msg, NULL, 0, 0, PM_REMOVE) != 0)
    {
      TranslateMessage(&msg);
      DispatchMessage(&msg);
    }
    Sleep(1);
  }
  SPDLOG_DEBUG("CEFLaunch first loop end");

  CefRefPtr<CefCommandLine> command_line =
      CefCommandLine::GetGlobalCommandLine();

  bool use_alloy_style = command_line->HasSwitch("use-alloy-style");
  CefRefPtr<SimpleHandler> pHandler(new SimpleHandler(use_alloy_style, pData));
  if (! (pCefApp->LaunchBrowser(pHandler, use_alloy_style)))
  {
    SPDLOG_DEBUG("CEFLaunch: LaunchBrowser failed. Exit.");
    pHandler = nullptr;
    // perform (at max) 10 precautionary loops even though 1 `CefDoMessageLoopWork()`
    // seems to be sufficient
    for (size_t i = 0; i < 10; ++i)
    {
      if (PeekMessage(&msg, NULL, 0, 0, PM_REMOVE) != 0)
      {
        TranslateMessage(&msg);
        DispatchMessage(&msg);
      }
      SPDLOG_DEBUG(std::string("Sub-loop failed exit: " + std::to_string(i)).c_str());
      // pHandler (SimpleHandler) destructor called
      if (pData->IsExit())
      {
        break;
      }
      Sleep(3);
    }
    return -1;
  }

  SPDLOG_DEBUG("CefRunMessageLoop");

  // Run custom message loop inside the WndProc and process the main window
  // as well as the CEF messages
  while (!(pHandler->ExitMessageLoop()))
  {
    if (pData->pqcws->QueryContinue() != S_OK) // Cancel button clicked
    {
      SPDLOG_DEBUG("Sub-loop");
      pHandler->CloseAllBrowsers(true);
      pData->UpdateUser("");
      // pData->SetCancel(true);
      // // perform (at max) 10 precautionary loops even though 1 `CefDoMessageLoopWork()`
      // // seems to be sufficient
      // for (size_t i = 0; i < 10; ++i)
      // {
      //   if (PeekMessage(&msg, NULL, 0, 0, PM_REMOVE) != 0)
      //   {
      //     TranslateMessage(&msg);
      //     DispatchMessage(&msg);
      //   }
      //   CefDoMessageLoopWork();
      //   if (pData->IsExit())
      //   {
      //     break;
      //   }
      //   SPDLOG_DEBUG(std::string("Sub-loop: " + std::to_string(i)).c_str());
      //   Sleep(3);
      // }
      break;
    }
    if (PeekMessage(&msg, NULL, 0, 0, PM_REMOVE) != 0)
    {
      TranslateMessage(&msg);
      DispatchMessage(&msg);
    }
    CefDoMessageLoopWork();
    Sleep(5); // as precaution to relieve the CPU (though unlikely that its needed)
  }
  pHandler = nullptr; // Release for the destructor to be called subsequently
  if (pData->strUsername == "") // User clicked the close button or cancel
  {
    pData->SetCancel(true);
  }
  // perform (at max) 10 precautionary loops even though 1 `CefDoMessageLoopWork()`
  // seems to be sufficient
  for (size_t i = 0; i < 10; ++i)
  {
    if (PeekMessage(&msg, NULL, 0, 0, PM_REMOVE) != 0)
    {
      TranslateMessage(&msg);
      DispatchMessage(&msg);
    }
    CefDoMessageLoopWork();
    SPDLOG_DEBUG(std::string("Sub-loop: " + std::to_string(i)).c_str());
    // pHandler (SimpleHandler) destructor called
    if (pData->IsExit())
    {
      break;
    }
    Sleep(3);
  }

  SPDLOG_DEBUG("CefRunMessageLoop end");
  return 0;
}
