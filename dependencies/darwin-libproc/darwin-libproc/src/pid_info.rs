use std::io;
use std::mem;

fn pid_info<T>(
    pid: libc::pid_t,
    flavor: libc::c_int,
    arg: u64,
) -> io::Result<T> {
    let mut info = mem::MaybeUninit::<T>::uninit();
    let size = mem::size_of::<T>() as libc::c_int;

    let result = unsafe {
        darwin_libproc_sys::proc_pidinfo(
            pid,
            flavor,
            arg,
            info.as_mut_ptr() as *mut libc::c_void,
            size,
        )
    };

    match result {
        value if value <= 0 => Err(io::Error::last_os_error()),
        value if value != size => Err(io::Error::new(
            io::ErrorKind::Other,
            "invalid value returned",
        )),
        _ => unsafe { Ok(info.assume_init()) },
    }
}

/// Returns filled `proc_taskinfo` struct for `pid` given.
pub fn task_info(
    pid: libc::pid_t,
) -> io::Result<darwin_libproc_sys::proc_taskinfo> {
    pid_info(pid, darwin_libproc_sys::PROC_PIDTASKINFO as libc::c_int, 0)
}

/// Returns filled `proc_taskallinfo` struct for `pid` given.
pub fn task_all_info(
    pid: libc::pid_t,
) -> io::Result<darwin_libproc_sys::proc_taskallinfo> {
    pid_info(
        pid,
        darwin_libproc_sys::PROC_PIDTASKALLINFO as libc::c_int,
        0,
    )
}

/// Returns filled `proc_vnodepathinfo` struct for pid given.
pub fn vnode_path_info(
    pid: libc::pid_t,
) -> io::Result<darwin_libproc_sys::proc_vnodepathinfo> {
    pid_info(
        pid,
        darwin_libproc_sys::PROC_PIDVNODEPATHINFO as libc::c_int,
        0,
    )
}
