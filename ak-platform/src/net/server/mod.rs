use std::{
    io,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};

use interprocess::local_socket::{GenericFilePath, ListenerOptions, tokio::prelude::*};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;
use tokio_stream::Stream as AsyncStream;

use crate::string::PlatformString;

pub mod creds;

pub enum SocketPermMode {
    Owner,
    Everyone,
    Admin,
}

pub struct ConnectedLocalStream(LocalSocketStream);

impl AsyncRead for ConnectedLocalStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl AsyncWrite for ConnectedLocalStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

pub struct ListenerStream {
    rx: mpsc::Receiver<io::Result<ConnectedLocalStream>>,
}

impl AsyncStream for ListenerStream {
    type Item = io::Result<ConnectedLocalStream>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

pub async fn listen(
    path: PlatformString,
    perm: SocketPermMode,
) -> Result<ListenerStream, Box<dyn std::error::Error + Send + Sync>> {
    let path_str = path.for_current();

    #[cfg(unix)]
    if let Some(parent) = Path::new(&path_str).parent() {
        std::fs::create_dir_all(parent)?;
    }
    #[cfg(not(unix))]
    let _ = Path::new(&path_str);

    let name = path_str.as_str().to_fs_name::<GenericFilePath>()?;

    #[cfg(unix)]
    let mode: libc::mode_t = match perm {
        SocketPermMode::Owner => 0o600,
        SocketPermMode::Everyone => 0o666,
        SocketPermMode::Admin => 0o660,
    };
    #[cfg(not(unix))]
    let _ = perm;

    // Restrict umask to close the TOCTOU window where the socket file briefly
    // exists with permissive default permissions before the final chmod runs.
    #[cfg(unix)]
    let old_umask = unsafe { libc::umask(0o777 & !mode) };

    let create_result = ListenerOptions::new()
        .name(name)
        .try_overwrite(true)
        .create_tokio();

    #[cfg(unix)]
    unsafe {
        libc::umask(old_umask)
    };

    let listener = create_result?;

    // Apply the final file mode to the socket path now that the listener is bound.
    // ListenerOptions::mode() uses fchmod(fd) which returns EINVAL on macOS sockets,
    // so we use path-based chmod here instead.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path_str, std::fs::Permissions::from_mode(mode as u32))?;
    }

    log::debug!("Starting socket on {path_str}");
    let (tx, rx) = mpsc::channel(1);
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok(stream) => {
                    if tx.send(Ok(ConnectedLocalStream(stream))).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                    break;
                }
            }
        }
    });

    Ok(ListenerStream { rx })
}

#[cfg(test)]
mod tests {
    use super::{SocketPermMode, listen};
    use crate::string::PlatformString;

    fn ps(s: &str) -> PlatformString {
        PlatformString::new_with_default(s)
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn listen_creates_socket() {
        let path = "/tmp/ak-test-net-creates.sock";
        let _listener = listen(ps(path), SocketPermMode::Owner).await.unwrap();
        assert!(std::fs::metadata(path).is_ok());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn listen_removes_stale_socket() {
        let path = "/tmp/ak-test-net-stale.sock";
        let _first = listen(ps(path), SocketPermMode::Owner).await.unwrap();
        drop(_first);
        listen(ps(path), SocketPermMode::Owner).await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn listen_permission_owner() {
        use std::os::unix::fs::MetadataExt;
        let path = "/tmp/ak-test-net-perm-owner.sock";
        let _listener = listen(ps(path), SocketPermMode::Owner).await.unwrap();
        let mode = std::fs::metadata(path).unwrap().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn listen_permission_everyone() {
        use std::os::unix::fs::MetadataExt;
        let path = "/tmp/ak-test-net-perm-everyone.sock";
        let _listener = listen(ps(path), SocketPermMode::Everyone).await.unwrap();
        let mode = std::fs::metadata(path).unwrap().mode() & 0o777;
        assert_eq!(mode, 0o666);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn listen_creates_pipe() {
        use tokio::net::windows::named_pipe::ClientOptions;
        let path = r"\\.\pipe\ak-test-server-creates";
        let _listener = listen(ps(path), SocketPermMode::Owner).await.unwrap();
        ClientOptions::new().open(path).unwrap();
    }
}
