//! Command Palette overlay — triggered by Ctrl+P.
//!
//! Renders a fuzzy-search modal over all notes in the current vault.
//! The user can type a query to filter results, navigate with ArrowUp/ArrowDown,
//! confirm with Enter, or dismiss with Escape or a backdrop click.

use crate::app::AppState;
use crate::tab_manager::TabManager;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Extracts the display title from a vault-relative path (strips dirs and `.md` extension).
fn note_title(path: &str) -> &str {
    let name = path
        .rsplit(|c: char| c == '/' || c == '\\')
        .next()
        .unwrap_or(path);
    name.strip_suffix(".md").unwrap_or(name)
}

/// Builds the absolute filesystem path from vault root + relative path.
fn abs_path(vault_root: &str, rel: &str) -> String {
    let sep = if vault_root.contains('\\') { "\\" } else { "/" };
    format!("{}{}{}", vault_root, sep, rel)
}

/// Command palette modal component.
///
/// Reads [`AppState`] and [`TabManager`] from context.
/// Closes itself by setting `state.show_command_palette` to `false`.
#[component]
pub fn CommandPalette() -> impl IntoView {
    let state = use_context::<AppState>().expect("AppState context required");
    let tab_mgr = use_context::<TabManager>().expect("TabManager context required");

    let query: RwSignal<String> = RwSignal::new(String::new());
    let selected_index: RwSignal<usize> = RwSignal::new(0);
    let input_ref: NodeRef<leptos::html::Input> = NodeRef::new();

    // Auto-focus the search input when the palette mounts.
    Effect::new(move |_| {
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
        }
    });

    // Filtered note list — max 10 results, case-insensitive title match.
    let filtered = move || {
        let q = query.get().to_lowercase();
        let notes = state.note_list.get();
        if q.is_empty() {
            notes.into_iter().take(10).collect::<Vec<_>>()
        } else {
            notes
                .into_iter()
                .filter(|p| note_title(p).to_lowercase().contains(&q))
                .take(10)
                .collect::<Vec<_>>()
        }
    };

    // Opens a note: fetches content via IPC and opens (or activates) its tab.
    let open_note = {
        let state = state.clone();
        let tab_mgr = tab_mgr.clone();
        move |rel: String| {
            let state = state.clone();
            let tab_mgr = tab_mgr.clone();
            let vault_root = state.vault_path.get_untracked();
            let full_path = abs_path(&vault_root, &rel);
            spawn_local(async move {
                match crate::ipc::get_note(&full_path).await {
                    Ok(content) => {
                        tab_mgr.open(&full_path, &rel, &content);
                        state.active_view.set(crate::app::ActiveView::Editor);
                        state.show_command_palette.set(false);
                    }
                    Err(e) => {
                        leptos::logging::warn!("[forge] command palette open_note failed: {}", e);
                    }
                }
            });
        }
    };

    // Keyboard handler for the modal div (arrow navigation, Enter, Escape).
    let open_note_kbd = open_note.clone();
    let on_modal_keydown = move |ev: web_sys::KeyboardEvent| {
        match ev.key().as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                let len = filtered().len();
                selected_index.update(|i| {
                    *i = (*i + 1).min(len.saturating_sub(1));
                });
            }
            "ArrowUp" => {
                ev.prevent_default();
                selected_index.update(|i| {
                    *i = i.saturating_sub(1);
                });
            }
            "Enter" => {
                ev.prevent_default();
                let results = filtered();
                let idx = selected_index.get_untracked();
                if let Some(path) = results.into_iter().nth(idx) {
                    open_note_kbd(path);
                }
            }
            "Escape" => {
                ev.prevent_default();
                state.show_command_palette.set(false);
            }
            _ => {}
        }
    };

    let state_close = state.clone();

    view! {
        // Semi-transparent backdrop — clicking it closes the palette.
        <div
            style="position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.6);z-index:1000;display:flex;align-items:flex-start;justify-content:center;padding-top:20vh;"
            on:mousedown=move |ev| {
                // Close only when the backdrop itself is clicked, not the modal.
                if ev.target() == ev.current_target() {
                    state_close.show_command_palette.set(false);
                }
            }
        >
            // Modal container — intercepts keyboard events.
            <div
                style="background:var(--trl-abyss);border:1px solid var(--trl-abyss-light);border-radius:8px;width:540px;max-width:90vw;overflow:hidden;box-shadow:0 20px 60px rgba(0,0,0,0.5);"
                on:keydown=on_modal_keydown
            >
                // Search input
                <input
                    node_ref=input_ref
                    type="text"
                    placeholder="Search notes..."
                    style="width:100%;background:var(--trl-abyss-deep);color:var(--trl-text);border:none;border-bottom:1px solid var(--trl-abyss-light);padding:14px 16px;font-size:16px;outline:none;box-sizing:border-box;"
                    on:input=move |ev| {
                        let val = event_target_value(&ev);
                        query.set(val);
                        // Reset selection when query changes.
                        selected_index.set(0);
                    }
                    prop:value=move || query.get()
                />

                // Results list
                <div style="max-height:400px;overflow-y:auto;">
                    {move || {
                        let results = filtered();
                        let open_note_item = open_note.clone();
                        results
                            .into_iter()
                            .enumerate()
                            .map(|(idx, path)| {
                                let title = note_title(&path).to_string();
                                let path_clone = path.clone();
                                let open_note_item = open_note_item.clone();
                                let is_selected = move || selected_index.get() == idx;
                                view! {
                                    <div
                                        style=move || {
                                            let bg = if is_selected() { "background:var(--trl-cyan);" } else { "" };
                                            format!(
                                                "padding:10px 16px;cursor:pointer;color:var(--trl-text);font-size:13px;{}",
                                                bg
                                            )
                                        }
                                        on:mouseenter=move |_| {
                                            selected_index.set(idx);
                                        }
                                        on:mousedown=move |ev| {
                                            ev.prevent_default();
                                            open_note_item(path_clone.clone());
                                        }
                                    >
                                        {title}
                                    </div>
                                }
                            })
                            .collect_view()
                    }}
                </div>

                // Footer keyboard hints
                <div style="padding:8px 16px;font-size:11px;color:var(--trl-text-tertiary);border-top:1px solid var(--trl-abyss-light);">
                    "Enter to open  |  Arrows to navigate  |  Esc to close"
                </div>
            </div>
        </div>
    }
}
