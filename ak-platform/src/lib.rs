pub mod client;
pub mod config;
pub mod dpop;
pub mod generated;
pub mod grpc;
pub mod http;
pub mod log;
pub mod net;
pub mod oauth;
pub mod oauth2_http;
pub mod paths;
pub mod setup;
pub mod shared;
pub mod storage;
pub mod string;
pub mod tui;

/// Rust ignores SIGPIPE, so printing to a closed pipe (`ak ... | head`) panics.
/// Restore the default so CLI commands exit quietly like other Unix tools.
/// Not for long-running daemons, where a peer closing a socket would kill the process.
pub fn restore_sigpipe() {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

/// `std::process::Command` that doesn't flash a console window when spawned from a
/// GUI-subsystem app on Windows.
pub fn command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}
