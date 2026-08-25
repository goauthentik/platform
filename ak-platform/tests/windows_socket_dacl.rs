//! Integration coverage for the DACLs `net::server::listen` attaches to Windows
//! named pipes.
//!
//! These were dropped in the Go -> Rust port, so the failure mode worth guarding
//! against is "no descriptor applied at all" — a pipe created with the default
//! descriptor still works locally, it just silently grants the wrong set of
//! callers. Each test therefore reads the DACL back off the live pipe and
//! compares it to the one its `SocketPermMode` is supposed to produce.

#![cfg(windows)]

use std::ffi::c_void;

use ak_platform::net::server::SocketPermMode;
use ak_platform::net::{client, server};
use ak_platform::string::PlatformString;
use windows::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1,
    SE_FILE_OBJECT,
};
use windows::Win32::Security::{
    CheckTokenMembership, CreateWellKnownSid, DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    PSID, SECURITY_MAX_SID_SIZE, WinBuiltinAdministratorsSid,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    READ_CONTROL,
};
use windows::core::{BOOL, HSTRING, PWSTR};

/// A DACL, always held in the spelling Windows itself produces.
///
/// The same access mask has several valid SDDL spellings (`FA` vs `0x1f01ff`,
/// differing ACE order), so both the expected and the observed descriptor are
/// run through `ConvertSecurityDescriptorToStringSecurityDescriptorW` before
/// being compared — otherwise a passing test could fail on formatting alone.
#[derive(Debug, PartialEq, Eq)]
struct Dacl(String);

impl Dacl {
    /// Reads back what Windows actually stored on the pipe at `path`.
    ///
    /// `GetNamedSecurityInfoW` only documents file-system objects (files,
    /// directories, shares) for `SE_FILE_OBJECT` — named pipes aren't in that
    /// list and querying one by path fails with `ERROR_INVALID_PARAMETER`. The
    /// documented way to read a pipe's DACL is `GetSecurityInfo` against a
    /// handle opened for `READ_CONTROL`, same as a client would open the pipe.
    fn of_pipe(path: &str) -> Self {
        let handle = unsafe {
            CreateFileW(
                &HSTRING::from(path),
                READ_CONTROL.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )
        }
        .unwrap_or_else(|e| panic!("could not open {path}: {e}"));

        let mut psd = PSECURITY_DESCRIPTOR::default();
        let status = unsafe {
            GetSecurityInfo(
                handle,
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                None,
                None,
                None,
                None,
                Some(&mut psd),
            )
        };
        unsafe { CloseHandle(handle) }.unwrap();
        assert_eq!(status, ERROR_SUCCESS, "could not read the DACL of {path}");

        let dacl = Self::serialize(psd);
        unsafe { LocalFree(Some(HLOCAL(psd.0))) };
        dacl
    }

    /// The DACL described by an SDDL literal.
    fn of_sddl(sddl: &str) -> Self {
        let mut psd = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                &HSTRING::from(sddl),
                SDDL_REVISION_1,
                &mut psd,
                None,
            )
        }
        .unwrap_or_else(|e| panic!("{sddl} is not valid SDDL: {e}"));

        let dacl = Self::serialize(psd);
        unsafe { LocalFree(Some(HLOCAL(psd.0))) };
        dacl
    }

    fn serialize(psd: PSECURITY_DESCRIPTOR) -> Self {
        let mut sddl = PWSTR::null();
        unsafe {
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                psd,
                SDDL_REVISION_1,
                DACL_SECURITY_INFORMATION,
                &mut sddl,
                None,
            )
        }
        .unwrap();

        let owned = unsafe { sddl.to_string() }.unwrap();
        unsafe { LocalFree(Some(HLOCAL(sddl.0 as *mut c_void))) };
        Self(owned)
    }
}

/// Whether this process's token would pass an access check against the built-in
/// Administrators SID. On a UAC-split token this stays false until the process
/// is actually elevated, which is exactly the line `SocketPermMode::Admin` draws.
fn is_administrator() -> bool {
    let mut sid = [0u8; SECURITY_MAX_SID_SIZE as usize];
    let mut len = sid.len() as u32;
    let psid = PSID(sid.as_mut_ptr() as *mut c_void);
    unsafe { CreateWellKnownSid(WinBuiltinAdministratorsSid, None, Some(psid), &mut len) }.unwrap();

    let mut is_member = BOOL::default();
    unsafe { CheckTokenMembership(None, psid, &mut is_member) }.unwrap();
    is_member.as_bool()
}

#[tokio::test]
async fn owner_mode_grants_the_owner_only() {
    let path = r"\\.\pipe\ak-test-dacl-owner";
    let _listener = server::listen(
        PlatformString::new_with_default(path),
        SocketPermMode::Owner,
    )
    .await
    .unwrap();

    assert_eq!(Dacl::of_pipe(path), Dacl::of_sddl("D:(A;;FA;;;OW)"));
}

#[tokio::test]
async fn everyone_mode_grants_world() {
    let path = r"\\.\pipe\ak-test-dacl-everyone";
    let _listener = server::listen(
        PlatformString::new_with_default(path),
        SocketPermMode::Everyone,
    )
    .await
    .unwrap();

    assert_eq!(Dacl::of_pipe(path), Dacl::of_sddl("D:(A;;FA;;;WD)"));
}

#[tokio::test]
async fn admin_mode_grants_administrators_and_system_only() {
    let path = r"\\.\pipe\ak-test-dacl-admin";
    let _listener = server::listen(
        PlatformString::new_with_default(path),
        SocketPermMode::Admin,
    )
    .await
    .unwrap();

    assert_eq!(
        Dacl::of_pipe(path),
        Dacl::of_sddl("D:(A;;FA;;;BA)(A;;FA;;;SY)")
    );
}

#[tokio::test]
async fn everyone_mode_accepts_a_client() {
    let path = PlatformString::new_with_default(r"\\.\pipe\ak-test-dacl-everyone-connect");
    let _listener = server::listen(path.clone(), SocketPermMode::Everyone)
        .await
        .unwrap();

    client::connect(path).await.unwrap();
}

/// The descriptor is only meaningful if it actually turns callers away, so check
/// the access check too — but only when the test process is outside the
/// Administrators group, since an elevated runner is legitimately allowed in.
#[tokio::test]
async fn admin_mode_rejects_an_unelevated_client() {
    if is_administrator() {
        return;
    }

    let path = PlatformString::new_with_default(r"\\.\pipe\ak-test-dacl-admin-connect");
    let _listener = server::listen(path.clone(), SocketPermMode::Admin)
        .await
        .unwrap();

    let Err(err) = client::connect(path).await else {
        panic!("a non-administrator connected to an admin-only pipe");
    };
    assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
}
