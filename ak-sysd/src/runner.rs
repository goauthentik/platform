//! Platform-specific startup wrapper around [`Agent`].
//!
//! This module owns *how* the agent is hosted for the lifetime of the process,
//! keeping that concern out of [`Agent`] (which only knows how to `start`/`stop`):
//!
//! - **Unix** (systemd / launchd, or a foreground dev run): installs an
//!   out-of-runtime force-exit backstop so a *second* SIGINT/SIGTERM always
//!   kills the process even if the tokio runtime is wedged, then shuts down
//!   gracefully on the first SIGINT/SIGTERM.
//! - **Windows**: hosts the agent as a proper Windows service under the Service
//!   Control Manager (SCM) via the `windows-service` crate, reporting status
//!   and shutting down on the SCM `Stop`/`Shutdown` control.

use crate::agent::Agent;
use eyre::Result;

/// Runs the system agent for the current platform, returning once it has shut
/// down cleanly (or after forcing exit).
pub async fn run(config: String) -> Result<()> {
    #[cfg(unix)]
    {
        unix::run(config).await
    }
    #[cfg(windows)]
    {
        windows::run(config).await
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = config;
        eyre::bail!("unsupported platform: no agent runner available");
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    /// Graceful-shutdown budget before we stop waiting on components and force
    /// the process to exit.
    const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);
    /// Exit status used for signal-driven termination (128 + SIGINT).
    const FORCE_EXIT_STATUS: i32 = 130;

    pub async fn run(config: String) -> Result<()> {
        // Arm the force-exit backstop *before* starting components, so a second
        // signal kills the process even if a component wedges the runtime.
        match install_force_exit_backstop() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to setup shutdown signals: {e:?}");
                return Ok(());
            }
        };

        let agent = Agent::new(config).await?;
        agent.start().await?;

        wait_for_signal().await?;
        tracing::info!("shutdown signal received; stopping (send the signal again to force quit)");

        // Bound graceful shutdown so a hung component can't wedge exit forever.
        match tokio::time::timeout(SHUTDOWN_TIMEOUT, agent.stop()).await {
            Ok(res) => res,
            Err(_) => {
                tracing::warn!(
                    "graceful shutdown timed out after {SHUTDOWN_TIMEOUT:?}, forcing exit"
                );
                std::process::exit(FORCE_EXIT_STATUS);
            }
        }
    }

    /// Registers async-signal-safe handlers (via signal-hook) so that a *second*
    /// SIGINT/SIGTERM force-terminates the process immediately. These run on the
    /// signal-delivery thread and do not depend on the async runtime being able
    /// to schedule tasks; they chain with tokio's own handlers (both go through
    /// signal-hook-registry), so [`wait_for_signal`] still sees the first signal.
    fn install_force_exit_backstop() -> Result<()> {
        use signal_hook::consts::{SIGINT, SIGTERM};
        let forced = Arc::new(AtomicBool::new(false));
        for sig in [SIGINT, SIGTERM] {
            // Order matters: the conditional shutdown must be registered before
            // the flag setter so the first delivery only arms the flag and the
            // second delivery triggers the immediate exit.
            signal_hook::flag::register_conditional_shutdown(
                sig,
                FORCE_EXIT_STATUS,
                Arc::clone(&forced),
            )?;
            signal_hook::flag::register(sig, Arc::clone(&forced))?;
        }
        Ok(())
    }

    /// Resolves on the first SIGINT (Ctrl+C) or SIGTERM.
    async fn wait_for_signal() -> Result<()> {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate())?;
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
        Ok(())
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::ffi::OsString;
    use std::sync::OnceLock;
    use std::time::Duration;
    use tokio::runtime::Handle;
    use tokio_util::sync::CancellationToken;
    use windows_service::define_windows_service;
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::service_dispatcher;

    /// Must match the service name registered by the installer
    /// (`vpkg/windows/Package.wxs` → `<ServiceInstall Name="ak_sysd" ...>`).
    const SERVICE_NAME: &str = "ak_sysd";
    const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

    /// The SCM invokes `service_main` on a thread it creates, so the config path
    /// and a handle to the already-running tokio runtime are stashed here for it
    /// to pick up.
    static CONFIG: OnceLock<String> = OnceLock::new();
    static RUNTIME: OnceLock<Handle> = OnceLock::new();

    define_windows_service!(ffi_service_main, service_main);

    pub async fn run(config: String) -> Result<()> {
        CONFIG
            .set(config)
            .map_err(|_| eyre::eyre!("service config already set"))?;
        RUNTIME
            .set(Handle::current())
            .map_err(|_| eyre::eyre!("service runtime handle already set"))?;

        // `StartServiceCtrlDispatcher` blocks until the service stops and invokes
        // `service_main` on a separate SCM-created thread. Run it on a blocking
        // thread so it doesn't starve the async runtime.
        tokio::task::spawn_blocking(|| service_dispatcher::start(SERVICE_NAME, ffi_service_main))
            .await??;
        Ok(())
    }

    fn service_main(_arguments: Vec<OsString>) {
        if let Err(e) = run_service() {
            tracing::error!("windows service exited with error: {e:?}");
        }
    }

    /// Runs on the SCM dispatcher thread (outside the tokio runtime), so all
    /// async work is driven through the captured runtime handle.
    fn run_service() -> Result<()> {
        let rt = RUNTIME
            .get()
            .ok_or_else(|| eyre::eyre!("runtime handle not initialized"))?
            .clone();
        let config = CONFIG
            .get()
            .ok_or_else(|| eyre::eyre!("config not initialized"))?
            .clone();

        // Tripped by the SCM control handler to request shutdown.
        let shutdown = CancellationToken::new();

        let handler_shutdown = shutdown.clone();
        let event_handler = move |control| -> ServiceControlHandlerResult {
            match control {
                ServiceControl::Stop | ServiceControl::Shutdown => {
                    handler_shutdown.cancel();
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        };
        let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

        let report = |state: ServiceState,
                      controls: ServiceControlAccept,
                      wait_hint: Duration|
         -> Result<()> {
            status_handle.set_service_status(ServiceStatus {
                service_type: SERVICE_TYPE,
                current_state: state,
                controls_accepted: controls,
                exit_code: ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint,
                process_id: None,
            })?;
            Ok(())
        };

        report(
            ServiceState::StartPending,
            ServiceControlAccept::empty(),
            Duration::from_secs(10),
        )?;

        let agent = rt.block_on(Agent::new(config))?;
        rt.block_on(agent.start())?;

        report(
            ServiceState::Running,
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            Duration::default(),
        )?;

        // Block until the SCM asks us to stop.
        rt.block_on(shutdown.cancelled());

        report(
            ServiceState::StopPending,
            ServiceControlAccept::empty(),
            Duration::from_secs(15),
        )?;

        if let Err(e) = rt.block_on(agent.stop()) {
            tracing::warn!("agent stop failed: {e:?}");
        }

        report(
            ServiceState::Stopped,
            ServiceControlAccept::empty(),
            Duration::default(),
        )?;
        Ok(())
    }
}
