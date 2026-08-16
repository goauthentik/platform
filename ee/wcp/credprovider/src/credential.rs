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
use crate::syscalls::{AuthPackageLookup, LocalAccountPassword, PasswordCheck, PasswordStore};
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

/// The seams a `Credential` reaches the outside world through, so tests can
/// drive the sign-in logic without touching LSA or the account database.
pub struct CredentialDeps {
    pub auth_flow: Box<dyn AuthFlow>,
    pub password: Box<dyn LocalAccountPassword>,
    pub auth_package: Box<dyn AuthPackageLookup>,
    pub store: Box<dyn PasswordStore>,
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
    deps: CredentialDeps,
    outcome: Mutex<Option<Outcome>>,
    /// The credential actually handed to LSA, kept past `GetSerialization` so
    /// `ReportResult` can rotate it once the logon has succeeded.
    serialized: Mutex<Option<(String, String)>>,
}

impl Credential {
    pub fn new(
        sid: String,
        qualified_username: String,
        is_local_user: bool,
        cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
        deps: CredentialDeps,
    ) -> Self {
        Self {
            sid,
            qualified_username,
            is_local_user,
            cpus,
            deps,
            outcome: Mutex::new(None),
            serialized: Mutex::new(None),
        }
    }
}

impl Credential_Impl {
    /// The password to hand LSA for this account.
    ///
    /// Domain accounts keep the throwaway password they have always had. For a
    /// local account the password is established once and then reused, because
    /// the only way to set a password we do not already know is an
    /// administrative reset, and that orphans the account's DPAPI master key —
    /// stored credentials, EFS files and personal certificates — every time it
    /// runs.
    fn account_password(&self, username: &str) -> Result<String> {
        if !self.is_local_user {
            return helpers::generate_random_password();
        }

        match self.stored_password(username) {
            // Correct, just no longer accepted for logon. Knowing it means a
            // change still works, so the master key survives; only a failed
            // change falls through to a reset.
            Some((stored, PasswordCheck::Expired)) => {
                log::info!("Connect: stored password has expired; changing it");
                if let Some(new) = self.replace_password(username, &stored) {
                    return Ok(new);
                }
            }
            Some((stored, _)) => return Ok(stored),
            None => {}
        }

        let password = helpers::generate_random_password()?;
        self.deps.password.reset(username, &password)?;
        // A vault we cannot write to costs another reset next time. Bad, but
        // not worth failing a sign-in over.
        if let Err(e) = self.deps.store.save(&self.sid, &password) {
            log::error!("Connect: could not store the account password: {e}");
        }
        Ok(password)
    }

    /// The stored password and what the account thinks of it, or `None` when
    /// there is nothing usable to reuse.
    fn stored_password(&self, username: &str) -> Option<(String, PasswordCheck)> {
        let stored = match self.deps.store.load(&self.sid) {
            Ok(Some(stored)) => stored,
            Ok(None) => return None,
            // A broken vault is a miss, not a failure: sign-in still works,
            // it just costs a reset.
            Err(e) => {
                log::error!("Connect: could not read the stored password: {e}");
                return None;
            }
        };

        match self.deps.password.validate(username, &stored) {
            Ok(PasswordCheck::Rejected) => {
                log::info!("Connect: stored password was rejected; resetting the account");
                None
            }
            Ok(check) => Some((stored, check)),
            // Inconclusive. Resetting on the strength of a guess would destroy
            // the master key we are here to protect; submitting the stored
            // password costs at worst one failed logon, after which `validate`
            // has a definite answer.
            Err(e) => {
                log::warn!("Connect: could not verify the stored password ({e}); using it anyway");
                Some((stored, PasswordCheck::Valid))
            }
        }
    }

    /// Swap `old` for a freshly generated password, returning the new one.
    ///
    /// `NetUserChangePassword` rather than a reset: supplying the old password
    /// lets LSA re-encrypt the DPAPI master key instead of orphaning it. It
    /// also works on an expired password, which is why that path lands here
    /// too. Best-effort — a minimum-password-age policy rejects this on every
    /// logon, and the stored password stays valid either way.
    fn replace_password(&self, username: &str, old: &str) -> Option<String> {
        let new = match helpers::generate_random_password() {
            Ok(new) => new,
            Err(e) => {
                log::error!("failed to generate a replacement password: {e}");
                return None;
            }
        };

        if let Err(e) = self.deps.password.change(username, old, &new) {
            log::warn!("password change failed ({e}); keeping the current one");
            return None;
        }

        // The account now has a password the vault does not know. `validate`
        // catches that on the next sign-in, at the cost of one reset.
        if let Err(e) = self.deps.store.save(&self.sid, &new) {
            log::error!("changed the account password but could not store it: {e}");
        }
        Some(new)
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
        *self.serialized.lock().unwrap_or_else(|e| e.into_inner()) = None;
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

        let auth_package = match self.deps.auth_package.negotiate_package() {
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
        // Only a credential that was actually submitted is eligible for
        // rotation, so this is recorded here rather than in `Connect`.
        *self.serialized.lock().unwrap_or_else(|e| e.into_inner()) = Some((username, password));
        Ok(())
    }

    fn ReportResult(
        &self,
        ntsstatus: NTSTATUS,
        _ntssubstatus: NTSTATUS,
        ppszoptionalstatustext: *mut PWSTR,
        pcpsioptionalstatusicon: *mut CREDENTIAL_PROVIDER_STATUS_ICON,
    ) -> Result<()> {
        let submitted = self
            .serialized
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();

        // STATUS_SUCCESS; the logon session exists, so the password we
        // submitted is now cached in it and can be changed rather than reset.
        if ntsstatus.0 == 0
            && self.is_local_user
            && let Some((username, old)) = submitted
        {
            let _ = self.replace_password(&username, &old);
        }

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

        let result = self.deps.auth_flow.run(&mut should_continue);

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

                let password = match self.account_password(&username) {
                    Ok(password) => password,
                    Err(e) => {
                        log::error!("Connect: could not establish a password: {e}");
                        return Err(E_FAIL.into());
                    }
                };

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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::syscalls::PasswordCheck;
    use std::collections::HashMap;
    use std::sync::Arc;
    use windows::Win32::Security::Authentication::Identity::KERB_INTERACTIVE_UNLOCK_LOGON;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{
        CPGSR_RETURN_CREDENTIAL_FINISHED, CPUS_LOGON, CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION,
        CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE, CREDENTIAL_PROVIDER_STATUS_ICON,
    };
    use windows::core::Interface;

    const SID: &str = "S-1-5-21-1-2-3-1001";
    const AUTH_PACKAGE: u32 = 7;

    struct FakeAuthFlow(AuthResult);

    impl AuthFlow for FakeAuthFlow {
        fn run(&self, _should_continue: &mut dyn FnMut() -> bool) -> AuthResult {
            match &self.0 {
                AuthResult::Completed { username } => AuthResult::Completed {
                    username: username.clone(),
                },
                AuthResult::Cancelled => AuthResult::Cancelled,
                AuthResult::Failed { reason } => AuthResult::Failed {
                    reason: reason.clone(),
                },
            }
        }
    }

    struct FakeAuthPackage;

    impl AuthPackageLookup for FakeAuthPackage {
        fn negotiate_package(&self) -> Result<u32> {
            Ok(AUTH_PACKAGE)
        }
    }

    /// Tracks the account's password and how each call was made, so tests can
    /// assert that a rotation went through `change` rather than `reset`.
    #[derive(Default)]
    struct AccountState {
        password: Option<String>,
        check: Option<PasswordCheck>,
        change_fails: bool,
        resets: Vec<String>,
        changes: Vec<(String, String)>,
    }

    #[derive(Clone, Default)]
    struct FakePassword(Arc<Mutex<AccountState>>);

    impl FakePassword {
        fn state(&self) -> std::sync::MutexGuard<'_, AccountState> {
            self.0.lock().unwrap_or_else(|e| e.into_inner())
        }
    }

    impl LocalAccountPassword for FakePassword {
        fn reset(&self, _username: &str, password: &str) -> Result<()> {
            let mut state = self.state();
            state.resets.push(password.to_string());
            state.password = Some(password.to_string());
            Ok(())
        }

        fn change(&self, _username: &str, old: &str, new: &str) -> Result<()> {
            let mut state = self.state();
            if state.change_fails {
                return Err(E_FAIL.into());
            }
            state.changes.push((old.to_string(), new.to_string()));
            state.password = Some(new.to_string());
            Ok(())
        }

        fn validate(&self, _username: &str, password: &str) -> Result<PasswordCheck> {
            let state = self.state();
            match state.check {
                // Inconclusive, the case that must not trigger a reset.
                None => Err(E_FAIL.into()),
                Some(PasswordCheck::Valid) if state.password.as_deref() == Some(password) => {
                    Ok(PasswordCheck::Valid)
                }
                Some(PasswordCheck::Valid) => Ok(PasswordCheck::Rejected),
                Some(PasswordCheck::Expired) => Ok(PasswordCheck::Expired),
                Some(PasswordCheck::Rejected) => Ok(PasswordCheck::Rejected),
            }
        }
    }

    #[derive(Clone, Default)]
    struct FakeStore {
        entries: Arc<Mutex<HashMap<String, String>>>,
        load_fails: bool,
        save_fails: bool,
    }

    impl FakeStore {
        fn get(&self, sid: &str) -> Option<String> {
            self.entries
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(sid)
                .cloned()
        }
    }

    impl PasswordStore for FakeStore {
        fn load(&self, sid: &str) -> eyre::Result<Option<String>> {
            if self.load_fails {
                return Err(eyre::eyre!("vault unreachable"));
            }
            Ok(self.get(sid))
        }

        fn save(&self, sid: &str, password: &str) -> eyre::Result<()> {
            if self.save_fails {
                return Err(eyre::eyre!("vault read-only"));
            }
            self.entries
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(sid.to_string(), password.to_string());
            Ok(())
        }
    }

    fn credential(
        is_local_user: bool,
        password: &FakePassword,
        store: &FakeStore,
    ) -> ICredentialProviderCredential {
        let qualified = if is_local_user {
            r"COMPUTER\alice".to_string()
        } else {
            "alice".to_string()
        };
        Credential::new(
            SID.to_string(),
            qualified,
            is_local_user,
            CPUS_LOGON,
            CredentialDeps {
                auth_flow: Box::new(FakeAuthFlow(AuthResult::Completed {
                    username: "alice".to_string(),
                })),
                password: Box::new(password.clone()),
                auth_package: Box::new(FakeAuthPackage),
                store: Box::new(store.clone()),
            },
        )
        .into()
    }

    /// Drives `Connect` then `GetSerialization`. The returned buffer is the
    /// caller's to free.
    fn submit(
        credential: &ICredentialProviderCredential,
    ) -> Option<CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION> {
        let connectable: IConnectableCredentialProviderCredential = credential.cast().unwrap();
        unsafe { connectable.Connect(None) }.unwrap();

        let mut response = CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE::default();
        let mut serialization = CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION::default();
        let mut status_text = PWSTR::null();
        let mut status_icon = CREDENTIAL_PROVIDER_STATUS_ICON::default();
        unsafe {
            credential
                .GetSerialization(
                    &mut response,
                    &mut serialization,
                    &mut status_text,
                    &mut status_icon,
                )
                .unwrap();
        }
        (response == CPGSR_RETURN_CREDENTIAL_FINISHED).then_some(serialization)
    }

    /// The password packed for LSA. Local accounts only — domain accounts use
    /// `CredPackAuthenticationBufferW`, which is not this layout. Reads the
    /// `LSA_UNICODE_STRING` offsets back out of the flat buffer the same way
    /// `helpers`' packing tests do.
    fn sign_in(credential: &ICredentialProviderCredential) -> Option<String> {
        let buf = submit(credential)?.rgbSerialization;
        let password = unsafe {
            let logon = &(*(buf as *const KERB_INTERACTIVE_UNLOCK_LOGON)).Logon;
            let ptr = buf.add(logon.Password.Buffer.0 as usize) as *const u16;
            let chars = std::slice::from_raw_parts(ptr, logon.Password.Length as usize / 2);
            String::from_utf16_lossy(chars)
        };
        unsafe { CoTaskMemFree(Some(buf as *const _)) };
        Some(password)
    }

    fn report_result(credential: &ICredentialProviderCredential, status: NTSTATUS) {
        let mut status_text = PWSTR::null();
        let mut status_icon = CREDENTIAL_PROVIDER_STATUS_ICON::default();
        unsafe {
            credential
                .ReportResult(status, NTSTATUS(0), &mut status_text, &mut status_icon)
                .unwrap();
        }
    }

    #[test]
    fn first_sign_in_resets_the_account_and_stores_the_password() {
        let password = FakePassword::default();
        let store = FakeStore::default();
        let cred = credential(true, &password, &store);

        let serialized = sign_in(&cred).unwrap();

        assert_eq!(password.state().resets, vec![serialized.clone()]);
        assert_eq!(store.get(SID).as_deref(), Some(serialized.as_str()));
    }

    #[test]
    fn a_valid_stored_password_is_reused_without_a_reset() {
        let password = FakePassword::default();
        password.state().password = Some("stored".to_string());
        password.state().check = Some(PasswordCheck::Valid);
        let store = FakeStore::default();
        store.save(SID, "stored").unwrap();

        let serialized = sign_in(&credential(true, &password, &store)).unwrap();

        assert_eq!(serialized, "stored");
        assert!(password.state().resets.is_empty());
    }

    #[test]
    fn a_rejected_stored_password_triggers_a_reset() {
        let password = FakePassword::default();
        password.state().password = Some("current".to_string());
        password.state().check = Some(PasswordCheck::Valid);
        let store = FakeStore::default();
        store.save(SID, "stale").unwrap();

        let serialized = sign_in(&credential(true, &password, &store)).unwrap();

        assert_ne!(serialized, "stale");
        assert_eq!(password.state().resets, vec![serialized.clone()]);
        assert_eq!(store.get(SID).as_deref(), Some(serialized.as_str()));
    }

    #[test]
    fn an_expired_stored_password_is_changed_rather_than_reset() {
        let password = FakePassword::default();
        password.state().password = Some("stored".to_string());
        password.state().check = Some(PasswordCheck::Expired);
        let store = FakeStore::default();
        store.save(SID, "stored").unwrap();

        let serialized = sign_in(&credential(true, &password, &store)).unwrap();

        assert!(password.state().resets.is_empty());
        assert_eq!(
            password.state().changes,
            vec![("stored".to_string(), serialized.clone())]
        );
        assert_eq!(store.get(SID).as_deref(), Some(serialized.as_str()));
    }

    #[test]
    fn an_unverifiable_stored_password_is_used_without_a_reset() {
        let password = FakePassword::default();
        let store = FakeStore::default();
        store.save(SID, "stored").unwrap();

        // `check` is None, so `validate` errors: we cannot tell, and resetting
        // would orphan the master key on a guess.
        let serialized = sign_in(&credential(true, &password, &store)).unwrap();

        assert_eq!(serialized, "stored");
        assert!(password.state().resets.is_empty());
    }

    #[test]
    fn a_broken_vault_still_signs_in() {
        let password = FakePassword::default();
        let store = FakeStore {
            load_fails: true,
            save_fails: true,
            ..FakeStore::default()
        };

        let serialized = sign_in(&credential(true, &password, &store)).unwrap();

        assert_eq!(password.state().resets, vec![serialized]);
    }

    #[test]
    fn a_domain_user_touches_neither_the_store_nor_the_account() {
        let password = FakePassword::default();
        let store = FakeStore::default();

        let serialization = submit(&credential(false, &password, &store)).unwrap();
        unsafe { CoTaskMemFree(Some(serialization.rgbSerialization as *const _)) };

        assert_eq!(serialization.ulAuthenticationPackage, AUTH_PACKAGE);
        assert!(password.state().resets.is_empty());
        assert!(password.state().changes.is_empty());
        assert_eq!(store.get(SID), None);
    }

    #[test]
    fn a_successful_logon_rotates_the_password_with_a_change() {
        let password = FakePassword::default();
        let store = FakeStore::default();
        let cred = credential(true, &password, &store);

        let serialized = sign_in(&cred).unwrap();
        report_result(&cred, NTSTATUS(0));

        let state = password.state();
        assert_eq!(state.changes.len(), 1);
        assert_eq!(state.changes[0].0, serialized);
        let rotated = state.changes[0].1.clone();
        assert_ne!(rotated, serialized);
        drop(state);
        assert_eq!(store.get(SID).as_deref(), Some(rotated.as_str()));
        assert_eq!(password.state().resets.len(), 1);
    }

    #[test]
    fn a_failed_logon_rotates_nothing() {
        let password = FakePassword::default();
        let store = FakeStore::default();
        let cred = credential(true, &password, &store);

        let serialized = sign_in(&cred).unwrap();
        report_result(&cred, STATUS_LOGON_FAILURE);

        assert!(password.state().changes.is_empty());
        assert_eq!(store.get(SID).as_deref(), Some(serialized.as_str()));
    }

    #[test]
    fn a_failed_rotation_leaves_the_stored_password_intact() {
        let password = FakePassword::default();
        password.state().change_fails = true;
        let store = FakeStore::default();
        let cred = credential(true, &password, &store);

        let serialized = sign_in(&cred).unwrap();
        report_result(&cred, NTSTATUS(0));

        assert!(password.state().changes.is_empty());
        assert_eq!(store.get(SID).as_deref(), Some(serialized.as_str()));
    }

    #[test]
    fn deselecting_the_tile_cancels_a_pending_rotation() {
        let password = FakePassword::default();
        let store = FakeStore::default();
        let cred = credential(true, &password, &store);

        sign_in(&cred).unwrap();
        unsafe { cred.SetDeselected() }.unwrap();
        report_result(&cred, NTSTATUS(0));

        assert!(password.state().changes.is_empty());
    }

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
