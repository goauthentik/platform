//! Spawns `ak_cef.exe` in the interactive session and exchanges
//! `wire`-framed messages with it over its inherited standard handles: it
//! writes its result to stdout, and reads a cancel signal from stdin.
//! Anonymous, inherited pipes rather than named ones with a custom DACL —
//! matching GCPW's own approach (`CreatePipeForChildProcess`,
//! `gcp_utils.cc`) — because the child never opens anything by name at all,
//! so there is no DACL for a hardened box's Object Manager namespace to
//! disagree with. See `BROWSER_PRIVILEGE.md`'s "Roads not taken".

use std::ffi::c_void;
use std::fs::File;
use std::mem::size_of;
use std::os::windows::io::FromRawHandle;
use std::path::Path;

use windows::{
    Win32::{
        Foundation::{
            CloseHandle, E_FAIL, HANDLE, HANDLE_FLAG_INHERIT, HANDLE_FLAGS, SetHandleInformation,
            WAIT_OBJECT_0,
        },
        Security::{SE_IMPERSONATE_NAME, SECURITY_ATTRIBUTES},
        System::Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock},
        System::Pipes::CreatePipe,
        System::Threading::{
            CREATE_UNICODE_ENVIRONMENT, CreateProcessW, CreateProcessWithTokenW,
            GetExitCodeProcess, LOGON_WITH_PROFILE, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION,
            STARTF_USESTDHANDLES, STARTUPINFOW, WaitForSingleObject,
        },
        UI::Shell::{CPUS_CREDUI, CREDENTIAL_PROVIDER_USAGE_SCENARIO},
        UI::WindowsAndMessaging::AllowSetForegroundWindow,
    },
    core::{PCWSTR, PWSTR},
};

use crate::syscalls::{self, ForegroundControl};
use crate::sysd;
use ak_ee_wcp_wire::{AuthResult, HostReport};

/// Spawns `ak_cef.exe` and waits for its result. `should_continue` is polled
/// while waiting, so LogonUI cancelling (the user backing out of the tile)
/// tears the browser process down instead of orphaning it.
pub trait AuthFlow {
    fn run(&self, should_continue: &mut dyn FnMut() -> bool) -> AuthResult;
}

pub struct CefAuthFlow {
    pub cef_exe: std::path::PathBuf,
    pub cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
}

impl AuthFlow for CefAuthFlow {
    fn run(&self, should_continue: &mut dyn FnMut() -> bool) -> AuthResult {
        run_cef_host(&self.cef_exe, self.cpus, should_continue)
    }
}

/// Only `CPUS_CREDUI` may fall back to launching in the caller's own session.
/// It is debug-gated and runs on an ordinary desktop; the logon scenarios
/// must never take this fallback, or Chromium ends up on the secure desktop
/// with this process's own SYSTEM token instead of the service account's.
fn may_launch_in_current_session(cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO) -> bool {
    cpus == CPUS_CREDUI
}

/// The desktop LogonUI draws on. A window created on any other desktop of the
/// same window station is fully functional but invisible to the person signing
/// in, so the logon scenarios have to name it: with `lpDesktop` left NULL,
/// `CreateProcess*` gives the child whichever desktop the caller happens to be
/// on, which is only incidentally the right one.
const SECURE_DESKTOP: &str = r"WinSta0\Winlogon";

/// `CPUS_CREDUI` is the debug-gated scenario that runs on the ordinary
/// interactive desktop, so it keeps `lpDesktop` NULL and inherits it.
fn desktop_for(cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO) -> Option<Vec<u16>> {
    if may_launch_in_current_session(cpus) {
        return None;
    }
    Some(
        SECURE_DESKTOP
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect(),
    )
}

struct StdPipes {
    /// This process's own ends, read/written after the child is spawned.
    result_read: HANDLE,
    cancel_write: HANDLE,
    /// The child's ends, handed off via `STARTUPINFOW`'s `hStdOutput`/
    /// `hStdInput` and closed here once the child has its own inherited
    /// copies.
    child_stdout: HANDLE,
    child_stdin: HANDLE,
}

/// One anonymous, inheritable pipe. `CreatePipe`'s `SECURITY_ATTRIBUTES`
/// marks *both* handles it returns as inheritable, so the caller is
/// responsible for clearing that flag on whichever end it keeps for
/// itself — otherwise this process's own copy would leak into every future
/// child it spawns, not just this one.
fn create_inherited_pipe() -> windows::core::Result<(HANDLE, HANDLE)> {
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: true.into(),
    };
    let mut read = HANDLE::default();
    let mut write = HANDLE::default();
    unsafe { CreatePipe(&mut read, &mut write, Some(&sa), 0)? };
    Ok((read, write))
}

fn keep_private(handle: HANDLE) -> windows::core::Result<()> {
    unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0)) }
}

/// Two anonymous pipes: the child reads the cancel signal from its inherited
/// stdin and writes its result to its inherited stdout. An *inherited*
/// handle is a duplicate of one this process (SYSTEM on the real logon
/// scenarios) already opened and validated — the child's own, low-privilege
/// token is never consulted at all, unlike a named pipe it has to open by
/// path itself.
fn create_std_pipes() -> windows::core::Result<StdPipes> {
    let (child_stdin, cancel_write) = create_inherited_pipe()?;
    if let Err(e) = keep_private(cancel_write) {
        unsafe {
            let _ = CloseHandle(child_stdin);
            let _ = CloseHandle(cancel_write);
        }
        return Err(e);
    }

    let (result_read, child_stdout) = match create_inherited_pipe() {
        Ok(p) => p,
        Err(e) => {
            unsafe {
                let _ = CloseHandle(child_stdin);
                let _ = CloseHandle(cancel_write);
            }
            return Err(e);
        }
    };
    if let Err(e) = keep_private(result_read) {
        unsafe {
            let _ = CloseHandle(child_stdin);
            let _ = CloseHandle(cancel_write);
            let _ = CloseHandle(result_read);
            let _ = CloseHandle(child_stdout);
        }
        return Err(e);
    }

    Ok(StdPipes {
        result_read,
        cancel_write,
        child_stdout,
        child_stdin,
    })
}

fn run_cef_host(
    cef_exe: &Path,
    cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
    should_continue: &mut dyn FnMut() -> bool,
) -> AuthResult {
    // Fetched here, not by `ak_cef.exe` itself: the service account it runs
    // as has no access to `ak-sysd`'s pipe (`BROWSER_PRIVILEGE.md`). Doing
    // this before the pipes/spawn also means a failure here costs nothing
    // beyond the round trip itself, rather than a spawned window that can
    // never load anything.
    let start = match sysd::sys_auth_start_async() {
        Ok(s) => s,
        Err(e) => {
            log::error!("sys_auth_start_async failed: {e}");
            return AuthResult::Failed {
                reason: e.to_string(),
            };
        }
    };

    let pipes = match create_std_pipes() {
        Ok(p) => p,
        Err(e) => {
            log::error!("failed to create IPC pipes: {e}");
            return AuthResult::Failed {
                reason: "failed to create IPC pipes".to_string(),
            };
        }
    };

    let spawn = spawn_cef_host(cef_exe, &pipes, cpus, &start.url, &start.header_token);
    // Our copies of the child's ends are only needed up to the spawn call,
    // which duplicates them into the child's own handle table (or fails,
    // in which case there is no child to hold them at all either way).
    unsafe {
        let _ = CloseHandle(pipes.child_stdin);
        let _ = CloseHandle(pipes.child_stdout);
    }
    let process = match spawn {
        Ok(p) => p,
        Err(e) => {
            log::error!("failed to launch {}: {e}", cef_exe.display());
            unsafe {
                let _ = CloseHandle(pipes.result_read);
                let _ = CloseHandle(pipes.cancel_write);
            }
            return AuthResult::Failed {
                reason: "failed to launch sign-in window".to_string(),
            };
        }
    };

    let result = wait_for_result(
        pipes.result_read,
        pipes.cancel_write,
        process.hProcess,
        process.dwProcessId,
        should_continue,
    );

    unsafe {
        let _ = CloseHandle(pipes.cancel_write);
        let _ = WaitForSingleObject(process.hProcess, 5_000);
        let _ = CloseHandle(process.hProcess);
        let _ = CloseHandle(process.hThread);
    }

    result
}

/// `wait_for_result` polls every 200 ms, but the browser takes seconds to open
/// a window, so nudge on every fifth poll rather than enumerating an empty
/// desktop five times a second.
const NUDGE_INTERVAL_TICKS: u32 = 5;

/// Ten seconds of polls. Past that the window has either appeared or is not
/// going to, and the sign-in has no deadline this should compete with.
const NUDGE_BUDGET_TICKS: u32 = 50;

fn nudge_due(tick: u32, settled: bool) -> bool {
    !settled && tick < NUDGE_BUDGET_TICKS && tick.is_multiple_of(NUDGE_INTERVAL_TICKS)
}

/// Pushes `ak_cef.exe`'s window to the front from inside LogonUI.
///
/// The child asks for the foreground itself, but a freshly spawned process is
/// rarely allowed to take it, and the single `AllowSetForegroundWindow` issued
/// at spawn is spent by the time CEF has a window to apply it to. This process
/// is the one that can: right desktop, normally the foreground process
/// already, and exempt from a foreground lock it took itself.
///
/// Settles on the child *being* the foreground rather than on a call
/// succeeding, so a window that takes it and immediately loses it again to a
/// LogonUI repaint gets pushed back rather than counted as done.
struct ForegroundNudge {
    child_pid: u32,
    tick: u32,
    settled: bool,
    spawned: std::time::Instant,
    window_seen: bool,
}

impl ForegroundNudge {
    fn new(child_pid: u32) -> Self {
        Self {
            child_pid,
            tick: 0,
            settled: false,
            spawned: std::time::Instant::now(),
            window_seen: false,
        }
    }

    fn poll(&mut self, fg: &dyn syscalls::ForegroundControl) {
        let due = nudge_due(self.tick, self.settled);
        self.tick += 1;
        if !due {
            return;
        }

        if fg.foreground_pid() == Some(self.child_pid) {
            log::info!("the sign-in window holds the foreground");
            self.settled = true;
            return;
        }

        let Some(window) = fg.visible_top_level_window(self.child_pid) else {
            return;
        };
        if !self.window_seen {
            self.window_seen = true;
            // The run that works is the slow one, so how wide this gap is says
            // whether the grant at spawn had any chance of surviving it.
            log::info!(
                "the sign-in window appeared {}ms after the spawn",
                self.spawned.elapsed().as_millis()
            );
        }

        let allowed = fg.allow_set_foreground(self.child_pid);
        let taken = fg.set_foreground(window);
        log::info!("nudged the sign-in window ({window:#x}): re-armed={allowed} took={taken}");
    }
}

/// What the result-pipe reader thread saw. `Eof` and `Error` both end up as a
/// cancellation for the user, but they mean very different things — the host
/// died vs. the pipe misbehaved — so they stay separate long enough to be
/// logged.
enum PipeOutcome {
    Result(AuthResult),
    Eof,
    Error(String),
}

/// Turns the sign-in redirect's URL into a real outcome by validating its
/// token against `ak-sysd` — the one step `ak_cef.exe` cannot do itself
/// (`BROWSER_PRIVILEGE.md`). Runs on the result-pipe reader thread, not the
/// thread LogonUI called `Connect` on, so this blocking round trip does not
/// stall `should_continue` polling or the foreground nudge.
fn auth_result_for(url: &str) -> AuthResult {
    match sysd::sys_auth_validate(url) {
        Ok(Some(username)) => AuthResult::Completed { username },
        Ok(None) => AuthResult::Failed {
            reason: "token validation failed".to_string(),
        },
        Err(e) => AuthResult::Failed {
            reason: e.to_string(),
        },
    }
}

/// Polls in short slices so `should_continue` gets a turn. On cancellation it
/// asks `ak_cef.exe` to close over the control pipe rather than killing it.
///
/// Every route out of here other than a real `AuthResult` looks identical to
/// the user ("Login attempt cancelled"), so each one logs why: a silent
/// cancellation is indistinguishable from the sign-in window never appearing.
/// A crash before the child ever gets to send anything now surfaces as a
/// plain EOF (its inherited stdout closes when the process dies) rather than
/// a separate "never connected" error — there is no longer a separate
/// connect step to fail.
fn wait_for_result(
    result_read: HANDLE,
    cancel_write: HANDLE,
    process: HANDLE,
    child_pid: u32,
    should_continue: &mut dyn FnMut() -> bool,
) -> AuthResult {
    let mut result_file = unsafe { File::from_raw_handle(result_read.0) };
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let outcome = match ak_ee_wcp_wire::read_host_report(&mut result_file) {
            Ok(Some(HostReport::Redirected { url })) => PipeOutcome::Result(auth_result_for(&url)),
            Ok(Some(HostReport::Cancelled)) => PipeOutcome::Result(AuthResult::Cancelled),
            Ok(None) => PipeOutcome::Eof,
            Err(e) => PipeOutcome::Error(e.to_string()),
        };
        let _ = tx.send(outcome);
    });

    let mut cancel_signalled = false;
    // This loop runs on the thread LogonUI called `Connect` on, so it is on
    // the desktop the browser's window appears on — which `EnumWindows` and
    // `SetForegroundWindow` both need, and which no other thread here has.
    let mut nudge = ForegroundNudge::new(child_pid);
    loop {
        match rx.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(PipeOutcome::Result(outcome)) => {
                log::info!("sign-in window reported {outcome:?}");
                return outcome;
            }
            Ok(PipeOutcome::Eof) => {
                log::warn!(
                    "sign-in window closed the result pipe without sending a result ({}); \
                     treating as cancelled",
                    describe_exit(process)
                );
                return AuthResult::Cancelled;
            }
            Ok(PipeOutcome::Error(e)) => {
                log::error!("failed to read the sign-in result: {e}; treating as cancelled");
                return AuthResult::Cancelled;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                nudge.poll(&syscalls::RealSyscalls);
                if !cancel_signalled && !should_continue() {
                    log::info!("LogonUI withdrew the sign-in; asking the window to close");
                    cancel_signalled = true;
                    signal_cancel(cancel_write);
                }
                // Host exited without sending a result.
                if unsafe { WaitForSingleObject(process, 0) } == WAIT_OBJECT_0 {
                    log::warn!(
                        "sign-in window exited without sending a result ({})",
                        describe_exit(process)
                    );
                    return AuthResult::Cancelled;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                log::error!("the result-pipe reader thread died; treating as cancelled");
                return AuthResult::Cancelled;
            }
        }
    }
}

/// Exit code of `ak_cef.exe` rendered for a log line. The value is the whole
/// diagnosis when the host dies before it can say anything: `0xC0000005` is a
/// crash, `0xC0000142` is a `DLL_INIT_FAILED` from a desktop the process has no
/// access to, and `0` is a clean exit that simply skipped the result.
fn describe_exit(process: HANDLE) -> String {
    let mut code = 0u32;
    if unsafe { GetExitCodeProcess(process, &mut code) }.is_err() {
        return "exit code unavailable".to_string();
    }
    // 259 is STILL_ACTIVE.
    if code == 259 {
        return "still running".to_string();
    }
    format!("exit code {code:#010x}")
}

fn signal_cancel(cancel_write: HANDLE) {
    let mut f = unsafe { File::from_raw_handle(cancel_write.0) };
    let _ = ak_ee_wcp_wire::write_frame(&mut f, &ak_ee_wcp_wire::CancelSignal {});
    std::mem::forget(f);
}

/// Gets `ak_cef.exe` a token for the dedicated service account rather than
/// SYSTEM (`BROWSER_PRIVILEGE.md`), the same way for both logon and unlock.
/// Account-hardening is best-effort and only logged on failure — it is
/// idempotent, so a transient failure just costs a retry next time, and
/// does not block the token mint that follows. The password is not: a
/// broken keyring here means no way to log the account on at all.
fn acquire_service_account_token() -> windows::core::Result<HANDLE> {
    let password = syscalls::service_account_password().map_err(|e| {
        log::error!("could not establish the service account's password: {e}");
        windows::core::Error::from(E_FAIL)
    })?;

    let sid = syscalls::account_sid(syscalls::SERVICE_ACCOUNT_NAME)?;

    if let Err(e) = syscalls::deny_interactive_and_network_logon(&sid) {
        log::warn!("could not deny the service account interactive/network logon: {e}");
    }
    if let Err(e) = syscalls::ensure_desktop_access(&sid) {
        log::warn!("could not grant the service account secure-desktop access: {e}");
    }
    if let Err(e) = syscalls::ensure_base_named_objects_access(&sid) {
        log::warn!("could not grant the service account BaseNamedObjects access: {e}");
    }

    syscalls::service_account_token(syscalls::SERVICE_ACCOUNT_NAME, &password)
}

/// Minimal Windows command-line quoting: wraps `s` in quotes and escapes any
/// embedded ones, so `CommandLineToArgvW` (what `std::env::args()` on the
/// far end is built on) sees it as a single argument. Neither a URL nor an
/// opaque token legitimately contains the backslash-before-quote sequence
/// the full algorithm exists to handle.
fn quote_arg(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\\\""))
}

fn spawn_cef_host(
    cef_exe: &Path,
    pipes: &StdPipes,
    cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
    sign_in_url: &str,
    header_token: &str,
) -> windows::core::Result<PROCESS_INFORMATION> {
    let cmdline = format!(
        "\"{}\" --sign-in-url {} --header-token {}",
        cef_exe.display(),
        quote_arg(sign_in_url),
        quote_arg(header_token),
    );

    let mut si = STARTUPINFOW {
        cb: size_of::<STARTUPINFOW>() as u32,
        dwFlags: STARTF_USESTDHANDLES,
        hStdInput: pipes.child_stdin,
        hStdOutput: pipes.child_stdout,
        ..Default::default()
    };
    // Outlives every `CreateProcess*` call below; `lpDesktop` borrows it.
    let mut desktop = desktop_for(cpus);
    if let Some(desktop) = desktop.as_mut() {
        si.lpDesktop = PWSTR(desktop.as_mut_ptr());
    }
    let mut pi = PROCESS_INFORMATION::default();

    let token = if may_launch_in_current_session(cpus) {
        None
    } else {
        match acquire_service_account_token() {
            Ok(token) => Some(token),
            Err(e) => {
                log::error!("could not acquire the service account's token: {e}");
                return Err(e);
            }
        }
    };
    log::info!(
        "launching {} on desktop {} with {} token",
        cef_exe.display(),
        desktop
            .as_ref()
            .map(|_| SECURE_DESKTOP)
            .unwrap_or("<inherited>"),
        if token.is_some() {
            "the service account's"
        } else {
            "the caller's own"
        }
    );

    let spawned = match token {
        Some(token) => {
            // Confirmed enabled on the test box, but `SE_TCB_NAME` looked
            // that way too until it turned out not to be held at all —
            // enable it defensively rather than trust the default.
            if let Err(e) =
                syscalls::enable_privilege(SE_IMPERSONATE_NAME, "SeImpersonatePrivilege")
            {
                log::warn!("could not enable SeImpersonatePrivilege: {e}");
            }
            let result = spawn_with_token(token, &cmdline, &si, &mut pi);
            unsafe {
                let _ = CloseHandle(token);
            }
            result
        }
        None => spawn_in_current_session(&cmdline, &si, &mut pi),
    };
    spawned?;

    // A freshly spawned process may not bring its own window forward without
    // this. On its own it is not enough and never was: the grant is
    // single-shot, and the window it applies to does not exist for several
    // seconds yet, by which time any foreground change has spent it —
    // `ForegroundNudge` re-arms it once there is something to push. A failure
    // is about this process rather than the child, since the call is refused
    // when the caller is not itself entitled to the foreground.
    if let Err(e) = unsafe { AllowSetForegroundWindow(pi.dwProcessId) } {
        log::warn!("AllowSetForegroundWindow({}) failed: {e}", pi.dwProcessId);
    }
    log::info!(
        "spawned the sign-in window as pid {}, foreground_pid={:?}",
        pi.dwProcessId,
        syscalls::RealSyscalls.foreground_pid(),
    );

    Ok(pi)
}

/// Brokered through the Secondary Logon service rather than done directly,
/// which is why this needs only `SE_IMPERSONATE_NAME` — `CreateProcessAsUserW`
/// needed `SE_ASSIGNPRIMARYTOKEN_NAME`/`SE_INCREASE_QUOTA_NAME`, both
/// confirmed absent from LogonUI's token. `LOGON_WITH_PROFILE` loads the
/// account's registry hive but not its environment block; building one
/// explicitly is what makes `%TEMP%`/`%LOCALAPPDATA%` resolve to the service
/// account's own profile instead of SYSTEM's (see `BROWSER_PRIVILEGE.md`).
/// `CreateProcessWithTokenW` has no `bInheritHandles` parameter at all
/// (unlike `CreateProcessW`/`CreateProcessAsUserW`), but it does honor
/// `si`'s inheritable `hStdInput`/`hStdOutput` — confirmed against GCPW's
/// own equivalent call (`OSProcessManager::CreateProcessWithToken`,
/// `os_process_manager.cc`), which relies on exactly this.
fn spawn_with_token(
    token: HANDLE,
    cmdline: &str,
    si: &STARTUPINFOW,
    pi: &mut PROCESS_INFORMATION,
) -> windows::core::Result<()> {
    let mut cmdline_wide: Vec<u16> = cmdline.encode_utf16().chain(std::iter::once(0)).collect();

    // Best-effort: on this account's very first ever launch its profile may
    // not exist on disk yet — only `LOGON_WITH_PROFILE` below creates it —
    // so `CreateEnvironmentBlock` can fail here. Falling back to this
    // process's own environment rather than refusing the spawn entirely
    // matches every other "logged, not fatal" cleanup/setup step in this
    // file; the spawn is still worth attempting either way.
    let mut env_block: *mut c_void = std::ptr::null_mut();
    let has_env = unsafe { CreateEnvironmentBlock(&mut env_block, Some(token), false) }.is_ok();
    if !has_env {
        log::warn!(
            "could not build an environment block for the service account; \
             falling back to this process's own"
        );
    }
    let (creation_flags, environment) = if has_env {
        (
            PROCESS_CREATION_FLAGS(CREATE_UNICODE_ENVIRONMENT.0),
            Some(env_block as *const c_void),
        )
    } else {
        (PROCESS_CREATION_FLAGS(0), None)
    };

    let result = unsafe {
        CreateProcessWithTokenW(
            token,
            LOGON_WITH_PROFILE,
            PCWSTR::null(),
            Some(PWSTR(cmdline_wide.as_mut_ptr())),
            creation_flags,
            environment,
            PCWSTR::null(),
            si,
            pi,
        )
    };

    if has_env {
        unsafe {
            let _ = DestroyEnvironmentBlock(env_block);
        }
    }

    result
}

/// `CreateProcessW` may write into the command-line buffer it is handed, so
/// each attempt gets a fresh copy. `bInheritHandles` is `true` so the child
/// picks up `si`'s `hStdInput`/`hStdOutput`.
fn spawn_in_current_session(
    cmdline: &str,
    startup_info: &STARTUPINFOW,
    pi: &mut PROCESS_INFORMATION,
) -> windows::core::Result<()> {
    let mut cmdline_wide: Vec<u16> = cmdline.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        CreateProcessW(
            PCWSTR::null(),
            Some(PWSTR(cmdline_wide.as_mut_ptr())),
            None,
            None,
            true,
            PROCESS_CREATION_FLAGS(0),
            None,
            PCWSTR::null(),
            startup_info,
            pi,
        )
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use windows::Win32::System::Threading::TerminateProcess;
    use windows::Win32::UI::Shell::{CPUS_CHANGE_PASSWORD, CPUS_LOGON, CPUS_UNLOCK_WORKSTATION};

    const CHILD: u32 = 4242;
    const LOGONUI: u32 = 7;
    const CHILD_WINDOW: isize = 0xBEEF;

    #[derive(Default)]
    struct FakeForeground {
        /// `None` until the browser has opened a window a person could see.
        window: Cell<Option<isize>>,
        foreground: Cell<Option<u32>>,
        /// Stands in for the foreground rules refusing the call.
        grant_works: Cell<bool>,
        calls: RefCell<Vec<String>>,
    }

    impl FakeForeground {
        fn with_window_up() -> Self {
            let fake = Self {
                foreground: Cell::new(Some(LOGONUI)),
                grant_works: Cell::new(true),
                ..Self::default()
            };
            fake.window.set(Some(CHILD_WINDOW));
            fake
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl syscalls::ForegroundControl for FakeForeground {
        fn foreground_pid(&self) -> Option<u32> {
            self.foreground.get()
        }

        fn visible_top_level_window(&self, pid: u32) -> Option<isize> {
            self.calls.borrow_mut().push(format!("find({pid})"));
            self.window.get()
        }

        fn allow_set_foreground(&self, pid: u32) -> bool {
            self.calls.borrow_mut().push(format!("allow({pid})"));
            true
        }

        fn set_foreground(&self, window: isize) -> bool {
            self.calls.borrow_mut().push(format!("set({window:#x})"));
            if self.grant_works.get() {
                self.foreground.set(Some(CHILD));
                return true;
            }
            false
        }
    }

    /// Nudging on every poll would enumerate the desktop five times a second,
    /// on the thread LogonUI is waiting on, for the seconds before there is
    /// anything to find.
    #[test]
    fn nudges_on_an_interval_rather_than_every_poll() {
        let due: Vec<u32> = (0..NUDGE_INTERVAL_TICKS * 3)
            .filter(|tick| nudge_due(*tick, false))
            .collect();

        assert_eq!(
            due,
            vec![0, NUDGE_INTERVAL_TICKS, NUDGE_INTERVAL_TICKS * 2],
            "should nudge once per interval, starting immediately"
        );
    }

    /// There is nothing to hand the foreground to until the window exists.
    #[test]
    fn does_not_touch_the_foreground_before_a_window_exists() {
        let fake = FakeForeground {
            foreground: Cell::new(Some(LOGONUI)),
            ..FakeForeground::default()
        };
        let mut nudge = ForegroundNudge::new(CHILD);

        for _ in 0..NUDGE_INTERVAL_TICKS * 3 {
            nudge.poll(&fake);
        }

        assert!(
            fake.calls().iter().all(|c| c.starts_with("find(")),
            "expected only window lookups, got {:?}",
            fake.calls()
        );
    }

    /// The grant issued at spawn is single-shot and spent long before the
    /// window turns up, so every push re-arms it first.
    #[test]
    fn re_arms_the_grant_before_every_push() {
        let fake = FakeForeground::with_window_up();
        fake.grant_works.set(false);
        let mut nudge = ForegroundNudge::new(CHILD);

        nudge.poll(&fake);

        assert_eq!(
            fake.calls(),
            vec![
                format!("find({CHILD})"),
                format!("allow({CHILD})"),
                format!("set({CHILD_WINDOW:#x})"),
            ]
        );
    }

    /// Stopping on a successful call rather than on the child holding the
    /// foreground would miss LogonUI taking it straight back again.
    #[test]
    fn keeps_pushing_until_the_child_holds_the_foreground() {
        let fake = FakeForeground::with_window_up();
        fake.grant_works.set(false);
        let mut nudge = ForegroundNudge::new(CHILD);

        for _ in 0..NUDGE_INTERVAL_TICKS * 2 {
            nudge.poll(&fake);
        }
        assert_eq!(
            fake.calls()
                .iter()
                .filter(|c| c.starts_with("set("))
                .count(),
            2,
            "a refused push should be retried"
        );

        fake.grant_works.set(true);
        for _ in 0..NUDGE_INTERVAL_TICKS * 2 {
            nudge.poll(&fake);
        }
        let pushes = fake
            .calls()
            .iter()
            .filter(|c| c.starts_with("set("))
            .count();

        assert!(nudge.settled, "should stop once the child is foreground");
        assert_eq!(pushes, 3, "no further pushes after the child has focus");
    }

    #[test]
    fn gives_up_after_the_budget() {
        let fake = FakeForeground::with_window_up();
        fake.grant_works.set(false);
        let mut nudge = ForegroundNudge::new(CHILD);

        for _ in 0..NUDGE_BUDGET_TICKS * 2 {
            nudge.poll(&fake);
        }

        assert_eq!(
            fake.calls()
                .iter()
                .filter(|c| c.starts_with("set("))
                .count() as u32,
            NUDGE_BUDGET_TICKS / NUDGE_INTERVAL_TICKS,
        );
    }

    /// Exercises the real inherited-pipe / `CreateProcessW` machinery against
    /// a throwaway target, without needing an interactive token, elevation,
    /// or anything listening on the `ak-sysd` pipe. A failure here means
    /// `Connect` can never launch the sign-in window, which otherwise only
    /// surfaces as one generic "Sign-in failed" string.
    #[test]
    fn credui_spawn_succeeds_without_an_interactive_token() {
        let pipes = create_std_pipes().expect("create std pipes");

        // Any real executable will do: this asserts the process is created,
        // not what it does. It exits immediately on the unknown arguments.
        let exe = std::path::PathBuf::from(
            std::env::var("COMSPEC").unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".to_string()),
        );

        let spawned = spawn_cef_host(&exe, &pipes, CPUS_CREDUI, "https://example.com", "token");

        unsafe {
            let _ = CloseHandle(pipes.child_stdin);
            let _ = CloseHandle(pipes.child_stdout);
            let _ = CloseHandle(pipes.result_read);
            let _ = CloseHandle(pipes.cancel_write);
        }

        match spawned {
            Ok(pi) => unsafe {
                let _ = TerminateProcess(pi.hProcess, 0);
                let _ = CloseHandle(pi.hProcess);
                let _ = CloseHandle(pi.hThread);
            },
            Err(e) => panic!("spawn_cef_host under CPUS_CREDUI failed: {e}"),
        }
    }

    #[test]
    fn only_credui_may_fall_back_to_the_current_session() {
        assert!(may_launch_in_current_session(CPUS_CREDUI));
        for cpus in [CPUS_LOGON, CPUS_UNLOCK_WORKSTATION, CPUS_CHANGE_PASSWORD] {
            assert!(
                !may_launch_in_current_session(cpus),
                "{cpus:?} must require a real interactive-session token"
            );
        }
    }
}
