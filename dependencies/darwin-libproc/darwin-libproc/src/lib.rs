//! Idiomatic and safe wrappers for `libproc` of macOS.

#![cfg(target_os = "macos")]
#![doc(html_root_url = "https://docs.rs/darwin-libproc/0.2.0")]
#![deny(
    unused,
    unused_imports,
    future_incompatible,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    dead_code,
    deprecated,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_results
)]

mod list_pids;
mod name;
mod pid_cwd;
mod pid_info;
mod pid_path;
mod pid_rusage;
mod version;

// Structs re-export
pub use darwin_libproc_sys::{
    proc_bsdinfo, proc_fdinfo, proc_taskallinfo, proc_taskinfo,
};

// Wrappers
pub use self::list_pids::{
    all_pids, pgrp_only_pids, ppid_only_pids, ruid_only_pids, tty_only_pids,
    uid_only_pids,
};
pub use self::name::name;
pub use self::pid_cwd::pid_cwd;
pub use self::pid_info::{task_all_info, task_info, vnode_path_info};
pub use self::pid_path::pid_path;
pub use self::pid_rusage::*;
pub use self::version::version;
