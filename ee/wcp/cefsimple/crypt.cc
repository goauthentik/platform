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

bool Hash_SHA256(const std::string& strData, std::string& strHash)
{
    BCRYPT_ALG_HANDLE       hAlg            = NULL;
    BCRYPT_HASH_HANDLE      hHash           = NULL;
    NTSTATUS                status          = STATUS_UNSUCCESSFUL;
    DWORD                   cbData          = 0,
                            cbHash          = 0,
                            cbHashObject    = 0;
    PBYTE                   pbHashObject    = NULL;
    PBYTE                   pbHash          = NULL;

    BYTE data[1000]; //- todo: add dynamic allocation to match input data length
    ULONG dataSize = (ULONG)(strData.size());
    if (!(dataSize <= sizeof(data)))
    {
        wprintf(L"**** Error: Insufficient fixed size buffer. Input data length is too large.\n");
        return false;
    }

    bool bRet = false;
    for (size_t i = 0; i < strData.size(); ++i)
    {
        data[i] = (BYTE)(strData[i]);
    }

    //open an algorithm handle
    if(!NT_SUCCESS(status = BCryptOpenAlgorithmProvider(
                                                &hAlg,
                                                BCRYPT_SHA256_ALGORITHM,
                                                NULL,
                                                0)))
    {
        wprintf(L"**** Error 0x%x returned by BCryptOpenAlgorithmProvider\n", status);
        goto Cleanup;
    }

    //calculate the size of the buffer to hold the hash object
    if(!NT_SUCCESS(status = BCryptGetProperty(
                                        hAlg,
                                        BCRYPT_OBJECT_LENGTH,
                                        (PBYTE)&cbHashObject,
                                        sizeof(DWORD),
                                        &cbData,
                                        0)))
    {
        wprintf(L"**** Error 0x%x returned by BCryptGetProperty\n", status);
        goto Cleanup;
    }

    //allocate the hash object on the heap
    pbHashObject = (PBYTE)HeapAlloc (GetProcessHeap (), 0, cbHashObject);
    if(NULL == pbHashObject)
    {
        wprintf(L"**** memory allocation failed\n");
        goto Cleanup;
    }

   //calculate the length of the hash
    if(!NT_SUCCESS(status = BCryptGetProperty(
                                        hAlg,
                                        BCRYPT_HASH_LENGTH,
                                        (PBYTE)&cbHash,
                                        sizeof(DWORD),
                                        &cbData,
                                        0)))
    {
        wprintf(L"**** Error 0x%x returned by BCryptGetProperty\n", status);
        goto Cleanup;
    }

    //allocate the hash buffer on the heap
    pbHash = (PBYTE)HeapAlloc (GetProcessHeap (), 0, cbHash);
    if(NULL == pbHash)
    {
        wprintf(L"**** memory allocation failed\n");
        goto Cleanup;
    }

    //create a hash
    if(!NT_SUCCESS(status = BCryptCreateHash(
                                        hAlg,
                                        &hHash,
                                        pbHashObject,
                                        cbHashObject,
                                        NULL,
                                        0,
                                        0)))
    {
        wprintf(L"**** Error 0x%x returned by BCryptCreateHash\n", status);
        goto Cleanup;
    }


    //hash some data
    if(!NT_SUCCESS(status = BCryptHashData(
                                        hHash,
                                        (PBYTE)data,
                                        dataSize,
                                        0)))
    {
        wprintf(L"**** Error 0x%x returned by BCryptHashData\n", status);
        goto Cleanup;
    }

    //close the hash
    if(!NT_SUCCESS(status = BCryptFinishHash(
                                        hHash,
                                        pbHash,
                                        cbHash,
                                        0)))
    {
        wprintf(L"**** Error 0x%x returned by BCryptFinishHash\n", status);
        goto Cleanup;
    }

    wprintf(L"Success!\n");

    LPSTR pszString = NULL;
    DWORD cchString = 0;
    BOOL bEncodingRet = FALSE;
    BOOL bAlloc = FALSE;
    for (size_t i = 0; i < 2; ++i)
    {
        if (cchString > 0)
        {
            pszString = new char[cchString];
            bAlloc = TRUE;
        }
        bEncodingRet = CryptBinaryToStringA(
            /*[in]            const BYTE* */    pbHash,
            /*[in]            DWORD*/           cbHash,
            /*[in]            DWORD*/           CRYPT_STRING_BASE64URI | CRYPT_STRING_NOCRLF,
            /*[out, optional] LPSTR*/           pszString,
            /*[in, out]       DWORD* */         &cchString
            );
    }
    strHash = "";
    if (bEncodingRet == TRUE)
    {
        bRet = true;
        for (size_t i = 0; i < cchString; ++i)
        {
            strHash.append(1, pszString[i]);
        }
    }
    if (bAlloc == TRUE)
    {
        delete pszString;
    }

Cleanup:

    if(hAlg)
    {
        BCryptCloseAlgorithmProvider(hAlg,0);
    }

    if (hHash)
    {
        BCryptDestroyHash(hHash);
    }

    if(pbHashObject)
    {
        HeapFree(GetProcessHeap(), 0, pbHashObject);
    }

    if(pbHash)
    {
        HeapFree(GetProcessHeap(), 0, pbHash);
    }

    return bRet;
}

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
