// Copyright (c) 2013 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#ifndef CEF_TESTS_CEFSIMPLE_SIMPLE_HANDLER_H_
#define CEF_TESTS_CEFSIMPLE_SIMPLE_HANDLER_H_

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

// https://windows-cred-provider.pr.test.goauthentik.io
const std::string g_strTokenEndpoint = "/application/o/token/";

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
    std::string strURL = request->GetURL().ToString();
    Debug("URL: ");
    Debug(strURL.c_str());
    Debug(request->GetMethod().ToString().c_str());
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
        size_t nPos = strURL.find("code=") + 5;
        m_strCode = strURL.substr(nPos, strURL.find("&", nPos) - nPos);
        Debug(std::string("Code: " + m_strCode).c_str());
        nPos = strURL.find("state=") + 6;
        std::string strState = strURL.substr(nPos, strURL.find("&", nPos) - nPos);
        Debug(std::string("State:: " + strState).c_str());

        if (strState == m_strState) //- else notify error?
        {
          Hide();
          m_pData->UpdateStatus(L"Authenticating, please wait...");
          browser->GetMainFrame()->LoadURL(CefString(m_pData->strBaseURL + g_strTokenEndpoint));
        }
        else
        {
          MessageBox(
              NULL,
              (LPCWSTR)L"Server response is forged.",
              (LPCWSTR)L"Error",
              MB_OK
            );
        }

        return RV_CANCEL;
      }
    }
    if (strURL == (m_pData->strBaseURL + g_strTokenEndpoint))
    {
      std::string str = "Resource load: " + strURL;
      Debug(str.c_str());
      request->SetMethod("POST");

      //- todo: out to URLEncode
      std::string strPostData = "grant_type=authorization_code";
      strPostData += "&client_id=" + (m_pData->strClientID);
      strPostData += "&code=" + m_strCode;
      strPostData += "&code_verifier=" + m_strCodeVerifier;
      std::string strHash = "";
      strPostData += "&redirect_uri=goauthentik.io://windows/redirect";
      strPostData += "&scope=openid%20email%20profile%20offline_access%20windows";
      CefRefPtr<CefPostData> pPostData = CefPostData::Create();
      CefRefPtr<CefPostDataElement> pPostDataElement = CefPostDataElement::Create();
      pPostDataElement->SetToBytes(strPostData.size(), strPostData.c_str());
      pPostData->AddElement(pPostDataElement);
      request->SetPostData(pPostData);

      CefRequest::HeaderMap mapHeader;
      request->GetHeaderMap(mapHeader);
      if (mapHeader.find("Content-Type") != mapHeader.end())
      {
        mapHeader.erase("Content-Type");
      }
      mapHeader.insert(std::make_pair<CefString, CefString>("Content-Type", "application/x-www-form-urlencoded"));
      if (mapHeader.find("Accept") != mapHeader.end())
      {
        mapHeader.erase("Accept");
      }
      mapHeader.insert(std::make_pair<CefString, CefString>("Accept", "application/json"));
      // Must not set Content-Length, its auto-set.
      request->SetHeaderMap(mapHeader);
      pPostDataElement = (std::nullptr_t)NULL;
      pPostData = (std::nullptr_t)NULL;
    }

    return RV_CONTINUE;
  }

  CefRefPtr<CefResponseFilter> GetResourceResponseFilter(
    CefRefPtr<CefBrowser> browser,
    CefRefPtr<CefFrame> frame,
    CefRefPtr<CefRequest> request,
    CefRefPtr<CefResponse> response) {

    //CEF_REQUIRE_IO_THREAD();
    const std::string& url = request->GetURL();

    if ((url == (m_pData->strBaseURL + g_strTokenEndpoint)))
    {
      m_strResponseContent = ""; // reset
      return this;
    }

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

  void OnResourceLoadComplete(
    CefRefPtr<CefBrowser> browser,
    CefRefPtr<CefFrame> frame,
    CefRefPtr<CefRequest> request,
    CefRefPtr<CefResponse> response,
    URLRequestStatus status,
    int64_t received_content_length
  ) override {
    const std::string& url = request->GetURL();

    if (url == (m_pData->strBaseURL + g_strTokenEndpoint))
    {
      const std::string strTokenKey = "\"access_token\": \"";
      size_t nPos = m_strResponseContent.find(strTokenKey) + strTokenKey.size();
      m_strJWT = m_strResponseContent.substr(nPos, m_strResponseContent.find("\"", nPos) - nPos);
      m_strResponseContent = ""; // Delete JWT for security //- todo: do a proper overwrite
      Debug("JWT:");
      Debug(m_strJWT.c_str());
      auto validatedToken = TokenResponse();
      try {
        if (!ak_sys_token_validate("", m_strJWT, validatedToken)) {
          SPDLOG_WARN("failed to validate token");
        }
        m_pData->UpdateUser(validatedToken.username.c_str());
      } catch (const std::exception &ex) {
        Debug("Exception in ak_sys_token_validate");
        Debug(ex.what());
      }
      CloseAllBrowsers(false);
    }
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
