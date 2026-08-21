//! Bidirectional byte-pump used to bridge an unprivileged rendezvous
//! connection with a privileged socket, once some elevation mechanism
//! (`pkexec`, a `runas`-launched process, an XPC daemon, ...) has produced a
//! stream connected to the privileged side.

use tokio::io::{AsyncRead, AsyncWrite, copy_bidirectional};

/// Copies bytes in both directions between `a` and `b` until either side
/// closes. Returns once the pump is done — callers decide what "done" means
/// for their transport (dropped connection, EOF, process exit, ...).
pub async fn pump<A, B>(mut a: A, mut b: B) -> std::io::Result<()>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    copy_bidirectional(&mut a, &mut b).await?;
    Ok(())
}
