//! Spawns `ak_browser.exe` in the interactive session and exchanges
//! `wire`-framed messages with it over its inherited standard handles.
//!
//! Anonymous, inherited pipes rather than named ones with a custom DACL — as
//! GCPW does too (`CreatePipeForChildProcess`, `gcp_utils.cc`) — because the
//! child then opens nothing by name, leaving no DACL for a hardened box's
//! Object Manager namespace to disagree with. See `BROWSER_PRIVILEGE.md`.

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
use ak_ee_wcp_wire::{HostCommand, HostReport};

/// Outcome `credprovider` hands back from the sign-in flow. Never sent over
/// the wire: it is built from a [`HostReport`] plus, for `Redirected`, an
/// `ak-sysd` validation call only `credprovider` can reach — so, unlike
/// `HostCommand`/`HostReport`, this stays local rather than living in `wire`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthResult {
    Completed { username: String },
    Cancelled,
    Failed { reason: String },
}

/// Spawns `ak_browser.exe` and waits for its result. `login_hint` is the
/// selected tile's username, prefilled into the sign-in page.
/// `should_continue` is polled while waiting, so the user backing out of the
/// tile tears the browser down instead of orphaning it.
pub trait AuthFlow {
    fn run(
        &self,
        login_hint: Option<&str>,
        should_continue: &mut dyn FnMut() -> bool,
    ) -> AuthResult;

    /// Start the browser host early, before there is anything for it to load.
    ///
    /// Most of the delay between submit and a visible window is WebView2
    /// building an environment, which needs no URL, so tile selection is enough
    /// to start it. No sign-in begins here — `ak-sysd` is not called and no
    /// session exists until [`AuthFlow::run`].
    fn preload(&self) {}

    /// Tear down anything [`AuthFlow::preload`] started, so backing out of the
    /// tile does not leave a browser running on the logon screen.
    fn discard(&self) {}
}

/// A browser host started ahead of a sign-in and waiting for a `StartSignIn`.
///
/// Handles are raw integers so this stays `Send`/`Sync` inside the COM object
/// that owns it, and are wrapped back up at the point of use — the same trick
/// `ForegroundControl` plays with `HWND`.
struct Preloaded {
    process: usize,
    thread: usize,
    pid: u32,
    result_read: usize,
    cancel_write: usize,
}

impl Preloaded {
    /// A preloaded process can die while waiting — a missing WebView2 runtime
    /// is enough — and reusing its pipes hangs the sign-in until the flow times
    /// out.
    fn alive(&self) -> bool {
        unsafe { WaitForSingleObject(HANDLE(self.process as *mut c_void), 0) != WAIT_OBJECT_0 }
    }

    /// Asks the host to close, then releases everything. Best-effort: every
    /// caller is on a path with nothing useful to do about a failure.
    fn shut_down(self) {
        let cancel_write = HANDLE(self.cancel_write as *mut c_void);
        send_command(cancel_write, &HostCommand::Cancel);
        unsafe {
            let _ = CloseHandle(cancel_write);
            let process = HANDLE(self.process as *mut c_void);
            let _ = WaitForSingleObject(process, 5_000);
            let _ = CloseHandle(process);
            let _ = CloseHandle(HANDLE(self.thread as *mut c_void));
            let _ = CloseHandle(HANDLE(self.result_read as *mut c_void));
        }
    }
}

/// Tracks a preload that may still be running when the flow has moved on.
///
/// Preloading runs on its own thread, so it can finish after the tile was
/// deselected or after submit already started a host the normal way. Filing a
/// result then would leave a browser nobody looks at running on the logon
/// screen, so every path that stops wanting the current preload bumps
/// `generation` and the thread hands its result back instead.
///
/// Generic only so the state machine can be tested without spawning processes.
struct PreloadSlot<T> {
    generation: u64,
    loading: bool,
    ready: Option<T>,
}

impl<T> PreloadSlot<T> {
    fn new() -> Self {
        Self {
            generation: 0,
            loading: false,
            ready: None,
        }
    }

    /// `None` when a preload is already running or already done.
    fn begin(&mut self) -> Option<u64> {
        if self.loading || self.ready.is_some() {
            return None;
        }
        self.loading = true;
        Some(self.generation)
    }

    /// Files a finished preload, or hands it back for the caller to shut down
    /// when it is no longer wanted.
    fn finish(&mut self, generation: u64, value: T) -> Option<T> {
        if generation != self.generation {
            return Some(value);
        }
        self.loading = false;
        self.ready = Some(value);
        None
    }

    /// Gives up on a preload that failed, without disturbing a newer one.
    fn abandon(&mut self, generation: u64) {
        if generation == self.generation {
            self.loading = false;
        }
    }

    /// Takes whatever is ready, and makes anything still in flight unwanted.
    fn take(&mut self) -> Option<T> {
        self.generation += 1;
        self.loading = false;
        self.ready.take()
    }
}

pub struct BrowserAuthFlow {
    browser_exe: std::path::PathBuf,
    cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
    slot: std::sync::Arc<std::sync::Mutex<PreloadSlot<Preloaded>>>,
}

impl BrowserAuthFlow {
    pub fn new(browser_exe: std::path::PathBuf, cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO) -> Self {
        Self {
            browser_exe,
            cpus,
            slot: std::sync::Arc::new(std::sync::Mutex::new(PreloadSlot::new())),
        }
    }

    fn take_preloaded(&self) -> Option<Preloaded> {
        self.slot.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
}

impl AuthFlow for BrowserAuthFlow {
    fn run(
        &self,
        login_hint: Option<&str>,
        should_continue: &mut dyn FnMut() -> bool,
    ) -> AuthResult {
        run_browser_host(
            &self.browser_exe,
            self.cpus,
            self.take_preloaded(),
            login_hint,
            should_continue,
        )
    }

    fn preload(&self) {
        // The window would open on the remote session's desktop, where the
        // flow cannot complete: several seconds of a process nobody will see,
        // on a machine that may have a console session using this properly.
        if syscalls::is_remote_session() {
            log::info!("tile selected in a remote session; not preloading the sign-in window");
            return;
        }

        let (generation, dead) = {
            let mut slot = self.slot.lock().unwrap_or_else(|e| e.into_inner());
            // Worse than none: it reads as ready and its pipes never answer.
            let dead = match &slot.ready {
                Some(preloaded) if !preloaded.alive() => slot.ready.take(),
                _ => None,
            };
            (slot.begin(), dead)
        };
        if let Some(dead) = dead {
            dead.shut_down();
        }
        let Some(generation) = generation else {
            return;
        };

        // On its own thread: `SetSelected` runs on the LogonUI thread that
        // draws the tile, and a logon, a password rotation, two ACL grants and
        // a process creation stalled the very click this keeps smooth.
        let slot = self.slot.clone();
        let browser_exe = self.browser_exe.clone();
        let cpus = self.cpus;
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let host = preload_browser_host(&browser_exe, cpus);
            let unwanted = {
                let mut slot = slot.lock().unwrap_or_else(|e| e.into_inner());
                match host {
                    Some(host) => slot.finish(generation, host),
                    None => {
                        slot.abandon(generation);
                        None
                    }
                }
            };
            match unwanted {
                Some(host) => {
                    log::info!(
                        "the preloaded sign-in window was ready after {}ms but \
                         no longer wanted; shutting it down",
                        started.elapsed().as_millis()
                    );
                    host.shut_down();
                }
                None => log::info!(
                    "preloaded the sign-in window in {}ms",
                    started.elapsed().as_millis()
                ),
            }
        });
    }

    fn discard(&self) {
        if let Some(preloaded) = self.take_preloaded() {
            log::info!("tile deselected; shutting the preloaded sign-in window down");
            preloaded.shut_down();
        }
    }
}

impl Drop for BrowserAuthFlow {
    fn drop(&mut self) {
        self.discard();
    }
}

/// Writes one command to the host's control pipe, leaving the handle owned by
/// the caller.
fn send_command(cancel_write: HANDLE, command: &HostCommand) {
    let mut pipe = unsafe { File::from_raw_handle(cancel_write.0) };
    if let Err(e) = ak_ee_wcp_wire::write_host_command(&mut pipe, command) {
        log::warn!("could not send {command:?} to the sign-in window: {e}");
    }
    std::mem::forget(pipe);
}

/// Only `CPUS_CREDUI` may fall back to the caller's own session: it is
/// debug-gated and runs on an ordinary desktop. On a logon scenario this
/// fallback would put the browser on the secure desktop holding this process's
/// own SYSTEM token.
fn may_launch_in_current_session(cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO) -> bool {
    cpus == CPUS_CREDUI
}

/// The desktop LogonUI draws on. A window on any other desktop of the same
/// window station works but is invisible, and a NULL `lpDesktop` gives the
/// child whichever desktop the caller happens to be on — only incidentally the
/// right one. `CPUS_CREDUI` keeps it NULL and inherits the interactive
/// desktop.
const SECURE_DESKTOP: &str = r"WinSta0\Winlogon";

struct StdPipes {
    /// This process's own ends, read/written after the child is spawned.
    result_read: HANDLE,
    cancel_write: HANDLE,
    /// The child's ends, handed off via `STARTUPINFOW` and closed here once it
    /// has its own inherited copies.
    child_stdout: HANDLE,
    child_stdin: HANDLE,
}

/// One anonymous pipe, both ends inheritable — `CreatePipe` cannot mark just
/// one. The caller must `keep_private` the end it keeps, or that copy leaks
/// into every future child this process spawns.
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

fn close_all(handles: &[HANDLE]) {
    for &h in handles {
        unsafe {
            let _ = CloseHandle(h);
        }
    }
}

/// Two anonymous pipes: the child reads a cancel signal from its inherited
/// stdin and writes its result to its inherited stdout. An inherited handle is
/// a duplicate of one this process already opened and validated, so the child's
/// own low-privilege token is never consulted — unlike a named pipe it would
/// have to open by path.
fn create_std_pipes() -> windows::core::Result<StdPipes> {
    let (child_stdin, cancel_write) = create_inherited_pipe()?;
    if let Err(e) = keep_private(cancel_write) {
        close_all(&[child_stdin, cancel_write]);
        return Err(e);
    }

    let (result_read, child_stdout) = match create_inherited_pipe() {
        Ok(p) => p,
        Err(e) => {
            close_all(&[child_stdin, cancel_write]);
            return Err(e);
        }
    };
    if let Err(e) = keep_private(result_read) {
        close_all(&[child_stdin, cancel_write, result_read, child_stdout]);
        return Err(e);
    }

    Ok(StdPipes {
        result_read,
        cancel_write,
        child_stdout,
        child_stdin,
    })
}

/// Spawns a host with no sign-in attached, for [`AuthFlow::preload`].
///
/// Quiet on failure: nothing is at stake yet, and a sign-in that finds no
/// preloaded host just spawns one the slow way.
fn preload_browser_host(
    browser_exe: &Path,
    cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
) -> Option<Preloaded> {
    let pipes = match create_std_pipes() {
        Ok(p) => p,
        Err(e) => {
            log::warn!("could not create IPC pipes to preload the sign-in window: {e}");
            return None;
        }
    };

    let spawn = spawn_browser_host(browser_exe, &pipes, cpus, None);
    unsafe {
        let _ = CloseHandle(pipes.child_stdin);
        let _ = CloseHandle(pipes.child_stdout);
    }
    match spawn {
        Ok(process) => {
            log::info!(
                "preloaded the sign-in window as pid {} while the tile is selected",
                process.dwProcessId
            );
            Some(Preloaded {
                process: process.hProcess.0 as usize,
                thread: process.hThread.0 as usize,
                pid: process.dwProcessId,
                result_read: pipes.result_read.0 as usize,
                cancel_write: pipes.cancel_write.0 as usize,
            })
        }
        Err(e) => {
            log::warn!("could not preload the sign-in window: {e}");
            unsafe {
                let _ = CloseHandle(pipes.result_read);
                let _ = CloseHandle(pipes.cancel_write);
            }
            None
        }
    }
}

/// The pipes and process a sign-in runs against, however it was started.
struct RunningHost {
    process: HANDLE,
    thread: HANDLE,
    pid: u32,
    result_read: HANDLE,
    cancel_write: HANDLE,
}

fn run_browser_host(
    browser_exe: &Path,
    cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
    preloaded: Option<Preloaded>,
    login_hint: Option<&str>,
    should_continue: &mut dyn FnMut() -> bool,
) -> AuthResult {
    // Worse than none: its pipes would never answer.
    let preloaded = preloaded.filter(|p| {
        let alive = p.alive();
        if !alive {
            log::warn!("the preloaded sign-in window is gone; starting one now");
        }
        alive
    });

    // Not by `ak_browser.exe` itself: the service account it runs as cannot
    // reach `ak-sysd`'s pipe (`BROWSER_PRIVILEGE.md`). And now rather than at
    // preload time: selecting a tile must not start an authentication session.
    log::info!(
        "starting interactive auth with login hint {}",
        login_hint.unwrap_or("<none>")
    );
    let start = match sysd::sys_auth_start_async(login_hint) {
        Ok(s) => s,
        Err(e) => {
            log::error!("sys_auth_start_async failed: {e}");
            if let Some(preloaded) = preloaded {
                preloaded.shut_down();
            }
            return AuthResult::Failed {
                reason: e.to_string(),
            };
        }
    };

    let host = match preloaded {
        Some(preloaded) => {
            log::info!(
                "reusing the preloaded sign-in window (pid {})",
                preloaded.pid
            );
            let host = RunningHost {
                process: HANDLE(preloaded.process as *mut c_void),
                thread: HANDLE(preloaded.thread as *mut c_void),
                pid: preloaded.pid,
                result_read: HANDLE(preloaded.result_read as *mut c_void),
                cancel_write: HANDLE(preloaded.cancel_write as *mut c_void),
            };
            send_command(
                host.cancel_write,
                &HostCommand::StartSignIn {
                    url: start.url.clone(),
                    header_token: start.header_token.clone(),
                },
            );
            host
        }
        None => {
            let pipes = match create_std_pipes() {
                Ok(p) => p,
                Err(e) => {
                    log::error!("failed to create IPC pipes: {e}");
                    return AuthResult::Failed {
                        reason: "failed to create IPC pipes".to_string(),
                    };
                }
            };

            let spawn = spawn_browser_host(
                browser_exe,
                &pipes,
                cpus,
                Some((&start.url, &start.header_token)),
            );
            // The spawn duplicated them into the child's handle table, or
            // failed, in which case there is no child to hold them.
            unsafe {
                let _ = CloseHandle(pipes.child_stdin);
                let _ = CloseHandle(pipes.child_stdout);
            }
            match spawn {
                Ok(process) => RunningHost {
                    process: process.hProcess,
                    thread: process.hThread,
                    pid: process.dwProcessId,
                    result_read: pipes.result_read,
                    cancel_write: pipes.cancel_write,
                },
                Err(e) => {
                    log::error!("failed to launch {}: {e}", browser_exe.display());
                    unsafe {
                        let _ = CloseHandle(pipes.result_read);
                        let _ = CloseHandle(pipes.cancel_write);
                    }
                    return AuthResult::Failed {
                        reason: "failed to launch sign-in window".to_string(),
                    };
                }
            }
        }
    };

    let result = wait_for_result(
        host.result_read,
        host.cancel_write,
        host.process,
        host.pid,
        should_continue,
    );

    unsafe {
        let _ = CloseHandle(host.cancel_write);
        let _ = WaitForSingleObject(host.process, 5_000);
        let _ = CloseHandle(host.process);
        let _ = CloseHandle(host.thread);
    }

    result
}

/// `wait_for_result` polls every 200ms, but the browser takes seconds to open a
/// window; this keeps it from enumerating an empty desktop five times a
/// second.
const NUDGE_INTERVAL_TICKS: u32 = 5;

/// Ten seconds of polls: past that the window has either appeared or is not
/// going to, and the sign-in has no deadline this should compete with.
const NUDGE_BUDGET_TICKS: u32 = 50;

fn nudge_due(tick: u32, settled: bool) -> bool {
    !settled && tick < NUDGE_BUDGET_TICKS && tick.is_multiple_of(NUDGE_INTERVAL_TICKS)
}

/// Pushes `ak_browser.exe`'s window to the front from inside LogonUI.
///
/// The child asks for the foreground itself, but a freshly spawned process is
/// rarely allowed to take it, and the single `AllowSetForegroundWindow` from
/// the spawn is spent before it has a window to apply it to. This process can:
/// right desktop, normally the foreground already, and exempt from a foreground
/// lock it took itself.
///
/// Settles on the child *being* the foreground rather than on a call
/// succeeding, so a window that loses it again to a LogonUI repaint gets pushed
/// back rather than counted as done.
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
            // How wide this gap is says whether the grant at spawn had any
            // chance of surviving it.
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

/// What the result-pipe reader thread saw. `Eof` and `Error` both reach the
/// user as a cancellation, but the host dying and the pipe misbehaving want
/// different fixes, so they stay separate long enough to be logged.
enum PipeOutcome {
    Result(AuthResult),
    Eof,
    Error(String),
}

/// Turns the sign-in redirect's URL into a real outcome by validating its token
/// against `ak-sysd`, the one step `ak_browser.exe` cannot do itself
/// (`BROWSER_PRIVILEGE.md`). Runs on the result-pipe reader thread so this
/// blocking round trip does not stall `should_continue` or the nudge.
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
/// asks `ak_browser.exe` to close over the control pipe rather than killing it.
///
/// Every route out other than a real `AuthResult` reaches the user as the same
/// "Login attempt cancelled", so each logs why — including a crash before the
/// child sends anything, which surfaces only as a plain EOF.
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
    // This loop is on the thread LogonUI called `Connect` on, hence on the
    // desktop the browser's window appears on — which `EnumWindows` and
    // `SetForegroundWindow` both need and no other thread here has.
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
                    send_command(cancel_write, &HostCommand::Cancel);
                }
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

/// Exit code of `ak_browser.exe` rendered for a log line — the whole diagnosis
/// when the host dies before it can say anything: `0xC0000005` is a crash,
/// `0xC0000142` a `DLL_INIT_FAILED` from a desktop it cannot reach, `0` a clean
/// exit that simply skipped the result.
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

/// Gets `ak_browser.exe` a token for the dedicated service account rather than
/// SYSTEM (`BROWSER_PRIVILEGE.md`), the same way for logon and unlock.
///
/// The hardening steps are idempotent, so a transient failure costs one retry
/// next time and must not block the token mint. The password is not: without it
/// there is no way to log the account on at all.
fn acquire_service_account_token() -> windows::core::Result<HANDLE> {
    let (sid, password) = syscalls::service_account_password().map_err(|e| {
        log::error!("could not establish the service account's password: {e}");
        windows::core::Error::from(E_FAIL)
    })?;

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

/// Minimal Windows command-line quoting, enough for `CommandLineToArgvW` (what
/// `std::env::args()` on the far end is built on) to see one argument. Neither
/// a URL nor an opaque token legitimately contains the backslash-before-quote
/// sequence the full algorithm exists to handle.
fn quote_arg(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\\\""))
}

/// `sign_in` is `None` when preloading: the host comes up, builds its window
/// and waits for a `StartSignIn` on the control pipe instead.
fn spawn_browser_host(
    browser_exe: &Path,
    pipes: &StdPipes,
    cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
    sign_in: Option<(&str, &str)>,
) -> windows::core::Result<PROCESS_INFORMATION> {
    let cmdline = match sign_in {
        Some((url, header_token)) => format!(
            "\"{}\" --sign-in-url {} --header-token {}",
            browser_exe.display(),
            quote_arg(url),
            quote_arg(header_token),
        ),
        None => format!("\"{}\"", browser_exe.display()),
    };

    let mut si = STARTUPINFOW {
        cb: size_of::<STARTUPINFOW>() as u32,
        dwFlags: STARTF_USESTDHANDLES,
        hStdInput: pipes.child_stdin,
        hStdOutput: pipes.child_stdout,
        ..Default::default()
    };
    // Must outlive every `CreateProcess*` call below: `lpDesktop` borrows it.
    let mut desktop = (!may_launch_in_current_session(cpus)).then(|| {
        SECURE_DESKTOP
            .encode_utf16()
            .chain([0])
            .collect::<Vec<u16>>()
    });
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
        browser_exe.display(),
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
            // that way too until it turned out not to be held at all.
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

    // Necessary but never sufficient: the grant is single-shot and the window
    // it applies to does not exist for several seconds, by which time any
    // foreground change has spent it. `ForegroundNudge` re-arms it. A failure
    // is about this process, which is refused when not itself entitled to the
    // foreground.
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

/// Brokered through the Secondary Logon service, needing only
/// `SE_IMPERSONATE_NAME` where `CreateProcessAsUserW` would need
/// `SE_ASSIGNPRIMARYTOKEN_NAME`/`SE_INCREASE_QUOTA_NAME`, both absent from
/// LogonUI's token. It has no `bInheritHandles` parameter but honors `si`'s
/// inheritable handles regardless (`BROWSER_PRIVILEGE.md`), and
/// `LOGON_WITH_PROFILE` loads the account's hive but not its environment
/// block — hence building one below.
fn spawn_with_token(
    token: HANDLE,
    cmdline: &str,
    si: &STARTUPINFOW,
    pi: &mut PROCESS_INFORMATION,
) -> windows::core::Result<()> {
    let mut cmdline_wide = syscalls::wide(cmdline);

    // Best-effort: on the account's first ever launch its profile does not
    // exist on disk yet — only `LOGON_WITH_PROFILE` below creates it — so this
    // can fail, and the spawn is still worth attempting.
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
    let mut cmdline_wide = syscalls::wide(cmdline);
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
        /// `None` until the browser has a window a person could see.
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

    /// A browser nobody filed is a process left running on the logon screen.
    #[test]
    fn a_preload_that_lands_too_late_is_handed_back() {
        let mut slot: PreloadSlot<u32> = PreloadSlot::new();

        let generation = slot.begin().expect("nothing in flight yet");
        // The person clicked submit before it was ready.
        assert_eq!(slot.take(), None, "nothing was ready to take");

        assert_eq!(
            slot.finish(generation, 7),
            Some(7),
            "the late result should come back for shutting down"
        );
        assert_eq!(slot.take(), None, "and must not have been filed");
    }

    /// Deselecting and reselecting is ordinary at a logon screen.
    #[test]
    fn a_preload_does_not_survive_a_deselect() {
        let mut slot: PreloadSlot<u32> = PreloadSlot::new();

        let first = slot.begin().expect("first preload starts");
        slot.take(); // deselect
        let second = slot.begin().expect("reselect starts another");

        assert_eq!(slot.finish(first, 1), Some(1), "the old one is unwanted");
        assert_eq!(slot.finish(second, 2), None, "the new one is filed");
        assert_eq!(slot.take(), Some(2));
    }

    /// Selecting an already-selected tile must not start a second browser.
    #[test]
    fn only_one_preload_runs_at_a_time() {
        let mut slot: PreloadSlot<u32> = PreloadSlot::new();

        assert!(slot.begin().is_some());
        assert!(slot.begin().is_none(), "one is already in flight");

        let generation = 0;
        assert_eq!(slot.finish(generation, 5), None);
        assert!(slot.begin().is_none(), "one is already ready");
    }

    /// A failed preload has to clear the way for the next attempt, without
    /// disturbing one that started in the meantime.
    #[test]
    fn a_failed_preload_frees_the_slot() {
        let mut slot: PreloadSlot<u32> = PreloadSlot::new();

        let generation = slot.begin().expect("starts");
        slot.abandon(generation);
        assert!(slot.begin().is_some(), "the slot should be free again");

        // Reachable whenever the tile is deselected mid-preload: the late
        // failure must not clear the flag belonging to its replacement.
        let superseded = slot.begin();
        assert!(superseded.is_none(), "still loading from the retry above");
        slot.take();
        let current = slot.begin().expect("a fresh preload after the deselect");
        slot.abandon(generation);
        assert!(
            slot.loading,
            "an older failure must not cancel a newer preload"
        );
        assert_eq!(slot.finish(current, 9), None, "the newer one still files");
    }

    /// Nudging on every poll enumerates the desktop five times a second, on
    /// the thread LogonUI is waiting on, before there is anything to find.
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
    /// window turns up.
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

    /// Stopping on a successful call would miss LogonUI taking the foreground
    /// straight back.
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

    /// Exercises the real inherited-pipe/`CreateProcessW` machinery without an
    /// interactive token, elevation, or anything on the `ak-sysd` pipe. A
    /// failure here means `Connect` can never launch the sign-in window, which
    /// otherwise surfaces only as a generic "Sign-in failed".
    #[test]
    fn credui_spawn_succeeds_without_an_interactive_token() {
        let pipes = create_std_pipes().expect("create std pipes");

        // Any real executable will do: this asserts the process is created,
        // not what it does.
        let exe = std::path::PathBuf::from(
            std::env::var("COMSPEC").unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".to_string()),
        );

        let spawned = spawn_browser_host(
            &exe,
            &pipes,
            CPUS_CREDUI,
            Some(("https://example.com", "token")),
        );

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
            Err(e) => panic!("spawn_browser_host under CPUS_CREDUI failed: {e}"),
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
