// THIS CODE AND INFORMATION IS PROVIDED "AS IS" WITHOUT WARRANTY OF
// ANY KIND, EITHER EXPRESSED OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE IMPLIED WARRANTIES OF MERCHANTABILITY AND/OR FITNESS FOR A
// PARTICULAR PURPOSE.
//
// Copyright (C) Microsoft. All rights reserved.
/*
Abstract:
    Sample program for SHA 256 hashing using CNG
*/

#include "crypt.h"

#define WIN32_NO_STATUS
#include <windows.h>
#undef WIN32_NO_STATUS
#include <bcrypt.h>
#include <wincrypt.h>

#include <cstdlib>
#include <ctime>

#define NT_SUCCESS(Status)          (((NTSTATUS)(Status)) >= 0)
#define STATUS_UNSUCCESSFUL         ((NTSTATUS)0xC0000001L)

const std::string   strChars        = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const size_t        nCharsLen       = strChars.length();

bool GenerateRandomInt(size_t& nRandom, const size_t nBytesCount)
{
    if (!((nBytesCount >= 1) && (nBytesCount <= sizeof(nRandom))))
    {
        return false;
    }
    NTSTATUS            status          = STATUS_UNSUCCESSFUL;

    // The output seems to write to atleast 4 bytes, so assigning 8 just in case (64-bit machines).
    UCHAR               bBuffer[sizeof(nRandom)];
    ULONG               cbBuffer        = sizeof(bBuffer);

    memset(bBuffer, 0, cbBuffer);

    if(!NT_SUCCESS(status = BCryptGenRandom(
        /*[in, out] BCRYPT_ALG_HANDLE*/     NULL,
        /*[in, out] PUCHAR*/                bBuffer,
        /*[in]      ULONG*/                 (ULONG)nBytesCount,
        /*[in]      ULONG*/                 BCRYPT_USE_SYSTEM_PREFERRED_RNG
        ))
    )
    {
        wprintf(L"**** Error 0x%x returned by BCryptGenRandom\n", status);
        nRandom = 0;
        return false;
    }
    nRandom = 0;
    size_t nTmp = 0;
    for (SIZE_T i = 0; i < cbBuffer; ++i)
    {
        nTmp = bBuffer[i];
        nRandom += nTmp << (8*i);
    }

    return true;
}

size_t GetRandomInt(const size_t nExclusiveUpperBound)
{
    size_t nBytesCount = 0;
    size_t nMaxValue = 255;
    for (nBytesCount = 1; nBytesCount < 8; ++nBytesCount)
    {
        size_t nDiv = (nExclusiveUpperBound / nMaxValue);
        if ((nDiv == 0) || ((nDiv == 1) && ((nExclusiveUpperBound % nMaxValue) == 0)))
        {
            break;
        }
        nMaxValue = nMaxValue << 8;
        nMaxValue = nMaxValue + 255;
    }
    size_t nRandom = 0;
    if (! GenerateRandomInt(nRandom, nBytesCount))
    {
        // fallback
        srand((unsigned int)time(NULL));
        nRandom = rand();
    }
    nRandom = nRandom % nExclusiveUpperBound;
    return nRandom;
}

std::string GetRandomStr(const size_t nLength)
{
    std::string strRet = "";
    for (size_t i = 0; i < nLength; ++i)
    {
        strRet.append(1, strChars.at(GetRandomInt(nCharsLen)));
    }
    return strRet;
}

std::wstring GetRandomWStr(const size_t nLength)
{
    std::string str = GetRandomStr(nLength);
    return std::wstring(str.begin(), str.end());
}
