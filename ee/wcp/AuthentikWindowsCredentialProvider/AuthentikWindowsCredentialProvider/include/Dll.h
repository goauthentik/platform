#pragma once

// #include "pch.h"
// #include <windows.h>
#include "ClassFactory.h"
#include "Common.h"
#include "GUIDs.h"

// global dll hinstance
extern HINSTANCE g_hinst;     //;- Needed to load bitmap for icon
#define HINST_THISDLL g_hinst //;- Needed to load bitmap for icon

void DllAddRef();
void DllRelease();
