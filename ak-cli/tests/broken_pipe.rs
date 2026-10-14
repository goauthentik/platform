#![cfg(unix)]

use std::io::Read;
use std::process::{Command, Stdio};

// `ak completion bash | head -c1` must not panic (PLATFORM-4J, PLATFORM-3Z).
#[test]
fn closed_stdout_does_not_panic() -> std::io::Result<()> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ak"))
        .args(["completion", "bash"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    drop(child.stdout.take());
    let mut stderr = String::new();
    if let Some(mut e) = child.stderr.take() {
        e.read_to_string(&mut stderr)?;
    }
    let status = child.wait()?;
    assert_ne!(status.code(), Some(101), "panicked: {stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
    Ok(())
}
