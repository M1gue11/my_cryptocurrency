use project::security_utils::resolve_keystore_path;

#[test]
fn rejects_empty_path() {
    assert!(resolve_keystore_path("").is_err());
    assert!(resolve_keystore_path("   ").is_err());
}

#[test]
fn rejects_parent_dir_traversal() {
    assert!(resolve_keystore_path("../etc/passwd").is_err());
    assert!(resolve_keystore_path("nested/../../escape.json").is_err());
}

#[test]
fn rejects_absolute_paths() {
    assert!(resolve_keystore_path("/etc/passwd").is_err());
    assert!(resolve_keystore_path("C:\\Windows\\System32\\config\\SAM").is_err());
    assert!(resolve_keystore_path("\\\\server\\share\\file").is_err());
}

#[test]
fn accepts_simple_relative_path() {
    let resolved = resolve_keystore_path("test_sandbox_simple.json").unwrap();
    assert!(resolved.ends_with("test_sandbox_simple.json"));
}

#[test]
fn accepts_nested_relative_path() {
    let resolved = resolve_keystore_path("users/test_sandbox_alice.json").unwrap();
    assert!(resolved.ends_with("test_sandbox_alice.json"));
}

#[test]
fn accepts_path_with_sandbox_prefix() {
    let with = resolve_keystore_path("keys/test_sandbox_prefixed.json").unwrap();
    let without = resolve_keystore_path("test_sandbox_prefixed.json").unwrap();
    assert_eq!(with, without);
}
