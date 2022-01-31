#[test]
fn test_all_pids() {
    let me = unsafe { libc::getpid() };
    let result = darwin_libproc::all_pids();

    assert!(result.is_ok());
    let pids = result.unwrap();
    assert!(pids.len() > 0);
    assert!(pids.contains(&me));
}

#[test]
fn test_pgrp_only_pids() {
    let me = unsafe { libc::getpid() };
    let pgrp = unsafe { libc::getpgrp() };
    let result = darwin_libproc::pgrp_only_pids(pgrp);

    assert!(result.is_ok());
    let pids = result.unwrap();
    assert!(pids.len() > 0);
    assert!(pids.contains(&me));
}

#[test]
fn test_uid_only_pids() {
    let me = unsafe { libc::getpid() };
    let uid = unsafe { libc::getuid() };
    let result = darwin_libproc::uid_only_pids(uid);

    assert!(result.is_ok());
    let pids = result.unwrap();
    assert!(pids.len() > 0);
    assert!(pids.contains(&me));
}

#[test]
fn test_ruid_only_pids() {
    let me = unsafe { libc::getpid() };
    let ruid = unsafe { libc::getuid() };
    let result = darwin_libproc::uid_only_pids(ruid);

    assert!(result.is_ok());
    let pids = result.unwrap();
    assert!(pids.len() > 0);
    assert!(pids.contains(&me));
}

#[test]
fn test_ppid_only_pids() {
    let me = unsafe { libc::getpid() };
    let ppid = unsafe { libc::getppid() };
    let result = darwin_libproc::ppid_only_pids(ppid);

    assert!(result.is_ok());
    let pids = result.unwrap();
    assert!(pids.len() > 0);
    assert!(pids.contains(&me));
}
