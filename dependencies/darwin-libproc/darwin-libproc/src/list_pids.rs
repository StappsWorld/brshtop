use std::io;
use std::mem;
use std::ptr;

fn list_pids(r#type: u32, typeinfo: u32) -> io::Result<Vec<libc::pid_t>> {
    let size = unsafe {
        darwin_libproc_sys::proc_listpids(r#type, typeinfo, ptr::null_mut(), 0)
    };
    if size <= 0 {
        return Err(io::Error::last_os_error());
    }

    let capacity = size as usize / mem::size_of::<libc::pid_t>();
    let mut buffer: Vec<libc::pid_t> = Vec::with_capacity(capacity);

    let result = unsafe {
        darwin_libproc_sys::proc_listpids(
            r#type,
            typeinfo,
            buffer.as_mut_ptr() as *mut libc::c_void,
            size,
        )
    };
    if result <= 0 {
        return Err(io::Error::last_os_error());
    }

    let pids_count = result as usize / mem::size_of::<libc::pid_t>();
    unsafe {
        buffer.set_len(pids_count);
    }

    Ok(buffer)
}

/// Fetch pids for all processes running in system.
pub fn all_pids() -> io::Result<Vec<libc::pid_t>> {
    list_pids(darwin_libproc_sys::PROC_ALL_PIDS, 0)
}

/// Fetch pids for processes running in system in a given group.
pub fn pgrp_only_pids(pgrpid: libc::pid_t) -> io::Result<Vec<libc::pid_t>> {
    list_pids(darwin_libproc_sys::PROC_PGRP_ONLY, pgrpid as u32)
}

/// Fetch pids for processes running in system attached to a given TTY.
pub fn tty_only_pids(tty: libc::c_int) -> io::Result<Vec<libc::pid_t>> {
    list_pids(darwin_libproc_sys::PROC_TTY_ONLY, tty as u32)
}

/// Fetch pids for processes running in system with the given UID.
pub fn uid_only_pids(uid: libc::uid_t) -> io::Result<Vec<libc::pid_t>> {
    list_pids(darwin_libproc_sys::PROC_UID_ONLY, uid)
}

/// Fetch pids for processes running in system with the given RUID.
pub fn ruid_only_pids(ruid: libc::uid_t) -> io::Result<Vec<libc::pid_t>> {
    list_pids(darwin_libproc_sys::PROC_RUID_ONLY, ruid)
}

/// Fetch pids for processes running in system with the given PPID.
pub fn ppid_only_pids(ppid: libc::pid_t) -> io::Result<Vec<libc::pid_t>> {
    list_pids(darwin_libproc_sys::PROC_PPID_ONLY, ppid as u32)
}
