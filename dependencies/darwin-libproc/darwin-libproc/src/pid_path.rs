use std::ffi::OsString;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

/// Fetch executable absolute path for process with `pid` provided.
pub fn pid_path(pid: libc::pid_t) -> io::Result<PathBuf> {
    let mut buf: Vec<u8> = Vec::with_capacity(
        darwin_libproc_sys::PROC_PIDPATHINFO_MAXSIZE as usize,
    );

    let result = unsafe {
        darwin_libproc_sys::proc_pidpath(
            pid,
            buf.as_mut_ptr() as *mut libc::c_void,
            buf.capacity() as u32,
        )
    };

    if result <= 0 {
        Err(io::Error::last_os_error())
    } else {
        unsafe {
            buf.set_len(result as usize);
        }

        Ok(OsString::from_vec(buf).into())
    }
}
