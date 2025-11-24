#pragma once

#include "pch.h"
#include "Common.h"
#include "resource.h"
//#include "Dll.h"
#include "Helpers.h"
#include <map>
// extern "C"
// {
#include "cefsimple/cefsimple_win.h"
// }
#include <functional>

struct sHookData
{
    sHookData() = default;
    void Update(PWSTR pUserSid, HINSTANCE phInstance)
    {
        oMutex.lock();
        UserSid = pUserSid;
        hInstance = phInstance;
        oMutex.unlock();
    }
    void UpdateUser(const std::string& strUser)
    {
        oMutex.lock();
        strUsername = strUser;
        oMutex.unlock();
    }
    void UpdateNonce(const std::string& nonce)
    {
        oMutex.lock();
        strNonce = nonce;
        oMutex.unlock();
    }
    void SetExit(const bool bVal)
    {
        Debug(std::string("SetExit: " + std::to_string((size_t)bVal)).c_str());
        oMutex.lock();
        bExit = bVal;
        oMutex.unlock();
    }
    bool IsExit()
    {
        bool bVal = false;
        oMutex.lock();
        bVal = bExit;
        oMutex.unlock();
        return bVal;
    }
    void SetCancel(const bool bVal)
    {
        Debug(std::string("SetCancel: " + std::to_string((size_t)bVal)).c_str());
        oMutex.lock();
        bCancel = bVal;
        oMutex.unlock();
    }
    bool IsCancel()
    {
        bool bVal = false;
        oMutex.lock();
        bVal = bCancel;
        oMutex.unlock();
        return bVal;
    }
    void SetComplete(const bool bVal)
    {
        Debug(std::string("SetComplete: " + std::to_string((size_t)bVal)).c_str());
        oMutex.lock();
        bComplete = bVal;
        oMutex.unlock();
    }
    bool IsComplete()
    {
        bool bVal = false;
        oMutex.lock();
        bVal = bComplete;
        oMutex.unlock();
        return bVal;
    }
    void UpdateStatus(const std::wstring& strWText)
    {
        oMutex.lock();
        oStatus.strWText = strWText;
        oStatus.bUpdate = true;
        oMutex.unlock();
    }
    bool ReadStatus(std::wstring& strWText)
    {
        bool bRet = false;
        oMutex.lock();
        if (oStatus.bUpdate)
        {
            bRet = true;
            oStatus.bUpdate = false;
            strWText = oStatus.strWText;
        }
        oMutex.unlock();
        return bRet;
    }
    bool IsStatus()
    {
        bool bVal = false;
        oMutex.lock();
        bVal = oStatus.bUpdate;
        oMutex.unlock();
        return bVal;
    }
    PWSTR           UserSid = NULL;
    HINSTANCE       hInstance = NULL;
    std::string     strUsername = "";
    std::string     strNonce = "";
    bool            bExit = false;      // flag to exit the custom loop
    bool            bComplete = false;  // UI call complete
    bool            bCancel = false;    // whether the user clicked cancel
    std::mutex      oMutex;
    struct sStatus
    {
        std::wstring    strWText = L"";
        bool            bUpdate = "";
    };
    sStatus             oStatus;        // to display in UI
    IQueryContinueWithStatus* pqcws = nullptr;
};

class Credential : public ICredentialProviderCredential2,
                          IConnectableCredentialProviderCredential,
                          ICredentialProviderCredentialWithFieldOptions
{
public:
	Credential();
	// Credential(CefRefPtr<SimpleApp> pCefApp);

	// IUnknown
	IFACEMETHOD(QueryInterface)(__in REFIID riid, __deref_out void** ppv);
	IFACEMETHOD_(ULONG, AddRef)();
	IFACEMETHOD_(ULONG, Release)();

    // ICredentialProviderCredential
    IFACEMETHOD (Advise)(_In_ ICredentialProviderCredentialEvents* pcpce);
    IFACEMETHOD (UnAdvise)();

    IFACEMETHOD (SetSelected)(_Out_ BOOL* pbAutoLogon);
    IFACEMETHOD (SetDeselected)();

    IFACEMETHOD (GetFieldState)(DWORD dwFieldID,
        _Out_ CREDENTIAL_PROVIDER_FIELD_STATE* pcpfs,
        _Out_ CREDENTIAL_PROVIDER_FIELD_INTERACTIVE_STATE* pcpfis);

    IFACEMETHOD (GetStringValue)(DWORD dwFieldID, _Outptr_result_nullonfailure_ PWSTR* ppwsz);
    IFACEMETHOD (GetBitmapValue)(DWORD dwFieldID, _Outptr_result_nullonfailure_ HBITMAP* phbmp);
    IFACEMETHOD (GetCheckboxValue)(DWORD dwFieldID, _Out_ BOOL* pbChecked, _Outptr_result_nullonfailure_ PWSTR* ppwszLabel);
    IFACEMETHOD (GetComboBoxValueCount)(DWORD dwFieldID, _Out_ DWORD* pcItems, _Deref_out_range_(< , *pcItems) _Out_ DWORD* pdwSelectedItem);
    IFACEMETHOD (GetComboBoxValueAt)(DWORD dwFieldID, DWORD dwItem, _Outptr_result_nullonfailure_ PWSTR* ppwszItem);
    IFACEMETHOD (GetSubmitButtonValue)(DWORD dwFieldID, _Out_ DWORD* pdwAdjacentTo);

    IFACEMETHOD (SetStringValue)(DWORD dwFieldID, _In_ PCWSTR pwz);
    IFACEMETHOD (SetCheckboxValue)(DWORD dwFieldID, BOOL bChecked);
    IFACEMETHOD (SetComboBoxSelectedValue)(DWORD dwFieldID, DWORD dwSelectedItem);
    IFACEMETHOD (CommandLinkClicked)(DWORD dwFieldID);
    IFACEMETHOD (Connect)(IQueryContinueWithStatus* pqcws);
    IFACEMETHOD (Disconnect)();

    IFACEMETHOD (GetSerialization)(_Out_ CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE* pcpgsr,
        _Out_ CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION* pcpcs,
        _Outptr_result_maybenull_ PWSTR* ppwszOptionalStatusText,
        _Out_ CREDENTIAL_PROVIDER_STATUS_ICON* pcpsiOptionalStatusIcon);
    IFACEMETHOD (ReportResult)(NTSTATUS ntsStatus,
        NTSTATUS ntsSubstatus,
        _Outptr_result_maybenull_ PWSTR* ppwszOptionalStatusText,
        _Out_ CREDENTIAL_PROVIDER_STATUS_ICON* pcpsiOptionalStatusIcon);


    // ICredentialProviderCredential2
    IFACEMETHOD (GetUserSid)(_Outptr_result_nullonfailure_ PWSTR* ppszSid);

    // ICredentialProviderCredentialWithFieldOptions
    IFACEMETHOD (GetFieldOptions)(DWORD dwFieldID,
        _Out_ CREDENTIAL_PROVIDER_CREDENTIAL_FIELD_OPTIONS* pcpcfo);

    IFACEMETHOD (Initialize)(CREDENTIAL_PROVIDER_USAGE_SCENARIO cpus,
        _In_ CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR const* rgcpfd,
        _In_ FIELD_STATE_PAIR const* rgfsp,
        _In_ ICredentialProviderUser* pcpUser);
    static LRESULT CALLBACK Credential::CallWndProc(
        _In_ int nCode,
        _In_ WPARAM wParam,
        _In_ LPARAM lParam
    );
    static LRESULT CALLBACK Credential::WndProc(
        _In_ HWND hWnd,
        _In_ UINT uMsg,
        _In_ WPARAM wParam,
        _In_ LPARAM lParam
    );
    static HHOOK hHook;
    static LONG_PTR hWndProcOrig;
    static HWND m_hWndOwner;
    sHookData m_oHookData;
    std::wstring m_strPass = L"";
    struct sCefAppData
    {
        sCefAppData() = default;
        CefRefPtr<SimpleApp>                        pCefApp             = nullptr;
        bool                                        bInit               = false;            // whether CefApp has initialized (OnContextInitialized())
        std::mutex oMutex;
        void SetInit(const bool bVal)
        {
            Debug(std::string("sCefAppData::SetInit: " + std::to_string((size_t)bVal)).c_str());
            oMutex.lock();
            bInit = bVal;
            oMutex.unlock();
        }
        bool IsInit()
        {
            bool bVal = false;
            oMutex.lock();
            bVal = bInit;
            oMutex.unlock();
            return bVal;
        }
    };
    static sCefAppData                              m_oCefAppData;
    static inline std::function<void(sHookData*)>   m_pProvCallSet  = [] (sHookData*) {};
    static inline std::function<void()>             m_pProvCallShut = [] () {};
private:
	LONG m_cRef                                                         = 0;
	virtual ~Credential();
    CREDENTIAL_PROVIDER_USAGE_SCENARIO      m_cpus;                                         // The usage scenario for which we were enumerated.
    CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR    m_rgCredProvFieldDescriptors[FI_NUM_FIELDS];    // An array holding the type and name of each field in the tile.
    FIELD_STATE_PAIR                        m_rgFieldStatePairs[FI_NUM_FIELDS];             // An array holding the state of each field in the tile.
    PWSTR                                   m_rgFieldStrings[FI_NUM_FIELDS];                // An array holding the string value of each field. This is different from the name of the field held in _rgCredProvFieldDescriptors.
    PWSTR                                   m_pszUserSid                = nullptr; //-
    PWSTR                                   m_pszQualifiedUserName      = nullptr;          // The user name that's used to pack the authentication buffer
    ICredentialProviderCredentialEvents2*   m_pCredProvCredentialEvents = nullptr;          // Used to update fields.
    //// CredentialEvents2 for Begin and EndFieldUpdates.
    BOOL                                    m_fChecked                  = false;            // Tracks the state of our checkbox.
    DWORD                                   m_dwComboIndex = 0;                             // Tracks the current index of our combobox.
    bool                                    m_fShowControls             = false;            // Tracks the state of our show/hide controls link.
    bool                                    m_fIsLocalUser              = false;            // If the cred prov is assosiating with a local user tile
    std::vector<std::thread> m_vecThreads;
    static std::map<PWSTR, std::thread> m_mapThreads;
};

