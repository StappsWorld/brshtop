use std::io;

/// Returns the `libproc` version as a tuple of `(major, minor)` parts.
pub fn version() -> io::Result<(libc::c_int, libc::c_int)> {
    let mut major = 0;
    let mut minor = 0;

    let result =
        unsafe { darwin_libproc_sys::proc_libversion(&mut major, &mut minor) };

    if result != 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok((major, minor))
    }
}
