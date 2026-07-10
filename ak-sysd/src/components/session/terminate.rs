use crate::state::SessionRecord;
use eyre::Result;

/// Terminates the process associated with an expired session. Ported from
/// Go's `terminate_linux.go` on Linux; Windows support is a placeholder —
/// `pkg/agent_system/session/terminate_other.go` was not read in this pass,
/// so its exact behavior (real `TerminateProcess` call vs. a no-op stub) is
/// unconfirmed.
pub async fn terminate_session(session: &SessionRecord) -> Result<()> {
    if let Some(socket) = &session.local_socket {
        std::fs::remove_file(socket).ok();
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(pid) = session.pid {
            let pid = pid as libc::pid_t;
            unsafe { libc::kill(pid, libc::SIGTERM) };
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let still_alive = unsafe { libc::kill(pid, 0) } == 0;
            if still_alive {
                unsafe { libc::kill(pid, libc::SIGKILL) };
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        tracing::warn!(
            session = session.id,
            "Windows session termination not yet implemented — read \
             pkg/agent_system/session/terminate_other.go before implementing"
        );
    }

    Ok(())
}
