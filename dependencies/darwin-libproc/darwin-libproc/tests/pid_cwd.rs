#[test]
fn test_pid_cwd() {
    let me = unsafe { libc::getpid() };

    assert!(darwin_libproc::pid_cwd(me).is_ok());
}
