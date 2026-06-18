use std::sync::Mutex;

use windows::{
    core::{implement, w, Ref, Result, BOOL, PCWSTR, PWSTR},
    Win32::{
        Foundation::{E_ABORT, E_INVALIDARG, E_NOTIMPL, FALSE, NTSTATUS},
        Graphics::Gdi::HBITMAP,
        UI::Shell::{
            IConnectableCredentialProviderCredential,
            IConnectableCredentialProviderCredential_Impl, ICredentialProviderCredential,
            ICredentialProviderCredential2, ICredentialProviderCredential2_Impl,
            ICredentialProviderCredentialEvents, ICredentialProviderCredential_Impl,
            IQueryContinueWithStatus, CPFIS_NONE, CPFS_DISPLAY_IN_SELECTED_TILE,
            CPGSR_NO_CREDENTIAL_NOT_FINISHED, CPSI_SUCCESS,
            CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION,
            CREDENTIAL_PROVIDER_FIELD_INTERACTIVE_STATE, CREDENTIAL_PROVIDER_FIELD_STATE,
            CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE, CREDENTIAL_PROVIDER_STATUS_ICON,
        },
    },
};

use crate::auth::{run_auth_flow, AuthOutcome};

#[implement(
    ICredentialProviderCredential,
    ICredentialProviderCredential2,
    IConnectableCredentialProviderCredential
)]
pub struct Credential {
    _fields: [PWSTR; 4],
    /// Username reported by the Tauri auth window once it signals completion.
    /// Equivalent to `sHookData::strUsername` in the C++ implementation. Set in
    /// `Connect`, consumed by `GetSerialization`.
    auth_user: Mutex<Option<String>>,
}

impl Credential {
    pub fn new() -> Self {
        Self {
            _fields: [PWSTR::null(); 4],
            auth_user: Mutex::new(None),
        }
    }
}

impl ICredentialProviderCredential_Impl for Credential_Impl {
    fn Advise(&self, _pcpce: Ref<'_, ICredentialProviderCredentialEvents>) -> Result<()> {
        Ok(())
    }

    fn UnAdvise(&self) -> Result<()> {
        Ok(())
    }

    fn SetSelected(&self) -> Result<BOOL> {
        Ok(FALSE)
    }

    fn SetDeselected(&self) -> Result<()> {
        Ok(())
    }

    fn GetFieldState(
        &self,
        dwfieldid: u32,
        pcpfs: *mut CREDENTIAL_PROVIDER_FIELD_STATE,
        pcpfis: *mut CREDENTIAL_PROVIDER_FIELD_INTERACTIVE_STATE,
    ) -> Result<()> {
        unsafe {
            match dwfieldid {
                0 => {
                    *pcpfs = CPFS_DISPLAY_IN_SELECTED_TILE;
                    *pcpfis = CPFIS_NONE;
                }
                1 => {
                    *pcpfs = CPFS_DISPLAY_IN_SELECTED_TILE;
                    *pcpfis = CPFIS_NONE;
                }
                _ => return Err(E_INVALIDARG.into()),
            }
        }
        Ok(())
    }

    fn GetStringValue(&self, dwfieldid: u32) -> Result<PWSTR> {
        match dwfieldid {
            0 => {
                let text = "OAuth Authentication";
                Ok(PWSTR::from_raw(
                    text.encode_utf16()
                        .chain(std::iter::once(0))
                        .collect::<Vec<u16>>()
                        .as_mut_ptr(),
                ))
            }
            1 => Ok(PWSTR::null()),
            _ => return Err(E_INVALIDARG.into()),
        }
    }

    fn GetBitmapValue(&self, _dwfieldid: u32) -> Result<HBITMAP> {
        Err(E_NOTIMPL.into())
    }

    fn GetCheckboxValue(
        &self,
        _dwfieldid: u32,
        _pbchecked: *mut BOOL,
        _ppszlabel: *mut PWSTR,
    ) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn GetSubmitButtonValue(&self, dwfieldid: u32) -> Result<u32> {
        if dwfieldid == 1 {
            Ok(1)
        } else {
            Err(E_INVALIDARG.into())
        }
    }

    fn GetComboBoxValueCount(
        &self,
        dwfieldid: u32,
        pcitems: *mut u32,
        pdwselecteditem: *mut u32,
    ) -> Result<()> {
        if dwfieldid == 1 {
            unsafe {
                *pcitems = 3; // Microsoft, Google, GitHub
                *pdwselecteditem = 0; // Default to Microsoft
            }
            Ok(())
        } else {
            Err(E_NOTIMPL.into())
        }
    }

    fn GetComboBoxValueAt(&self, _dwfieldid: u32, _dwitem: u32) -> Result<PWSTR> {
        Err(E_INVALIDARG.into())
    }

    fn SetStringValue(&self, _dwfieldid: u32, _psz: &PCWSTR) -> Result<()> {
        Ok(())
    }

    fn SetCheckboxValue(&self, _dwfieldid: u32, _bchecked: BOOL) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn SetComboBoxSelectedValue(&self, _dwfieldid: u32, _dwselecteditem: u32) -> Result<()> {
        Err(E_INVALIDARG.into())
    }

    fn CommandLinkClicked(&self, _dwfieldid: u32) -> Result<()> {
        Ok(())
    }

    fn GetSerialization(
        &self,
        pcpgsr: *mut CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE,
        _pcpcs: *mut CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION,
        ppszoptionalstatustext: *mut PWSTR,
        pcpsioptionalstatusicon: *mut CREDENTIAL_PROVIDER_STATUS_ICON,
    ) -> Result<()> {
        // By this point Connect() has already run the auth flow and stored the
        // signalled username (mirrors the C++ flow where Connect blocks until
        // sHookData reports complete, then GetSerialization packs the logon).
        let user = self.auth_user.lock().unwrap().clone();

        unsafe {
            match user {
                Some(username) => {
                    log::info!(
                        "GetSerialization: auth signalled for '{username}'. \
                         Credential packing (KerbInteractiveUnlockLogonPack) is TODO."
                    );
                    // TODO: build KERB_INTERACTIVE_UNLOCK_LOGON for `username`
                    // (reset local password + pack) and return
                    // CPGSR_RETURN_CREDENTIAL_FINISHED. Deferred until the real
                    // authentication/account-mapping work lands.
                    let msg = format!("Authenticated as {username} (logon packing TODO)");
                    *ppszoptionalstatustext = crate::utils::cotask_pwstr(&msg);
                    *pcpsioptionalstatusicon = CPSI_SUCCESS;
                    *pcpgsr = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
                }
                None => {
                    *pcpgsr = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
                }
            }
        }
        Ok(())
        // // Launch OAuth authentication
        // match self.launch_oauth_auth() {
        //     Ok(token_info) => {
        //         unsafe {
        //             *pcpgsr = CPGSR_RETURN_CREDENTIAL_FINISHED;

        //             // Create credential serialization
        //             let username = token_info.user_info.email.clone();
        //             let username_wide: Vec<u16> =
        //                 username.encode_utf16().chain(std::iter::once(0)).collect();

        //             (*pcpcs).ulAuthenticationPackage = 0; // MSV1_0
        //             (*pcpcs).cbSerialization = (username_wide.len() * 2) as u32;
        //             (*pcpcs).rgbSerialization = username_wide.as_ptr() as *mut u8;

        //             // Set success status
        //             let status_text = format!("Welcome, {}!", token_info.user_info.name);
        //             *ppszoptionalstatustext = PWSTR::from_raw(
        //                 status_text
        //                     .encode_utf16()
        //                     .chain(std::iter::once(0))
        //                     .collect::<Vec<u16>>()
        //                     .as_mut_ptr(),
        //             );
        //             *pcpsioptionalstatusicon = CPSI_SUCCESS;
        //         }
        //         Ok(())
        //     }
        //     Err(e) => {
        //         unsafe {
        //             *pcpgsr = CPGSR_NO_CREDENTIAL_NOT_FINISHED;

        //             let error_text = "OAuth authentication failed";
        //             *ppszoptionalstatustext = PWSTR::from_raw(
        //                 error_text
        //                     .encode_utf16()
        //                     .chain(std::iter::once(0))
        //                     .collect::<Vec<u16>>()
        //                     .as_mut_ptr(),
        //             );
        //             *pcpsioptionalstatusicon = CPSI_ERROR;
        //         }
        //         Err(e)
        //     }
        // }
    }

    fn ReportResult(
        &self,
        _ntsresult: NTSTATUS,
        _ntssubstatus: NTSTATUS,
        _ppszoptionalstatustext: *mut PWSTR,
        _pcpsioptionalstatusicon: *mut CREDENTIAL_PROVIDER_STATUS_ICON,
    ) -> Result<()> {
        Ok(())
    }
}

impl ICredentialProviderCredential2_Impl for Credential_Impl {
    fn GetUserSid(&self) -> Result<PWSTR> {
        Ok(PWSTR::null())
    }
}

impl IConnectableCredentialProviderCredential_Impl for Credential_Impl {
    fn Connect(&self, pqcws: Ref<'_, IQueryContinueWithStatus>) -> Result<()> {
        log::info!("Connect: launching Tauri auth window and waiting for signal");

        // Show a status message while the external window is up, like the C++
        // provider's pqcws->SetStatusMessage(L"Please sign in...").
        if let Some(q) = pqcws.as_ref() {
            unsafe {
                let _ = q.SetStatusMessage(w!("Please sign in to your authentik account..."));
            }
        }

        // Blocks until the auth-app signals completion or cancellation over the
        // named pipe (the equivalent of the C++ `while (!IsComplete())` loop).
        match run_auth_flow() {
            AuthOutcome::Completed { username } => {
                log::info!("Connect: authentication complete (user: {username})");
                *self.auth_user.lock().unwrap() = Some(username);
                Ok(())
            }
            AuthOutcome::Cancelled => {
                log::info!("Connect: authentication cancelled");
                *self.auth_user.lock().unwrap() = None;
                // Returning an error (not S_OK) avoids the Disconnect button and
                // tells LogonUI the connect attempt did not succeed.
                Err(E_ABORT.into())
            }
        }
    }

    fn Disconnect(&self) -> Result<()> {
        log::debug!("Credential::IConnectableCredentialProviderCredential_Impl::Disconnect");
        Ok(())
    }
}
