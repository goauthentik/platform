use std::sync::Mutex;

use windows::{
    core::{implement, w, Ref, Result, BOOL, PCWSTR, PWSTR},
    Win32::{
        Foundation::{E_ABORT, E_FAIL, E_INVALIDARG, E_NOTIMPL, FALSE, NTSTATUS},
        Graphics::Gdi::HBITMAP,
        System::Com::CoTaskMemFree,
        UI::Shell::{
            IConnectableCredentialProviderCredential,
            IConnectableCredentialProviderCredential_Impl, ICredentialProviderCredential,
            ICredentialProviderCredential2, ICredentialProviderCredential2_Impl,
            ICredentialProviderCredentialEvents, ICredentialProviderCredential_Impl,
            IQueryContinueWithStatus, CPFIS_NONE, CPFS_DISPLAY_IN_SELECTED_TILE,
            CPGSR_NO_CREDENTIAL_NOT_FINISHED, CPGSR_RETURN_CREDENTIAL_FINISHED, CPSI_NONE,
            CPSI_SUCCESS, CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION,
            CREDENTIAL_PROVIDER_FIELD_INTERACTIVE_STATE, CREDENTIAL_PROVIDER_FIELD_STATE,
            CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE, CREDENTIAL_PROVIDER_STATUS_ICON,
            CREDENTIAL_PROVIDER_USAGE_SCENARIO,
        },
    },
};

use crate::auth::{run_auth_flow, AuthOutcome};

struct AuthData {
    username: String,
    password: String,
}

#[implement(
    ICredentialProviderCredential,
    ICredentialProviderCredential2,
    IConnectableCredentialProviderCredential
)]
pub struct Credential {
    _fields: [PWSTR; 4],
    /// Username + random password set during Connect(), consumed by GetSerialization().
    auth_data: Mutex<Option<AuthData>>,
    /// Usage scenario from the provider, needed for KERB MessageType selection.
    cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
}

impl Credential {
    pub fn new(cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO) -> Self {
        Self {
            _fields: [PWSTR::null(); 4],
            auth_data: Mutex::new(None),
            cpus,
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
                0 | 1 => {
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
            _ => Err(E_INVALIDARG.into()),
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
        _dwfieldid: u32,
        _pcitems: *mut u32,
        _pdwselecteditem: *mut u32,
    ) -> Result<()> {
        Err(E_NOTIMPL.into())
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
        pcpcs: *mut CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION,
        ppszoptionalstatustext: *mut PWSTR,
        pcpsioptionalstatusicon: *mut CREDENTIAL_PROVIDER_STATUS_ICON,
    ) -> Result<()> {
        let data = self.auth_data.lock().unwrap().take();

        unsafe {
            *pcpsioptionalstatusicon = CPSI_NONE;
            *ppszoptionalstatustext = PWSTR::null();

            let Some(AuthData { username, password }) = data else {
                *pcpgsr = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
                return Ok(());
            };

            let domain = crate::helpers::get_computer_name();

            match crate::helpers::kerb_interactive_unlock_logon_pack(
                &domain, &username, &password, self.cpus,
            ) {
                Ok((buf, len)) => {
                    match crate::helpers::retrieve_negotiate_auth_package() {
                        Ok(auth_pkg) => {
                            (*pcpcs).rgbSerialization = buf;
                            (*pcpcs).cbSerialization = len;
                            (*pcpcs).ulAuthenticationPackage = auth_pkg;
                            (*pcpcs).clsidCredentialProvider = crate::CLSID_CREDENTIAL_PROVIDER;
                            *pcpsioptionalstatusicon = CPSI_SUCCESS;
                            *pcpgsr = CPGSR_RETURN_CREDENTIAL_FINISHED;
                            log::info!("GetSerialization: packed credential for '{username}'");
                        }
                        Err(e) => {
                            log::error!("retrieve_negotiate_auth_package failed: {e}");
                            CoTaskMemFree(Some(buf as *const _));
                            *pcpgsr = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
                        }
                    }
                }
                Err(e) => {
                    log::error!("kerb_interactive_unlock_logon_pack failed: {e}");
                    *pcpgsr = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
                }
            }
        }
        Ok(())
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
        log::info!("Connect: launching Tauri auth window");

        if let Some(q) = pqcws.as_ref() {
            unsafe {
                let _ = q.SetStatusMessage(w!("Please sign in to your authentik account..."));
            }
        }

        match run_auth_flow() {
            AuthOutcome::Completed { username } => {
                log::info!("Connect: authentication complete (user: {username})");

                let password = crate::helpers::generate_random_password();
                if let Err(e) = crate::helpers::reset_local_password(&username, &password) {
                    log::error!("Connect: failed to reset local password for '{username}': {e}");
                    return Err(E_FAIL.into());
                }
                log::info!("Connect: local password reset for '{username}'");

                *self.auth_data.lock().unwrap() = Some(AuthData { username, password });
                Ok(())
            }
            AuthOutcome::Cancelled => {
                log::info!("Connect: authentication cancelled");
                *self.auth_data.lock().unwrap() = None;
                Err(E_ABORT.into())
            }
        }
    }

    fn Disconnect(&self) -> Result<()> {
        Ok(())
    }
}
