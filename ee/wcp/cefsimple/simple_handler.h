// Copyright (c) 2013 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#ifndef CEF_TESTS_CEFSIMPLE_SIMPLE_HANDLER_H_
#define CEF_TESTS_CEFSIMPLE_SIMPLE_HANDLER_H_

#include "rust/cxx.h"
#include "authentik_sys_bridge/ffi.h"
#include <spdlog/spdlog.h>

#include <string>
#include "crypt.h"

#define OAUTH_CHALLENGE_LEN 64
#define OAUTH_STATE_LEN 10

#include <list>

#include "include/cef_client.h"
#include "include/cef_command_ids.h"
#include "ak_common/include/ak_log.h"

#include "Credential.h"

class SimpleHandler : public CefClient,
                      public CefDisplayHandler,
                      public CefLifeSpanHandler,
                      public CefLoadHandler,
                      public CefRequestHandler,
                      public CefResourceRequestHandler,
                      public CefResponseFilter,
                      public CefContextMenuHandler,
                      public CefCommandHandler {
 public:
  explicit SimpleHandler(bool is_alloy_style, sHookData* pData);
  ~SimpleHandler() override;

  // Provide access to the single global instance of this object.
  static SimpleHandler* GetInstance();

  sHookData* m_pData = nullptr;

  // CefClient methods:
  CefRefPtr<CefDisplayHandler> GetDisplayHandler() override { return this; }
  CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() override { return this; }
  CefRefPtr<CefLoadHandler> GetLoadHandler() override { return this; }
  CefRefPtr<CefRequestHandler> GetRequestHandler() override { return this; }
  CefRefPtr<CefContextMenuHandler> GetContextMenuHandler() override { return this; }
  CefRefPtr<CefCommandHandler> GetCommandHandler() override { return this; }
  void OnBeforeContextMenu(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                           CefRefPtr<CefContextMenuParams> params,
                           CefRefPtr<CefMenuModel> model) override {
    model->Clear();
    model->AddItem(IDC_RELOAD, "Reload");
  }
  bool OnChromeCommand(CefRefPtr<CefBrowser> browser, int command_id,
                       cef_window_open_disposition_t disposition) override {
    return (command_id != IDC_RELOAD);
  }
  CefRefPtr<CefResourceRequestHandler> GetResourceRequestHandler(
      CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, CefRefPtr<CefRequest> request,
      bool is_navigation, bool is_download, const CefString& request_initiator,
      bool& disable_default_handling) override {
    return this;
  }

  bool OnResourceResponse(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                          CefRefPtr<CefRequest> request, CefRefPtr<CefResponse> response) override {
    std::string strURL =
        "URL: " + request->GetURL().ToString() + " " + request->GetMethod().ToString();
    spdlog::debug(strURL.c_str());
    std::string str = "OnResourceResponse ProcessID: " + std::to_string(GetCurrentProcessId()) +
                      ", ThreadID: " + std::to_string(GetCurrentThreadId());
    spdlog::debug(str.c_str());
    return false;
  }

  ReturnValue OnBeforeResourceLoad(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                                   CefRefPtr<CefRequest> request,
                                   CefRefPtr<CefCallback> callback) override {
    const std::string strKey = "goauthentik.io://";
    std::string strURL = request->GetURL().ToString();
    spdlog::debug(strURL.c_str());

    CefString headerKey;
    headerKey.FromString("X-Authentik-Platform-Auth-DTH");
    CefString headerValue;
    headerValue.FromString(m_pData->strHeaderToken);

    request->SetHeaderByName(headerKey, headerValue, true);
    if (strURL.length() >= strKey.length()) {
      if (strURL.substr(0, strKey.length()) == strKey) {
        spdlog::debug("URL inhibited: ", strURL.c_str());
        spdlog::debug("OnBeforeResourceLoad ProcessID: ", std::to_string(GetCurrentProcessId()),
                      ", ThreadID: ", std::to_string(GetCurrentThreadId()));
        Hide();
        m_pData->UpdateStatus(L"Authenticating, please wait...");
        std::string validatedToken;
        try {
          ak_sys_auth_url(strURL, validatedToken);
          spdlog::debug("successfully validated token");
          m_pData->UpdateUserToken(validatedToken);
        } catch (const rust::Error& ex) {
          SPDLOG_WARN("failed to validate token");
          SPDLOG_WARN("Exception in ak_sys_auth_url: ", ex.what());
        }
        CloseAllBrowsers(false);

        return RV_CANCEL;
      }
    }
    return RV_CONTINUE;
  }

  CefRefPtr<CefResponseFilter> GetResourceResponseFilter(CefRefPtr<CefBrowser> browser,
                                                         CefRefPtr<CefFrame> frame,
                                                         CefRefPtr<CefRequest> request,
                                                         CefRefPtr<CefResponse> response) {
    return nullptr;
  }

  bool InitFilter() override { return true; }
  FilterStatus Filter(void* data_in, size_t data_in_size, size_t& data_in_read, void* data_out,
                      size_t data_out_size, size_t& data_out_written) override {
    return RESPONSE_FILTER_DONE;
  }

  // CefDisplayHandler methods:
  void OnTitleChange(CefRefPtr<CefBrowser> browser, const CefString& title) override;

  // CefLifeSpanHandler methods:
  void OnAfterCreated(CefRefPtr<CefBrowser> browser) override;
  bool DoClose(CefRefPtr<CefBrowser> browser) override;
  void OnBeforeClose(CefRefPtr<CefBrowser> browser) override;

  // CefLoadHandler methods:
  void OnLoadError(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, ErrorCode errorCode,
                   const CefString& errorText, const CefString& failedUrl) override;

  void ShowMainWindow();

  // Request that all existing browser windows close.
  void CloseAllBrowsers(bool force_close);
  void Show();
  void Hide();

  bool IsClosing() const { return is_closing_; }
  bool CloseCalled() const { return close_called_; }
  bool ExitMessageLoop() { return exit_message_loop_; }
  void SetWindowInitialized(const bool bVal) { window_initialized_ = bVal; }
  bool IsWindowInitialized() { return window_initialized_; }

 private:
  // Platform-specific implementation.
  void PlatformTitleChange(CefRefPtr<CefBrowser> browser, const CefString& title);
  void PlatformShowWindow(CefRefPtr<CefBrowser> browser);

  // True if this client is Alloy style, otherwise Chrome style.
  const bool is_alloy_style_;

  // List of existing browser windows. Only accessed on the CEF UI thread.
  typedef std::list<CefRefPtr<CefBrowser>> BrowserList;
  BrowserList browser_list_;

  bool is_closing_ = false;
  bool exit_message_loop_ = false;        // to exit the CEF message loop
  bool window_initialized_ = false;       // CefDeleteCookiesCallback::OnComplete() called
  bool close_called_ = false;             // CloseAllBrowsers(...) called
  size_t window_initialized_count_ = 10;  // timeout while waiting for window_initialized_

  // Include the default reference counting implementation.
  IMPLEMENT_REFCOUNTING(SimpleHandler);
};

#endif  // CEF_TESTS_CEFSIMPLE_SIMPLE_HANDLER_H_
