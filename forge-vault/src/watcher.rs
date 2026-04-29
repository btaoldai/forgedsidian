//! File-system watcher for live vault reloading.
//!
//! Uses the [`notify`] crate to watch the vault root directory recursively.
//! File events are debounced and sent as [`VaultEvent`] variants through a
//! `tokio::sync::mpsc` channel so the Tauri layer can re-index incrementally
//! and notify the frontend.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A high-level vault file-system event.
#[derive(Debug, Clone)]
pub enum VaultEvent {
    /// A Markdown file was created or modified — re-index it.
    Changed(PathBuf),
    /// A Markdown file was deleted — remove it from the index and graph.
    Removed(PathBuf),
    /// A Markdown file was renamed (old path, new path).
    Renamed { from: PathBuf, to: PathBuf },
}

/// Handle to a running vault watcher.
///
/// Dropping this struct stops the watcher thread (the inner `RecommendedWatcher`
/// is dropped, which signals the OS to stop delivering events).
pub struct VaultWatcher {
    /// Keep the watcher alive — dropping it stops file-system notifications.
    _watcher: RecommendedWatcher,
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

impl VaultWatcher {
    /// Start watching `vault_root` recursively for Markdown file changes.
    ///
    /// Returns the watcher handle and a receiver for [`VaultEvent`]s.
    /// The caller is responsible for consuming the receiver (typically in a
    /// `tokio::spawn` loop).
    ///
    /// # Errors
    /// Returns an `std::io::Error` if the watcher cannot be initialised.
    pub fn start(
        vault_root: &Path,
    ) -> Result<(Self, mpsc::UnboundedReceiver<VaultEvent>), std::io::Error> {
        let (tx, rx) = mpsc::unbounded_channel();
        let root = vault_root.to_path_buf();

        let event_tx = tx.clone();
        let mut watcher =
            notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    Self::handle_raw_event(&event_tx, &event);
                }
                Err(e) => {
                    error!(error = %e, "file watcher error");
                }
            })
            .map_err(std::io::Error::other)?;

        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(std::io::Error::other)?;

        info!(root = %root.display(), "vault watcher started");

        Ok((Self { _watcher: watcher }, rx))
    }

    /// Filter and convert a raw `notify::Event` into zero or more `VaultEvent`s.
    ///
    /// Only `.md` files are forwarded. Hidden directories (`.git`, `.obsidian`,
    /// `.forge-index`) are silently ignored.
    fn handle_raw_event(tx: &mpsc::UnboundedSender<VaultEvent>, event: &Event) {
        for path in &event.paths {
            // Skip non-Markdown files.
            if path.extension().is_none_or(|ext| ext != "md") {
                continue;
            }

            // Skip hidden directories (e.g. .git, .obsidian, .forge-index).
            if path
                .components()
                .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
            {
                continue;
            }

            let vault_event = match &event.kind {
                EventKind::Create(_) | EventKind::Modify(_) => {
                    debug!(path = %path.display(), "md file changed");
                    Some(VaultEvent::Changed(path.clone()))
                }
                EventKind::Remove(_) => {
                    debug!(path = %path.display(), "md file removed");
                    Some(VaultEvent::Removed(path.clone()))
                }
                _ => None,
            };

            if let Some(ve) = vault_event {
                if tx.send(ve).is_err() {
                    warn!("vault event receiver dropped — stopping watcher dispatch");
                    return;
                }
            }
        }
    }
}
