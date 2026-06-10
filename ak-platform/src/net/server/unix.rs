use std::{error::Error, fs, path::Path};
use std::os::unix::fs::PermissionsExt;

use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;

use super::{ListenerStream, SocketPermMode};

pub async fn listen(
    path: String,
    perm: SocketPermMode,
) -> Result<ListenerStream, Box<dyn Error + Send + Sync>> {
    let p = Path::new(&path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    // Remove stale socket to avoid EADDRINUSE
    if p.exists() {
        fs::remove_file(p)?;
    }

    let mode: libc::mode_t = match perm {
        SocketPermMode::Owner => 0o600,
        SocketPermMode::Everyone => 0o666,
        SocketPermMode::Admin => 0o660,
    };

    // Restrict umask to eliminate the TOCTOU window where the socket briefly
    // exists with permissive default permissions before chmod runs.
    let old_umask = unsafe { libc::umask(0o777 & !mode) };
    let result = UnixListener::bind(&path);
    unsafe { libc::umask(old_umask) };

    let uds = result?;
    fs::set_permissions(&path, fs::Permissions::from_mode(u32::from(mode)))?;

    Ok(ListenerStream::Unix(UnixListenerStream::new(uds)))
}
