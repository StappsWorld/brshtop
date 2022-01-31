use std::convert::TryFrom;
use std::io;
use std::mem;
use std::ptr;

use mach::{boolean, vm_types};

use heim_common::prelude::Error;

use crate::{Pid, ProcessError, Status};

// Process status values, declared at `bsd/sys/proc.h`
// ex. http://fxr.watson.org/fxr/source/bsd/sys/proc.h?v=xnu-792.6.70#L149
// Used in `extern_proc.p_stat` field

/// Process being created by fork.
pub const SIDL: libc::c_char = 1;
/// Currently runnable.
pub const SRUN: libc::c_char = 2;
/// Sleeping on an address.
pub const SSLEEP: libc::c_char = 3;
/// Process debugging or suspension.
pub const SSTOP: libc::c_char = 4;
/// Awaiting collection by parent.
pub const SZOMB: libc::c_char = 5;

impl TryFrom<libc::c_char> for Status {
    type Error = Error;

    fn try_from(value: libc::c_char) -> Result<Status, Self::Error> {
        match value {
            SIDL => Ok(Status::Idle),
            SRUN => Ok(Status::Running),
            SSLEEP => Ok(Status::Sleeping),
            SSTOP => Ok(Status::Stopped),
            SZOMB => Ok(Status::Zombie),
            other => {
                let inner = io::Error::from(io::ErrorKind::InvalidData);

                Err(Error::from(inner)
                    .with_message(format!("Unknown process p_stat value: {:?}", other,)))
            }
        }
    }
}

#[allow(non_camel_case_types)]
type caddr_t = *const libc::c_char;
#[allow(non_camel_case_types)]
type segsz_t = i32;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct vmspace {
    pub dummy: i32,
    pub dummy2: caddr_t,
    pub dummy3: [i32; 5],
    pub dummy4: [caddr_t; 3],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct pcred {
    pub pc_lock: [libc::c_char; 72],
    pub pc_ucred: *mut libc::xucred,
    pub p_ruid: libc::uid_t,
    pub p_svuid: libc::uid_t,
    pub p_rgid: libc::gid_t,
    pub p_svgid: libc::gid_t,
    pub p_refcnt: libc::c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct kinfo_proc {
    pub kp_proc: extern_proc,
    pub kp_eproc: kinfo_proc_eproc,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct run_sleep_queue {
    p_forw: vm_types::user_addr_t,
    p_back: vm_types::user_addr_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union p_un {
    pub p_st1: run_sleep_queue,
    pub p_starttime: libc::timeval,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct extern_proc {
    pub p_un: p_un,
    pub p_vmspace: vm_types::user_addr_t,
    pub p_sigacts: vm_types::user_addr_t,

    pub p_flag: libc::c_int,
    pub p_stat: libc::c_char,
    pub p_pid: libc::pid_t,
    pub p_oppid: libc::pid_t,
    pub p_dupfd: libc::c_int,
    pub user_stack: caddr_t,
    pub exit_thread: *mut libc::c_void,
    pub p_debugger: libc::c_int,
    pub sigwait: boolean::boolean_t,
    pub p_estcpu: libc::c_uint,
    pub p_cpticks: libc::c_int,
    pub p_pctcpu: u32,
    pub p_wchan: *mut libc::c_void,
    pub p_wmesg: *mut libc::c_char,
    pub p_swtime: libc::c_uint,
    pub p_slptime: libc::c_uint,
    pub p_realtimer: libc::itimerval,
    pub p_rtime: libc::timeval,
    pub p_uticks: u64,
    pub p_sticks: u64,
    pub p_iticks: u64,
    pub p_traceflag: libc::c_int,
    pub p_tracep: *mut libc::c_void,
    pub p_siglist: libc::c_int,
    // TODO: It was a pointer to `struct vnode`
    pub p_textvp: *mut libc::c_void,
    pub p_holdcnt: libc::c_int,
    pub p_sigmask: libc::sigset_t,
    pub p_sigignore: libc::sigset_t,
    pub p_sigcatch: libc::sigset_t,
    pub p_priority: libc::c_uchar,
    pub p_usrpri: libc::c_uchar,
    pub p_nice: libc::c_char,
    pub p_comm: [libc::c_char; 17],
    // TODO: It was a pointer to `struct proc`, declared at `bsd/sys/proc.h`
    pub p_pgrp: *mut libc::c_void,
    // TODO: It was a pointer to `struct user`, declared at `bsd/sys/user.h`
    // but it is not used anymore and we do not need it too
    pub p_addr: *mut libc::c_void,
    pub p_xstat: libc::c_ushort,
    pub p_acflag: libc::c_ushort,
    pub p_ru: *mut libc::rusage,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct kinfo_proc_eproc {
    // TODO: It should be a pointer to `struct proc`
    pub e_paddr: *mut libc::c_void,
    // TODO: It should be a pointer to `struct session`
    // but since we are not using it and it's declaration kinda big,
    // it was skipped. Same goes to `e_tsess` field below.
    pub e_sess: *mut libc::c_void,
    pub e_pcred: pcred,
    pub e_ucred: libc::xucred,
    pub e_vm: vmspace,
    pub e_ppid: libc::pid_t,
    pub e_pgid: libc::pid_t,
    pub e_jobc: libc::c_short,
    pub e_tdev: libc::dev_t,
    pub e_tpgid: libc::pid_t,
    pub e_tsess: *mut libc::c_void, // TODO: See `TODO` comment from above
    pub e_wmesg: [libc::c_char; 8],
    pub e_xsize: segsz_t,
    pub e_xrssize: libc::c_short,
    pub e_xccount: libc::c_short,
    pub e_xswrss: libc::c_short,
    pub e_flag: i32,
    pub e_login: [libc::c_char; 12],
    pub e_spare: [i32; 4],
}

pub fn processes() -> Result<Vec<kinfo_proc>, Error> {
    let mut name: [i32; 3] = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_ALL];
    let mut size: libc::size_t = 0;
    let mut processes: Vec<kinfo_proc> = vec![];

    loop {
        // Dry-run to get the size required for the process list
        let result = unsafe {
            libc::sysctl(
                name.as_mut_ptr(),
                3,
                ptr::null_mut(),
                &mut size,
                ptr::null_mut(),
                0,
            )
        };
        if result < 0 {
            return Err(Error::last_os_error().with_sysctl(name.as_ref()));
        }

        // Reserve enough room to store the whole process list
        let num_processes = size / mem::size_of::<kinfo_proc>();
        if num_processes > processes.capacity() {
            processes.reserve_exact(num_processes - processes.capacity());
        }

        // Attempt to store the process list in `processes`
        let result = unsafe {
            libc::sysctl(
                name.as_mut_ptr(),
                3,
                processes.as_mut_ptr() as *mut libc::c_void,
                &mut size,
                ptr::null_mut(),
                0,
            )
        };

        if result < 0 {
            // `libc::ENOMEM` indicates there was not enough space in `processes` to store the whole
            // process list which can occur when a new process spawns between getting the size and
            // storing. If this is the case then simply try again.
            let err = Error::last_os_error();
            if let Some(libc::ENOMEM) = err.raw_os_error() {
                continue;
            } else {
                return Err(err.with_sysctl(name.as_ref()));
            }
        } else {
            // Getting the list succeeded so let `processes` know how many processes it holds
            let length = size / mem::size_of::<kinfo_proc>();
            unsafe {
                processes.set_len(length);
            }
            debug_assert!(!processes.is_empty());

            return Ok(processes);
        }
    }
}

pub fn process(pid: Pid) -> Result<kinfo_proc, ProcessError> {
    let mut name: [i32; 4] = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_PID, pid];
    let mut size: libc::size_t = mem::size_of::<kinfo_proc>();
    let mut info = mem::MaybeUninit::<kinfo_proc>::uninit();

    let result = unsafe {
        libc::sysctl(
            name.as_mut_ptr(),
            4,
            info.as_mut_ptr() as *mut libc::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if result < 0 {
        return Err(Error::last_os_error().into());
    }

    // sysctl succeeds but size is zero, happens when process has gone away
    if size == 0 {
        return Err(ProcessError::NoSuchProcess(pid));
    }

    unsafe { Ok(info.assume_init()) }
}

#[cfg(test)]
mod tests {
    use std::mem;

    use super::{kinfo_proc, kinfo_proc_eproc, pcred, vmspace};

    #[test]
    fn test_layout() {
        assert_eq!(mem::size_of::<vmspace>(), 64);
        assert_eq!(mem::align_of::<vmspace>(), 8);

        assert_eq!(mem::size_of::<pcred>(), 104);
        assert_eq!(mem::align_of::<pcred>(), 8);

        assert_eq!(mem::size_of::<kinfo_proc>(), 648);
        assert_eq!(mem::align_of::<kinfo_proc>(), 8);

        assert_eq!(mem::size_of::<kinfo_proc_eproc>(), 352);
        assert_eq!(mem::align_of::<kinfo_proc_eproc>(), 8);
    }
}
