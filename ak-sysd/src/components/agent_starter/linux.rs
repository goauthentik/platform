use super::command_stdout;
use eyre::{Result, bail};
use std::process::Command;

struct User {
    name: String,
    uid: u32,
}

#[derive(Debug, PartialEq)]
enum SessionType {
    X11,
    Wayland,
    Tty,
}

pub fn logged_in_via_gui() -> Result<Option<String>> {
    let user = login_user()?;
    if user.name == "gdm" || user.name == "root" {
        return Ok(None);
    }
    match session_type(user.uid) {
        Ok(SessionType::Tty) => {
            tracing::debug!("user {} is logged in via TTY, not GUI", user.name);
            Ok(None)
        }
        Ok(_) => Ok(Some(user.name)),
        Err(e) => {
            tracing::debug!(
                "failed to get display session type for {}: {e:?}",
                user.name
            );
            Ok(None)
        }
    }
}

/// Spawns the agent via `sudo` as the login user with their display and
/// session bus environment.
pub fn run(path: &str, _user: &str, env: &[(&str, &str)]) -> Result<()> {
    let user = login_user()?;
    let session_type = match session_type(user.uid) {
        Ok(SessionType::Tty) => {
            bail!(
                "user {:?} ({}) is not running a GUI session",
                user.name,
                user.uid
            )
        }
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("assuming wayland session: {e:?}");
            SessionType::Wayland
        }
    };

    let mut vars = Vec::new();
    if session_type == SessionType::X11 {
        let display = x11_display(&user.name).unwrap_or_else(|e| {
            tracing::warn!("failed to get X11 display, using default :0: {e:?}");
            ":0".to_string()
        });
        vars.push(format!("DISPLAY={display}"));
    } else {
        let display = wayland_display(user.uid).unwrap_or_else(|e| {
            tracing::warn!("failed to get wayland display, using default wayland-0: {e:?}");
            "wayland-0".to_string()
        });
        // xdg-open on Wayland still needs DISPLAY.
        let x11 = display.strip_prefix("wayland-").unwrap_or(&display);
        vars.push(format!("WAYLAND_DISPLAY={display}"));
        vars.push(format!("DISPLAY=:{x11}"));
    }
    vars.push(format!(
        "DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/{}/bus",
        user.uid
    ));
    let dir = std::path::Path::new(path)
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let ld = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
    vars.push(format!("LD_LIBRARY_PATH={dir}:{ld}"));
    vars.extend(env.iter().map(|(k, v)| format!("{k}={v}")));

    let mut cmd = Command::new("sudo");
    cmd.args(["-n", "-i", "-u", &user.name, "-H", "env"])
        .args(vars)
        .arg(path);
    tracing::info!(?cmd, "running command");
    cmd.spawn()
        .map_err(|e| eyre::eyre!("open path {path:?}: {e}"))?;
    Ok(())
}

/// First user reported by `users`.
fn login_user() -> Result<User> {
    let out = command_stdout(&mut Command::new("users"))?;
    let name = out.trim().split(' ').next().unwrap_or_default().to_string();
    if name.is_empty() {
        bail!("no user session found");
    }
    let uid = command_stdout(Command::new("id").args(["-u", &name]))?
        .trim()
        .parse()?;
    Ok(User { name, uid })
}

/// A user without a `Display` session is treated as Wayland.
fn session_type(uid: u32) -> Result<SessionType> {
    let session = command_stdout(Command::new("loginctl").args([
        "show-user",
        &uid.to_string(),
        "-p",
        "Display",
        "--value",
    ]))?;
    let session = session.trim();
    if session.is_empty() {
        return Ok(SessionType::Wayland);
    }
    let kind = command_stdout(Command::new("loginctl").args([
        "show-session",
        session,
        "-p",
        "Type",
        "--value",
    ]))?;
    match kind.trim() {
        "x11" => Ok(SessionType::X11),
        "wayland" => Ok(SessionType::Wayland),
        "tty" => Ok(SessionType::Tty),
        "" => bail!("empty GUI session type"),
        other => bail!("unknown GUI session type: {other:?}"),
    }
}

fn wayland_display(uid: u32) -> Result<String> {
    let mut names: Vec<String> = std::fs::read_dir(format!("/run/user/{uid}"))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("wayland-") && !n.ends_with(".lock"))
        .collect();
    names.sort();
    names
        .into_iter()
        .next()
        .ok_or_else(|| eyre::eyre!("wayland socket not found"))
}

fn x11_display(user: &str) -> Result<String> {
    parse_who_display(&command_stdout(&mut Command::new("who"))?, user)
        .ok_or_else(|| eyre::eyre!("display not found on who output"))
}

/// Finds `<user> :<N> ` in `who` output.
fn parse_who_display(out: &str, user: &str) -> Option<String> {
    out.lines().find_map(|line| {
        let mut it = line.split_whitespace();
        let (name, display) = (it.next()?, it.next()?);
        let is_display = display
            .strip_prefix(':')
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
        (name == user && is_display && it.next().is_some()).then(|| display.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn who_display() {
        let out = "gdm      tty1         2024-01-01 10:00 (tty1)\n\
                   jens     :1           2024-01-01 10:00 (:1)\n";
        assert_eq!(parse_who_display(out, "jens").as_deref(), Some(":1"));
        assert_eq!(parse_who_display(out, "gdm"), None);
    }
}
