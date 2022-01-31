#[test]
fn test_pid_path() {
    let me = unsafe { libc::getpid() };

    assert!(darwin_libproc::pid_path(me).is_ok());
}
