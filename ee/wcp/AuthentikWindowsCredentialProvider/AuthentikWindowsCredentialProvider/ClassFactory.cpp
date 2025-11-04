#include "pch.h"

#include "ClassFactory.h"

ClassFactory::ClassFactory() : m_cRef(1) {}

IFACEMETHODIMP ClassFactory::QueryInterface(__in REFIID riid,
                                            __deref_out void **ppv) {
  static const QITAB qit[] = {
      QITABENT(ClassFactory, IClassFactory),
      {0},
  };
  return QISearch(this, qit, riid, ppv);
}

IFACEMETHODIMP_(ULONG) ClassFactory::AddRef() {
  return InterlockedIncrement(&m_cRef);
}

IFACEMETHODIMP_(ULONG) ClassFactory::Release() {
  if (InterlockedDecrement(&m_cRef) == 0) {
    delete this;
    return 0;
  }
  return m_cRef;
}

IFACEMETHODIMP ClassFactory::CreateInstance(__in IUnknown *pUnkOuter,
                                            __in REFIID riid,
                                            __deref_out void **ppv) {
  HRESULT hr;
  if (!pUnkOuter) {
    Provider *pProvider = new (std::nothrow) Provider();
    if (pProvider) {
      hr = pProvider->QueryInterface(riid, ppv);
      pProvider->Release();
    } else {
      hr = E_OUTOFMEMORY;
    }
  } else {
    *ppv = NULL;
    hr = CLASS_E_NOAGGREGATION;
  }
  return hr;
}

IFACEMETHODIMP ClassFactory::LockServer(__in BOOL bLock) {
  if (bLock) {
    DllAddRef();
  } else {
    DllRelease();
  }
  return S_OK;
}

ClassFactory::~ClassFactory() {}
