// --- macOS proc helpers ---
//
// sysinfo silently drops the parent ProcInfo when it can't read the parent's
// exe path (sandboxed / system-owned processes). proc_pidinfo + proc_pidpath
// are lower-level syscalls that don't have that restriction.

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
