use std::ffi::OsStr;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::slice;

use super::vnode_path_info;

/// Fetch current working directory for process with `pid` provided.
pub fn pid_cwd(pid: libc::pid_t) -> io::Result<PathBuf> {
    let vnode_path = vnode_path_info(pid)?;
    let raw_path = unsafe {
        slice::from_raw_parts(
            vnode_path.pvi_cdir.vip_path.as_ptr() as *const u8,
            vnode_path.pvi_cdir.vip_path.len(),
        )
    };
    let first_null = memchr::memchr(0x00, &raw_path).unwrap_or(0);

    let os_str = OsStr::from_bytes(&raw_path[..first_null]);

    Ok(PathBuf::from(os_str.to_os_string()))
}
