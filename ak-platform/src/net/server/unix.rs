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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::MetadataExt;

    use super::{listen, SocketPermMode};

    #[tokio::test]
    async fn listen_creates_socket() {
        let path = "/tmp/ak-test-net-creates.sock".to_string();
        let _listener = listen(path.clone(), SocketPermMode::Owner).await.unwrap();
        assert!(fs::metadata(&path).is_ok());
    }

    #[tokio::test]
    async fn listen_removes_stale_socket() {
        let path = "/tmp/ak-test-net-stale.sock".to_string();
        let _first = listen(path.clone(), SocketPermMode::Owner).await.unwrap();
        // drop _first so the file stays but the listener is gone
        drop(_first);
        // second listen must not fail with EADDRINUSE
        listen(path.clone(), SocketPermMode::Owner).await.unwrap();
    }

    #[tokio::test]
    async fn listen_permission_owner() {
        let path = "/tmp/ak-test-net-perm-owner.sock".to_string();
        let _listener = listen(path.clone(), SocketPermMode::Owner).await.unwrap();
        let mode = fs::metadata(&path).unwrap().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[tokio::test]
    async fn listen_permission_everyone() {
        let path = "/tmp/ak-test-net-perm-everyone.sock".to_string();
        let _listener = listen(path.clone(), SocketPermMode::Everyone).await.unwrap();
        let mode = fs::metadata(&path).unwrap().mode() & 0o777;
        assert_eq!(mode, 0o666);
    }
}
