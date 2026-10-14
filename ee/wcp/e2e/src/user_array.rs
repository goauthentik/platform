//! Stand-in for the `ICredentialProviderUserArray` LogonUI passes to
//! `SetUserArray`, so a test pins the SID, qualified username and provider ID a
//! `Credential` is built from rather than depending on the machine's accounts.

use std::cell::RefCell;

use windows::{
    Win32::{
        Foundation::{E_INVALIDARG, E_NOTIMPL, PROPERTYKEY},
        Storage::EnhancedStorage::PKEY_Identity_QualifiedUserName,
        System::Com::{CoTaskMemAlloc, StructuredStorage::PROPVARIANT},
        UI::Shell::{
            CPAO_NONE, CREDENTIAL_PROVIDER_ACCOUNT_OPTIONS, ICredentialProviderUser,
            ICredentialProviderUser_Impl, ICredentialProviderUserArray,
            ICredentialProviderUserArray_Impl, Identity_LocalUserProvider,
        },
    },
    core::{GUID, PWSTR, Result, implement},
};

/// Anything but `Identity_LocalUserProvider` reads as non-local, the only
/// distinction the provider draws.
pub const NON_LOCAL_USER_PROVIDER: GUID = GUID::from_u128(0x2a1b3c4d_5e6f_4a8b_9c0d_1e2f3a4b5c6d);

/// `CoTaskMemAlloc` is the allocator every `PWSTR`-returning provider method
/// must use; the caller — here the real DLL — frees it.
fn cotask_pwstr(value: &str) -> PWSTR {
    let wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = wide.len() * 2;
    let ptr = unsafe { CoTaskMemAlloc(bytes) } as *mut u16;
    if ptr.is_null() {
        return PWSTR::null();
    }
    unsafe { std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len()) };
    PWSTR(ptr)
}

#[derive(Clone)]
pub struct TestUser {
    pub sid: String,
    pub qualified_username: String,
    pub is_local: bool,
}

impl TestUser {
    /// A local account, qualified `COMPUTER\name`.
    pub fn local(computer: &str, name: &str) -> Self {
        Self {
            sid: "S-1-5-21-0-0-0-1001".to_string(),
            qualified_username: format!("{computer}\\{name}"),
            is_local: true,
        }
    }

    /// Domain/UPN account: skips the local password reset and serializes
    /// through `CredPackAuthenticationBufferW`.
    pub fn non_local(upn: &str) -> Self {
        Self {
            sid: "S-1-5-21-0-0-0-1002".to_string(),
            qualified_username: upn.to_string(),
            is_local: false,
        }
    }
}

#[implement(ICredentialProviderUser)]
struct FakeUser {
    user: TestUser,
}

impl ICredentialProviderUser_Impl for FakeUser_Impl {
    fn GetSid(&self) -> Result<PWSTR> {
        Ok(cotask_pwstr(&self.user.sid))
    }

    fn GetProviderID(&self) -> Result<GUID> {
        Ok(if self.user.is_local {
            Identity_LocalUserProvider
        } else {
            NON_LOCAL_USER_PROVIDER
        })
    }

    fn GetStringValue(&self, key: *const PROPERTYKEY) -> Result<PWSTR> {
        let key = unsafe { key.as_ref() }.ok_or(windows::core::Error::from(E_INVALIDARG))?;
        if *key == PKEY_Identity_QualifiedUserName {
            Ok(cotask_pwstr(&self.user.qualified_username))
        } else {
            Err(E_INVALIDARG.into())
        }
    }

    fn GetValue(&self, _key: *const PROPERTYKEY) -> Result<PROPVARIANT> {
        Err(E_NOTIMPL.into())
    }
}

#[implement(ICredentialProviderUserArray)]
struct FakeUserArray {
    users: RefCell<Vec<TestUser>>,
}

/// Builds the `ICredentialProviderUserArray` to hand to `SetUserArray`.
pub fn user_array(users: Vec<TestUser>) -> ICredentialProviderUserArray {
    FakeUserArray {
        users: RefCell::new(users),
    }
    .into()
}

impl ICredentialProviderUserArray_Impl for FakeUserArray_Impl {
    fn SetProviderFilter(&self, _guidprovidertofilterto: *const GUID) -> Result<()> {
        Ok(())
    }

    fn GetAccountOptions(&self) -> Result<CREDENTIAL_PROVIDER_ACCOUNT_OPTIONS> {
        Ok(CPAO_NONE)
    }

    fn GetCount(&self) -> Result<u32> {
        Ok(self.users.borrow().len() as u32)
    }

    fn GetAt(&self, userindex: u32) -> Result<ICredentialProviderUser> {
        self.users
            .borrow()
            .get(userindex as usize)
            .cloned()
            .map(|user| FakeUser { user }.into())
            .ok_or(E_INVALIDARG.into())
    }
}
