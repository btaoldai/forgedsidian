//! Bidirectional Markdown editor with live preview.
//!
//! Two modes:
//! - **Edit**: full-height textarea with debounced auto-save (500ms).
//! - **Preview**: rendered Markdown (read-only). Click to switch to edit.
//!
//! Ctrl+S forces immediate save. Auto-saves on blur.

pub mod markdown;
pub mod wikilink;

#[cfg(test)]
mod tests;

pub use self::markdown::{render_wikilinks, md_to_html, is_markdown};
pub use self::wikilink::{
    find_wikilink_ancestor, extract_wikilink_at_cursor, is_external_file, navigate_wikilink, sleep_ms
};

use crate::app::{AppOptions, AppState};
use crate::ipc;
use crate::tab_manager::TabManager;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::ev;
use wasm_bindgen::JsCast;

/// Editor component: bidirectional Markdown editor with wikilink support.
///
/// State:
/// - `is_editing`: Toggle between edit/preview modes.
/// - `edited_body`: Current editor text.
/// - `is_saving`, `save_status`, `save_status_kind`: Save UI feedback.
/// - `has_unsaved`: Tracks unsaved changes.
/// - `is_md_file`: Whether current file is Markdown (affects default preview mode).
/// - `save_version`: Debounce counter to prevent redundant saves.
///
/// Effects:
/// - On note change: load body, reset UI state.
/// - On body change: update unsaved flag.
///
/// Events:
/// - Input: debounced auto-save via `debounced_save` (500ms default).
/// - Blur: immediate save.
/// - Ctrl+S: immediate save.
/// - Ctrl+Click on wikilink in edit mode: navigate via IPC.
/// - Click on rendered wikilink: navigate via IPC.
#[component]
pub fn Editor() -> impl IntoView {
    let state = use_context::<AppState>().expect("AppState not found");
    let tab_mgr = use_context::<TabManager>().expect("TabManager not found");
    let opts = use_context::<AppOptions>().expect("AppOptions not found");

    // Default to preview mode for .md files (Obsidian-like: reading view)
    let is_editing = RwSignal::new(false);
    let edited_body = RwSignal::new(String::new());
    let is_saving = RwSignal::new(false);
    let save_status = RwSignal::new(String::new());
    // CSS modifier: "ok" | "error" | "" — drives .forge-editor__save-status--*
    let save_status_kind = RwSignal::new(String::new());
    let has_unsaved = RwSignal::new(false);
    let is_md_file = RwSignal::new(true);

    // Debounce counter
    let save_version = RwSignal::new(0u32);

    // When note changes, load its body.
    Effect::new(move |_| {
        if let Some(note_id) = state.current_note.get() {
            let body = state.note_content.get_untracked();
            edited_body.set(body);
            is_md_file.set(is_markdown(&note_id));
            // Non-md files always open in edit mode; .md respects user pref.
            if is_markdown(&note_id) {
                is_editing.set(opts.default_edit_mode.get_untracked());
            } else {
                is_editing.set(true);
            }
            save_status.set(String::new());
            save_status_kind.set(String::new());
            has_unsaved.set(false);
            save_version.set(save_version.get_untracked() + 100);
        }
    });

    // Detect unsaved changes
    Effect::new(move |_| {
        let original = state.note_content.get();
        let current = edited_body.get();
        has_unsaved.set(current != original);
    });

    // Compute note_id for save operations (avoids reactive subscription in closures).
    let get_note_id = move || -> Option<String> {
        state.current_note.get_untracked()
    };

    // Immediate save
    let do_save = move |note_id: String, body: String| {
        if body != state.note_content.get_untracked() {
            is_saving.set(true);
            spawn_local(async move {
                match ipc::save_note(&note_id, &body).await {
                    Ok(()) => {
                        state.note_content.set(body.clone());
                        tab_mgr.mark_active_saved(body);
                        save_status.set("Saved".to_string());
                        save_status_kind.set("ok".to_string());
                        has_unsaved.set(false);
                        // Auto-clear after animation completes (2.5s).
                        // Guard with try_get_untracked: if the component was
                        // disposed (note switched), the signal no longer exists
                        // and we must skip the update to avoid a WASM panic.
                        spawn_local(async move {
                            sleep_ms(2500).await;
                            let still_ok = save_status_kind
                                .try_get_untracked()
                                .is_some_and(|v| v == "ok");
                            if still_ok {
                                save_status.set(String::new());
                                save_status_kind.set(String::new());
                            }
                        });
                    }
                    Err(e) => {
                        save_status.set(format!("Error: {}", e));
                        save_status_kind.set("error".to_string());
                    }
                }
                is_saving.set(false);
            });
        }
    };

    // Debounced save — respects auto_save toggle and delay from settings.
    let debounced_save = move |note_id: String, body: String| {
        if !opts.auto_save.get_untracked() {
            return; // auto-save disabled by user
        }
        let ver = save_version.get_untracked() + 1;
        save_version.set(ver);
        let delay = opts.auto_save_delay_ms.get_untracked() as i32;

        spawn_local(async move {
            sleep_ms(delay).await;
            // Guard against disposed signals (component unmounted).
            let current_ver = save_version.try_get_untracked().unwrap_or(0);
            if current_ver == ver {
                do_save(note_id, body);
            }
        });
    };

    view! {
        <article class="forge-editor">
            // Header: filename + unsaved indicator
            <header class="forge-editor__header" style="display:flex;align-items:center;justify-content:space-between;padding:6px 16px;border-bottom:1px solid var(--trl-abyss-light);flex-shrink:0;">
                <div style="display:flex;align-items:center;gap:8px;">
                    {move || {
                        if let Some(id) = state.current_note.get() {
                            let display = id
                                .rsplit(|c| c == '/' || c == '\\')
                                .next()
                                .unwrap_or(&id)
                                .to_string();
                            view! {
                                <span class="forge-editor__breadcrumb" style="font-size:13px;color:var(--trl-text-secondary);">{display}</span>
                            }.into_any()
                        } else {
                            view! {
                                <span class="forge-editor__breadcrumb" style="font-size:13px;color:var(--trl-text-tertiary);">"(no note)"</span>
                            }.into_any()
                        }
                    }}
                    {move || {
                        if has_unsaved.get() {
                            view! {
                                <span style="color:var(--trl-alert);font-size:11px;">"(modified)"</span>
                            }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }
                    }}
                    {move || {
                        let status = save_status.get();
                        let kind = save_status_kind.get();
                        if !status.is_empty() {
                            let cls = format!(
                                "forge-editor__save-status forge-editor__save-status--{}",
                                kind
                            );
                            view! {
                                <span class=cls>{status}</span>
                            }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }
                    }}
                </div>

                // Mode toggle button
                <div style="display:flex;gap:4px;">
                    {move || {
                        if state.current_note.get().is_some() {
                            view! {
                                <button
                                    class="forge-btn forge-btn--small"
                                    style="font-size:12px;padding:3px 10px;"
                                    on:click=move |_| {
                                        // Before switching to preview, save if needed
                                        if is_editing.get() {
                                            if let Some(note_id) = get_note_id() {
                                                let body = edited_body.get_untracked();
                                                do_save(note_id, body);
                                            }
                                        }
                                        is_editing.set(!is_editing.get_untracked());
                                    }
                                >
                                    {move || if is_editing.get() { "Preview" } else { "Edit" }}
                                </button>
                            }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }
                    }}
                </div>
            </header>

            // Content area: fills remaining height
            <div class="forge-editor__content">
                {move || {
                    if state.current_note.get().is_none() {
                        return view! {
                            <div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--trl-text-tertiary);">
                                <p>"Select a file from the sidebar"</p>
                            </div>
                        }.into_any();
                    }

                    if is_editing.get() {
                        // EDIT MODE: full-height textarea
                        let wrap_val = if opts.word_wrap.get() { "soft" } else { "off" };
                        let nowrap_style = if opts.word_wrap.get() {
                            ""
                        } else {
                            "white-space:pre;overflow-x:auto;"
                        };
                        view! {
                            <textarea
                                class="forge-editor__textarea"
                                wrap=wrap_val
                                style=nowrap_style
                                prop:value=move || edited_body.get()
                                on:input=move |e| {
                                    let value = event_target_value(&e);
                                    let body = value.clone();
                                    edited_body.set(value);
                                    has_unsaved.set(true);
                                    // Sync edit to the active tab's content cache.
                                    tab_mgr.update_active_content(body.clone());

                                    if let Some(note_id) = get_note_id() {
                                        debounced_save(note_id, body);
                                    }
                                }
                                on:blur=move |_| {
                                    if let Some(note_id) = get_note_id() {
                                        let body = edited_body.get_untracked();
                                        do_save(note_id, body);
                                    }
                                }
                                on:keydown=move |e: ev::KeyboardEvent| {
                                    if e.ctrl_key() && e.key() == "s" {
                                        e.prevent_default();
                                        if let Some(note_id) = get_note_id() {
                                            let body = edited_body.get_untracked();
                                            do_save(note_id, body);
                                        }
                                    }
                                }
                                on:click=move |e: ev::MouseEvent| {
                                    // Ctrl+Click on a [[wikilink]] in edit mode navigates.
                                    if !e.ctrl_key() {
                                        return;
                                    }
                                    // Get cursor position in textarea and check if it's inside a [[...]]
                                    if let Some(target) = e.target() {
                                        let textarea: Result<web_sys::HtmlTextAreaElement, _> = target.dyn_into();
                                        if let Ok(ta) = textarea {
                                            if let Ok(Some(pos)) = ta.selection_start() {
                                                let body = edited_body.get_untracked();
                                                if let Some(wiki_target) = extract_wikilink_at_cursor(&body, pos as usize) {
                                                    e.prevent_default();
                                                    let tab_mgr_clone = tab_mgr.clone();
                                                    let vp = state.vault_path.get_untracked();
                                                    spawn_local(async move {
                                                        navigate_wikilink(&wiki_target, &tab_mgr_clone, &vp).await;
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            />
                        }.into_any()
                    } else if is_md_file.get() {
                        // PREVIEW MODE for .md: rendered Markdown with clickable wikilinks.
                        // Wikilinks are rendered as <a class="forge-wikilink" data-target="...">.
                        // Clicking a wikilink navigates to the target note.
                        // Edit mode is toggled ONLY via the header button.
                        let eb = edited_body.get();
                        let nc = state.note_content.get();
                        let raw = if eb.is_empty() && !nc.is_empty() { nc } else { eb };
                        let rendered = md_to_html(&raw);
                        view! {
                            <div
                                class="forge-editor__rendered"
                                inner_html=rendered
                                on:click=move |e: ev::MouseEvent| {
                                    // Only handle wikilink clicks — edit toggle is via button.
                                    // Note: e.target() may return a Text node (nodeType 3)
                                    // when clicking on text inside an <a>. We must handle
                                    // both Element and Text node cases.
                                    if let Some(target) = e.target() {
                                        let maybe_el: Option<web_sys::HtmlElement> = {
                                            // Try direct cast to HtmlElement first.
                                            let direct: Result<web_sys::HtmlElement, _> =
                                                target.clone().dyn_into();
                                            if let Ok(el) = direct {
                                                Some(el)
                                            } else {
                                                // Likely a Text node — get its parentElement.
                                                let node: Result<web_sys::Node, _> = target.dyn_into();
                                                node.ok().and_then(|n| n.parent_element())
                                                    .and_then(|p| p.dyn_into::<web_sys::HtmlElement>().ok())
                                            }
                                        };
                                        if let Some(el) = maybe_el {
                                            // Walk up from click target to find a .forge-wikilink
                                            let wikilink_el = find_wikilink_ancestor(&el);
                                            if let Some(link_el) = wikilink_el {
                                                e.prevent_default();
                                                e.stop_propagation();
                                                if let Some(wiki_target) = link_el.get_attribute("data-target") {
                                                    // Navigate to the wikilink target.
                                                    let tab_mgr_clone = tab_mgr.clone();
                                                    let vp = state.vault_path.get_untracked();
                                                    spawn_local(async move {
                                                        navigate_wikilink(&wiki_target, &tab_mgr_clone, &vp).await;
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            />
                        }.into_any()
                    } else {
                        // PREVIEW MODE for non-md: show source with syntax hint.
                        let eb = edited_body.get();
                        let nc = state.note_content.get();
                        let raw = if eb.is_empty() && !nc.is_empty() { nc } else { eb };
                        view! {
                            <pre
                                class="forge-editor__raw"
                                style="margin:0;height:100%;padding:1.5em 2em;"
                            >
                                {raw}
                            </pre>
                        }.into_any()
                    }
                }}
            </div>
        </article>
    }
}
