// Copyright (c) 2013 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#ifndef CEF_TESTS_CEFSIMPLE_CEFSIMPLE_WIN_H_
#define CEF_TESTS_CEFSIMPLE_CEFSIMPLE_WIN_H_

// #define UNICODE
#include <windows.h>
#include "cefsimple/simple_app.h"

struct sHookData;

// extern "C"
// {
    int CEFLaunch(sHookData* pData, CefRefPtr<SimpleApp> pCefApp);
// }

#endif  // CEF_TESTS_CEFSIMPLE_CEFSIMPLE_WIN_H_
