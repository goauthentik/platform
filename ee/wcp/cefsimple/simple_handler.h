// Copyright (c) 2013 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#ifndef CEF_TESTS_CEFSIMPLE_SIMPLE_HANDLER_H_
#define CEF_TESTS_CEFSIMPLE_SIMPLE_HANDLER_H_

#pragma warning(push)
#pragma warning(disable: 4005)
#include <jwt-cpp/jwt.h>
#pragma warning(pop)
#include <openssl/rand.h>
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
#include "Debug.h"

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
  std::string m_strResponseContent = "";
  std::string m_strJWT = "";

  std::string GetCodeVerifier() { return m_strCodeVerifier; }
  std::string GetState() { return m_strState; }
  std::string m_strCode = "";
  std::string m_strCodeVerifier = "";
  std::string m_strCodeChallenge = "";
  std::string m_strState = "";

  // CefClient methods:
  CefRefPtr<CefDisplayHandler> GetDisplayHandler() override { return this; }
  CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() override { return this; }
  CefRefPtr<CefLoadHandler> GetLoadHandler() override { return this; }
  CefRefPtr<CefRequestHandler> GetRequestHandler() override { return this; }
  CefRefPtr<CefContextMenuHandler> GetContextMenuHandler() override { return this; }
  CefRefPtr<CefCommandHandler> GetCommandHandler() override { return this; }
  void OnBeforeContextMenu(
    CefRefPtr<CefBrowser> browser,
    CefRefPtr<CefFrame> frame,
    CefRefPtr<CefContextMenuParams> params,
    CefRefPtr<CefMenuModel> model
  ) override {
      model->Clear();
      model->AddItem(IDC_RELOAD, "Reload");
  }
  bool OnChromeCommand(
    CefRefPtr<CefBrowser> browser,
    int command_id,
    cef_window_open_disposition_t disposition
  ) override {
    return (command_id != IDC_RELOAD);
  }
  CefRefPtr<CefResourceRequestHandler> GetResourceRequestHandler(
    CefRefPtr<CefBrowser> browser,
    CefRefPtr<CefFrame> frame,
    CefRefPtr<CefRequest> request,
    bool is_navigation,
    bool is_download,
    const CefString& request_initiator,
    bool& disable_default_handling
  ) override { return this; }
  bool OnResourceResponse(
    CefRefPtr<CefBrowser> browser,
    CefRefPtr<CefFrame> frame,
    CefRefPtr<CefRequest> request,
    CefRefPtr<CefResponse> response
  ) override {
    std::string strURL = "URL: " + request->GetURL().ToString() + " " + request->GetMethod().ToString();
    Debug(strURL.c_str());
    std::string str = "OnResourceResponse ProcessID: " + std::to_string(GetCurrentProcessId()) + ", ThreadID: " + std::to_string(GetCurrentThreadId());
    Debug(str.c_str());

    return false;
  }
  ReturnValue OnBeforeResourceLoad(
    CefRefPtr<CefBrowser> browser,
    CefRefPtr<CefFrame> frame,
    CefRefPtr<CefRequest> request,
    CefRefPtr<CefCallback> callback
  ) override {
    const std::string strKey = "goauthentik.io://";
    std::string strURL = request->GetURL().ToString();
    Debug(strURL.c_str());
    if (strURL.length() >= strKey.length())
    {
      if (strURL.substr(0, strKey.length()) == strKey)
      {
        Debug("URL inhibited: ");
        //Debug(std::hash<std::thread::id>{}(std::thread::get_id));
        Debug(strURL.c_str());
        std::string str = "OnBeforeResourceLoad ProcessID: " + std::to_string(GetCurrentProcessId()) + ", ThreadID: " + std::to_string(GetCurrentThreadId());
        Debug(str.c_str());
        Hide();
        m_pData->UpdateStatus(L"Authenticating, please wait...");
        TokenResponse validatedToken;
        try {
          if (!ak_sys_auth_url(strURL, validatedToken)) {
            SPDLOG_WARN("failed to validate token");
          } else {
            SPDLOG_DEBUG("successfully validated token");
            m_pData->UpdateUser(validatedToken.username.c_str());
          }
        } catch (const rust::Error &ex) {
          Debug("Exception in ak_sys_auth_url");
          Debug(ex.what());
        }
        CloseAllBrowsers(false);

        return RV_CANCEL;
      }
    }
    return RV_CONTINUE;
  }

  CefRefPtr<CefResponseFilter> GetResourceResponseFilter(
    CefRefPtr<CefBrowser> browser,
    CefRefPtr<CefFrame> frame,
    CefRefPtr<CefRequest> request,
    CefRefPtr<CefResponse> response) {
    return nullptr;
  }

  bool InitFilter() override { return true; }
  FilterStatus Filter(void* data_in,
      size_t data_in_size,
      size_t& data_in_read,
      void* data_out,
      size_t data_out_size,
      size_t& data_out_written
  ) override {
      // const size_t max_write = std::min(data_in_size, data_out_size);
      // memcpy(data_out, data_in, max_write);
      // data_out_size = max_write; //?
      // data_out_written = data_in_size; //?
      // LOG(INFO) << "The data here:" << std::string((char*)data_out);
      // data_in_read = data_in_size;
      // return RESPONSE_FILTER_DONE;
      const size_t max_write = std::min(data_in_size, data_out_size);
      //-memcpy(data_out, data_in, max_write);
      // data_out_size = max_write; //?
      // data_out_written = data_in_size; //?
      //-data_out_written = max_write;
      // LOG(INFO) << "The data here:" << std::string((char*)data_out);
      data_in_read = data_in_size;

      // Alternate output message to display
      std::string strDataOut = "";
      // Only generate output at the first trigger of this method to avoid multiples of the display message
      if (m_strResponseContent != "")
      {
        strDataOut = "Authenticating... please wait";
        // check the output buffer size sufficiency while copying
        memcpy(data_out, (void*)(strDataOut.c_str()), std::min(strDataOut.size(), data_out_size));
      }
      data_out_written = strDataOut.size();

      // Append-copy input data as the input may arrive in parts (multiple triggers of this method)
      m_strResponseContent += std::string((char*)data_in, data_in_size);

      std::string strIn = "";
      size_t nCount = 0;
      for (size_t i = 0; i < data_in_size; ++i)
      {
        strIn.append(1, ((char*)data_in)[i]);
        ++nCount;
      }
      std::string strOut = "";
      for (size_t i = 0; i < data_out_written; ++i)
      {
        strOut.append(1, ((char*)data_out)[i]);
      }
      Debug("-------------------");
      Debug(std::string("max_write:" + std::to_string(max_write)).c_str());
      Debug(std::string("Data in: Size:" + std::to_string(data_in_size) + " Read: " + std::to_string(data_in_read)).c_str());
      Debug(strIn.c_str());
      Debug(std::string("Data out: Size:" + std::to_string(data_out_size) + " Written: " + std::to_string(data_out_written)).c_str());
      Debug(strOut.c_str());
      Debug("-------------------");
      return RESPONSE_FILTER_DONE; //- todo: check for residual data
  }

  // CefDisplayHandler methods:
  void OnTitleChange(CefRefPtr<CefBrowser> browser,
                     const CefString& title) override;

  // CefLifeSpanHandler methods:
  void OnAfterCreated(CefRefPtr<CefBrowser> browser) override;
  bool DoClose(CefRefPtr<CefBrowser> browser) override;
  void OnBeforeClose(CefRefPtr<CefBrowser> browser) override;

  // CefLoadHandler methods:
  void OnLoadError(CefRefPtr<CefBrowser> browser,
                   CefRefPtr<CefFrame> frame,
                   ErrorCode errorCode,
                   const CefString& errorText,
                   const CefString& failedUrl) override;

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
  void PlatformTitleChange(CefRefPtr<CefBrowser> browser,
                           const CefString& title);
  void PlatformShowWindow(CefRefPtr<CefBrowser> browser);

  // True if this client is Alloy style, otherwise Chrome style.
  const bool is_alloy_style_;

  // List of existing browser windows. Only accessed on the CEF UI thread.
  typedef std::list<CefRefPtr<CefBrowser>> BrowserList;
  BrowserList browser_list_;

  bool is_closing_ = false;
  bool exit_message_loop_ = false;  // to exit the CEF message loop
  bool window_initialized_ = false; // CefDeleteCookiesCallback::OnComplete() called
  bool close_called_ = false;       // CloseAllBrowsers(...) called
  size_t window_initialized_count_ = 10;  // timeout while waiting for window_initialized_

  // Include the default reference counting implementation.
  IMPLEMENT_REFCOUNTING(SimpleHandler);
};

#endif  // CEF_TESTS_CEFSIMPLE_SIMPLE_HANDLER_H_
