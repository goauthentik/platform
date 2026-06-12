use std::{ffi::OsString, fmt, path::PathBuf};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub pid: u32,
    pub exe: PathBuf,
    pub cmdline: Vec<OsString>,
    pub parent: Option<Box<ProcInfo>>,
}

#[derive(Debug)]
pub enum ProcInfoError {
    ProcessNotFound(u32),
    ExeNotAvailable(u32),
}

impl fmt::Display for ProcInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcInfoError::ProcessNotFound(pid) => write!(f, "process {pid} not found"),
            ProcInfoError::ExeNotAvailable(pid) => {
                write!(f, "exe path not available for process {pid}")
            }
        }
    }
}

impl std::error::Error for ProcInfoError {}

impl ProcInfo {
    pub fn from_pid(pid: u32) -> Result<Self, ProcInfoError> {
        let sysinfo_pid = Pid::from_u32(pid);
        let parent_pid_opt = {
            // First pass: fetch the target process and record its parent PID.
            let mut sys = System::new();
            let kind = ProcessRefreshKind::nothing()
                .with_exe(UpdateKind::OnlyIfNotSet)
                .with_cmd(UpdateKind::OnlyIfNotSet);
            sys.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[sysinfo_pid]),
                false,
                kind,
            );
            let process = sys
                .process(sysinfo_pid)
                .ok_or(ProcInfoError::ProcessNotFound(pid))?;
            process.parent()
        };

        // Second pass: fetch both PIDs together so we pay one OS scan.
        let mut pids = vec![sysinfo_pid];
        if let Some(ppid) = parent_pid_opt {
            pids.push(ppid);
        }
        let mut sys = System::new();
        let kind = ProcessRefreshKind::nothing()
            .with_exe(UpdateKind::OnlyIfNotSet)
            .with_cmd(UpdateKind::OnlyIfNotSet);
        sys.refresh_processes_specifics(ProcessesToUpdate::Some(&pids), false, kind);

        let process = sys
            .process(sysinfo_pid)
            .ok_or(ProcInfoError::ProcessNotFound(pid))?;
        let exe = process
            .exe()
            .ok_or(ProcInfoError::ExeNotAvailable(pid))?
            .to_path_buf();
        let cmdline = process.cmd().to_vec();

        let parent = parent_pid_opt.and_then(|ppid| {
            let p = sys.process(ppid)?;
            let exe = p.exe()?.to_path_buf();
            let cmdline = p.cmd().to_vec();
            Some(Box::new(ProcInfo {
                pid: ppid.as_u32(),
                exe,
                cmdline,
                parent: None,
            }))
        });

        Ok(Self {
            pid,
            exe,
            cmdline,
            parent,
        })
    }
}

impl fmt::Display for ProcInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cmdline = self
            .cmdline
            .iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        write!(
            f,
            "Process <id={}, exe={}, cmdline={}>",
            self.pid,
            self.exe.display(),
            cmdline,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_current_pid() {
        let pid = std::process::id();
        let info = ProcInfo::from_pid(pid).expect("should find current process");
        assert_eq!(info.pid, pid);
        assert!(info.exe.exists(), "exe path should exist: {:?}", info.exe);
        assert!(!info.cmdline.is_empty(), "cmdline should not be empty");
        let parent = info.parent.expect("current process should have a parent");
        assert!(parent.pid > 0, "parent pid should be non-zero");
        assert!(parent.parent.is_none(), "parent should not recurse further");
    }

    #[test]
    fn not_found_error() {
        // PID 0 is never a valid user-space process
        let err = ProcInfo::from_pid(0).unwrap_err();
        assert!(matches!(err, ProcInfoError::ProcessNotFound(0)));
    }

    #[test]
    fn display_format() {
        let pid = std::process::id();
        let info = ProcInfo::from_pid(pid).unwrap();
        let s = info.to_string();
        assert!(s.starts_with("Process <id="), "unexpected format: {s}");
        assert!(s.contains("exe="), "missing exe: {s}");
        assert!(s.contains("cmdline="), "missing cmdline: {s}");
    }
}
