#include "pch.h"

#include "Debug.h"
#include "Provider.h"
#include "ak_version.h"
#include "authentik_sys_bridge/ffi.h"

#include "include/cef_command_line.h"
#include "include/cef_sandbox_win.h"
#include "include/cef_version.h"

extern void DllAddRef();
extern void DllRelease();
extern std::string g_strPath;

Provider::Provider() : m_cRef(1) {
  // Setup callbacks
  Credential::m_pProvCallSet = [this](sHookData *pData) {
    this->SetCefApp(pData);
  };
  Credential::m_pProvCallShut = [this]() { this->ShutCefApp(); };

  DllAddRef();
}

void Provider::SetCefApp(sHookData *pData) {
  Debug("SetCefApp");
  if (m_pCefApp == nullptr) {
    int exit_code;

    std::string str =
        "SetCefApp ProcessID: " + std::to_string(GetCurrentProcessId()) +
        ", ThreadID: " + std::to_string(GetCurrentThreadId());
    Debug(str.c_str());

#if defined(ARCH_CPU_32_BITS) //- todo: remove?
    // Run the main thread on 32-bit Windows using a fiber with the preferred
    // 4MiB stack size. This function must be called at the top of the
    // executable entry point function (`main()` or `wWinMain()`). It is used in
    // combination with the initial stack size of 0.5MiB configured via the
    // `/STACK:0x80000` linker flag on executable targets. This saves
    // significant memory on threads (like those in the Windows thread pool, and
    // others) whose stack size can only be controlled via the linker flag.
    exit_code = CefRunWinMainWithPreferredStackSize(
        wWinMain, hInstance, lpCmdLine, SW_NORMAL); // nCmdShow: SW_NORMAL
                                                    // if (exit_code >= 0) {
    //   // The fiber has completed so return here.
    //   return exit_code;
    // }
#endif

    void *sandbox_info = nullptr;

#if defined(CEF_USE_SANDBOX)
    // Manage the life span of the sandbox information object. This is necessary
    // for sandbox support on Windows. See cef_sandbox_win.h for complete
    // details.
    CefScopedSandboxInfo scoped_sandbox;
    sandbox_info = scoped_sandbox.sandbox_info();
#endif
    Debug("CefScopedSandboxInfo");
    // Provide CEF with command-line arguments.
    CefMainArgs main_args((HINSTANCE)GetModuleHandle(NULL));

    exit_code = 0;

    Debug("CefMainArgs");
    // CEF applications have multiple sub-processes (render, GPU, etc) that
    // share the same executable. This function checks the command-line and, if
    // this is a sub-process, executes the appropriate logic.

    // exit_code = CefExecuteProcess(main_args, nullptr, sandbox_info);
    // Debug("CefExecuteProcess");
    // if (exit_code >= 0) {
    //     Debug("Cef: exit_code");
    //   // The sub-process has completed so return here.
    //   return exit_code;
    // }

    Debug("CefCommandLine::CreateCommandLine");
    // Parse command-line arguments for use in this method.
    CefRefPtr<CefCommandLine> command_line =
        CefCommandLine::CreateCommandLine();
    command_line->InitFromString(::GetCommandLineW());

    // Specify CEF global settings here.
    CefSettings settings;

    // Specify the path for the sub-process executable.
    std::string strPath = g_strPath + "\\ak_cef.exe";
    CefString(&settings.browser_subprocess_path).FromASCII(strPath.c_str());
    // std::string strRPath = g_strPath + "\\" + GetRandomStr(5);
    // CefString(&settings.root_cache_path).FromASCII(strRPath.c_str());
    // CefString(&settings.cache_path).FromASCII(std::string(strRPath +
    // "\\CPath" + GetRandomStr(5)).c_str());

    strPath = g_strPath + "\\ceflog.txt";
    CefString(&settings.log_file).FromASCII(strPath.c_str());
    settings.log_severity = LOGSEVERITY_INFO;

    std::string strUserAgent = std::string("authentik Platform/WCP/CredProvider@")
        .append(AK_WCP_VERSION)
        .append(" (CEF ")
        .append(CEF_VERSION)
        .append(")");
    CefString(&settings.user_agent).FromString(strUserAgent);

#if !defined(CEF_USE_SANDBOX)
    settings.no_sandbox = true;
#endif

    settings.multi_threaded_message_loop = false;

    Debug("CefSettings");
    // SimpleApp implements application-level callbacks for the browser process.
    // It will create the first browser instance in OnContextInitialized() after
    // CEF has initialized.
    // CefRefPtr<SimpleApp> app(new SimpleApp());
    m_pCefApp = new SimpleApp(pData);
    Debug("Cef: new SimpleApp");

    Debug(std::string("app.get:::" + std::to_string((size_t)(m_pCefApp.get())))
              .c_str());
    // Initialize the CEF browser process. May return false if initialization
    // fails or if early exit is desired (for example, due to process singleton
    // relaunch behavior).
    if (!CefInitialize(main_args, settings, m_pCefApp.get(), sandbox_info)) {
      Debug("CefGetExitCode");
      // return CefGetExitCode();
      m_pCefApp = nullptr;
    }
    Debug("CefInitialize");
    // Run the CEF message loop. This will block until CefQuitMessageLoop() is
    // called.
    // CefRunMessageLoop();

    // Debug("CefRunMessageLoop");

    // // Shut down CEF.
    // CefShutdown();
    // Debug("CefShutdown");
    Credential::m_oCefAppData.pCefApp = m_pCefApp;
  }
}

void Provider::ShutCefApp() {
  Debug("ShutCefApp");
  if (m_pCefApp) {
    Debug("CefShutdown");
    Credential::m_oCefAppData.SetInit(false);
    Credential::m_oCefAppData.pCefApp = nullptr;
    // Shut down CEF.
    CefShutdown();
    m_pCefApp = nullptr;
    Debug("CefShutdown end");
  }
}

Provider::~Provider() {
  Debug("~Provider");
  ReleaseEnumeratedCredentials();
  if (m_pCredProviderUserArray != nullptr) {
    m_pCredProviderUserArray->Release();
    m_pCredProviderUserArray = nullptr;
  }

  DllRelease();
}

IFACEMETHODIMP Provider::QueryInterface(__in REFIID riid,
                                        __deref_out void **ppv) {
  static const QITAB qit[] = {
      QITABENT(Provider, ICredentialProvider), // IID_ICredentialProvider
      QITABENT(
          Provider,
          ICredentialProviderSetUserArray), // IID_ICredentialProviderSetUserArray
      {0},
  };
  return QISearch(this, qit, riid, ppv);
}

IFACEMETHODIMP_(ULONG) Provider::AddRef() {
  return InterlockedIncrement(&m_cRef);
}

IFACEMETHODIMP_(ULONG) Provider::Release() {
  if (InterlockedDecrement(&m_cRef) == 0) {
    delete this;
    return 0;
  }
  return m_cRef;
}

// Called by LogonUI to give you a callback.  Providers often use the callback
// if they some event would cause them to need to change the set of tiles that
// they enumerated.
IFACEMETHODIMP Provider::Advise(_In_ ICredentialProviderEvents * /*pcpe*/,
                                _In_ UINT_PTR /*upAdviseContext*/) {
  return E_NOTIMPL;
}

// Called by LogonUI when the ICredentialProviderEvents callback is no longer
// valid.
IFACEMETHODIMP Provider::UnAdvise() { return E_NOTIMPL; }

// SetUsageScenario is the provider's cue that it's going to be asked for tiles
// in a subsequent call.
IFACEMETHODIMP
Provider::SetUsageScenario(CREDENTIAL_PROVIDER_USAGE_SCENARIO cpus,
                           DWORD dwFlags) {
  HRESULT hr;

  if (!ak_sys_auth_interactive_available()) {
    Debug("Interactive authentication not available");
    hr = E_NOTIMPL;
    return hr;
  }

  // Decide which scenarios to support here. Returning E_NOTIMPL simply tells
  // the caller that we're not designed for that scenario.
  switch (cpus) {
  case CPUS_LOGON:
  case CPUS_UNLOCK_WORKSTATION:
    //// The reason why we need _fRecreateEnumeratedCredentials is because
    /// ICredentialProviderSetUserArray::SetUserArray() is called after
    /// ICredentialProvider::SetUsageScenario(), / while we need the
    /// ICredentialProviderUserArray during enumeration in
    /// ICredentialProvider::GetCredentialCount()
    m_cpus = cpus; // Save usage scenario
    m_fRecreateEnumeratedCredentials =
        true; // Recreate credentials anyways (in all cases)...
    hr = S_OK;
    break;

  case CPUS_CHANGE_PASSWORD:
  case CPUS_CREDUI:
    hr = E_NOTIMPL;
    break;

  default:
    hr = E_INVALIDARG;
    break;
  }

  return hr;
}

// Not implemented, even though its required as per MS docs... //-
IFACEMETHODIMP Provider::SetSerialization(
    _In_ CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION const * /*pcpcs*/) {
  return E_NOTIMPL;
}

// Called by LogonUI to determine the number of fields in your tiles.  This
// does mean that all your tiles must have the same number of fields.
// This number must include both visible and invisible fields. If you want a
// tile to have different fields from the other tiles you enumerate for a given
// usage scenario you must include them all in this count and then hide/show
// them as desired using the field descriptors.
IFACEMETHODIMP Provider::GetFieldDescriptorCount(_Out_ DWORD *pdwCount) {
  *pdwCount = FI_NUM_FIELDS;
  return S_OK;
}

// Gets the field descriptor for a particular field.
IFACEMETHODIMP Provider::GetFieldDescriptorAt(
    DWORD dwIndex,
    _Outptr_result_nullonfailure_ CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR *
        *ppcpfd) {
  HRESULT hr = E_INVALIDARG;
  *ppcpfd = nullptr;

  // Verify dwIndex is a valid field.
  if ((dwIndex < FI_NUM_FIELDS) && ppcpfd) {
    hr = FieldDescriptorCoAllocCopy(s_rgCredProvFieldDescriptors[dwIndex],
                                    ppcpfd);
  } else {
    hr = E_INVALIDARG;
  }

  return hr;
}

// Sets pdwCount to the number of tiles that we wish to show at this time.
// Sets pdwDefault to the index of the tile which should be used as the default.
// The default tile is the tile which will be shown in the zoomed view by
// default. If more than one provider specifies a default the last used cred
// prov gets to pick the default. If *pbAutoLogonWithDefault is TRUE, LogonUI
// will immediately call GetSerialization on the credential you've specified as
// the default and will submit that credential for authentication without
// showing any further UI.
IFACEMETHODIMP
Provider::GetCredentialCount(_Out_ DWORD *pdwCount, _Out_ DWORD *pdwDefault,
                             _Out_ BOOL *pbAutoLogonWithDefault) {
  *pdwDefault = CREDENTIAL_PROVIDER_NO_DEFAULT;
  *pbAutoLogonWithDefault = FALSE;

  if (m_fRecreateEnumeratedCredentials) {
    m_fRecreateEnumeratedCredentials = false;
    ReleaseEnumeratedCredentials();
    CreateEnumeratedCredentials();
  }

  return m_pCredProviderUserArray->GetCount(pdwCount);
}

// Returns the credential at the index specified by dwIndex. This function is
// called by logonUI to enumerate the tiles.
IFACEMETHODIMP Provider::GetCredentialAt(
    DWORD dwIndex,
    _Outptr_result_nullonfailure_ ICredentialProviderCredential **ppcpc) {
  HRESULT hr = E_INVALIDARG;
  *ppcpc = nullptr;

  if ((dwIndex < m_oCredentials.dwCredentialsCount) && ppcpc) {
    hr = (m_oCredentials.parrCredentials[dwIndex])
             ? m_oCredentials.parrCredentials[dwIndex]->QueryInterface(
                   IID_PPV_ARGS(ppcpc))
             : E_POINTER;
  }

  return hr;
}

// This function will be called by LogonUI after SetUsageScenario succeeds.
// Sets the User Array with the list of users to be enumerated on the logon
// screen.
IFACEMETHODIMP
Provider::SetUserArray(_In_ ICredentialProviderUserArray *users) {
  if (m_pCredProviderUserArray) {
    m_pCredProviderUserArray->Release();
    m_pCredProviderUserArray = nullptr;
  }
  m_pCredProviderUserArray = users;
  m_pCredProviderUserArray->AddRef();
  return S_OK;
}

void Provider::CreateEnumeratedCredentials() {
  switch (m_cpus) {
  case CPUS_LOGON:
  case CPUS_UNLOCK_WORKSTATION: {
    EnumerateCredentials();
    break;
  }
  default:
    break;
  }
}

void Provider::ReleaseEnumeratedCredentials() {
  for (DWORD dwIndex = 0; dwIndex < m_oCredentials.dwCredentialsCount;
       ++dwIndex) {
    if (m_oCredentials.parrCredentials[dwIndex] != nullptr) {
      m_oCredentials.parrCredentials[dwIndex]->Release();
      m_oCredentials.parrCredentials[dwIndex] = nullptr;
    }
  }
  delete m_oCredentials.parrCredentials;
  m_oCredentials.parrCredentials = nullptr;
  m_oCredentials.dwCredentialsCount = 0;
}

IFACEMETHODIMP Provider::EnumerateCredentials() {
  HRESULT hr = E_UNEXPECTED;
  if (m_pCredProviderUserArray != nullptr) {
    DWORD dwUserCount;
    hr = m_pCredProviderUserArray->GetCount(&dwUserCount);
    if (SUCCEEDED(hr)) {
      m_oCredentials.parrCredentials =
          new (std::nothrow) Credential *[dwUserCount];
      if (m_oCredentials.parrCredentials != nullptr) {
        m_oCredentials.dwCredentialsCount = 0; // Update count
        for (DWORD dwIndex = 0; dwIndex < dwUserCount; ++dwIndex) {
          ICredentialProviderUser *pCredUser;
          hr = m_pCredProviderUserArray->GetAt(dwIndex, &pCredUser);
          if (SUCCEEDED(hr)) {
            m_oCredentials.parrCredentials[dwIndex] =
                new (std::nothrow) Credential();
            if (m_oCredentials.parrCredentials[dwIndex] != nullptr) {
              hr = m_oCredentials.parrCredentials[dwIndex]->Initialize(
                  m_cpus, s_rgCredProvFieldDescriptors, s_rgFieldStatePairs,
                  pCredUser);
              if (SUCCEEDED(hr)) {
                ++(m_oCredentials.dwCredentialsCount); // Update count
              } else {
                m_oCredentials.parrCredentials[dwIndex]->Release();
                m_oCredentials.parrCredentials[dwIndex] = nullptr;
              }
            } else {
              hr = E_OUTOFMEMORY;
            }
            pCredUser->Release();
          }
        }
      } else {
        hr = E_OUTOFMEMORY;
      }
    }
  }
  return hr;
}
