#pragma once

#include <unknwn.h>
#include "Dll.h"
#include "GUIDs.h"
#include "Provider.h"

class ClassFactory : public IClassFactory {
 public:
  ClassFactory();

  // IUnknown
  IFACEMETHOD(QueryInterface)(__in REFIID riid, __deref_out void** ppv);
  IFACEMETHOD_(ULONG, AddRef)();
  IFACEMETHOD_(ULONG, Release)();

  // IClassFactory
  IFACEMETHOD(CreateInstance)(__in IUnknown* pUnkOuter, __in REFIID riid, __deref_out void** ppv);
  IFACEMETHOD(LockServer)(__in BOOL bLock);

 private:
  LONG m_cRef = 0;
  ~ClassFactory();
};