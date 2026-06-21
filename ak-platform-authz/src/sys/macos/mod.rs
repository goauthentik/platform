use ak_macos_touchid::{AccessRequest, authenticate_with_touchid};
use ak_platform::{net::server::proc_info::{ProcInfo, proc_exe_path, proc_parent_pid}, prelude::BoxError};

use crate::sys::{AuthorizationRequest, macos::{app::{find_app_bundle, populate_from_bundle}}};

pub mod app;

pub async fn lookup_app_info(pc: Option<ProcInfo>) -> Result<AccessRequest, BoxError> {
    let mut ar = AccessRequest::default();

    let Some(pc) = pc else {
        tracing::debug!("No proc info given");
        return Ok(ar);
    };

    // Walk the pre-fetched chain (pc → pc.parent); both are already in memory.
    let mut current = Some(&pc);
    while let Some(proc) = current {
        tracing::trace!(pid = proc.pid, cmd = proc.cmdline, "Checking process");
        if let Some(bundle) = find_app_bundle(&proc.exe) {
            tracing::trace!(bundle = %bundle.display(), "Found app bundle");
            populate_from_bundle(&mut ar, &bundle);
            return Ok(ar);
        }
        current = proc.parent.as_deref();
    }

    // Continue walking up the process tree beyond the one parent level stored
    // in ProcInfo. proc_parent_pid/proc_exe_path are kept decoupled here so
    // that a process with an unreadable exe (sandboxed, system-owned) doesn't
    // stop the walk — we can still get its parent PID and keep climbing.
    let mut walk_pid = pc.parent.as_ref().map(|p| p.pid);
    while let Some(pid) = walk_pid {
        let Some(ppid) = proc_parent_pid(pid) else { break };

        if let Some(exe) = proc_exe_path(ppid) {
            tracing::trace!(pid = ppid, exe = %exe.display(), "Checking process (tree walk)");
            if let Some(bundle) = find_app_bundle(&exe) {
                tracing::trace!(bundle = %bundle.display(), "Found app bundle");
                populate_from_bundle(&mut ar, &bundle);
                return Ok(ar);
            }
        }

        walk_pid = Some(ppid);
    }

    // No bundle found anywhere in the tree — use the direct process exe name
    if let Some(name) = pc.exe.file_name().and_then(|n| n.to_str()) {
        tracing::trace!("No bundle found, using exe name");
        ar.requesting_app = name.to_string();
    }

    Ok(ar)
}

pub async fn prompt(req: AuthorizationRequest) -> Result<bool, BoxError> {
    let res = authenticate_with_touchid(AccessRequest {
        title: "authentik Access Request".to_string(),
        reason: req.msg.for_current(),
        ..lookup_app_info(req.proc_info).await?
    });
    Result::<bool, BoxError>::Ok(res)
}
