#[test]
fn test_name() {
    let me = unsafe { libc::getpid() };

    assert!(darwin_libproc::name(me).is_ok());
}
