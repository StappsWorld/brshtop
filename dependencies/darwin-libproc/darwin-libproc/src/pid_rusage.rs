use std::io;
use std::mem;

pub use darwin_libproc_sys::{
    rusage_info_t, rusage_info_v0, rusage_info_v1, rusage_info_v2,
    rusage_info_v3, rusage_info_v4,
};

mod private {
    pub trait Sealed {}
    impl Sealed for darwin_libproc_sys::rusage_info_v0 {}
    impl Sealed for darwin_libproc_sys::rusage_info_v1 {}
    impl Sealed for darwin_libproc_sys::rusage_info_v2 {}
    impl Sealed for darwin_libproc_sys::rusage_info_v3 {}
    impl Sealed for darwin_libproc_sys::rusage_info_v4 {}
}

/// `proc_pid_rusage` can return different versioned `rusage_info_v*` structs.
///
/// This sealed trait implemented for all possible variants
/// and used by [`pid_rusage`]
pub trait RusageFlavor: private::Sealed {
    #[doc(hidden)]
    fn flavor() -> u32;
}

impl RusageFlavor for rusage_info_v0 {
    fn flavor() -> u32 {
        darwin_libproc_sys::RUSAGE_INFO_V0
    }
}

impl RusageFlavor for rusage_info_v1 {
    fn flavor() -> u32 {
        darwin_libproc_sys::RUSAGE_INFO_V1
    }
}

impl RusageFlavor for rusage_info_v2 {
    fn flavor() -> u32 {
        darwin_libproc_sys::RUSAGE_INFO_V2
    }
}

impl RusageFlavor for rusage_info_v3 {
    fn flavor() -> u32 {
        darwin_libproc_sys::RUSAGE_INFO_V3
    }
}

impl RusageFlavor for rusage_info_v4 {
    fn flavor() -> u32 {
        darwin_libproc_sys::RUSAGE_INFO_V4
    }
}

/// Return resource usage information for the given pid, which can be a live process or a zombie.
pub fn pid_rusage<T: RusageFlavor>(pid: libc::pid_t) -> io::Result<T> {
    let mut rusage = mem::MaybeUninit::<T>::uninit();
    let result = unsafe {
        darwin_libproc_sys::proc_pid_rusage(
            pid,
            T::flavor() as i32,
            rusage.as_mut_ptr() as *mut _,
        )
    };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { rusage.assume_init() })
    }
}
