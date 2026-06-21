use std::{fmt, path::PathBuf};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::prelude::BoxError;

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub pid: u32,
    pub exe: PathBuf,
    pub cmdline: String,
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

// On macOS, proc_pidinfo/proc_pidpath are lower-level syscalls that work for
// sandboxed and system-owned processes where sysinfo silently fails to read
// the exe path.
#[cfg(target_os = "macos")]
mod sys {
    use std::path::PathBuf;

    unsafe extern "C" {
        fn proc_pidinfo(
            pid: libc::c_int,
            flavor: libc::c_int,
            arg: u64,
            buffer: *mut libc::c_void,
            buffersize: libc::c_int,
        ) -> libc::c_int;

        fn proc_pidpath(
            pid: libc::c_int,
            buffer: *mut libc::c_void,
            buffersize: u32,
        ) -> libc::c_int;
    }

    // PROC_PIDT_SHORTBSDINFO = 13 from <sys/proc_info.h>.
    // Struct is 64 bytes; pbsi_pid is at offset 0, pbsi_ppid at offset 4.
    // _rest pads to the full 64 bytes so proc_pidinfo doesn't reject the buffer.
    const PROC_PIDT_SHORTBSDINFO: libc::c_int = 13;

    #[repr(C)]
    struct ProcBsdShortInfo {
        pbsi_pid: u32,
        pbsi_ppid: u32,
        _rest: [u8; 56],
    }

    pub fn proc_parent_pid(pid: u32) -> Option<u32> {
        let mut info = std::mem::MaybeUninit::<ProcBsdShortInfo>::zeroed();
        let ret = unsafe {
            proc_pidinfo(
                pid as libc::c_int,
                PROC_PIDT_SHORTBSDINFO,
                0,
                info.as_mut_ptr() as *mut libc::c_void,
                std::mem::size_of::<ProcBsdShortInfo>() as libc::c_int,
            )
        };
        if ret <= 0 {
            return None;
        }
        let ppid = unsafe { (*info.as_ptr()).pbsi_ppid };
        if ppid > 1 { Some(ppid) } else { None }
    }

    pub fn proc_exe_path(pid: u32) -> Option<PathBuf> {
        let mut buf = vec![0u8; 4096];
        let ret = unsafe {
            proc_pidpath(
                pid as libc::c_int,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len() as u32,
            )
        };
        if ret <= 0 {
            return None;
        }
        let s = std::str::from_utf8(&buf[..ret as usize])
            .ok()?
            .trim_end_matches('\0');
        Some(PathBuf::from(s))
    }
}

#[cfg(target_os = "macos")]
pub use sys::{proc_exe_path, proc_parent_pid};

impl ProcInfo {
    // On macOS: sysinfo for process detection and cmdline; proc_exe_path /
    // proc_parent_pid for exe paths, which work for sandboxed and system-owned
    // processes where sysinfo silently returns None for the exe.
    #[cfg(target_os = "macos")]
    pub fn from_pid(pid: u32) -> Result<Self, ProcInfoError> {
        let sysinfo_pid = Pid::from_u32(pid);

        // Pass 1: verify the process exists and record its parent PID.
        let parent_pid_opt = {
            let mut sys = System::new();
            let kind = ProcessRefreshKind::nothing().with_cmd(UpdateKind::OnlyIfNotSet);
            sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[sysinfo_pid]), false, kind);
            sys.process(sysinfo_pid)
                .ok_or(ProcInfoError::ProcessNotFound(pid))?
                .parent()
        };

        // Pass 2: fetch target + parent together for their cmdlines.
        let mut pids = vec![sysinfo_pid];
        if let Some(ppid) = parent_pid_opt {
            pids.push(ppid);
        }
        let mut sys = System::new();
        let kind = ProcessRefreshKind::nothing().with_cmd(UpdateKind::OnlyIfNotSet);
        sys.refresh_processes_specifics(ProcessesToUpdate::Some(&pids), false, kind);

        let cmdline = sys
            .process(sysinfo_pid)
            .map(|p| {
                p.cmd()
                    .iter()
                    .map(|s| s.to_str().unwrap_or(""))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();

        let exe = sys::proc_exe_path(pid).ok_or(ProcInfoError::ExeNotAvailable(pid))?;

        let parent = parent_pid_opt.and_then(|ppid| {
            let ppid_u32 = ppid.as_u32();
            let exe = sys::proc_exe_path(ppid_u32)?;
            let cmdline = sys
                .process(ppid)
                .map(|p| {
                    p.cmd()
                        .iter()
                        .map(|s| s.to_str().unwrap_or(""))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            Some(Box::new(ProcInfo {
                pid: ppid_u32,
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

    #[cfg(not(target_os = "macos"))]
    pub fn from_pid(pid: u32) -> Result<Self, ProcInfoError> {
        let sysinfo_pid = Pid::from_u32(pid);
        let parent_pid_opt = {
            // First pass: fetch the target process and record its parent PID.
            let mut sys = System::new();
            let kind = ProcessRefreshKind::nothing()
                .with_exe(UpdateKind::OnlyIfNotSet)
                .with_cmd(UpdateKind::OnlyIfNotSet);
            sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[sysinfo_pid]), false, kind);
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
        let cmdline = process
            .cmd()
            .to_vec()
            .iter()
            .map(|f| f.to_str().unwrap_or(""))
            .collect::<Vec<&str>>()
            .join(" ");

        let parent = parent_pid_opt.and_then(|ppid| {
            let p = sys.process(ppid)?;
            let exe = p.exe()?.to_path_buf();
            let cmdline = p
                .cmd()
                .to_vec()
                .iter()
                .map(|f| f.to_str().unwrap_or(""))
                .collect::<Vec<&str>>()
                .join(" ");
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

    pub fn parent_cmdline(&self) -> Result<String, BoxError> {
        let p = match &self.parent {
            Some(p) => p,
            None => return Err("Process has no parent process".into()),
        };
        Ok(p.cmdline.clone())
    }

    pub fn unique_process_id(&self) -> Result<String, BoxError> {
        let p = self.parent.clone().ok_or("failed to get parent process")?;
        let first_exe = p
            .cmdline
            .split(" ")
            .next()
            .ok_or("failed to get first exe")?;
        Ok(format!("{}:{}", p.exe.to_string_lossy(), first_exe))
    }
}

impl fmt::Display for ProcInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Process <id={}, exe={}, cmdline={}>",
            self.pid,
            self.exe.display(),
            self.cmdline,
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
        // PID 0 is the System Idle Process on Windows, so use u32::MAX which is
        // never a valid PID on any OS.
        let err = ProcInfo::from_pid(u32::MAX).unwrap_err();
        assert!(matches!(err, ProcInfoError::ProcessNotFound(u32::MAX)));
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
