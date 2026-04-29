//! `<TagsPanel />` — tag browser panel for the vault sidebar.
//!
//! Displays all vault tags as interactive pills fetched via [`crate::ipc::list_vault_tags`].
//! Clicking a tag loads the matching notes via [`crate::ipc::notes_by_tag`] and renders
//! them as a clickable list. Clicking a note calls the `on_note_click` callback.
//!
//! # Reactive behaviour
//! - Tags are (re-)loaded whenever `AppState::vault_path` changes (new vault opened or
//!   app mount with empty vault).
//! - Selecting a tag clears any previous note list, then fetches new results asynchronously.
//! - A "× Clear" button resets the active tag filter without re-fetching all tags.

use crate::app::AppState;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Sidebar tags panel — browse all vault tags and filter notes by tag.
///
/// # Props
/// - `on_note_click` — callback invoked with the relative note path when the user clicks a note.
#[component]
pub fn TagsPanel(
    /// Called with the relative note path when a note is selected.
    on_note_click: impl Fn(String) + Clone + Send + 'static,
) -> impl IntoView {
    let state = use_context::<AppState>().expect("AppState must be provided by <App />");

    // All distinct tags in the vault, sorted alphabetically.
    let tags = RwSignal::new(Vec::<String>::new());
    // Currently active tag filter, if any.
    let selected_tag = RwSignal::new(Option::<String>::None);
    // Notes belonging to the active tag.
    let tag_notes = RwSignal::new(Vec::<String>::new());
    // True while fetching notes for a selected tag.
    let panel_loading = RwSignal::new(false);

    // Reload tag list whenever the vault changes (covers initial mount + vault switch).
    // No reactive deps other than vault_path → runs once on mount, then on vault change.
    Effect::new(move |_| {
        let _vault = state.vault_path.get(); // track: re-run when vault opens/switches
        // Reset filter on vault change so stale state from previous vault is cleared.
        selected_tag.set(None);
        tag_notes.set(Vec::new());
        spawn_local(async move {
            let result = crate::ipc::list_vault_tags().await;
            tags.set(result);
        });
    });

    // Handler: user clicks a tag pill — fetch its notes.
    let on_tag_click = move |tag: String| {
        selected_tag.set(Some(tag.clone()));
        panel_loading.set(true);
        tag_notes.set(Vec::new());
        spawn_local(async move {
            let notes = crate::ipc::notes_by_tag(&tag).await;
            tag_notes.set(notes);
            panel_loading.set(false);
        });
    };

    // Handler: clear the active tag filter.
    let on_clear_filter = move |_| {
        selected_tag.set(None);
        tag_notes.set(Vec::new());
    };

    view! {
        <div class="forge-tags-panel" style="padding:4px 8px 8px 8px;">

            // ── Panel header ──────────────────────────────────────────────────────────
            <div style="display:flex;align-items:center;justify-content:space-between;padding:4px 0 6px 0;">
                <span style="font-size:11px;color:var(--trl-text-tertiary);font-weight:600;\
                             text-transform:uppercase;letter-spacing:0.05em;">
                    {move || {
                        let n = tags.get().len();
                        if n == 0 { "Tags".to_string() } else { format!("Tags ({})", n) }
                    }}
                </span>
                {move || {
                    if selected_tag.get().is_some() {
                        view! {
                            <button
                                style="font-size:10px;color:var(--trl-text-secondary);background:none;border:none;\
                                       cursor:pointer;padding:2px 6px;border-radius:3px;"
                                on:click=on_clear_filter
                            >
                                "\u{00D7} Clear"
                            </button>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }}
            </div>

            // ── Tag pills ─────────────────────────────────────────────────────────────
            {move || {
                let tag_list = tags.get();
                let vp = state.vault_path.get();

                if vp.is_empty() {
                    return view! {
                        <p style="font-size:12px;color:var(--trl-abyss-mid);padding:4px 0;margin:0;">
                            "No vault open"
                        </p>
                    }.into_any();
                }
                if tag_list.is_empty() {
                    return view! {
                        <p style="font-size:12px;color:var(--trl-abyss-mid);padding:4px 0;margin:0;">
                            "No tags found in this vault"
                        </p>
                    }.into_any();
                }

                let active = selected_tag.get();
                let on_tag = on_tag_click.clone();

                view! {
                    <div
                        class="forge-tags-panel__pills"
                        style="display:flex;flex-wrap:wrap;gap:4px;margin-bottom:8px;"
                    >
                        <For each=move || tag_list.clone() key=|t| t.clone() let:tag>
                            {
                                let is_active = active.as_deref() == Some(tag.as_str());
                                let on_tag = on_tag.clone();
                                let pill_style = if is_active {
                                    "padding:2px 8px;font-size:11px;border-radius:12px;\
                                     cursor:pointer;background:var(--trl-cyan);color:#fff;\
                                     border:1px solid var(--trl-cyan);white-space:nowrap;".to_string()
                                } else {
                                    "padding:2px 8px;font-size:11px;border-radius:12px;\
                                     cursor:pointer;background:var(--trl-abyss-light);color:var(--trl-cyan);\
                                     border:1px solid var(--trl-cyan);white-space:nowrap;".to_string()
                                };
                                view! {
                                    <button
                                        class="forge-tag-pill"
                                        style=pill_style
                                        on:click=move |_| on_tag(tag.clone())
                                    >
                                        {format!("#{}", tag)}
                                    </button>
                                }
                            }
                        </For>
                    </div>
                }.into_any()
            }}

            // ── Notes for selected tag ────────────────────────────────────────────────
            {move || {
                let Some(tag_name) = selected_tag.get() else {
                    return view! { <></> }.into_any();
                };

                if panel_loading.get() {
                    return view! {
                        <p style="font-size:12px;color:var(--trl-abyss-mid);padding:4px 0;margin:0;">
                            "Loading..."
                        </p>
                    }.into_any();
                }

                let notes = tag_notes.get();

                if notes.is_empty() {
                    return view! {
                        <p style="font-size:12px;color:var(--trl-abyss-mid);padding:4px 0;margin:0;">
                            {format!("No notes tagged #{}", tag_name)}
                        </p>
                    }.into_any();
                }

                let on_click = on_note_click.clone();
                let note_count = notes.len();
                view! {
                    <div>
                        <p style="font-size:10px;color:var(--trl-text-tertiary);margin:0 0 6px 0;">
                            {format!("{} note(s) — #{}", note_count, tag_name)}
                        </p>
                        <ul style="list-style:none;margin:0;padding:0;">
                            <For each=move || notes.clone() key=|p| p.clone() let:path>
                                {
                                    let on_click = on_click.clone();
                                    let display = {
                                        let name = path
                                            .rsplit(['/', '\\'])
                                            .next()
                                            .unwrap_or(path.as_str());
                                        name.strip_suffix(".md").unwrap_or(name).to_string()
                                    };
                                    view! {
                                        <li>
                                            <button
                                                class="forge-sidebar__button"
                                                style="width:100%;text-align:left;padding:4px 6px;\
                                                       font-size:12px;background:none;border:none;\
                                                       cursor:pointer;color:var(--trl-text);border-radius:3px;\
                                                       display:block;"
                                                on:click=move |_| on_click(path.clone())
                                            >
                                                {display}
                                            </button>
                                        </li>
                                    }
                                }
                            </For>
                        </ul>
                    </div>
                }.into_any()
            }}

        </div>
    }
}
