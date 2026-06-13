use crate::globals::CONFIG;
use std::path::{Component, Path, PathBuf};

/// Resolve a user-supplied wallet keystore path into a sanitized absolute path
/// rooted at [`CONFIG.wallet_keys_dir`].
///
/// Rejects:
/// - Empty paths.
/// - Absolute paths, root references (`/`, `\`), and Windows drive/UNC prefixes.
/// - Any `..` (parent directory) component.
/// - Paths whose canonicalized parent escapes the sandbox root.
///
/// The sandbox root and intermediate directories are created on demand so that
/// fresh keystores can be written before the file itself exists.
pub fn resolve_keystore_path(user_input: &str) -> Result<PathBuf, String> {
    if user_input.trim().is_empty() {
        return Err("Wallet path cannot be empty".to_string());
    }

    let input_path = Path::new(user_input);
    for component in input_path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(format!(
                    "Wallet path '{}' must be relative and free of '..' or absolute prefixes",
                    user_input
                ));
            }
        }
    }

    let sandbox_root = PathBuf::from(&CONFIG.wallet_keys_dir);
    std::fs::create_dir_all(&sandbox_root)
        .map_err(|e| format!("Failed to create wallet sandbox directory: {}", e))?;
    let sandbox_canonical = sandbox_root
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize wallet sandbox: {}", e))?;

    // Accept both forms:
    //   "wallet.json"       -> <sandbox>/wallet.json
    //   "keys/wallet.json"  -> keys/wallet.json (when the input is already
    //                          rooted at the sandbox directory name)
    // In either case we canonicalize the parent below and verify it stays
    // inside the sandbox, so the duplicated-prefix check is the only place
    // that needs to disambiguate.
    let sandbox_first_component = sandbox_root.components().next();
    let input_first_component = input_path.components().next();
    let input_starts_with_sandbox = matches!(
        (sandbox_first_component, input_first_component),
        (Some(s), Some(i)) if s == i
    );
    let candidate = if input_starts_with_sandbox {
        input_path.to_path_buf()
    } else {
        sandbox_root.join(input_path)
    };
    let parent = candidate
        .parent()
        .ok_or_else(|| format!("Wallet path '{}' has no parent directory", user_input))?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("Failed to create wallet parent directory: {}", e))?;
    let parent_canonical = parent
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize wallet path parent: {}", e))?;

    if !parent_canonical.starts_with(&sandbox_canonical) {
        return Err(format!(
            "Wallet path '{}' escapes the wallet keys directory '{}'. \
             Pass the keystore filename relative to that directory (e.g. \
             'my_wallet.json' or 'keys/my_wallet.json').",
            user_input, CONFIG.wallet_keys_dir
        ));
    }

    let file_name = candidate
        .file_name()
        .ok_or_else(|| format!("Wallet path '{}' must include a file name", user_input))?;
    Ok(parent_canonical.join(file_name))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // POSIX-style absolute
        assert!(resolve_keystore_path("/etc/passwd").is_err());
        // Windows-style absolute / drive prefix / UNC
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
        // Same file resolved with and without the prefix must yield the same
        // canonical path - this is what unblocks legacy sessions / .env defaults
        // that already use 'keys/...'.
        let with = resolve_keystore_path("keys/test_sandbox_prefixed.json").unwrap();
        let without = resolve_keystore_path("test_sandbox_prefixed.json").unwrap();
        assert_eq!(with, without);
    }
}
