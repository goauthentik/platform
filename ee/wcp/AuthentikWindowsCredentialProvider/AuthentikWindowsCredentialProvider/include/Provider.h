#pragma once

#include "Credential.h"


class Provider : public ICredentialProvider, public ICredentialProviderSetUserArray
{
//protected:
public:
	Provider();
    __override ~Provider();

public:
	// IUnknown
	IFACEMETHOD(QueryInterface)(__in REFIID riid, __deref_out void** ppv);
	IFACEMETHOD_(ULONG, AddRef)();
	IFACEMETHOD_(ULONG, Release)();

    // ICredentialProvider
    IFACEMETHOD(Advise)(_In_  ICredentialProviderEvents* pcpe, _In_  UINT_PTR upAdviseContext);
    IFACEMETHOD(UnAdvise)(void);

    IFACEMETHOD(SetUsageScenario)(CREDENTIAL_PROVIDER_USAGE_SCENARIO cpus, DWORD dwFlags);
    IFACEMETHOD(SetSerialization)(_In_  const CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION* pcpcs);

    IFACEMETHOD(GetFieldDescriptorCount)(_Out_  DWORD* pdwCount);
    IFACEMETHOD(GetFieldDescriptorAt)(DWORD dwIndex,
                                    _Outptr_result_nullonfailure_  CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR** ppcpfd);
    IFACEMETHOD(GetCredentialCount)(_Out_  DWORD* pdwCount,
                                    _Out_  DWORD* pdwDefault,
                                    _Out_  BOOL* pbAutoLogonWithDefault);
    IFACEMETHOD(GetCredentialAt)(DWORD dwIndex, _COM_Outptr_  ICredentialProviderCredential** ppcpc);

    IFACEMETHODIMP SetUserArray(_In_ ICredentialProviderUserArray* users);
    void SetCefApp(sHookData* pData);
    void ShutCefApp();

private:
	LONG                                    m_cRef = 0;             // Reference count
    struct sCredentials {
        Credential**    parrCredentials = nullptr;
        DWORD           dwCredentialsCount = 0;
        }                                   m_oCredentials;
    bool                                    m_fRecreateEnumeratedCredentials = false;
    CREDENTIAL_PROVIDER_USAGE_SCENARIO      m_cpus;
    ICredentialProviderUserArray*           m_pCredProviderUserArray = nullptr;
    CefRefPtr<SimpleApp>                    m_pCefApp = nullptr;

    void ReleaseEnumeratedCredentials();
    void CreateEnumeratedCredentials();

    IFACEMETHOD(EnumerateCredentials)();
};