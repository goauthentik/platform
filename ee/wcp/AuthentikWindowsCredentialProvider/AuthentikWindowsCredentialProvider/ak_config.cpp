#include "pch.h"
#include "Credential.h"
#include "authentik_sys_bridge/ffi.h"
#include <spdlog/spdlog.h>

EXTERN_C GUID CLSID_CredentialProvider;

HRESULT ak_get_config_sysd(WCPOAuthConfig* config) {
    try {
        if (ak_sys_wcp_oauth_config(*config) == true) {
            return S_OK;
        }
    } catch (const std::exception& ex) {
        Debug("Exception in ak_sys_wcp_oauth_config");
        Debug(ex.what());
        return E_INVALIDARG;
    }
    return E_INVALIDARG;
}

HRESULT ak_get_config_registry(WCPOAuthConfig* config) {
    // Fetch authentication base URL from registry
    int nAttempts = 0;
    OLECHAR* pStr = NULL;
    HRESULT hr = S_OK;
    hr = StringFromCLSID(CLSID_CredentialProvider, &pStr);
    if (hr != S_OK)
    {
        return hr;
    }
    std::wstring strGUIDW = std::wstring((WCHAR*)pStr);
    CoTaskMemFree(pStr); // Free StringFromCLSID allocated memory

    std::string strGUID = std::string(strGUIDW.begin(), strGUIDW.end());
    std::string strSubKey = "CLSID\\" + strGUID;
    Debug(std::string("strGUID: " + strGUID).c_str());
    Debug(std::string("strSubKey: " + strSubKey).c_str());
    LSTATUS nStatus = -1;
    std::string strURL = "";
    strURL.resize(700);
    DWORD nSize = 0;
    { // Read URL from registry
        nAttempts = 0;
        while((
                (nStatus = RegGetValueA(
                    HKEY_CLASSES_ROOT,              // [in]                HKEY    hkey,
                    strSubKey.c_str(),              // [in, optional]      LPCSTR  lpSubKey,
                    "URL",                          // [in, optional]      LPCSTR  lpValue,
                    RRF_RT_REG_SZ,                  // [in, optional]      DWORD   dwFlags,
                    NULL,                           // [out, optional]     LPDWORD pdwType,
                    strURL.data(),                  // [out, optional]     PVOID   pvData,
                    &nSize                          // [in, out, optional] LPDWORD pcbData
                )) == ERROR_MORE_DATA
            ) && (nAttempts < 5) // in case the resize/ allocation fails
        )
        {
            strURL.resize(nSize);
            Debug(std::string("Resize: " + std::to_string(nSize)).c_str());
        }
        if (nStatus != ERROR_SUCCESS)
        {
            return E_FAIL;
        }
        // Remove trailing null
        while((strURL.find_last_of('\0') == (strURL.size() - 1)) && (strURL.size() > 0))
        {
            strURL = strURL.substr(0, strURL.size() - 1);
        }
        // Remove trailing `/`(s)
        while((strURL.find_last_of("/") == (strURL.size() - 1)) && (strURL.size() > 0))
        {
            strURL = strURL.substr(0, strURL.size() - 1);
        }
        Debug(std::string("strURL: " + strURL).c_str());

        config->url = strURL;
    }
    std::string strClientID = "";
    strClientID.resize(200);
    { // Read Client ID from registry
        nStatus = -1;
        nSize = 0;
        nAttempts = 0;
        while((
                (nStatus = RegGetValueA(
                    HKEY_CLASSES_ROOT,              // [in]                HKEY    hkey,
                    strSubKey.c_str(),              // [in, optional]      LPCSTR  lpSubKey,
                    "ClientID",                     // [in, optional]      LPCSTR  lpValue,
                    RRF_RT_REG_SZ,                  // [in, optional]      DWORD   dwFlags,
                    NULL,                           // [out, optional]     LPDWORD pdwType,
                    strClientID.data(),                  // [out, optional]     PVOID   pvData,
                    &nSize                          // [in, out, optional] LPDWORD pcbData
                )) == ERROR_MORE_DATA
            ) && (nAttempts < 5) // in case the resize/ allocation fails
        )
        {
            strClientID.resize(nSize);
            Debug(std::string("Resize: " + std::to_string(nSize)).c_str());
        }
        if (nStatus != ERROR_SUCCESS)
        {
            return E_FAIL;
        }
        // Remove trailing null
        while((strClientID.find_last_of('\0') == (strClientID.size() - 1)) && (strClientID.size() > 0))
        {
            strClientID = strClientID.substr(0, strClientID.size() - 1);
        }
        Debug(std::string("strClientID: " + strClientID).c_str());

        config->client_id = strClientID;
    }
    return hr;
}

HRESULT ak_get_config(WCPOAuthConfig* config) {
    if (ak_get_config_registry(config) == S_OK) {
        spdlog::debug("Loaded config from registry");
        return S_OK;
    }
    if (ak_get_config_sysd(config) == S_OK) {
        spdlog::debug("Loaded config from sysd");
        return S_OK;
    }
    spdlog::warn("Failed to load config");
    return E_INVALIDARG;
}
