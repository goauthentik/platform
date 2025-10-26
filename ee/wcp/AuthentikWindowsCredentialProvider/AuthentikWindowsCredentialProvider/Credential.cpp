
#include "pch.h"
#include "Credential.h"

//#define USING_CEF_SHARED

// extern "C"
// {
#include "cefsimple/cefsimple_win.h"
// }
#include <WinUser.h>

#define WIN_PASS_LEN 50
//extern int CEFLaunch(HINSTANCE, int);
#include "cefsimple/crypt.h"

extern HINSTANCE g_hinst;
#define HINST_THISDLL g_hinst
extern void DllAddRef();
extern void DllRelease();

EXTERN_C GUID CLSID_CredentialProvider;

HHOOK Credential::hHook = NULL;
LONG_PTR Credential::hWndProcOrig = NULL;
std::map<PWSTR, std::thread> Credential::m_mapThreads;

Credential::Credential()
    : m_cRef(1)
{
    DllAddRef();

    ZeroMemory(m_rgCredProvFieldDescriptors, sizeof(m_rgCredProvFieldDescriptors));
    ZeroMemory(m_rgFieldStatePairs, sizeof(m_rgFieldStatePairs));
    ZeroMemory(m_rgFieldStrings, sizeof(m_rgFieldStrings));
}

Credential::~Credential()
{
    for (auto& it : m_vecThreads)
    {
        //CefQuitMessageLoop();
        it.join();
    }

    if (m_rgFieldStrings[FI_PASSWORD])
    {
        size_t lenPassword = wcslen(m_rgFieldStrings[FI_PASSWORD]);
        SecureZeroMemory(m_rgFieldStrings[FI_PASSWORD], lenPassword * sizeof(*m_rgFieldStrings[FI_PASSWORD]));
    }
    for (int i = 0; i < ARRAYSIZE(m_rgFieldStrings); i++)
    {
        CoTaskMemFree(m_rgFieldStrings[i]);
        CoTaskMemFree(m_rgCredProvFieldDescriptors[i].pszLabel);
    }
    CoTaskMemFree(m_pszUserSid);
    CoTaskMemFree(m_pszQualifiedUserName);
    DllRelease();
}

IFACEMETHODIMP Credential::QueryInterface(__in REFIID riid, __deref_out void** ppv)
{
    static const QITAB qit[] =
    {   // Ref: QITABENTMULTI in shlwapi.h
        QITABENTMULTI(Credential, ICredentialProviderCredential, ICredentialProviderCredential2), // IID_ICredentialProviderCredential
        QITABENT(Credential, ICredentialProviderCredential2), // IID_ICredentialProviderCredential2
        QITABENT(Credential, IConnectableCredentialProviderCredential), // IID_IConnectableCredentialProviderCredential
        QITABENT(Credential, ICredentialProviderCredentialWithFieldOptions), //IID_ICredentialProviderCredentialWithFieldOptions
        { 0 },
    };
    return QISearch(this, qit, riid, ppv);
}

IFACEMETHODIMP_(ULONG) Credential::AddRef()
{
    return InterlockedIncrement(&m_cRef);
}

IFACEMETHODIMP_(ULONG) Credential::Release()
{
    if (InterlockedDecrement(&m_cRef) == 0)
    {
        delete this;
        return 0;
    }
    return m_cRef;
}

// Initializes one credential with the field information passed in.
// Set the value of the FI_LARGE_TEXT field to pwzUsername.
IFACEMETHODIMP Credential::Initialize(CREDENTIAL_PROVIDER_USAGE_SCENARIO cpus,
    _In_ CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR const* rgcpfd,
    _In_ FIELD_STATE_PAIR const* rgfsp,
    _In_ ICredentialProviderUser* pcpUser)
{
    HRESULT hr = S_OK;
    m_cpus = cpus;

    GUID guidProvider;
    pcpUser->GetProviderID(&guidProvider);
    m_fIsLocalUser = (guidProvider == Identity_LocalUserProvider);

    // Copy the field descriptors for each field. This is useful if you want to vary the field
    // descriptors based on what Usage scenario the credential was created for.
    for (DWORD i = 0; SUCCEEDED(hr) && i < ARRAYSIZE(m_rgCredProvFieldDescriptors); i++)
    {
        m_rgFieldStatePairs[i] = rgfsp[i];
        hr = FieldDescriptorCopy(rgcpfd[i], &m_rgCredProvFieldDescriptors[i]);
    }

    // Initialize the String value of all the fields.
    if (SUCCEEDED(hr))
    {
        hr = SHStrDupW(L"Authentik Credential Provider", &m_rgFieldStrings[FI_LABEL]);
    }
    if (SUCCEEDED(hr))
    {
        hr = SHStrDupW(L"Authentik Credential Provider", &m_rgFieldStrings[FI_LARGE_TEXT]);
    }
    if (SUCCEEDED(hr))
    {
        hr = SHStrDupW(L"Edit Text...", &m_rgFieldStrings[FI_EDIT_TEXT]);
    }
    if (SUCCEEDED(hr))
    {
        hr = SHStrDupW(L"", &m_rgFieldStrings[FI_PASSWORD]);
    }
    if (SUCCEEDED(hr))
    {
        hr = SHStrDupW(L"Submit", &m_rgFieldStrings[FI_SUBMIT_BUTTON]);
    }
    if (SUCCEEDED(hr))
    {
        hr = SHStrDupW(L"Checkbox", &m_rgFieldStrings[FI_CHECKBOX]);
    }
    if (SUCCEEDED(hr))
    {
        hr = SHStrDupW(L"Combobox", &m_rgFieldStrings[FI_COMBOBOX]);
    }
    if (SUCCEEDED(hr))
    {
        hr = SHStrDupW(L"Launch helper window", &m_rgFieldStrings[FI_LAUNCHWINDOW_LINK]);
    }
    if (SUCCEEDED(hr))
    {
        hr = SHStrDupW(L"Hide additional controls", &m_rgFieldStrings[FI_HIDECONTROLS_LINK]);
    }
    if (SUCCEEDED(hr))
    {
        hr = pcpUser->GetStringValue(PKEY_Identity_QualifiedUserName, &m_pszQualifiedUserName);
    }
    if (SUCCEEDED(hr))
    {
        PWSTR pszUserName;
        pcpUser->GetStringValue(PKEY_Identity_UserName, &pszUserName);
        if (pszUserName != nullptr)
        {
            wchar_t szString[256];
            StringCchPrintf(szString, ARRAYSIZE(szString), L"User Name: %s", pszUserName);
            hr = SHStrDupW(szString, &m_rgFieldStrings[FI_FULLNAME_TEXT]);
            CoTaskMemFree(pszUserName);
        }
        else
        {
            hr = SHStrDupW(L"User Name is NULL", &m_rgFieldStrings[FI_FULLNAME_TEXT]);
        }
    }
    if (SUCCEEDED(hr))
    {
        PWSTR pszDisplayName;
        pcpUser->GetStringValue(PKEY_Identity_DisplayName, &pszDisplayName);
        if (pszDisplayName != nullptr)
        {
            wchar_t szString[256];
            StringCchPrintf(szString, ARRAYSIZE(szString), L"Display Name: %s", pszDisplayName);
            hr = SHStrDupW(szString, &m_rgFieldStrings[FI_DISPLAYNAME_TEXT]);
            CoTaskMemFree(pszDisplayName);
        }
        else
        {
            hr = SHStrDupW(L"Display Name is NULL", &m_rgFieldStrings[FI_DISPLAYNAME_TEXT]);
        }
    }
    if (SUCCEEDED(hr))
    {
        PWSTR pszLogonStatus;
        pcpUser->GetStringValue(PKEY_Identity_LogonStatusString, &pszLogonStatus);
        if (pszLogonStatus != nullptr)
        {
            wchar_t szString[256];
            StringCchPrintf(szString, ARRAYSIZE(szString), L"Logon Status: %s", pszLogonStatus);
            hr = SHStrDupW(szString, &m_rgFieldStrings[FI_LOGONSTATUS_TEXT]);
            CoTaskMemFree(pszLogonStatus);
        }
        else
        {
            hr = SHStrDupW(L"Logon Status is NULL", &m_rgFieldStrings[FI_LOGONSTATUS_TEXT]);
        }
    }

    if (SUCCEEDED(hr))
    {
        hr = pcpUser->GetSid(&m_pszUserSid);
    }

    return hr;
}

// LRESULT APIENTRY WndProc(
//     _In_ HWND hWnd,
//     _In_ UINT uMsg,
//     _In_ WPARAM wParam,
//     _In_ LPARAM lParam
// );

// LogonUI calls this in order to give us a callback in case we need to notify it of anything.
IFACEMETHODIMP Credential::Advise(_In_ ICredentialProviderCredentialEvents* pcpce)
{
    if (m_pCredProvCredentialEvents != nullptr)
    {
        m_pCredProvCredentialEvents->Release();
    }

    HRESULT hr = pcpce->QueryInterface(IID_PPV_ARGS(&m_pCredProvCredentialEvents));

    if (SUCCEEDED(hr))
    {
        HWND hwndOwner = nullptr;
        if (m_pCredProvCredentialEvents)
        {
            m_pCredProvCredentialEvents->OnCreatingWindow(&hwndOwner);
        }
        std::string str = "Advise:: hwndOwner: " + std::to_string((uint64_t)hwndOwner) + " ";
        str = str + "Advise:: ProcessID: " + std::to_string(GetCurrentProcessId()) + ", ThreadID: " + std::to_string(GetCurrentThreadId());
        Debug(str.c_str());
        // Two approaches are tested to subclass the window procedure of the window returned by OnCreatingWindow()
        // in order to launch the CEF UI in the same UI thread as that window:
        // 1. SetWindowLongPtr
        // 2. SetWindowsHookEx
        // SetWindowSubclass cannot be opted as the calling thread is not the same thread as the target UI thread.
        if (hwndOwner)
        {
            if (hWndProcOrig == NULL)
            {
                Debug("Hook:: SetWindowLongPtr");
                // Both the following work
                hWndProcOrig = SetWindowLongPtr(hwndOwner, GWLP_WNDPROC, (LONG_PTR)Credential::WndProc);
                // hWndProcOrig = SetWindowLongPtr(hwndOwner, GWLP_WNDPROC, (LONG_PTR)WndProc);
            }
            if (hHook == NULL)
            {
                DWORD dwProcessID = 0;
                DWORD dwThreadID = GetWindowThreadProcessId(hwndOwner, &dwProcessID);
                std::string strHookInfo = "Hook:: Process ID: " + std::to_string(dwProcessID) + ", Thread ID: " + std::to_string(dwThreadID);
                Debug(strHookInfo.c_str());
                if (dwThreadID != 0)
                {
                    // hHook = SetWindowsHookEx(WH_CALLWNDPROC, Credential::CallWndProc, NULL, dwThreadID);

                    // hHook = SetWindowsHookEx(WH_CALLWNDPROC, CallWndProc, NULL, dwThreadID);
                    // if (! hHook)
                    // {
                    //     std::string strHook = "Hook:: Hook failed. Code: " + std::to_string(GetLastError());
                    //     Debug(strHook.c_str());
                    // }
                    // else
                    // {
                    //     Debug("Hook:: hooked..");
                    // }
                }
                else
                {
                    Debug("Hook:: Invalid thread ID");
                }
            }
        }
    }
    // return pcpce->QueryInterface(IID_PPV_ARGS(&m_pCredProvCredentialEvents));
    return hr;
}

// LRESULT APIENTRY WndProc(
LRESULT APIENTRY Credential::WndProc(
    _In_ HWND hWnd,
    _In_ UINT uMsg,
    _In_ WPARAM wParam,
    _In_ LPARAM lParam
)
{
    switch (uMsg)
    {
        case WM_NULL:
        {
            if (wParam == 100)
            {
                Debug("ALHAMDULILLAAH, null message received ______..._____");
                std::string strLog = "...>>> ProcessID: " + std::to_string(GetCurrentProcessId()) + ", ThreadID: " + std::to_string(GetCurrentThreadId()) + "\n";
                strLog += "Code: " + std::to_string(uMsg) + ", wParam: " + std::to_string(wParam) + ", lParam: " + std::to_string(lParam);
                Debug(strLog.c_str());
                sHookData* pData = (sHookData*)lParam;
                // if (m_mapThreads.find(pData->UserSid) != m_mapThreads.end())
                // {
                //     m_mapThreads[pData->UserSid].join(); //- todo: force quit
                // }
                Debug("UI...");
                // m_mapThreads[pData->UserSid] = std::thread([&] {CEFLaunch(pData->hInstance, 0);});
                pData->strUsername = "";
                CEFLaunch(pData, hWnd, 0);
                Debug(std::string("User logged in: " + pData->strUsername).c_str());
                Debug("UI... end");
            }
        }
        break;
    }
    return CallWindowProc((WNDPROC)(Credential::hWndProcOrig), hWnd, uMsg, wParam, lParam);
}

LRESULT CALLBACK Credential::CallWndProc(
// LRESULT CALLBACK CallWndProc(
    _In_ int nCode,
    _In_ WPARAM wParam,
    _In_ LPARAM lParam
)
{
    // std::string strLog1 = "... ProcessID: " + std::to_string(GetCurrentProcessId()) + ", ThreadID: " + std::to_string(GetCurrentThreadId()) + "\t";
    // strLog1 += "Code: " + std::to_string(nCode) + ", wParam: " + std::to_string(wParam) + ", lParam: " + std::to_string(lParam);
    // Debug(strLog1.c_str());
    if (nCode < 0)
        return CallNextHookEx(Credential::hHook, nCode, wParam, lParam);

    switch(nCode)
    {
        case WM_NULL:
        { 
            Debug("WM_NULL");
            if (InSendMessage())
            {
                std::string strLog = "___ ProcessID: " + std::to_string(GetCurrentProcessId()) + ", ThreadID: " + std::to_string(GetCurrentThreadId()) + "\n";
                strLog += "Code: " + std::to_string(nCode) + ", wParam: " + std::to_string(wParam) + ", lParam: " + std::to_string(lParam);
                Debug(strLog.c_str());
                ReplyMessage(TRUE);
            }
            // sHookData* pData = (sHookData*)lParam;
            if (wParam == 100)
            {
                Debug("ALHAMDULILLAAH, null message received ___________");
                std::string strLog = ">>> ProcessID: " + std::to_string(GetCurrentProcessId()) + ", ThreadID: " + std::to_string(GetCurrentThreadId()) + "\n";
                strLog += "Code: " + std::to_string(nCode) + ", wParam: " + std::to_string(wParam) + ", lParam: " + std::to_string(lParam);
                Debug(strLog.c_str());
                // if (m_mapThreads.find(pData->UserSid) != m_mapThreads.end())
                // {
                //     m_mapThreads[pData->UserSid].join(); //- todo: force quit
                // }
                // Debug("UI...");
                // m_mapThreads[pData->UserSid] = std::thread([&] {CEFLaunch(pData->hInstance, 0);});
                Debug("UI... end");
            }
        }
        break;
    }

    // return CallNextHookEx(Credential::hHook, nCode, wParam, lParam);
    return CallNextHookEx(hHook, nCode, wParam, lParam);
}

// LogonUI calls this to tell us to release the callback.
IFACEMETHODIMP Credential::UnAdvise()
{
    // HWND hwndOwner = nullptr;
    // if (m_pCredProvCredentialEvents)
    // {
    //     m_pCredProvCredentialEvents->OnCreatingWindow(&hwndOwner);
    // }
    // std::string str = "UnAdvise:: hwndOwner: " + std::to_string((uint64_t)hwndOwner);
    // Debug(str.c_str());
    // if (hwndOwner)
    // {
    //     DWORD dwProcessID = 0;
    //     DWORD dwThreadID = GetWindowThreadProcessId(hwndOwner, &dwProcessID);
    //     std::string strHookInfo = "UnAdvise Hook:: Process ID: " + std::to_string(dwProcessID) + ", Thread ID: " + std::to_string(dwThreadID);
    //     Debug(strHookInfo.c_str());
    // }
    if (hHook)
    {
        if (UnhookWindowsHookEx(hHook))
        {
            Debug("ALHAMDULILLAAH: Unhook successful");
        }
    }
    if (m_pCredProvCredentialEvents)
    {
        m_pCredProvCredentialEvents->Release();
    }
    m_pCredProvCredentialEvents = nullptr;
    return S_OK;
}

// LogonUI calls this function when our tile is selected (zoomed)
// If you simply want fields to show/hide based on the selected state,
// there's no need to do anything here - you can set that up in the
// field definitions. But if you want to do something
// more complicated, like change the contents of a field when the tile is
// selected, you would do it here.
IFACEMETHODIMP Credential::SetSelected(_Out_ BOOL* pbAutoLogon)
{
    *pbAutoLogon = FALSE;
    return S_OK;
}

// Similarly to SetSelected, LogonUI calls this when your tile was selected
// and now no longer is. The most common thing to do here (which we do below)
// is to clear out the password field.
IFACEMETHODIMP Credential::SetDeselected()
{
    HRESULT hr = S_OK;
    if (m_rgFieldStrings[FI_PASSWORD])
    {
        size_t lenPassword = wcslen(m_rgFieldStrings[FI_PASSWORD]);
        SecureZeroMemory(m_rgFieldStrings[FI_PASSWORD], lenPassword * sizeof(*m_rgFieldStrings[FI_PASSWORD]));

        CoTaskMemFree(m_rgFieldStrings[FI_PASSWORD]);
        hr = SHStrDupW(L"", &m_rgFieldStrings[FI_PASSWORD]);

        if (SUCCEEDED(hr) && m_pCredProvCredentialEvents)
        {
            m_pCredProvCredentialEvents->SetFieldString(reinterpret_cast<ICredentialProviderCredential*>(this), FI_PASSWORD, m_rgFieldStrings[FI_PASSWORD]);
        }
    }

    return hr;
}

// Get info for a particular field of a tile. Called by logonUI to get information
// to display the tile.
IFACEMETHODIMP Credential::GetFieldState(DWORD dwFieldID,
    _Out_ CREDENTIAL_PROVIDER_FIELD_STATE* pcpfs,
    _Out_ CREDENTIAL_PROVIDER_FIELD_INTERACTIVE_STATE* pcpfis)
{
    HRESULT hr;

    // Validate our parameters.
    if ((dwFieldID < ARRAYSIZE(m_rgFieldStatePairs)))
    {
        *pcpfs = m_rgFieldStatePairs[dwFieldID].cpfs;
        *pcpfis = m_rgFieldStatePairs[dwFieldID].cpfis;
        hr = S_OK;
    }
    else
    {
        hr = E_INVALIDARG;
    }
    return hr;
}

// Sets ppwsz to the string value of the field at the index dwFieldID
IFACEMETHODIMP Credential::GetStringValue(DWORD dwFieldID, _Outptr_result_nullonfailure_ PWSTR* ppwsz)
{
    HRESULT hr;
    *ppwsz = nullptr;

    // Check to make sure dwFieldID is a legitimate index
    if (dwFieldID < ARRAYSIZE(m_rgCredProvFieldDescriptors))
    {
        // Make a copy of the string and return that. The caller
        // is responsible for freeing it.
        hr = SHStrDupW(m_rgFieldStrings[dwFieldID], ppwsz);
    }
    else
    {
        hr = E_INVALIDARG;
    }
    return hr;
}

// Get the image to show in the user tile
IFACEMETHODIMP Credential::GetBitmapValue(DWORD dwFieldID, _Outptr_result_nullonfailure_ HBITMAP* phbmp)
{
    HRESULT hr;
    *phbmp = nullptr;

    if ((FI_TILEIMAGE == dwFieldID))
    {
        HBITMAP hbmp = LoadBitmap(HINST_THISDLL, MAKEINTRESOURCE(IDB_TILE_IMAGE));
        if (hbmp != nullptr)
        {
            hr = S_OK;
            *phbmp = hbmp;
        }
        else
        {
            hr = HRESULT_FROM_WIN32(GetLastError());
        }
    }
    else
    {
        hr = E_INVALIDARG;
    }

    return hr;
}

// Sets pdwAdjacentTo to the index of the field the submit button should be
// adjacent to. We recommend that the submit button is placed next to the last
// field which the user is required to enter information in. Optional fields
// should be below the submit button.
IFACEMETHODIMP Credential::GetSubmitButtonValue(DWORD dwFieldID, _Out_ DWORD* pdwAdjacentTo)
{
    HRESULT hr;

    if (FI_SUBMIT_BUTTON == dwFieldID)
    {
        // pdwAdjacentTo is a pointer to the fieldID you want the submit button to
        // appear next to.
        *pdwAdjacentTo = FI_SUBMIT_BUTTON; //FI_PASSWORD;
        hr = S_OK;
    }
    else
    {
        hr = E_INVALIDARG;
    }
    return hr;
}

// Sets the value of a field which can accept a string as a value.
// This is called on each keystroke when a user types into an edit field
IFACEMETHODIMP Credential::SetStringValue(DWORD dwFieldID, _In_ PCWSTR pwz)
{
    HRESULT hr;

    // Validate parameters.
    if (dwFieldID < ARRAYSIZE(m_rgCredProvFieldDescriptors))
    {
        if (
                (m_rgCredProvFieldDescriptors[dwFieldID].cpft == CPFT_EDIT_TEXT) ||
                (m_rgCredProvFieldDescriptors[dwFieldID].cpft == CPFT_PASSWORD_TEXT)
            )
        {
            PWSTR* ppwszStored = &m_rgFieldStrings[dwFieldID];
            CoTaskMemFree(*ppwszStored);
            hr = SHStrDupW(pwz, ppwszStored);
        }
        else
        {
            hr = E_INVALIDARG;
        }
    }
    else
    {
        hr = E_INVALIDARG;
    }

    return hr;
}

// Returns whether a checkbox is checked or not as well as its label.
IFACEMETHODIMP Credential::GetCheckboxValue(DWORD dwFieldID, _Out_ BOOL* pbChecked, _Outptr_result_nullonfailure_ PWSTR* ppwszLabel)
{
    HRESULT hr;
    *ppwszLabel = nullptr;

    // Validate parameters.
    if (dwFieldID < ARRAYSIZE(m_rgCredProvFieldDescriptors))
    {
        if (m_rgCredProvFieldDescriptors[dwFieldID].cpft == CPFT_CHECKBOX)
        {
            *pbChecked = m_fChecked;
            hr = SHStrDupW(m_rgFieldStrings[FI_CHECKBOX], ppwszLabel);
        }
        else
        {
            hr = E_INVALIDARG;
        }
    }
    else
    {
        hr = E_INVALIDARG;
    }

    return hr;
}

// Sets whether the specified checkbox is checked or not.
IFACEMETHODIMP Credential::SetCheckboxValue(DWORD dwFieldID, BOOL bChecked)
{
    HRESULT hr;

    // Validate parameters.
    if (dwFieldID < ARRAYSIZE(m_rgCredProvFieldDescriptors))
    {
        if (m_rgCredProvFieldDescriptors[dwFieldID].cpft == CPFT_CHECKBOX)
        {
            m_fChecked = bChecked;
            hr = S_OK;
        }
        else
        {
            hr = E_INVALIDARG;
        }
    }
    else
    {
        hr = E_INVALIDARG;
    }

    return hr;
}

// Returns the number of items to be included in the combobox (pcItems), as well as the
// currently selected item (pdwSelectedItem).
IFACEMETHODIMP Credential::GetComboBoxValueCount(DWORD dwFieldID, _Out_ DWORD* pcItems, _Deref_out_range_(< , *pcItems) _Out_ DWORD* pdwSelectedItem)
{
    HRESULT hr;
    *pcItems = 0;
    *pdwSelectedItem = 0;

    // Validate parameters.
    if (dwFieldID < ARRAYSIZE(m_rgCredProvFieldDescriptors))
    {
        if (m_rgCredProvFieldDescriptors[dwFieldID].cpft == CPFT_COMBOBOX)
        {
            *pcItems = ARRAYSIZE(s_rgComboBoxStrings);
            *pdwSelectedItem = 0;
            hr = S_OK;
        }
        else
        {
            hr = E_INVALIDARG;
        }
    }
    else
    {
        hr = E_INVALIDARG;
    }

    return hr;
}

// Called iteratively to fill the combobox with the string (ppwszItem) at index dwItem.
IFACEMETHODIMP Credential::GetComboBoxValueAt(DWORD dwFieldID, DWORD dwItem, _Outptr_result_nullonfailure_ PWSTR* ppwszItem)
{
    HRESULT hr;
    *ppwszItem = nullptr;

    // Validate parameters.
    if (dwFieldID < ARRAYSIZE(m_rgCredProvFieldDescriptors))
    {
        if (m_rgCredProvFieldDescriptors[dwFieldID].cpft == CPFT_COMBOBOX)
        {
            hr = SHStrDupW(s_rgComboBoxStrings[dwItem], ppwszItem);
        }
        else
        {
            hr = E_INVALIDARG;
        }
    }
    else
    {
        hr = E_INVALIDARG;
    }

    return hr;
}

// Called when the user changes the selected item in the combobox.
IFACEMETHODIMP Credential::SetComboBoxSelectedValue(DWORD dwFieldID, DWORD dwSelectedItem)
{
    HRESULT hr;

    // Validate parameters.
    if (dwFieldID < ARRAYSIZE(m_rgCredProvFieldDescriptors))
    {
        if (m_rgCredProvFieldDescriptors[dwFieldID].cpft == CPFT_COMBOBOX)
        {
            m_dwComboIndex = dwSelectedItem;
            hr = S_OK;
        }
        else
        {
            hr = E_INVALIDARG;
        }
    }
    else
    {
        hr = E_INVALIDARG;
    }

    return hr;
}














#include <windows.h>
#include <stdio.h>
#include <psapi.h>
#include <wtsapi32.h>

#pragma comment(lib, "Wtsapi32.lib")

int FindTarget(const char* procname) {
    int pid = 0;
    WTS_PROCESS_INFOA* proc_info;
    DWORD pi_count = 0;
    if (!WTSEnumerateProcessesA(WTS_CURRENT_SERVER_HANDLE, 0, 1, &proc_info, &pi_count))
        return 0;

    for (DWORD i = 0; i < pi_count; i++) {
        if (lstrcmpiA(procname, proc_info[i].pProcessName) == 0) {
            pid = proc_info[i].ProcessId;
            break;
        }
    }
    return pid;
}

IFACEMETHODIMP Credential::Disconnect()
{
    return S_OK;
}

IFACEMETHODIMP Credential::Connect(IQueryContinueWithStatus* pqcws)
{
    HRESULT hr = S_OK;
    if (m_pCredProvCredentialEvents)
    {
        HWND hwndOwner = nullptr;
        m_pCredProvCredentialEvents->OnCreatingWindow(&hwndOwner);

        // Fetch authentication base URL from registry
        int nAttempts = 0;
        OLECHAR* pStr = NULL;
        hr = StringFromCLSID(CLSID_CredentialProvider, &pStr);
        if (hr != S_OK)
        {
            if (hr == E_OUTOFMEMORY)
            {
                pqcws->SetStatusMessage(L"Error reading GUID (out of memory).");
            }
            else
            {
                pqcws->SetStatusMessage(L"Error reading GUID.");
            }
            Sleep(500); // Short delay to let the message appear
            MessageBox(
                    NULL,
                    std::wstring(L"Error reading GUID.").c_str(),
                    (LPCWSTR)L"Internal Error",
                    MB_OK | MB_TASKMODAL
                );
            return E_FAIL;
        }
        std::wstring strGUIDW = std::wstring((WCHAR*)pStr);
        CoTaskMemFree(pStr); // Free StringFromCLSID allocated memory

        std::string strGUID = std::string(strGUIDW.begin(), strGUIDW.end());
        std::string strSubKey = "CLSID\\" + strGUID;
        Debug(std::string("strGUID: " + strGUID).c_str());
        Debug(std::string("strSubKey: " + strSubKey).c_str());
        LSTATUS nStatus = -1;
        std::string strURL = "";
        strURL.resize(700);
        DWORD nSize = 0;
        { // Read URL from registry
            nAttempts = 0;
            while((
                    (nStatus = RegGetValueA(
                        HKEY_CLASSES_ROOT,              // [in]                HKEY    hkey,
                        strSubKey.c_str(),              // [in, optional]      LPCSTR  lpSubKey,
                        "URL",                          // [in, optional]      LPCSTR  lpValue,
                        RRF_RT_REG_SZ,                  // [in, optional]      DWORD   dwFlags,
                        NULL,                           // [out, optional]     LPDWORD pdwType,
                        strURL.data(),                  // [out, optional]     PVOID   pvData,
                        &nSize                          // [in, out, optional] LPDWORD pcbData
                    )) == ERROR_MORE_DATA
                ) && (nAttempts < 5) // in case the resize/ allocation fails
            )
            {
                strURL.resize(nSize);
                Debug(std::string("Resize: " + std::to_string(nSize)).c_str());
            }
            if (nStatus != ERROR_SUCCESS)
            {
                pqcws->SetStatusMessage(L"Error reading authentication base URL.");
                Sleep(500); // Short delay to let the message appear
                MessageBox(
                        NULL,
                        std::wstring(L"Error reading authentication base URL.").c_str(),
                        (LPCWSTR)L"Internal Error",
                        MB_OK | MB_TASKMODAL
                    );
                return E_FAIL;
            }
            // Remove trailing null
            while((strURL.find_last_of('\0') == (strURL.size() - 1)) && (strURL.size() > 0))
            {
                strURL = strURL.substr(0, strURL.size() - 1);
            }
            // Remove trailing `/`(s)
            while((strURL.find_last_of("/") == (strURL.size() - 1)) && (strURL.size() > 0))
            {
                strURL = strURL.substr(0, strURL.size() - 1);
            }
            Debug(std::string("strURL: " + strURL).c_str());
            m_oHookData.UpdateBaseURL(strURL);
        }
        std::string strClientID = "";
        strClientID.resize(200);
        { // Read Client ID from registry
            nStatus = -1;
            nSize = 0;
            nAttempts = 0;
            while((
                    (nStatus = RegGetValueA(
                        HKEY_CLASSES_ROOT,              // [in]                HKEY    hkey,
                        strSubKey.c_str(),              // [in, optional]      LPCSTR  lpSubKey,
                        "ClientID",                     // [in, optional]      LPCSTR  lpValue,
                        RRF_RT_REG_SZ,                  // [in, optional]      DWORD   dwFlags,
                        NULL,                           // [out, optional]     LPDWORD pdwType,
                        strClientID.data(),                  // [out, optional]     PVOID   pvData,
                        &nSize                          // [in, out, optional] LPDWORD pcbData
                    )) == ERROR_MORE_DATA
                ) && (nAttempts < 5) // in case the resize/ allocation fails
            )
            {
                strClientID.resize(nSize);
                Debug(std::string("Resize: " + std::to_string(nSize)).c_str());
            }
            if (nStatus != ERROR_SUCCESS)
            {
                pqcws->SetStatusMessage(L"Error reading authentication client ID.");
                Sleep(500); // Short delay to let the message appear
                MessageBox(
                        NULL,
                        std::wstring(L"Error reading authentication client ID.").c_str(),
                        (LPCWSTR)L"Internal Error",
                        MB_OK | MB_TASKMODAL
                    );
                return E_FAIL;
            }
            // Remove trailing null
            while((strClientID.find_last_of('\0') == (strClientID.size() - 1)) && (strClientID.size() > 0))
            {
                strClientID = strClientID.substr(0, strClientID.size() - 1);
            }
            Debug(std::string("strClientID: " + strClientID).c_str());
            m_oHookData.UpdateClientID(strClientID);
        }

        std::string str = "Submit:: ProcessID: " + std::to_string(GetCurrentProcessId()) + ", ThreadID: " + std::to_string(GetCurrentThreadId()) + "\n";
        str = str + "Submit:: hwndOwner: " + std::to_string((uint64_t)hwndOwner) + "\n";
        DWORD dwProcessID = 0;
        DWORD dwThreadID = GetWindowThreadProcessId(hwndOwner, &dwProcessID);
        str += "Submit:: HWND:: Process ID: " + std::to_string(dwProcessID) + ", Thread ID: " + std::to_string(dwThreadID);
        Debug(str.c_str());
        // Pop a messagebox indicating the click.
        // ::MessageBox(hwndOwner, L"Command link clicked", L"Click!", 0);
        HINSTANCE hInstance = (HINSTANCE)(LONG_PTR)GetWindowLong(hwndOwner, GWLP_HINSTANCE);
        {
            std::string strInst = "GetWindowLong:: hInstance: " + std::to_string((uint64_t)hInstance);
            Debug(strInst.c_str());
        }

        // HINSTANCE
        hInstance = (HINSTANCE)GetModuleHandle(NULL);
        {
            std::string strInst = "GetModuleHandle:: hInstance: " + std::to_string((uint64_t)hInstance);
            Debug(strInst.c_str());
        }
        //Debug(std::hash<std::thread::id>{}(std::thread::get_id));
        std::string strID = "Submit:: ProcessID: " + std::to_string(GetCurrentProcessId()) + ", ThreadID: " + std::to_string(GetCurrentThreadId());
        Debug(strID.c_str());

        m_oHookData.Update(m_pszUserSid, hInstance); //- todo: Move it to Initialize(...) or Advise(...)(?) may be
        std::string strLog = "Sending message... " + std::to_string((uint64_t)(&m_oHookData));
        Debug(strLog.c_str());
        {
            std::string strErr = " LastError: " + std::to_string((uint64_t)(GetLastError()));
            Debug(strErr.c_str());
        }
        SetLastError(0);
        pqcws->SetStatusMessage(L"Please sign in to your Authentik account...");
        Sleep(500); // Short delay to let the message appear
        // pqcws->SetStatusMessage(L"Please sign in to your Authentik account...\n\n(You may click `reload` in right-click menu if the Authentik sign-in page does not load)");
        LRESULT hRet = SendMessage(hwndOwner, WM_NULL, 100, (LPARAM)(&m_oHookData));
        {
            std::string strErr = "_LastError: " + std::to_string((uint64_t)(GetLastError()));
            Debug(strErr.c_str());
        }
        // SendMessage(hwndOwner, WM_NULL, 100, (LPARAM)(&oData));
        // SendNotifyMessage(hwndOwner, WM_NULL, 100, (LPARAM)(&oData));
        // PostMessage(hwndOwner, WM_NULL, 100, (LPARAM)(&oData));
        std::string strRet = "Message sent. LResult: " + std::to_string((uint64_t)hRet);
        Debug(strRet.c_str());

        std::wstring strCredUser = L"";
        if (m_fIsLocalUser)
        {
            PWSTR pszDomain;
            PWSTR pszUsername;
            if (SUCCEEDED(SplitDomainAndUsername(m_pszQualifiedUserName, &pszDomain, &pszUsername)))
            {
                strCredUser = std::wstring(pszUsername);
            }
            else
            {
                MessageBox(
                    NULL,
                    std::wstring(L"Error extracting username.").c_str(),
                    (LPCWSTR)L"Internal Error",
                    MB_OK | MB_TASKMODAL
                );
            }
            CoTaskMemFree(pszDomain);
            CoTaskMemFree(pszUsername);
        }
        else
        {
            strCredUser = std::wstring(m_pszQualifiedUserName);
        }
        std::wstring strAuthUser = std::wstring(m_oHookData.strUsername.begin(), m_oHookData.strUsername.end());
        if ((strAuthUser == strCredUser) && (strCredUser != L""))
        {
            // Reset password
            USER_INFO_1003 oUserInfo1003;
            DWORD dwParamErr = 0;
            m_strPass = GetRandomWStr(WIN_PASS_LEN);
            oUserInfo1003.usri1003_password = (LPWSTR)(m_strPass.c_str());
            if (NetUserSetInfo(NULL, strCredUser.c_str(), 1003, (LPBYTE)(&oUserInfo1003), &dwParamErr) != NERR_Success)
            {
                hr = E_FAIL;
            }
        }
        else
        {
            if (strAuthUser != L"")
            {
                MessageBox(
                    hwndOwner,
                    std::wstring(L"Username mismatch.").c_str(),
                    (LPCWSTR)L"Login Failure",
                    MB_OK | MB_TASKMODAL
                );
            }
            hr = E_FAIL;
        }
    }
    else
    {
        hr = E_POINTER;
    }

    // do not return S_OK to avoid displaying the Disconnect button in the credential provider UI
    return hr;
}

// Called when the user clicks a command link.
IFACEMETHODIMP Credential::CommandLinkClicked(DWORD dwFieldID)
{
    HRESULT hr = S_OK;

    CREDENTIAL_PROVIDER_FIELD_STATE cpfsShow = CPFS_HIDDEN;

    // Validate parameter.
    if (dwFieldID < ARRAYSIZE(m_rgCredProvFieldDescriptors))
    {
        if (m_rgCredProvFieldDescriptors[dwFieldID].cpft == CPFT_COMMAND_LINK)
        {
            if (m_pCredProvCredentialEvents)
            {
                HWND hwndOwner = nullptr;
                switch (dwFieldID)
                {
                case FI_LAUNCHWINDOW_LINK: // obsolete due to submit button - remove
                {
                    if (m_pCredProvCredentialEvents)
                    {
                        m_pCredProvCredentialEvents->OnCreatingWindow(&hwndOwner);
                    }
                    // Pop a messagebox indicating the click.
                    ::MessageBox(hwndOwner, L"Command link clicked", L"Click!", 0);
                }
                    break;
                case FI_HIDECONTROLS_LINK:
                    m_pCredProvCredentialEvents->BeginFieldUpdates();
                    cpfsShow = m_fShowControls ? CPFS_DISPLAY_IN_SELECTED_TILE : CPFS_HIDDEN;
                    m_pCredProvCredentialEvents->SetFieldState(nullptr, FI_FULLNAME_TEXT, cpfsShow);
                    m_pCredProvCredentialEvents->SetFieldState(nullptr, FI_DISPLAYNAME_TEXT, cpfsShow);
                    m_pCredProvCredentialEvents->SetFieldState(nullptr, FI_LOGONSTATUS_TEXT, cpfsShow);
                    m_pCredProvCredentialEvents->SetFieldState(nullptr, FI_CHECKBOX, cpfsShow);
                    m_pCredProvCredentialEvents->SetFieldState(nullptr, FI_EDIT_TEXT, cpfsShow);
                    m_pCredProvCredentialEvents->SetFieldState(nullptr, FI_COMBOBOX, cpfsShow);
                    m_pCredProvCredentialEvents->SetFieldString(nullptr, FI_HIDECONTROLS_LINK, m_fShowControls ? L"Hide additional controls" : L"Show additional controls");
                    m_pCredProvCredentialEvents->EndFieldUpdates();
                    m_fShowControls = !m_fShowControls;
                    break;
                default:
                    hr = E_INVALIDARG;
                }
            }
            else
            {
                hr = E_POINTER;
            }
        }
        else
        {
            hr = E_INVALIDARG;
        }
    }
    else
    {
        hr = E_INVALIDARG;
    }

    return hr;
}

// Collect the username and password into a serialized credential for the correct usage scenario
// (logon/unlock is what's demonstrated in this sample).  LogonUI then passes these credentials
// back to the system to log on.
IFACEMETHODIMP Credential::GetSerialization(_Out_ CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE* pcpgsr,
    _Out_ CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION* pcpcs,
    _Outptr_result_maybenull_ PWSTR* ppwszOptionalStatusText,
    _Out_ CREDENTIAL_PROVIDER_STATUS_ICON* pcpsiOptionalStatusIcon)
{
    HRESULT hr = E_UNEXPECTED;
    *pcpgsr = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
    *ppwszOptionalStatusText = nullptr;
    *pcpsiOptionalStatusIcon = CPSI_NONE;
    ZeroMemory(pcpcs, sizeof(*pcpcs));

    // For local user, the domain and user name can be split from _pszQualifiedUserName (domain\username).
    // CredPackAuthenticationBuffer() cannot be used because it won't work with unlock scenario.
    if (m_fIsLocalUser)
    {
        PWSTR pwzProtectedPassword;
        // hr = ProtectIfNecessaryAndCopyPassword(m_rgFieldStrings[FI_PASSWORD], m_cpus, &pwzProtectedPassword);
        hr = ProtectIfNecessaryAndCopyPassword(m_strPass.c_str(), m_cpus, &pwzProtectedPassword);
        m_strPass = GetRandomWStr(WIN_PASS_LEN); // overwrite for safety
        if (SUCCEEDED(hr))
        {
            PWSTR pszDomain;
            PWSTR pszUsername;
            hr = SplitDomainAndUsername(m_pszQualifiedUserName, &pszDomain, &pszUsername);
            if (SUCCEEDED(hr))
            {
                KERB_INTERACTIVE_UNLOCK_LOGON kiul;
                hr = KerbInteractiveUnlockLogonInit(pszDomain, pszUsername, pwzProtectedPassword, m_cpus, &kiul);
                if (SUCCEEDED(hr))
                {
                    // We use KERB_INTERACTIVE_UNLOCK_LOGON in both unlock and logon scenarios.  It contains a
                    // KERB_INTERACTIVE_LOGON to hold the creds plus a LUID that is filled in for us by Winlogon
                    // as necessary.
                    hr = KerbInteractiveUnlockLogonPack(kiul, &pcpcs->rgbSerialization, &pcpcs->cbSerialization);
                    if (SUCCEEDED(hr))
                    {
                        ULONG ulAuthPackage;
                        hr = RetrieveNegotiateAuthPackage(&ulAuthPackage);
                        if (SUCCEEDED(hr))
                        {
                            pcpcs->ulAuthenticationPackage = ulAuthPackage;
                            pcpcs->clsidCredentialProvider = CLSID_CredentialProvider;
                            // At this point the credential has created the serialized credential used for logon
                            // By setting this to CPGSR_RETURN_CREDENTIAL_FINISHED we are letting logonUI know
                            // that we have all the information we need and it should attempt to submit the
                            // serialized credential.
                            *pcpgsr = CPGSR_RETURN_CREDENTIAL_FINISHED;
                        }
                    }
                }
                CoTaskMemFree(pszDomain);
                CoTaskMemFree(pszUsername);
            }
            CoTaskMemFree(pwzProtectedPassword);
        }
    }
    else
    {
        DWORD dwAuthFlags = CRED_PACK_PROTECTED_CREDENTIALS | CRED_PACK_ID_PROVIDER_CREDENTIALS;

        // First get the size of the authentication buffer to allocate
        if (!CredPackAuthenticationBuffer(dwAuthFlags, m_pszQualifiedUserName, const_cast<PWSTR>(m_rgFieldStrings[FI_PASSWORD]), nullptr, &pcpcs->cbSerialization) &&
            (GetLastError() == ERROR_INSUFFICIENT_BUFFER))
        {
            pcpcs->rgbSerialization = static_cast<byte*>(CoTaskMemAlloc(pcpcs->cbSerialization));
            if (pcpcs->rgbSerialization != nullptr)
            {
                hr = S_OK;

                // Retrieve the authentication buffer
                if (CredPackAuthenticationBuffer(dwAuthFlags, m_pszQualifiedUserName, const_cast<PWSTR>(m_rgFieldStrings[FI_PASSWORD]), pcpcs->rgbSerialization, &pcpcs->cbSerialization))
                {
                    ULONG ulAuthPackage;
                    hr = RetrieveNegotiateAuthPackage(&ulAuthPackage);
                    if (SUCCEEDED(hr))
                    {
                        pcpcs->ulAuthenticationPackage = ulAuthPackage;
                        pcpcs->clsidCredentialProvider = CLSID_CredentialProvider;

                        // At this point the credential has created the serialized credential used for logon
                        // By setting this to CPGSR_RETURN_CREDENTIAL_FINISHED we are letting logonUI know
                        // that we have all the information we need and it should attempt to submit the
                        // serialized credential.
                        *pcpgsr = CPGSR_RETURN_CREDENTIAL_FINISHED;
                    }
                }
                else
                {
                    hr = HRESULT_FROM_WIN32(GetLastError());
                    if (SUCCEEDED(hr))
                    {
                        hr = E_FAIL;
                    }
                }

                if (FAILED(hr))
                {
                    CoTaskMemFree(pcpcs->rgbSerialization);
                }
            }
            else
            {
                hr = E_OUTOFMEMORY;
            }
        }
    }
    return hr;
}

struct REPORT_RESULT_STATUS_INFO
{
    NTSTATUS ntsStatus;
    NTSTATUS ntsSubstatus;
    PWSTR     pwzMessage;
    CREDENTIAL_PROVIDER_STATUS_ICON cpsi;
};

static const REPORT_RESULT_STATUS_INFO s_rgLogonStatusInfo[] =
{
    { STATUS_LOGON_FAILURE, STATUS_SUCCESS, const_cast<PWSTR>(L"Incorrect password or username."), CPSI_ERROR, },
    { STATUS_ACCOUNT_RESTRICTION, STATUS_ACCOUNT_DISABLED, const_cast<PWSTR>(L"The account is disabled."), CPSI_WARNING },
};

// ReportResult is completely optional.  Its purpose is to allow a credential to customize the string
// and the icon displayed in the case of a logon failure.  For example, we have chosen to
// customize the error shown in the case of bad username/password and in the case of the account
// being disabled.
HRESULT Credential::ReportResult(NTSTATUS ntsStatus,
    NTSTATUS ntsSubstatus,
    _Outptr_result_maybenull_ PWSTR* ppwszOptionalStatusText,
    _Out_ CREDENTIAL_PROVIDER_STATUS_ICON* pcpsiOptionalStatusIcon)
{
    *ppwszOptionalStatusText = nullptr;
    *pcpsiOptionalStatusIcon = CPSI_NONE;

    DWORD dwStatusInfo = (DWORD)-1;

    // Look for a match on status and substatus.
    for (DWORD i = 0; i < ARRAYSIZE(s_rgLogonStatusInfo); i++)
    {
        if (s_rgLogonStatusInfo[i].ntsStatus == ntsStatus && s_rgLogonStatusInfo[i].ntsSubstatus == ntsSubstatus)
        {
            dwStatusInfo = i;
            break;
        }
    }

    if ((DWORD)-1 != dwStatusInfo)
    {
        if (SUCCEEDED(SHStrDupW(s_rgLogonStatusInfo[dwStatusInfo].pwzMessage, ppwszOptionalStatusText)))
        {
            *pcpsiOptionalStatusIcon = s_rgLogonStatusInfo[dwStatusInfo].cpsi;
        }
    }

    // If we failed the logon, try to erase the password field.
    if (FAILED(HRESULT_FROM_NT(ntsStatus)))
    {
        if (m_pCredProvCredentialEvents)
        {
            m_pCredProvCredentialEvents->SetFieldString(reinterpret_cast<ICredentialProviderCredential*>(this), FI_PASSWORD, L"");
        }
    }

    // Since nullptr is a valid value for *ppwszOptionalStatusText and *pcpsiOptionalStatusIcon
    // this function can't fail.
    return S_OK;
}

// Gets the SID of the user corresponding to the credential.
HRESULT Credential::GetUserSid(_Outptr_result_nullonfailure_ PWSTR* ppszSid)
{
    *ppszSid = nullptr;
    HRESULT hr = E_UNEXPECTED;
    if (m_pszUserSid != nullptr)
    {
        hr = SHStrDupW(m_pszUserSid, ppszSid);
    }
    // Return S_FALSE with a null SID in ppszSid for the
    // credential to be associated with an empty user tile.

    return hr;
}

// GetFieldOptions to enable the password reveal button and touch keyboard auto-invoke in the password field.
HRESULT Credential::GetFieldOptions(DWORD dwFieldID,
    _Out_ CREDENTIAL_PROVIDER_CREDENTIAL_FIELD_OPTIONS* pcpcfo)
{
    *pcpcfo = CPCFO_NONE;
    if (dwFieldID == FI_PASSWORD)
    {
        *pcpcfo = CPCFO_ENABLE_PASSWORD_REVEAL;
    }
    else if (dwFieldID == FI_TILEIMAGE)
    {
        *pcpcfo = CPCFO_ENABLE_TOUCH_KEYBOARD_AUTO_INVOKE;
    }

    return S_OK;
}
