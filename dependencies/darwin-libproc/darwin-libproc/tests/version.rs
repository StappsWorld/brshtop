#[test]
fn test_version() {
    assert!(darwin_libproc::version().is_ok());
}
