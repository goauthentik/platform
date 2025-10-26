#pragma once

#include "pch.h"
#include "Common.h"
#include "resource.h"
//#include "Dll.h"
#include "Helpers.h"
#include <map>

struct sHookData
{
    sHookData() = default;
    void Update(PWSTR pUserSid, HINSTANCE phInstance)
    {
        UserSid = pUserSid;
        hInstance = phInstance;
    }
    void UpdateUser(const std::string& strUser)
    {
        strUsername = strUser;
    }
    void UpdateBaseURL(const std::string& strURL)
    {
        strBaseURL = strURL;
    }
    void UpdateClientID(const std::string& strAuthClientID)
    {
        strClientID = strAuthClientID;
    }
    PWSTR           UserSid = NULL;
    HINSTANCE       hInstance = NULL;
    std::string     strUsername = "";
    std::string     strBaseURL = "";
    std::string     strClientID = "";
};

class Credential : public ICredentialProviderCredential2,
                          IConnectableCredentialProviderCredential,
                          ICredentialProviderCredentialWithFieldOptions
{
public:
	Credential();

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
    sHookData m_oHookData;
    std::wstring m_strPass = L"";
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

