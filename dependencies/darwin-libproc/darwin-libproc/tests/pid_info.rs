#[test]
fn test_task_info() {
    let me = unsafe { libc::getpid() };
    let result = darwin_libproc::task_info(me);

    assert!(result.is_ok(), "{:#?}", result.unwrap_err());
}

#[test]
fn test_task_all_info() {
    let me = unsafe { libc::getpid() };
    let result = darwin_libproc::task_all_info(me);

    assert!(result.is_ok(), "{:#?}", result.unwrap_err());
}

#[test]
fn test_vnode_path_info() {
    let me = unsafe { libc::getpid() };
    let result = darwin_libproc::vnode_path_info(me);

    assert!(result.is_ok());
}
