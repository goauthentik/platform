
#include "Provider.h"
#include "pch.h"

extern void DllAddRef();
extern void DllRelease();

Provider::Provider() : m_cRef(1) { DllAddRef(); }

Provider::~Provider() {
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
