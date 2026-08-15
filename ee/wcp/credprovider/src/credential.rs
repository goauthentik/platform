use std::sync::Mutex;

use windows::{
    Win32::{
        Foundation::{E_FAIL, E_INVALIDARG, E_NOTIMPL, FALSE, NTSTATUS},
        Graphics::Gdi::HBITMAP,
        Security::Credentials::{
            STATUS_ACCOUNT_DISABLED, STATUS_ACCOUNT_RESTRICTION, STATUS_LOGON_FAILURE,
        },
        UI::Shell::{
            CPCFO_ENABLE_TOUCH_KEYBOARD_AUTO_INVOKE, CPCFO_NONE, CPGSR_NO_CREDENTIAL_FINISHED,
            CPGSR_NO_CREDENTIAL_NOT_FINISHED, CPGSR_RETURN_CREDENTIAL_FINISHED, CPSI_ERROR,
            CPSI_NONE, CPSI_SUCCESS, CPSI_WARNING, CREDENTIAL_PROVIDER_CREDENTIAL_FIELD_OPTIONS,
            CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION,
            CREDENTIAL_PROVIDER_FIELD_INTERACTIVE_STATE, CREDENTIAL_PROVIDER_FIELD_STATE,
            CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE, CREDENTIAL_PROVIDER_STATUS_ICON,
            CREDENTIAL_PROVIDER_USAGE_SCENARIO, IConnectableCredentialProviderCredential,
            IConnectableCredentialProviderCredential_Impl, ICredentialProviderCredential,
            ICredentialProviderCredential_Impl, ICredentialProviderCredential2,
            ICredentialProviderCredential2_Impl, ICredentialProviderCredentialEvents,
            ICredentialProviderCredentialWithFieldOptions,
            ICredentialProviderCredentialWithFieldOptions_Impl, IQueryContinueWithStatus,
        },
    },
    core::{BOOL, PCWSTR, PWSTR, Ref, Result, implement, w},
};

use crate::helpers;
use crate::ipc::AuthFlow;
use crate::strings::cotask_pwstr;
use crate::syscalls::{AuthPackageLookup, LocalAccountPasswordReset};
use crate::tile;
use ak_ee_wcp_wire::{AuthResult, FieldKind, TILE_FIELDS};

/// Outcome of `Connect`'s browser flow, consumed by `GetSerialization` —
/// mirrors the original design where `Connect` always succeeds and defers
/// the cancelled/failed-vs-completed decision to `GetSerialization`.
enum Outcome {
    Completed { username: String, password: String },
    Cancelled,
    Failed { reason: String },
}

#[implement(
    ICredentialProviderCredential,
    ICredentialProviderCredential2,
    IConnectableCredentialProviderCredential,
    ICredentialProviderCredentialWithFieldOptions
)]
pub struct Credential {
    sid: String,
    qualified_username: String,
    is_local_user: bool,
    cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
    auth_flow: Box<dyn AuthFlow>,
    password_reset: Box<dyn LocalAccountPasswordReset>,
    auth_package: Box<dyn AuthPackageLookup>,
    outcome: Mutex<Option<Outcome>>,
}

impl Credential {
    pub fn new(
        sid: String,
        qualified_username: String,
        is_local_user: bool,
        cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
        auth_flow: Box<dyn AuthFlow>,
        password_reset: Box<dyn LocalAccountPasswordReset>,
        auth_package: Box<dyn AuthPackageLookup>,
    ) -> Self {
        Self {
            sid,
            qualified_username,
            is_local_user,
            cpus,
            auth_flow,
            password_reset,
            auth_package,
            outcome: Mutex::new(None),
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
        *self.outcome.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(())
    }

    fn GetFieldState(
        &self,
        dwfieldid: u32,
        pcpfs: *mut CREDENTIAL_PROVIDER_FIELD_STATE,
        pcpfis: *mut CREDENTIAL_PROVIDER_FIELD_INTERACTIVE_STATE,
    ) -> Result<()> {
        let (state, interactive) = tile::field_state_at(dwfieldid)?;
        unsafe {
            *pcpfs = state;
            *pcpfis = interactive;
        }
        Ok(())
    }

    fn GetStringValue(&self, dwfieldid: u32) -> Result<PWSTR> {
        let field = TILE_FIELDS
            .get(dwfieldid as usize)
            .ok_or(windows::core::Error::from(E_INVALIDARG))?;
        Ok(match field.kind {
            FieldKind::TileImage => PWSTR::null(),
            _ => cotask_pwstr(field.text),
        })
    }

    fn GetBitmapValue(&self, dwfieldid: u32) -> Result<HBITMAP> {
        let field = TILE_FIELDS
            .get(dwfieldid as usize)
            .ok_or(windows::core::Error::from(E_INVALIDARG))?;
        if field.kind != FieldKind::TileImage {
            return Err(E_NOTIMPL.into());
        }
        tile::load_tile_bitmap()
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
        let field = TILE_FIELDS
            .get(dwfieldid as usize)
            .ok_or(windows::core::Error::from(E_INVALIDARG))?;
        if field.kind == FieldKind::SubmitButton {
            Ok(dwfieldid)
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
        Err(E_NOTIMPL.into())
    }

    fn GetSerialization(
        &self,
        pcpgsr: *mut CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE,
        pcpcs: *mut CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION,
        ppszoptionalstatustext: *mut PWSTR,
        pcpsioptionalstatusicon: *mut CREDENTIAL_PROVIDER_STATUS_ICON,
    ) -> Result<()> {
        let outcome = self
            .outcome
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();

        unsafe {
            *ppszoptionalstatustext = PWSTR::null();
            *pcpsioptionalstatusicon = CPSI_NONE;
        }

        let (username, password) = match outcome {
            Some(Outcome::Completed { username, password }) => (username, password),
            Some(Outcome::Cancelled) => {
                log::info!("GetSerialization: sign-in was cancelled");
                unsafe {
                    *pcpgsr = CPGSR_NO_CREDENTIAL_FINISHED;
                    *ppszoptionalstatustext = cotask_pwstr("Login attempt cancelled");
                    *pcpsioptionalstatusicon = CPSI_WARNING;
                }
                return Ok(());
            }
            // No outcome at all means `Connect` never ran (or `SetDeselected`
            // cleared it first), which reaches the user as the same "cancelled"
            // string but is a different bug entirely.
            None => {
                log::warn!("GetSerialization: no outcome recorded; Connect did not run");
                unsafe {
                    *pcpgsr = CPGSR_NO_CREDENTIAL_FINISHED;
                    *ppszoptionalstatustext = cotask_pwstr("Login attempt cancelled");
                    *pcpsioptionalstatusicon = CPSI_WARNING;
                }
                return Ok(());
            }
            Some(Outcome::Failed { reason }) => {
                log::error!("GetSerialization: sign-in failed: {reason}");
                unsafe {
                    *pcpgsr = CPGSR_NO_CREDENTIAL_FINISHED;
                    *ppszoptionalstatustext = cotask_pwstr("Sign-in failed. Please try again.");
                    *pcpsioptionalstatusicon = CPSI_ERROR;
                }
                return Ok(());
            }
        };

        let packed = if self.is_local_user {
            let domain = helpers::computer_name();
            helpers::pack_kerb_interactive_unlock_logon(&domain, &username, &password, self.cpus)
        } else {
            helpers::pack_authentication_buffer(&username, &password)
        };

        let (buf, len) = match packed {
            Ok(v) => v,
            Err(e) => {
                log::error!("failed to pack credential serialization: {e}");
                unsafe { *pcpgsr = CPGSR_NO_CREDENTIAL_NOT_FINISHED };
                return Ok(());
            }
        };

        let auth_package = match self.auth_package.negotiate_package() {
            Ok(pkg) => pkg,
            Err(e) => {
                log::error!("failed to resolve Negotiate auth package: {e}");
                unsafe {
                    windows::Win32::System::Com::CoTaskMemFree(Some(buf as *const _));
                    *pcpgsr = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
                }
                return Ok(());
            }
        };

        unsafe {
            (*pcpcs).rgbSerialization = buf;
            (*pcpcs).cbSerialization = len;
            (*pcpcs).ulAuthenticationPackage = auth_package;
            (*pcpcs).clsidCredentialProvider = crate::CLSID_CREDENTIAL_PROVIDER;
            *pcpsioptionalstatusicon = CPSI_SUCCESS;
            *pcpgsr = CPGSR_RETURN_CREDENTIAL_FINISHED;
        }
        log::info!("GetSerialization: packed credential for '{username}'");
        Ok(())
    }

    fn ReportResult(
        &self,
        ntsstatus: NTSTATUS,
        _ntssubstatus: NTSTATUS,
        ppszoptionalstatustext: *mut PWSTR,
        pcpsioptionalstatusicon: *mut CREDENTIAL_PROVIDER_STATUS_ICON,
    ) -> Result<()> {
        let (text, icon) = match ntsstatus {
            STATUS_LOGON_FAILURE => (Some("Incorrect password or username."), CPSI_ERROR),
            STATUS_ACCOUNT_RESTRICTION | STATUS_ACCOUNT_DISABLED => {
                (Some("The account is disabled."), CPSI_WARNING)
            }
            _ => (None, CPSI_NONE),
        };

        unsafe {
            *ppszoptionalstatustext = text.map(cotask_pwstr).unwrap_or(PWSTR::null());
            *pcpsioptionalstatusicon = icon;
        }
        Ok(())
    }
}

impl ICredentialProviderCredential2_Impl for Credential_Impl {
    fn GetUserSid(&self) -> Result<PWSTR> {
        if self.sid.is_empty() {
            Ok(PWSTR::null())
        } else {
            Ok(cotask_pwstr(&self.sid))
        }
    }
}

impl ICredentialProviderCredentialWithFieldOptions_Impl for Credential_Impl {
    fn GetFieldOptions(
        &self,
        fieldid: u32,
    ) -> Result<CREDENTIAL_PROVIDER_CREDENTIAL_FIELD_OPTIONS> {
        let field = TILE_FIELDS
            .get(fieldid as usize)
            .ok_or(windows::core::Error::from(E_INVALIDARG))?;
        Ok(if field.kind == FieldKind::TileImage {
            CPCFO_ENABLE_TOUCH_KEYBOARD_AUTO_INVOKE
        } else {
            CPCFO_NONE
        })
    }
}

impl IConnectableCredentialProviderCredential_Impl for Credential_Impl {
    fn Connect(&self, pqcws: Ref<'_, IQueryContinueWithStatus>) -> Result<()> {
        log::info!(
            "Connect: starting sign-in for '{}' (local: {})",
            self.qualified_username,
            self.is_local_user
        );
        if let Some(q) = pqcws.as_ref() {
            unsafe {
                let _ = q.SetStatusMessage(w!("Please sign in to your authentik account..."));
            }
        }

        let mut should_continue = || -> bool {
            match pqcws.as_ref() {
                Some(q) => unsafe { q.QueryContinue() }.is_ok(),
                None => true,
            }
        };

        let result = self.auth_flow.run(&mut should_continue);

        let outcome = match result {
            AuthResult::Completed { username } => {
                if !usernames_match(&username, &self.qualified_username, self.is_local_user) {
                    log::warn!(
                        "Connect: authenticated username '{username}' does not match tile user '{}'",
                        self.qualified_username
                    );
                    // Matches the original behavior of failing Connect directly
                    // (rather than deferring to GetSerialization) on a mismatch,
                    // which also suppresses the tile's Disconnect button.
                    return Err(E_FAIL.into());
                }

                let password = match helpers::generate_random_password() {
                    Ok(password) => password,
                    Err(e) => {
                        log::error!("Connect: failed to generate a password: {e}");
                        return Err(E_FAIL.into());
                    }
                };
                if self.is_local_user
                    && let Err(e) = self.password_reset.reset(&username, &password)
                {
                    log::error!("Connect: failed to reset local password: {e}");
                    return Err(E_FAIL.into());
                }

                Outcome::Completed { username, password }
            }
            AuthResult::Cancelled => Outcome::Cancelled,
            AuthResult::Failed { reason } => Outcome::Failed { reason },
        };

        *self.outcome.lock().unwrap_or_else(|e| e.into_inner()) = Some(outcome);
        Ok(())
    }

    fn Disconnect(&self) -> Result<()> {
        Ok(())
    }
}

/// The browser flow authenticates against the qualified username shown on
/// the tile; for local accounts that's `domain\username`, so compare only
/// the username portion.
fn usernames_match(authenticated: &str, qualified: &str, is_local_user: bool) -> bool {
    let expected = if is_local_user {
        qualified.rsplit('\\').next().unwrap_or(qualified)
    } else {
        qualified
    };
    expected.eq_ignore_ascii_case(authenticated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_user_matches_on_username_portion_only() {
        assert!(usernames_match("alice", r"COMPUTER\alice", true));
        assert!(usernames_match("Alice", r"COMPUTER\alice", true));
        assert!(!usernames_match("bob", r"COMPUTER\alice", true));
    }

    #[test]
    fn domain_user_matches_full_qualified_name() {
        assert!(usernames_match(
            "alice@example.com",
            "alice@example.com",
            false
        ));
        assert!(!usernames_match("alice", "alice@example.com", false));
    }
}
