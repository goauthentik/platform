// Copyright (c) 2013 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#include "cefsimple/simple_app.h"

#include <string>
#include "ak_log.h"
#include "Credential.h"

#include "include/cef_browser.h"
#include "include/cef_command_line.h"
#include "include/views/cef_browser_view.h"
#include "include/views/cef_window.h"
#include "include/wrapper/cef_helpers.h"
#include "cefsimple/simple_handler.h"
#include "ak_cred_provider/include/resource.h"
#include "include/cef_image.h"
#include "authentik_sys_bridge/ffi.h"
#include "rust/cxx.h"

// GetModuleHandle(NULL) returns a handle to the module that was used to create the process.
// This fails when the resource is compiled into a DLL.
// Use `__ImageBase` instead provided a Microsoft compiler is used.
EXTERN_C IMAGE_DOS_HEADER __ImageBase;
#define HINST_THISCOMPONENT ((HINSTANCE)&__ImageBase)

namespace {

  class SimpleDeleteCookiesCallback : public CefDeleteCookiesCallback
{
  public:
  SimpleDeleteCookiesCallback(CefRefPtr<CefBrowserView> browser_view, CefRefPtr<CefWindow> window, CefRefPtr<SimpleHandler> handler,sHookData* pData)
    : m_pBrowserView(browser_view), m_pWindow(window), m_pHandler(handler), m_pData(pData) {}
  ~SimpleDeleteCookiesCallback() {}
  void OnComplete(int num_deleted) override
  {
    SPDLOG_DEBUG(std::string("DeleteCookiesCallback: " + std::to_string(num_deleted) + " cookies deleted").c_str());
    if (!(m_pHandler->CloseCalled()))
    {
      std::string url = "";
      try {
        AuthStartAsync start;
        if (!ak_sys_auth_start_async(start)) {
          SPDLOG_DEBUG("Failed to start auth async");
          return;
        }
        url = start.url.c_str();
        m_pData->UpdateHeaderToken(start.header_token.c_str());
      } catch (const rust::Error &ex) {
        SPDLOG_WARN("Exception in ak_sys_auth_start_async", ex.what());
      }
      SPDLOG_DEBUG(std::string("m_pBrowserView: " + std::to_string((size_t)(m_pBrowserView.get()))).c_str());
      SPDLOG_DEBUG(std::string("m_pBrowserView->GetBrowser(): " + std::to_string((size_t)(m_pBrowserView->GetBrowser().get()))).c_str());
      SPDLOG_DEBUG(std::string("m_pBrowserView->GetBrowser()->GetMainFrame(): " + std::to_string((size_t)(m_pBrowserView->GetBrowser()->GetMainFrame().get()))).c_str());
      m_pBrowserView->GetBrowser()->GetMainFrame()->LoadURL(url);
    }

    if (m_pWindow)
    {
      SPDLOG_DEBUG("Show window");
      m_pWindow->Show();
    }
    // Notify SimpleHandler
    m_pHandler->SetWindowInitialized(true);
  }
  private:
  CefRefPtr<CefBrowserView> m_pBrowserView = nullptr;
  CefRefPtr<CefWindow> m_pWindow = nullptr;
  CefRefPtr<SimpleHandler> m_pHandler = nullptr;
  sHookData* m_pData = nullptr;

  IMPLEMENT_REFCOUNTING(SimpleDeleteCookiesCallback);
  DISALLOW_COPY_AND_ASSIGN(SimpleDeleteCookiesCallback);
};

class SimpleCookieManagerCallback : public CefCompletionCallback
{
  public:
  SimpleCookieManagerCallback(CefRefPtr<CefBrowserView> browser_view, CefRefPtr<CefWindow> window, CefRefPtr<SimpleHandler> handler,sHookData* pData)
    : m_pDeleteCookiesCallback(new SimpleDeleteCookiesCallback(browser_view, window, handler, pData)) {}
  ~SimpleCookieManagerCallback() {}
  void OnComplete() override
  {
    SPDLOG_DEBUG("CookieManagerCallback");
    if (! (CefRequestContext::GetGlobalContext()))
    {
      SPDLOG_DEBUG("Error: CefRequestContext::GetGlobalContext is nullptr");
    }
    else
    {
      auto pCookieManager = CefRequestContext::GetGlobalContext()->GetCookieManager(nullptr);
      if (pCookieManager)
      {
        SPDLOG_DEBUG("CookieManager");
        pCookieManager->DeleteCookies("", "", m_pDeleteCookiesCallback);
      }
    }
  }
  private:
  CefRefPtr<SimpleDeleteCookiesCallback> m_pDeleteCookiesCallback = nullptr;

  IMPLEMENT_REFCOUNTING(SimpleCookieManagerCallback);
  DISALLOW_COPY_AND_ASSIGN(SimpleCookieManagerCallback);
};

// When using the Views framework this object provides the delegate
// implementation for the CefWindow that hosts the Views-based browser.
class SimpleWindowDelegate : public CefWindowDelegate {
 public:
  SimpleWindowDelegate(CefRefPtr<CefBrowserView> browser_view,
                       cef_runtime_style_t runtime_style,
                       cef_show_state_t initial_show_state,
                       CefRefPtr<SimpleHandler> handler,
                       sHookData* pData
                      )
      : browser_view_(browser_view),
        runtime_style_(runtime_style),
        initial_show_state_(initial_show_state),
        m_pHandler(handler),
        m_pData(pData),
        size_(CefSize(600, 700)) {}
  ~SimpleWindowDelegate() {}

  void OnWindowClosing(CefRefPtr<CefWindow> window) override {
    m_pCookieManagerCallback = nullptr;
  }

  void OnWindowCreated(CefRefPtr<CefWindow> window) override {
    SPDLOG_DEBUG("OnWindowCreated");
    m_pCookieManagerCallback = new SimpleCookieManagerCallback(browser_view_, window, m_pHandler, m_pData);
    CefCookieManager::GetGlobalManager(m_pCookieManagerCallback);

    // Add the browser view and show the window.
    window->AddChildView(browser_view_);
    window->SetAlwaysOnTop(false);
    // CefWindow::CenterWindow(...) considers taskbar which is not present in logon UI,
    // hence the manual calculation
    CefRect rect = CefDisplay::GetPrimaryDisplay()->GetBounds();
    CefPoint pt = window->GetPosition();
    pt.x = (rect.width - size_.width) / 2;
    pt.y = (rect.height - size_.height) / 2;
    window->SetPosition(pt);

    // Set CEF window icons
    HMODULE hModule = HINST_THISCOMPONENT; // get the handle to the current module
    if (hModule)
    {
      HRSRC hResource = FindResource(hModule, MAKEINTRESOURCE(IDB_TILE_IMAGE), RT_BITMAP); // substitute RESOURCE_ID and RESOURCE_TYPE.
      if (hResource)
      {
        HGLOBAL hMemory = LoadResource(hModule, hResource);
        if (hMemory)
        {
          DWORD dwSize = SizeofResource(hModule, hResource);
          LPVOID lpAddress = LockResource(hMemory);
          if (lpAddress)
          {
            int nWidth = 1;
            int nHeight = 1;
            std::vector<uint8_t> vecData;
            vecData.resize(dwSize); // 24-bit bitmap
            memcpy(vecData.data(), (LPVOID)((size_t)lpAddress+40), dwSize-40); // Copy but skip 40 byte bitmap header
            int nTmp = dwSize / 3;  // 24-bit pixel
            for (int i = 16; i <= 1024; ++i)
            {
              if ((nTmp / i) == i)
              {
                nWidth = nHeight = i;
                break;
              }
            }
            if (nWidth != 1)
            {
              // 24-bit bitmap to 32-bit bitmap
              std::vector<uint8_t> vecBytes = {};
              vecBytes.resize(nWidth * nHeight * 4);
              for (size_t j = 0; j < nHeight; ++j)
              {
                for (size_t i = 0; i < nWidth; ++i)
                {
                  // bitmap is upside down
                  vecBytes[((j*nWidth*4) + (i*4)) + 0] = vecData[(((nHeight - 1 - j)*nWidth*3) + (i*3) + 0)];
                  vecBytes[((j*nWidth*4) + (i*4)) + 1] = vecData[(((nHeight - 1 - j)*nWidth*3) + (i*3) + 1)];
                  vecBytes[((j*nWidth*4) + (i*4)) + 2] = vecData[(((nHeight - 1 - j)*nWidth*3) + (i*3) + 2)];
                  vecBytes[((j*nWidth*4) + (i*4)) + 3] = 0xFF;
                }
              }
              CefRefPtr<CefImage> pImage = CefImage::CreateImage();
              pImage->AddBitmap(1, nWidth, nHeight, CEF_COLOR_TYPE_BGRA_8888, CEF_ALPHA_TYPE_OPAQUE, vecBytes.data(), vecBytes.size());
              window->SetWindowAppIcon(pImage);
              window->SetWindowIcon(pImage);
            }
          }
        }
      }
    }

    // Disabled to show the window asynchronously upon Cookie Manager setup and cookie delete
    // if (initial_show_state_ != CEF_SHOW_STATE_HIDDEN) {
    //   window->Show();
    // }
  }

  void OnWindowDestroyed(CefRefPtr<CefWindow> window) override {
    browser_view_ = nullptr;
  }

  bool IsFrameless(CefRefPtr<CefWindow> window) override {
    return false;
  }

  bool CanResize(CefRefPtr<CefWindow> window) override {
    return false;
  }

  bool CanMinimize(CefRefPtr<CefWindow> window) override {
    return false;
  }

  bool CanMaximize(CefRefPtr<CefWindow> window) override {
    return false;
  }

  bool CanClose(CefRefPtr<CefWindow> window) override {
    // Allow the window to close if the browser says it's OK.
    CefRefPtr<CefBrowser> browser = browser_view_->GetBrowser();
    if (browser) {
      return browser->GetHost()->TryCloseBrowser();
    }
    return true;
  }

  CefSize GetPreferredSize(CefRefPtr<CefView> view) override {
    return size_;
  }

  cef_show_state_t GetInitialShowState(CefRefPtr<CefWindow> window) override {
    return initial_show_state_;
  }

  cef_runtime_style_t GetWindowRuntimeStyle() override {
    return runtime_style_;
  }

 private:
  CefRefPtr<CefBrowserView> browser_view_;
  const cef_runtime_style_t runtime_style_;
  const cef_show_state_t initial_show_state_;
  CefRefPtr<SimpleHandler> m_pHandler = nullptr;
  sHookData* m_pData = nullptr;
  const CefSize size_;
  CefRefPtr<SimpleCookieManagerCallback> m_pCookieManagerCallback = nullptr;

  IMPLEMENT_REFCOUNTING(SimpleWindowDelegate);
  DISALLOW_COPY_AND_ASSIGN(SimpleWindowDelegate);
};

class SimpleBrowserViewDelegate : public CefBrowserViewDelegate {
 public:
  explicit SimpleBrowserViewDelegate(cef_runtime_style_t runtime_style, CefRefPtr<SimpleHandler> handler, sHookData* pData)
      : runtime_style_(runtime_style), m_pHandler(handler), m_pData(pData) {}

  bool OnPopupBrowserViewCreated(CefRefPtr<CefBrowserView> browser_view,
                                 CefRefPtr<CefBrowserView> popup_browser_view,
                                 bool is_devtools) override {
    // Create a new top-level Window for the popup. It will show itself after
    // creation.
    CefWindow::CreateTopLevelWindow(new SimpleWindowDelegate(
        popup_browser_view, runtime_style_, CEF_SHOW_STATE_NORMAL, m_pHandler, m_pData));

    // We created the Window.
    return true;
  }

  cef_runtime_style_t GetBrowserRuntimeStyle() override {
    return runtime_style_;
  }

 private:
  const cef_runtime_style_t runtime_style_;
  CefRefPtr<SimpleHandler> m_pHandler = nullptr;
  sHookData* m_pData = nullptr;

  IMPLEMENT_REFCOUNTING(SimpleBrowserViewDelegate);
  DISALLOW_COPY_AND_ASSIGN(SimpleBrowserViewDelegate);
};

}  // namespace

SimpleApp::SimpleApp() = default;

void SimpleApp::OnContextInitialized() {
  CEF_REQUIRE_UI_THREAD();
  SPDLOG_DEBUG("OnContextInitialized");
  if (m_pData)
  {
    Credential::m_oCefAppData.SetInit(true);
  }
}

bool SimpleApp::LaunchBrowser(CefRefPtr<SimpleHandler> handler, const bool use_alloy_style) {
  SPDLOG_DEBUG("LaunchBrowser");
  CEF_REQUIRE_UI_THREAD();
  SPDLOG_DEBUG("LaunchBrowser UI thread");

  CefRefPtr<CefCommandLine> command_line =
      CefCommandLine::GetGlobalCommandLine();

  // Check if Alloy style will be used.
  cef_runtime_style_t runtime_style = CEF_RUNTIME_STYLE_DEFAULT;
  // bool use_alloy_style = command_line->HasSwitch("use-alloy-style");
  if (use_alloy_style) {
    runtime_style = CEF_RUNTIME_STYLE_ALLOY;
  }

  // SimpleHandler implements browser-level callbacks.
  // CefRefPtr<SimpleHandler> handler(new SimpleHandler(use_alloy_style, m_pData));

  // Specify CEF browser settings here.
  CefBrowserSettings browser_settings;

  std::string url = "";

  // Views is enabled by default (add `--use-native` to disable).
  const bool use_views = !command_line->HasSwitch("use-native");

  // If using Views create the browser using the Views framework, otherwise
  // create the browser using the native platform framework.
  if (use_views) {
    // Create the BrowserView.
    CefRefPtr<CefBrowserView> browser_view = CefBrowserView::CreateBrowserView(
        handler, url, browser_settings, nullptr, nullptr,
        new SimpleBrowserViewDelegate(runtime_style, handler, m_pData));

    // Optionally configure the initial show state.
    cef_show_state_t initial_show_state = CEF_SHOW_STATE_NORMAL;
    const std::string& show_state_value =
        command_line->GetSwitchValue("initial-show-state");
    if (show_state_value == "minimized") {
      initial_show_state = CEF_SHOW_STATE_MINIMIZED;
    } else if (show_state_value == "maximized") {
      initial_show_state = CEF_SHOW_STATE_MAXIMIZED;
    }
#if defined(OS_MAC)
    // Hidden show state is only supported on MacOS.
    else if (show_state_value == "hidden") {
      initial_show_state = CEF_SHOW_STATE_HIDDEN;
    }
#endif

    // Create the Window. It will show itself after creation.
    CefWindow::CreateTopLevelWindow(new SimpleWindowDelegate(
        browser_view, runtime_style, initial_show_state, handler, m_pData));
  } else {
    // Information used when creating the native window.
    CefWindowInfo window_info;

    SPDLOG_DEBUG("SetAsPopup");
#if defined(OS_WIN)
    // On Windows we need to specify certain flags that will be passed to
    // CreateWindowEx().
    window_info.SetAsPopup(nullptr, "cefsimple");
    // RECT rect;
    // GetClientRect(m_hWnd, &rect);
    // window_info.SetAsChild(m_hWnd, CefRect(rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top));
#endif
    SPDLOG_DEBUG("SetAsPopup end");

    // Alloy style will create a basic native window. Chrome style will create a
    // fully styled Chrome UI window.
    window_info.runtime_style = runtime_style;

    // Create the first browser window. Todo: Add cookie delete code.
    CefBrowserHost::CreateBrowser(window_info, handler, url, browser_settings,
                                  nullptr, nullptr);
    SPDLOG_DEBUG("CreateBrowser");
  }
  return true;
}

CefRefPtr<CefClient> SimpleApp::GetDefaultClient() {
  // Called when a new browser window is created via Chrome style UI.
  return SimpleHandler::GetInstance();
}
