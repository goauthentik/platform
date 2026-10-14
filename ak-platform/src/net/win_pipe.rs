//! Named pipe helpers for telling who is on the other end of a pipe, without letting
//! the other end act as us.

use std::io;
use std::os::windows::io::{AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle};
use std::time::Duration;

use interprocess::local_socket::tokio::Stream;
use interprocess::os::windows::named_pipe::local_socket::tokio::Stream as PipeStream;
use windows::Win32::Foundation::{
    CloseHandle, ERROR_PIPE_BUSY, GENERIC_READ, GENERIC_WRITE, HANDLE,
};
use windows::Win32::Security::{
    GetTokenInformation, IsWellKnownSid, RevertToSelf, TOKEN_QUERY, TOKEN_USER, TokenUser,
    WinLocalSystemSid,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
};
use windows::Win32::System::Pipes::ImpersonateNamedPipeClient;
use windows::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};
use windows::core::HSTRING;

/// Connects at identification level, so the server can tell who we are but can't act
/// as us. `interprocess` leaves the default impersonation level.
pub async fn connect(path: &str) -> io::Result<Stream> {
    let path = HSTRING::from(path);
    loop {
        if let Some(h) = open(&path)? {
            return PipeStream::try_from(h)
                .map(Stream::from)
                .map_err(|e| io::Error::other(e.to_string()));
        }
        // All server instances are taken; one frees up once a connection is accepted.
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// `None` while the pipe is busy. Kept out of [`connect`] so the raw, non-`Send`
/// `HANDLE` never lives across an await.
fn open(path: &HSTRING) -> io::Result<Option<OwnedHandle>> {
    let handle = unsafe {
        CreateFileW(
            path,
            (GENERIC_READ | GENERIC_WRITE).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
            None,
        )
    };
    match handle {
        // SAFETY: we just opened this handle and nothing else owns it.
        Ok(h) => Ok(Some(unsafe { OwnedHandle::from_raw_handle(h.0) })),
        Err(e) if e.code() == ERROR_PIPE_BUSY.to_hresult() => Ok(None),
        // CreateFileW fails with Win32 errors, which live in the HRESULT's low bits.
        Err(e) => Err(io::Error::from_raw_os_error(e.code().0 & 0xFFFF)),
    }
}

/// Whether the client of a server pipe runs as SYSTEM. Windows only lets a server look
/// at its client once it has read from the pipe, so this fails before the first read.
pub fn peer_is_system(pipe: BorrowedHandle<'_>) -> bool {
    unsafe {
        if ImpersonateNamedPipeClient(HANDLE(pipe.as_raw_handle())).is_err() {
            return false;
        }
        let mut token = HANDLE::default();
        // As self: an identification-level token can't be used to open anything.
        let opened = OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, true, &mut token);
        // Carrying on with the client's identity on this thread is never acceptable.
        if RevertToSelf().is_err() {
            std::process::abort();
        }
        if opened.is_err() {
            return false;
        }
        // u64s to satisfy TOKEN_USER's alignment.
        let mut buf = [0u64; 64];
        let mut len = 0;
        let info = GetTokenInformation(
            token,
            TokenUser,
            Some(buf.as_mut_ptr().cast()),
            size_of_val(&buf) as u32,
            &mut len,
        );
        let _ = CloseHandle(token);
        info.is_ok()
            && IsWellKnownSid(
                (*buf.as_ptr().cast::<TOKEN_USER>()).User.Sid,
                WinLocalSystemSid,
            )
            .as_bool()
    }
}
