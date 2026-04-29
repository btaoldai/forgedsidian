//! Tauri IPC command handlers.
//!
//! All commands follow the same contract:
//! - Accept only serialisable types as parameters
//! - Return `Result<T, String>` where the `String` is a user-facing error
//! - Delegate immediately to the relevant engine crate (no business logic here)
//!
//! ## Module layout
//! - [`vault_ops`] — vault lifecycle: open, pick, list notes, search, graph, canvas
//! - [`file_ops`]  — file CRUD: read, save, create, move, delete
//! - [`scan`]      — directory scanning with `.forgeignore` support
//!
//! ## Mutex choice
//! We use `tokio::sync::Mutex` (async-aware) so that guards are `Send` across
//! `.await` points.  `std::sync::Mutex` guards are `!Send` and would cause a
//! compile error when held over an `await`.

pub mod file_ops;
pub mod scan;
pub mod tags;
pub mod vault_ops;

// Security validators (reject_traversal, validate_vault_path,
// harden_vault_index_permissions) are defined below as `pub fn` and are
// reachable from integration tests via `forge_app_lib::commands::<name>`.

use forge_canvas::Canvas;
use forge_vault::{store::VaultStore, watcher::VaultWatcher};
use std::path::{Component, Path, PathBuf};
use tokio::sync::Mutex;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

// ---------------------------------------------------------------------------
// Shared application state (managed by Tauri)
// ---------------------------------------------------------------------------

/// The global state injected into every command handler.
pub struct ForgeState {
    /// Async mutex: guards are `Send`, safe to hold across `.await` points.
    pub store: Mutex<Option<VaultStore>>,
    /// Canvas state — also async for consistency.
    pub canvas: Mutex<Canvas>,
    /// File watcher handle — kept alive so notifications continue.
    /// `None` until `open_vault` is called.
    pub watcher: Mutex<Option<VaultWatcher>>,
    /// Vault root path — set when `open_vault` is called.
    pub vault_path: Mutex<Option<PathBuf>>,
}

impl Default for ForgeState {
    fn default() -> Self {
        Self {
            store: Mutex::new(None),
            canvas: Mutex::new(Canvas::new()),
            watcher: Mutex::new(None),
            vault_path: Mutex::new(None),
        }
    }
}

impl ForgeState {
    /// Create an uninitialised state.  `open_vault` must be called before most
    /// other commands will succeed.
    pub fn new() -> Self {
        Self::default()
    }
}

// ---------------------------------------------------------------------------
// Security: Path validation (shared by all command modules)
// ---------------------------------------------------------------------------

/// Validate a vault path to prevent path traversal attacks.
///
/// The path must be **absolute** (rooted by the OS: `/tmp` on Unix or
/// `C:\foo` on Windows) and free of `..` components. It is then
/// canonicalised (resolves symlinks and redundant separators) and verified
/// to be an existing directory.
///
/// - Max length: 1024 characters
/// - Absolute check is cross-platform via `PathBuf::is_absolute`
/// - `..` components are rejected as defense-in-depth (even though
///   canonicalise would normally resolve them)
///
/// Returns the canonical absolute path if valid, or a user-facing error
/// message otherwise.
pub fn validate_vault_path(path_str: &str) -> Result<PathBuf, String> {
    const MAX_PATH_LEN: usize = 1024;
    if path_str.len() > MAX_PATH_LEN {
        return Err(format!("path too long (max {} chars)", MAX_PATH_LEN));
    }

    let path = PathBuf::from(path_str);

    // Require an absolute path (rejects "", "./foo", "folder/note", etc.)
    if !path.is_absolute() {
        return Err("path must be absolute; relative paths are not allowed".to_string());
    }

    // Defense in depth: reject any `..` component before canonicalising.
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err("path traversal not allowed: '..' detected".to_string());
        }
    }

    let canonical = path
        .canonicalize()
        .map_err(|e| format!("failed to canonicalize path: {}", e))?;

    if !canonical.is_dir() {
        return Err(format!(
            "vault path must be a directory: {}",
            canonical.display()
        ));
    }

    Ok(canonical)
}

/// Validate that a relative path contains no path traversal (`..`).
pub fn reject_traversal(path: &Path) -> Result<(), String> {
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err("path traversal not allowed".to_string());
        }
    }
    Ok(())
}

/// Set restrictive permissions on vault index directory (`chmod 600` on Unix).
///
/// **Security**: Prevents other users/processes from reading the vault index.
/// On Windows, this is a no-op (ACLs are not enforced here).
#[cfg(unix)]
pub fn harden_vault_index_permissions(vault_path: &std::path::Path) -> Result<(), String> {
    let index_dir = vault_path.join(".forge-index");

    if index_dir.exists() {
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&index_dir, perms)
            .map_err(|e| format!("failed to harden index directory permissions: {}", e))?;
        tracing::info!(path = %index_dir.display(), "hardened .forge-index permissions to 0600");
    }

    Ok(())
}

/// No-op on Windows (ACLs not configured here).
#[cfg(not(unix))]
pub fn harden_vault_index_permissions(_vault_path: &std::path::Path) -> Result<(), String> {
    Ok(())
}
