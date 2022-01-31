#[test]
fn test_pid_rusage() {
    let me = unsafe { libc::getpid() };

    assert!(
        darwin_libproc::pid_rusage::<darwin_libproc::rusage_info_v0>(me)
            .is_ok()
    );
    assert!(
        darwin_libproc::pid_rusage::<darwin_libproc::rusage_info_v1>(me)
            .is_ok()
    );
    assert!(
        darwin_libproc::pid_rusage::<darwin_libproc::rusage_info_v2>(me)
            .is_ok()
    );
    assert!(
        darwin_libproc::pid_rusage::<darwin_libproc::rusage_info_v3>(me)
            .is_ok()
    );
    assert!(
        darwin_libproc::pid_rusage::<darwin_libproc::rusage_info_v4>(me)
            .is_ok()
    );
}
