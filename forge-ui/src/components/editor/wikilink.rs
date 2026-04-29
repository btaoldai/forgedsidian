//! Wikilink navigation and extraction helpers.
//!
//! Provides utilities for finding wikilinks in rendered HTML and Markdown text,
//! resolving them to files via IPC, and opening them in tabs or external apps.

use crate::ipc;
use crate::tab_manager::TabManager;
use wasm_bindgen::JsCast;

/// Sleep for `ms` milliseconds in WASM using JS setTimeout.
///
/// Silently returns early if `window` is unavailable (e.g., SSR context).
pub async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(win) = web_sys::window() {
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Walk up from `el` to find an ancestor with class `forge-wikilink`.
///
/// Returns the matching element if found within 5 levels, or `None`.
pub fn find_wikilink_ancestor(el: &web_sys::HtmlElement) -> Option<web_sys::HtmlElement> {
    let mut current: Option<web_sys::HtmlElement> = Some(el.clone());
    for _ in 0..5 {
        if let Some(ref node) = current {
            if node.class_list().contains("forge-wikilink") {
                return current;
            }
            current = node
                .parent_element()
                .and_then(|p| p.dyn_into::<web_sys::HtmlElement>().ok());
        } else {
            break;
        }
    }
    None
}

/// Extract a wikilink target at the given cursor position in raw Markdown.
///
/// Scans backwards from `cursor` for `[[` and forwards for `]]`.
/// Returns the target (before `|` if aliased) if the cursor is inside a wikilink.
pub fn extract_wikilink_at_cursor(text: &str, cursor: usize) -> Option<String> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    if cursor > len {
        return None;
    }

    // Scan backwards for `[[`
    let mut start = None;
    if cursor >= 2 {
        let search_from = cursor.min(len);
        for i in (0..search_from).rev() {
            if i + 1 < len && bytes[i] == b'[' && bytes[i + 1] == b'[' {
                start = Some(i + 2);
                break;
            }
            // If we hit `]]` before `[[`, cursor is not inside a wikilink.
            if i + 1 < len && bytes[i] == b']' && bytes[i + 1] == b']' {
                return None;
            }
        }
    }

    let start = start?;

    // Scan forwards for `]]`
    for i in start..len.saturating_sub(1) {
        if bytes[i] == b']' && bytes[i + 1] == b']' {
            let inner = &text[start..i];
            // Return target part (before `|` if aliased).
            let target = inner.split('|').next().unwrap_or(inner);
            return Some(target.to_string());
        }
    }

    None
}

/// Check if a wikilink target refers to a non-Markdown file that should be
/// opened in the user's default application (browser for `.html`, etc.).
pub fn is_external_file(target: &str) -> bool {
    let lower = target.to_lowercase();
    // Strip heading fragment before checking extension.
    let stem = lower.split('#').next().unwrap_or(&lower);
    matches!(
        stem.rsplit('.').next(),
        Some("html" | "htm" | "pdf" | "png" | "jpg" | "jpeg" | "svg" | "gif"
           | "mp4" | "mp3" | "wav" | "ogg" | "webm" | "webp" | "xlsx" | "docx"
           | "pptx" | "csv" | "zip" | "7z" | "tar" | "gz")
    )
}

/// Navigate to a wikilink target.
///
/// Routing logic:
/// - **`.md` targets** (or no extension): resolve via IPC, open in a tab.
/// - **`.html` / media / office files**: build the absolute path inside the
///   vault and open in the user's default OS application (browser, viewer…).
///   This is cross-platform (Windows / macOS / Linux) via `tauri-plugin-shell`.
///
/// Accepts `vault_path` directly to avoid calling `use_context` inside an
/// async task (which runs outside the Leptos reactive tree and panics).
pub async fn navigate_wikilink(target: &str, tab_mgr: &TabManager, vault_path: &str) {
    // ── External files → open in default OS app ──────────────────────
    if is_external_file(target) {
        // Strip heading fragment for file path construction.
        let file_part = target.split('#').next().unwrap_or(target);
        // Build absolute path: vault_root / target (normalise separators).
        let abs_path = if vault_path.is_empty() {
            file_part.to_string()
        } else {
            let sep = if vault_path.contains('\\') { "\\" } else { "/" };
            format!("{}{}{}", vault_path, sep, file_part.replace('/', sep))
        };
        match ipc::open_in_default_app(&abs_path).await {
            Ok(()) => {
                web_sys::console::log_1(
                    &format!("Opened in default app: {}", abs_path).into(),
                );
            }
            Err(e) => {
                web_sys::console::error_1(
                    &format!("Failed to open '{}': {}", abs_path, e).into(),
                );
            }
        }
        return;
    }

    // ── Markdown notes → resolve and open in editor tab ──────────────
    match ipc::resolve_wikilink(target).await {
        Ok(Some(abs_path)) => {
            // Read the note content then open in a tab.
            match ipc::get_note(&abs_path).await {
                Ok(body) => {
                    // Derive relative path from absolute path and vault root.
                    let vault = vault_path;
                    let rel_path = if !vault.is_empty() && abs_path.starts_with(vault) {
                        let trimmed = &abs_path[vault.len()..];
                        trimmed.trim_start_matches(['/', '\\']).to_string()
                    } else {
                        abs_path.clone()
                    };
                    tab_mgr.open(&abs_path, &rel_path, &body);
                }
                Err(e) => {
                    web_sys::console::warn_1(
                        &format!("Failed to read resolved wikilink '{}': {}", target, e).into(),
                    );
                }
            }
        }
        Ok(None) => {
            web_sys::console::warn_1(
                &format!("Wikilink target not found: [[{}]]", target).into(),
            );
            // TODO Phase 18bis: propose note creation dialog
        }
        Err(e) => {
            web_sys::console::error_1(
                &format!("resolve_wikilink IPC error: {}", e).into(),
            );
        }
    }
}
