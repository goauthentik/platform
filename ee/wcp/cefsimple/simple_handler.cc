// Copyright (c) 2013 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#include "cefsimple/simple_handler.h"

#include <sstream>
#include <string>

#include "include/base/cef_callback.h"
#include "include/cef_app.h"
#include "include/cef_parser.h"
#include "include/views/cef_browser_view.h"
#include "include/views/cef_window.h"
#include "include/wrapper/cef_closure_task.h"
#include "include/wrapper/cef_helpers.h"

namespace {

SimpleHandler* g_instance = nullptr;

// Returns a data: URI with the specified contents.
std::string GetDataURI(const std::string& data, const std::string& mime_type) {
  return "data:" + mime_type + ";base64," +
         CefURIEncode(CefBase64Encode(data.data(), data.size()), false)
             .ToString();
}

}  // namespace

SimpleHandler::SimpleHandler(bool is_alloy_style, sHookData* pData)
    : is_alloy_style_(is_alloy_style), m_pData(pData) {
  DCHECK(!g_instance);
  g_instance = this;
  SPDLOG_DEBUG("SimpleHandler");
}

SimpleHandler::~SimpleHandler() {
  g_instance = nullptr;
  SPDLOG_DEBUG("~SimpleHandler");
  if (m_pData)
  {
    SPDLOG_DEBUG("~SimpleHandler SetExit");
    m_pData->SetExit(true);
    m_pData = nullptr;
  }
}

// static
SimpleHandler* SimpleHandler::GetInstance() {
  return g_instance;
}

void SimpleHandler::OnTitleChange(CefRefPtr<CefBrowser> browser,
                                  const CefString& title) {
  CEF_REQUIRE_UI_THREAD();

  if (auto browser_view = CefBrowserView::GetForBrowser(browser)) {
    // Set the title of the window using the Views framework.
    CefRefPtr<CefWindow> window = browser_view->GetWindow();
    if (window) {
      window->SetTitle(title);
    }
  } else if (is_alloy_style_) {
    // Set the title of the window using platform APIs.
    PlatformTitleChange(browser, title);
  }
}

void SimpleHandler::OnAfterCreated(CefRefPtr<CefBrowser> browser) {
  CEF_REQUIRE_UI_THREAD();

  // Sanity-check the configured runtime style.
  CHECK_EQ(is_alloy_style_ ? CEF_RUNTIME_STYLE_ALLOY : CEF_RUNTIME_STYLE_CHROME,
           browser->GetHost()->GetRuntimeStyle());

  // Add to the list of existing browsers.
  browser_list_.push_back(browser);
}

bool SimpleHandler::DoClose(CefRefPtr<CefBrowser> browser) {
  CEF_REQUIRE_UI_THREAD();
  SPDLOG_DEBUG("DoClose");

  // Closing the main window requires special handling. See the DoClose()
  // documentation in the CEF header for a detailed destription of this
  // process.
  if (browser_list_.size() == 1) {
    // Set a flag to indicate that the window close should be allowed.
    is_closing_ = true;
  }

  // Allow the close. For windowed browsers this will result in the OS close
  // event being sent.
  return false;
}

void SimpleHandler::OnBeforeClose(CefRefPtr<CefBrowser> browser) {
  SPDLOG_DEBUG("OnBeforeClose any");
  CEF_REQUIRE_UI_THREAD();

  // wait for CefDeleteCookiesCallback::OnComplete() to complete
  if (! IsWindowInitialized())
  {
    SPDLOG_DEBUG(std::string("window_initialized_count_: " + std::to_string(window_initialized_count_)).c_str());
    SPDLOG_DEBUG(std::string("handler: " + std::to_string((size_t)(this))).c_str());
    if (window_initialized_count_ > 0) // timeout
    {
      --window_initialized_count_;
      // Execute on the UI thread for a later attempt
      CefPostDelayedTask(TID_UI, base::BindOnce(&SimpleHandler::OnBeforeClose, this, browser), 10);
      return;
    }
  }

  // Remove from the list of existing browsers.
  BrowserList::iterator bit = browser_list_.begin();
  for (; bit != browser_list_.end(); ++bit) {
    if ((*bit)->IsSame(browser)) {
      browser_list_.erase(bit);
      break;
    }
  }
  // todo: revise the above codeblock from cefsimple for select break;

  if (browser_list_.empty()) {
    // All browser windows have closed. Quit the application message loop.
    // CefQuitMessageLoop(); // only called for CefRunMessageLoop()
  }
  exit_message_loop_ = true; // to exit the message loop
}

void SimpleHandler::OnLoadError(CefRefPtr<CefBrowser> browser,
                                CefRefPtr<CefFrame> frame,
                                ErrorCode errorCode,
                                const CefString& errorText,
                                const CefString& failedUrl) {
  CEF_REQUIRE_UI_THREAD();

  // Don't display an error for downloaded files.
  if (errorCode == ERR_ABORTED) {
    return;
  }

  // Display a load error message using a data: URI.
  std::stringstream ss;
  // ss << "<html><body bgcolor=\"white\">"
  //       "<h2>Failed to load URL "
  //    << std::string(failedUrl) << " with error " << std::string(errorText)
  //    << " (" << errorCode << ").</h2></body></html>";
  ss << "<html><body bgcolor=\"white\">"
        "<h2>Failed to load remote resource.</h2></body></html>";

  frame->LoadURL(GetDataURI(ss.str(), "text/html"));
}

void SimpleHandler::ShowMainWindow() {
  if (!CefCurrentlyOn(TID_UI)) {
    // Execute on the UI thread.
    CefPostTask(TID_UI, base::BindOnce(&SimpleHandler::ShowMainWindow, this));
    return;
  }

  if (browser_list_.empty()) {
    return;
  }

  auto main_browser = browser_list_.front();

  if (auto browser_view = CefBrowserView::GetForBrowser(main_browser)) {
    // Show the window using the Views framework.
    if (auto window = browser_view->GetWindow()) {
      window->Show();
    }
  } else if (is_alloy_style_) {
    PlatformShowWindow(main_browser);
  }
}

void SimpleHandler::CloseAllBrowsers(bool force_close) {
  if (!CefCurrentlyOn(TID_UI)) {
    // Execute on the UI thread.
    CefPostTask(TID_UI, base::BindOnce(&SimpleHandler::CloseAllBrowsers, this,
                                       force_close));
    return;
  }
  SPDLOG_DEBUG("CloseAllBrowsers");
  close_called_ = true;

  if (browser_list_.empty()) {
    return;
  }

  BrowserList::const_iterator it = browser_list_.begin();
  for (; it != browser_list_.end(); ++it) {
    (*it)->GetHost()->CloseBrowser(force_close);
  }
}

void SimpleHandler::Show() {
  if (!CefCurrentlyOn(TID_UI)) {
    // Execute on the UI thread.
    CefPostTask(TID_UI, base::BindOnce(&SimpleHandler::Show, this));
    return;
  }

  if (browser_list_.empty()) {
    return;
  }

  BrowserList::const_iterator it = browser_list_.begin();
  for (; it != browser_list_.end(); ++it) {
    // (*it)->GetHost()->CloseBrowser(force_close);
    if (auto browser_view = CefBrowserView::GetForBrowser(*it)) {
      // Set the title of the window using the Views framework.
      CefRefPtr<CefWindow> window = browser_view->GetWindow();
      window->Show();
    }
  }
}

void SimpleHandler::Hide() {
  if (!CefCurrentlyOn(TID_UI)) {
    // Execute on the UI thread.
    CefPostTask(TID_UI, base::BindOnce(&SimpleHandler::Hide, this));
    return;
  }

  if (browser_list_.empty()) {
    return;
  }

  BrowserList::const_iterator it = browser_list_.begin();
  for (; it != browser_list_.end(); ++it) {
    // (*it)->GetHost()->CloseBrowser(force_close);
    if (auto browser_view = CefBrowserView::GetForBrowser(*it)) {
      // Set the title of the window using the Views framework.
      CefRefPtr<CefWindow> window = browser_view->GetWindow();
      window->Hide();
    }
  }
}

#if !defined(OS_MAC)
void SimpleHandler::PlatformShowWindow(CefRefPtr<CefBrowser> browser) {
  NOTIMPLEMENTED();
}




#endif
