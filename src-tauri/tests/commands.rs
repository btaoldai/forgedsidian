//! Integration tests for Tauri IPC security validators.
//!
//! Tests the pure helper functions (`reject_traversal`, `validate_vault_path`)
//! that guard path-based commands against traversal attacks. Runs
//! cross-platform (Windows/macOS/Linux) by using `std::env::temp_dir()` when
//! real filesystem state is needed.
//!
//! ## Strategy
//! Pure unit tests on helper logic (Option A from audit) — no Tauri runtime
//! mock required. These helpers are exposed as `pub fn` from
//! `forge_app_lib::commands`.

use forge_app_lib::commands;
use std::path::PathBuf;

// ===========================================================================
// reject_traversal — blocks `..` components in relative paths
// ===========================================================================

#[test]
fn test_reject_traversal_blocks_dotdot() {
    // Path with explicit `..` should be rejected.
    let path = PathBuf::from("notes/../etc/passwd");
    let result = commands::reject_traversal(&path);
    assert!(
        result.is_err(),
        "reject_traversal should block paths with .. components"
    );
    assert!(
        result.unwrap_err().contains("traversal not allowed"),
        "error message should mention traversal"
    );
}

#[test]
fn test_reject_traversal_accepts_normal_paths() {
    // Legitimate relative paths should pass.
    let path = PathBuf::from("folder/subfolder/note.md");
    let result = commands::reject_traversal(&path);
    assert!(
        result.is_ok(),
        "reject_traversal should accept legitimate relative paths"
    );
}

#[test]
fn test_reject_traversal_empty_path() {
    // Empty path has no Parent component — passes.
    let path = PathBuf::from("");
    let result = commands::reject_traversal(&path);
    assert!(result.is_ok(), "Empty path should pass reject_traversal");
}

#[test]
fn test_reject_traversal_current_dir_only() {
    // `.` alone has no `..` — passes.
    let path = PathBuf::from(".");
    let result = commands::reject_traversal(&path);
    assert!(
        result.is_ok(),
        "Current dir (.) alone should pass reject_traversal"
    );
}

#[test]
fn test_reject_traversal_dotdot_variations() {
    // Various `..` patterns must all be rejected.
    let patterns = vec![
        PathBuf::from(".."),
        PathBuf::from("notes/.."),
        PathBuf::from("a/b/../c"),
        PathBuf::from("a/../../b"),
    ];

    for path in patterns {
        let result = commands::reject_traversal(&path);
        assert!(
            result.is_err(),
            "Path {path:?} should be rejected for containing .."
        );
    }
}

#[test]
fn test_reject_traversal_with_multiple_variants() {
    // Mix of attacks and legitimate paths, tested together.
    let test_cases = vec![
        ("../etc/passwd", true),   // block
        ("../../root/.ssh", true), // block
        ("folder/note.md", false), // accept
        ("a/b/c/d.md", false),     // accept
    ];

    for (path_str, should_fail) in test_cases {
        let path = PathBuf::from(path_str);
        let result = commands::reject_traversal(&path);
        if should_fail {
            assert!(
                result.is_err(),
                "reject_traversal should block '{path_str}' but got Ok"
            );
        } else {
            assert!(
                result.is_ok(),
                "reject_traversal should accept '{path_str}' but got Err: {result:?}"
            );
        }
    }
}

#[test]
fn test_reject_traversal_unix_style_attack() {
    // Classic Unix-style path traversal.
    let path = PathBuf::from("notes/../../etc/passwd");
    let result = commands::reject_traversal(&path);
    assert!(result.is_err(), "Unix-style traversal should be blocked");
}

#[test]
fn test_reject_traversal_percent_encoded_is_literal() {
    // Percent-encoding is treated as literal text by Path — does NOT bypass.
    // This test documents current behaviour: %2e%2e is safe as a literal
    // folder name (not interpreted as ..).
    let path = PathBuf::from("notes/%2e%2e/etc/passwd");
    let result = commands::reject_traversal(&path);
    assert!(
        result.is_ok(),
        "percent-encoded .. is literal text, not a traversal component"
    );
}

#[test]
fn test_reject_traversal_deep_nested_is_safe() {
    // Deeply nested relative path with no `..` — must pass.
    let deep_path = (0..50)
        .map(|i| format!("level{i}"))
        .collect::<Vec<_>>()
        .join("/");
    let path = PathBuf::from(&deep_path);
    let result = commands::reject_traversal(&path);
    assert!(result.is_ok(), "Deep nested path without .. should pass");
}

#[test]
fn test_reject_traversal_cross_platform_separators() {
    // Rust's Path parses both forward and backward slashes on Windows.
    // On Unix, backslashes are literal characters — the `\\..` segment
    // is a filename, not a ParentDir component, so this path passes
    // reject_traversal on Unix (no `..` component).
    //
    // On Windows, `notes\..\etc` parses the `..` as ParentDir and is
    // correctly blocked. We assert platform-specific behaviour here.
    let path = PathBuf::from(r"notes\..\etc\passwd");
    let result = commands::reject_traversal(&path);

    #[cfg(windows)]
    assert!(
        result.is_err(),
        "Windows should reject backslash traversal: {result:?}"
    );

    #[cfg(unix)]
    assert!(
        result.is_ok(),
        "Unix treats backslashes as literal chars, no ParentDir: {:?}",
        result
    );
}

// ===========================================================================
// Composite scenario — reject_traversal used as a filter before path join
// ===========================================================================

#[test]
fn test_valid_relative_path_construction() {
    // Simulates how a real command validates a user-supplied relative
    // path before joining with the vault root. Cross-platform: uses
    // std::env::temp_dir() as the vault root stand-in.
    let vault_root = std::env::temp_dir();
    let note_rel = PathBuf::from("folder/note.md");

    // Step 1 — relative path must be traversal-free.
    let check = commands::reject_traversal(&note_rel);
    assert!(
        check.is_ok(),
        "valid relative path should pass traversal check"
    );

    // Step 2 — join with vault root (what commands do internally).
    let full = vault_root.join(&note_rel);
    assert!(
        full.starts_with(&vault_root),
        "joined path must stay under the vault root"
    );
}

#[test]
fn test_malicious_relative_path_blocked_before_join() {
    // Attacker-supplied path with `..` — must be caught before join.
    let note_rel = PathBuf::from("../../../etc/passwd");
    let check = commands::reject_traversal(&note_rel);
    assert!(
        check.is_err(),
        "attacker path must be rejected before path join"
    );
}

// ===========================================================================
// validate_vault_path — must be absolute, `..`-free, existing directory
// ===========================================================================

#[test]
fn test_validate_vault_path_accepts_temp_dir() {
    // std::env::temp_dir() is cross-platform: /tmp on Unix, %TEMP% on Windows.
    let temp = std::env::temp_dir();
    let temp_str = temp.to_str().expect("temp_dir is valid UTF-8");

    let result = commands::validate_vault_path(temp_str);
    assert!(
        result.is_ok(),
        "validate_vault_path should accept the OS temp dir ({temp_str}): {result:?}"
    );
    let canonical = result.unwrap();
    assert!(canonical.is_dir(), "result must be a directory");
    assert!(canonical.is_absolute(), "result must be absolute");
}

#[test]
fn test_validate_vault_path_rejects_relative_paths() {
    let result = commands::validate_vault_path("relative/path");
    assert!(result.is_err(), "relative paths must be rejected");
    assert!(
        result.unwrap_err().contains("absolute"),
        "error should mention absolute requirement"
    );
}

#[test]
fn test_validate_vault_path_rejects_current_dir() {
    let result = commands::validate_vault_path("./something");
    assert!(
        result.is_err(),
        "dot-prefixed relative paths must be rejected"
    );
}

#[test]
fn test_validate_vault_path_rejects_empty_string() {
    let result = commands::validate_vault_path("");
    assert!(result.is_err(), "empty string must be rejected");
}

#[test]
fn test_validate_vault_path_rejects_traversal() {
    // Build an absolute path with a `..` component — must be rejected BEFORE
    // canonicalize (defense in depth).
    let temp = std::env::temp_dir();
    let with_dotdot = temp.join("..").join("evil");
    let with_dotdot_str = with_dotdot
        .to_str()
        .expect("constructed path is valid UTF-8");

    let result = commands::validate_vault_path(with_dotdot_str);
    assert!(
        result.is_err(),
        "path containing .. must be rejected: {result:?}"
    );
    assert!(
        result.unwrap_err().contains("traversal"),
        "error should mention traversal"
    );
}

#[test]
fn test_validate_vault_path_rejects_nonexistent() {
    // Absolute path that does not exist — canonicalize fails.
    let temp = std::env::temp_dir();
    let missing = temp.join("forge-test-nonexistent-xyz-42");
    let missing_str = missing.to_str().expect("constructed path is valid UTF-8");

    let result = commands::validate_vault_path(missing_str);
    assert!(result.is_err(), "nonexistent path must be rejected");
    assert!(
        result.unwrap_err().contains("canonicalize"),
        "error should mention canonicalize failure"
    );
}

#[test]
fn test_validate_vault_path_rejects_too_long() {
    // Build a 1100-char absolute path (exceeds 1024 limit).
    let prefix = if cfg!(windows) { "C:\\" } else { "/" };
    let long = format!("{}{}", prefix, "a".repeat(1100));
    let result = commands::validate_vault_path(&long);
    assert!(result.is_err(), "paths over 1024 chars must be rejected");
    assert!(
        result.unwrap_err().contains("too long"),
        "error should mention length"
    );
}

#[test]
fn test_validate_vault_path_rejects_file_not_directory() {
    // Create a temp file, then ensure it is rejected (not a directory).
    let temp = std::env::temp_dir();
    let file_path = temp.join("forge-test-file.txt");
    std::fs::write(&file_path, b"test").expect("can write temp file");

    let file_str = file_path.to_str().expect("valid UTF-8");
    let result = commands::validate_vault_path(file_str);

    // Cleanup regardless of test outcome.
    let _ = std::fs::remove_file(&file_path);

    assert!(result.is_err(), "files (not dirs) must be rejected");
    assert!(
        result.unwrap_err().contains("directory"),
        "error should mention directory requirement"
    );
}
