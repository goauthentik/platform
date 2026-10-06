use super::command_stdout;
use eyre::{Result, bail};
use std::process::Command;

/// Console user as reported by `scutil`, skipping the setup assistant user.
pub fn logged_in_via_gui() -> Result<Option<String>> {
    let out = command_stdout(
        Command::new("/bin/sh").args(["-c", r#"scutil <<< "show State:/Users/ConsoleUser""#]),
    )?;
    Ok(parse_scutil_console_user(&out))
}

fn parse_scutil_console_user(out: &str) -> Option<String> {
    out.lines()
        .filter_map(|l| l.trim_start().strip_prefix("Name : "))
        .filter_map(|v| v.split_whitespace().next())
        .find(|name| *name != "_mbsetupuser")
        .map(str::to_string)
}

/// Launches the bundle via LaunchServices as `user`, waiting for `open`.
pub fn run(path: &str, user: &str, env: &[(&str, &str)]) -> Result<()> {
    if !std::fs::metadata(path)?.is_dir() {
        bail!("path is not an .app directory: {path}");
    }
    let mut cmd = Command::new("sudo");
    cmd.args(["-H", "-u", user, "/usr/bin/open"]);
    for (k, v) in env {
        cmd.arg("--env").arg(format!("{k}={v}"));
    }
    cmd.arg(path);
    let status = cmd.status()?;
    if !status.success() {
        bail!("open path {path:?}: {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn scutil_console_user() {
        let out = "<dictionary> {\n  GID : 20\n  Name : _mbsetupuser\n  Name : jens\n}\n";
        assert_eq!(
            super::parse_scutil_console_user(out).as_deref(),
            Some("jens")
        );
        assert_eq!(
            super::parse_scutil_console_user("<dictionary> {\n}\n"),
            None
        );
    }
}
