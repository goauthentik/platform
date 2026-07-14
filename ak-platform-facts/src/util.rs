use eyre::Result;

pub fn attempt<T>(name: &str, f: impl FnOnce() -> Result<T>) -> Option<T> {
    match f() {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("failed to gather {name} facts: {e:?}");
            None
        }
    }
}

/// Only reached on targets outside linux/macos/windows — dead code on
/// every platform this actually gets built for.
#[allow(dead_code)]
pub fn unsupported_platform<T>(subsystem: &str) -> Result<T> {
    eyre::bail!(
        "no {subsystem} implementation for platform {}",
        std::env::consts::OS
    )
}

pub fn run(cmd: &mut std::process::Command) -> Result<String> {
    let out = cmd.output()?;
    if !out.status.success() {
        eyre::bail!(
            "{:?} exited with {}: {}",
            cmd,
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8(out.stdout)?)
}
