//! Tab manager — multi-tab state for opening multiple files simultaneously.
//!
//! Each open file gets a [`Tab`] with its own content cache. The manager
//! tracks the active tab and syncs it with [`AppState::current_note`] and
//! [`AppState::note_content`] for backward compatibility with the editor.
//!
//! ## Usage
//! Provided as Leptos context by `<App />`. Components access it via:
//! ```ignore
//! let tabs = use_context::<TabManager>().expect("TabManager");
//! tabs.open(abs_path, rel_path, content);
//! ```

use leptos::prelude::*;
use uuid::Uuid;

/// Unique identifier for a tab.
pub type TabId = Uuid;

/// A single open tab representing one file.
#[derive(Clone, Debug)]
pub struct Tab {
    /// Unique tab identifier.
    pub id: TabId,
    /// Absolute path to the file (used for IPC save/load).
    pub file_path: String,
    /// Relative path within the vault (used for display).
    pub rel_path: String,
    /// Display title (file name, stripped of .md extension for markdown).
    pub title: String,
    /// Cached file content (synced on open, updated on edit/save).
    pub content: String,
    /// True if the tab has unsaved changes.
    pub modified: bool,
    /// True if this file is markdown (.md).
    pub is_markdown: bool,
}

/// Reactive tab manager — holds all open tabs and the active selection.
///
/// Provided as Leptos context. All fields are `RwSignal` for reactivity.
#[derive(Clone, Copy)]
pub struct TabManager {
    /// All currently open tabs.
    pub tabs: RwSignal<Vec<Tab>>,
    /// ID of the currently active (visible) tab, if any.
    pub active_tab_id: RwSignal<Option<TabId>>,
}

impl TabManager {
    /// Create a new empty tab manager.
    pub fn new() -> Self {
        Self {
            tabs: RwSignal::new(Vec::new()),
            active_tab_id: RwSignal::new(None),
        }
    }

    /// Open a file in a tab. If the file is already open, switch to it.
    /// Otherwise, create a new tab and activate it.
    ///
    /// Returns the `TabId` of the (new or existing) tab.
    pub fn open(&self, file_path: &str, rel_path: &str, content: &str) -> TabId {
        // Check if already open.
        let existing = self.tabs.get_untracked()
            .iter()
            .find(|t| t.file_path == file_path)
            .map(|t| t.id);

        if let Some(id) = existing {
            self.active_tab_id.set(Some(id));
            // Update content in case the file changed on disk.
            self.tabs.update(|tabs| {
                if let Some(tab) = tabs.iter_mut().find(|t| t.id == id) {
                    if !tab.modified {
                        tab.content = content.to_string();
                    }
                }
            });
            return id;
        }

        // Create a new tab.
        let id = Uuid::new_v4();
        let title = Self::make_title(rel_path);
        let is_markdown = rel_path.to_lowercase().ends_with(".md");

        let tab = Tab {
            id,
            file_path: file_path.to_string(),
            rel_path: rel_path.to_string(),
            title,
            content: content.to_string(),
            modified: false,
            is_markdown,
        };

        self.tabs.update(|tabs| tabs.push(tab));
        self.active_tab_id.set(Some(id));
        id
    }

    /// Switch to a tab by ID.
    pub fn activate(&self, id: TabId) {
        let exists = self.tabs.get_untracked().iter().any(|t| t.id == id);
        if exists {
            self.active_tab_id.set(Some(id));
        }
    }

    /// Close a tab by ID. If it was the active tab, activate an adjacent one.
    pub fn close(&self, id: TabId) {
        let tabs = self.tabs.get_untracked();
        let idx = tabs.iter().position(|t| t.id == id);
        let was_active = self.active_tab_id.get_untracked() == Some(id);

        if let Some(idx) = idx {
            // Determine the next active tab before removing.
            let next_active = if was_active {
                if tabs.len() <= 1 {
                    None
                } else if idx + 1 < tabs.len() {
                    Some(tabs[idx + 1].id)
                } else {
                    Some(tabs[idx - 1].id)
                }
            } else {
                self.active_tab_id.get_untracked()
            };

            self.tabs.update(|tabs| {
                tabs.remove(idx);
            });

            if was_active {
                self.active_tab_id.set(next_active);
            }
        }
    }

    /// Close all tabs except the one with the given ID.
    pub fn close_others(&self, keep_id: TabId) {
        self.tabs.update(|tabs| {
            tabs.retain(|t| t.id == keep_id);
        });
        self.active_tab_id.set(Some(keep_id));
    }

    /// Get the currently active tab (clone).
    pub fn active_tab(&self) -> Option<Tab> {
        let id = self.active_tab_id.get();
        id.and_then(|id| {
            self.tabs.get().into_iter().find(|t| t.id == id)
        })
    }

    /// Get the active tab non-reactively.
    pub fn active_tab_untracked(&self) -> Option<Tab> {
        let id = self.active_tab_id.get_untracked();
        id.and_then(|id| {
            self.tabs.get_untracked().into_iter().find(|t| t.id == id)
        })
    }

    /// Update the content of the active tab (called on each edit keystroke).
    pub fn update_active_content(&self, new_content: String) {
        if let Some(id) = self.active_tab_id.get_untracked() {
            self.tabs.update(|tabs| {
                if let Some(tab) = tabs.iter_mut().find(|t| t.id == id) {
                    tab.content = new_content;
                    tab.modified = true;
                }
            });
        }
    }

    /// Mark the active tab as saved (not modified).
    pub fn mark_active_saved(&self, saved_content: String) {
        if let Some(id) = self.active_tab_id.get_untracked() {
            self.tabs.update(|tabs| {
                if let Some(tab) = tabs.iter_mut().find(|t| t.id == id) {
                    tab.content = saved_content;
                    tab.modified = false;
                }
            });
        }
    }

    /// Switch to the next tab (wraps around).
    pub fn next_tab(&self) {
        let tabs = self.tabs.get_untracked();
        if tabs.len() <= 1 {
            return;
        }
        if let Some(id) = self.active_tab_id.get_untracked() {
            let idx = tabs.iter().position(|t| t.id == id).unwrap_or(0);
            let next = (idx + 1) % tabs.len();
            self.active_tab_id.set(Some(tabs[next].id));
        }
    }

    /// Switch to the previous tab (wraps around).
    pub fn prev_tab(&self) {
        let tabs = self.tabs.get_untracked();
        if tabs.len() <= 1 {
            return;
        }
        if let Some(id) = self.active_tab_id.get_untracked() {
            let idx = tabs.iter().position(|t| t.id == id).unwrap_or(0);
            let prev = if idx == 0 { tabs.len() - 1 } else { idx - 1 };
            self.active_tab_id.set(Some(tabs[prev].id));
        }
    }

    /// Derive a display title from a relative path.
    fn make_title(rel_path: &str) -> String {
        let name = rel_path
            .rsplit(|c: char| c == '/' || c == '\\')
            .next()
            .unwrap_or(rel_path);
        name.strip_suffix(".md").unwrap_or(name).to_string()
    }
}
