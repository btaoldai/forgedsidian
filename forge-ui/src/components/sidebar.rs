//! `<Sidebar />` — vault file tree with hierarchical folder navigation.
//!
//! Displays vault notes organized by folder structure with collapse/expand.
//! Search mode shows a flat list of matching results.
//! Includes actions to create new notes and folders.
//!
//! The sidebar has two tabs: **Files** (hierarchical folder tree) and **Tags**
//! (tag browser via [`crate::components::tags_panel::TagsPanel`]).

use crate::app::{AppOptions, AppState};
use crate::components::folder_tree::{file_name_from_path, split_path, FolderTree};
use crate::tab_manager::TabManager;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Which panel is active in the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidebarTab {
    /// Hierarchical file/folder tree.
    Files,
    /// Tag browser — all vault tags as pills, note list on selection.
    Tags,
}

/// Sidebar content — rendered inside the `<aside>` managed by App.
#[component]
pub fn SidebarContent() -> impl IntoView {
    let state = use_context::<AppState>().expect("AppState must be provided by <App />");
    let tab_mgr = use_context::<TabManager>().expect("TabManager must be provided by <App />");
    let opts = use_context::<AppOptions>().expect("AppOptions must be provided by <App />");

    // Active sidebar tab: Files (default) or Tags.
    let sidebar_tab = RwSignal::new(SidebarTab::Files);

    // Signals for create dialogs
    let show_new_note = RwSignal::new(false);
    let show_new_folder = RwSignal::new(false);
    let new_name = RwSignal::new(String::new());
    let create_error = RwSignal::new(String::new());

    // Reset search when the note list changes (new vault opened).
    Effect::new(move |_| {
        let _ = state.note_list.get();
        state.search_query.set(String::new());
        state.search_results.set(Vec::new());
    });

    // Handler for search input.
    let on_search_input = move |ev: leptos::ev::Event| {
        let query = event_target_value(&ev);
        state.search_query.set(query.clone());

        if query.is_empty() {
            state.search_results.set(Vec::new());
            return;
        }

        let vault_path = state.vault_path.get();
        if vault_path.is_empty() {
            return;
        }

        spawn_local(async move {
            match crate::ipc::search_notes(&query).await {
                Ok(results) => state.search_results.set(results),
                Err(e) => {
                    leptos::logging::warn!("Search failed: {}", e);
                    state.search_results.set(Vec::new());
                }
            }
        });
    };

    // File extensions that can be opened as text in the editor.
    let is_text_file = |path: &str| -> bool {
        let lower = path.to_lowercase();
        lower.ends_with(".md")
            || lower.ends_with(".txt")
            || lower.ends_with(".rs")
            || lower.ends_with(".py")
            || lower.ends_with(".js")
            || lower.ends_with(".ts")
            || lower.ends_with(".jsx")
            || lower.ends_with(".tsx")
            || lower.ends_with(".html")
            || lower.ends_with(".css")
            || lower.ends_with(".json")
            || lower.ends_with(".toml")
            || lower.ends_with(".yaml")
            || lower.ends_with(".yml")
            || lower.ends_with(".xml")
            || lower.ends_with(".sh")
            || lower.ends_with(".bash")
            || lower.ends_with(".zsh")
            || lower.ends_with(".fish")
            || lower.ends_with(".ps1")
            || lower.ends_with(".bat")
            || lower.ends_with(".cmd")
            || lower.ends_with(".c")
            || lower.ends_with(".h")
            || lower.ends_with(".cpp")
            || lower.ends_with(".hpp")
            || lower.ends_with(".java")
            || lower.ends_with(".go")
            || lower.ends_with(".rb")
            || lower.ends_with(".php")
            || lower.ends_with(".sql")
            || lower.ends_with(".lua")
            || lower.ends_with(".zig")
            || lower.ends_with(".nix")
            || lower.ends_with(".env")
            || lower.ends_with(".gitignore")
            || lower.ends_with(".dockerignore")
            || lower.ends_with(".editorconfig")
            || lower.ends_with(".csv")
            || lower.ends_with(".ini")
            || lower.ends_with(".cfg")
            || lower.ends_with(".conf")
            || lower.ends_with(".log")
    };

    // Handler for file selection (notes + text files).
    // Opens the file in a tab via TabManager (creates or switches to existing tab).
    let on_note_click = move |rel_path: String| {
        let vault_root = state.vault_path.get();
        let abs_path = if rel_path.contains(':') || rel_path.starts_with('/') || rel_path.starts_with('\\') {
            rel_path.clone()
        } else {
            let sep = if vault_root.contains('\\') { "\\" } else { "/" };
            format!("{}{}{}", vault_root, sep, rel_path)
        };

        if !is_text_file(&rel_path) {
            // Non-text file: show info in editor via a tab.
            let ext = rel_path.rsplit('.').next().unwrap_or("unknown");
            let content = format!(
                "# {}\n\n> Binary or unsupported file format (`.{}`)\n>\n> This file cannot be previewed in the editor yet.\n> Path: `{}`",
                file_name_from_path(&rel_path),
                ext,
                rel_path
            );
            tab_mgr.open(&abs_path, &rel_path, &content);
            state.active_view.set(crate::app::ActiveView::Editor);
            return;
        }

        // Check if already open in a tab — switch without IPC fetch.
        {
            let tabs = tab_mgr.tabs.get_untracked();
            if let Some(existing) = tabs.iter().find(|t| t.file_path == abs_path) {
                tab_mgr.activate(existing.id);
                state.active_view.set(crate::app::ActiveView::Editor);
                return;
            }
        }

        let is_md = rel_path.to_lowercase().ends_with(".md");
        let rel_for_tab = rel_path.clone();
        spawn_local(async move {
            // Use get_note for .md files (reads from index), read_file for others.
            let result = if is_md {
                crate::ipc::get_note(&abs_path).await
            } else {
                crate::ipc::read_file(&abs_path).await
            };

            match result {
                Ok(content) => {
                    tab_mgr.open(&abs_path, &rel_for_tab, &content);
                    state.active_view.set(crate::app::ActiveView::Editor);
                }
                Err(e) => {
                    leptos::logging::warn!("Failed to load file: {}", e);
                }
            }
        });
    };

    let note_display_name = move |path: &str| -> String {
        let name = file_name_from_path(path);
        if opts.show_extensions.get_untracked() {
            name.to_string()
        } else {
            // Remove .md extension for display
            name.strip_suffix(".md").unwrap_or(name).to_string()
        }
    };

    // Handler: reload note list (refresh from disk).
    let handle_reload = move |_| {
        state.is_loading.set(true);
        state.loading_status.set("Refreshing note list...".into());
        spawn_local(async move {
            match crate::ipc::list_notes().await {
                Ok(notes) => {
                    state.loading_status.set(format!("Loaded {} notes", notes.len()));
                    state.note_list.set(notes);
                }
                Err(e) => {
                    leptos::logging::warn!("reload list_notes failed: {}", e);
                    state.loading_status.set(format!("Reload failed: {}", e));
                }
            }
            // Also refresh all files + folder list.
            match crate::ipc::list_all_files().await {
                Ok(files) => state.all_files.set(files),
                Err(e) => leptos::logging::warn!("reload list_all_files failed: {}", e),
            }
            match crate::ipc::list_folders().await {
                Ok(folders) => state.folder_list.set(folders),
                Err(e) => leptos::logging::warn!("reload list_folders failed: {}", e),
            }
            state.is_loading.set(false);
        });
    };

    // Handler: switch vault via native folder picker.
    let handle_switch_vault = move |_| {
        state.is_loading.set(true);
        state.loading_status.set("Opening folder picker...".into());
        spawn_local(async move {
            match crate::ipc::pick_and_open_vault().await {
                Ok(Some(path)) => {
                    state.loading_status.set(format!("Loading vault: {}...", &path));
                    state.vault_path.set(path);
                    state.search_query.set(String::new());
                    state.search_results.set(Vec::new());
                    // Clear all open tabs when switching vault.
                    tab_mgr.tabs.set(Vec::new());
                    tab_mgr.active_tab_id.set(None);
                    state.loading_status.set("Fetching note list...".into());
                    match crate::ipc::list_notes().await {
                        Ok(notes) => {
                            state.loading_status.set(format!("Loaded {} notes", notes.len()));
                            state.note_list.set(notes);
                        }
                        Err(e) => leptos::logging::warn!("list_notes failed: {}", e),
                    }
                    match crate::ipc::list_all_files().await {
                        Ok(files) => state.all_files.set(files),
                        Err(e) => leptos::logging::warn!("list_all_files failed: {}", e),
                    }
                    match crate::ipc::list_folders().await {
                        Ok(folders) => state.folder_list.set(folders),
                        Err(e) => leptos::logging::warn!("list_folders failed: {}", e),
                    }
                    state.is_loading.set(false);
                }
                Ok(None) => {
                    state.is_loading.set(false);
                    state.loading_status.set(String::new());
                }
                Err(e) => {
                    leptos::logging::warn!("switch vault failed: {}", e);
                    state.is_loading.set(false);
                }
            }
        });
    };

    // Handler: create new note
    let handle_create_note = move |_| {
        let name = new_name.get();
        if name.trim().is_empty() {
            create_error.set("Name cannot be empty".into());
            return;
        }
        create_error.set(String::new());

        spawn_local(async move {
            match crate::ipc::create_note("", &name).await {
                Ok(rel_path) => {
                    show_new_note.set(false);
                    new_name.set(String::new());
                    // Refresh all file lists (note_list alone is insufficient
                    // because the tree view reads all_files + folder_list).
                    if let Ok(notes) = crate::ipc::list_notes().await {
                        state.note_list.set(notes);
                    }
                    if let Ok(files) = crate::ipc::list_all_files().await {
                        state.all_files.set(files);
                    }
                    if let Ok(folders) = crate::ipc::list_folders().await {
                        state.folder_list.set(folders);
                    }
                    // Open the newly created note in a tab.
                    let vault_root = state.vault_path.get_untracked();
                    let sep = if vault_root.contains('\\') { "\\" } else { "/" };
                    let abs_path = format!("{}{}{}", vault_root, sep, rel_path);
                    if let Ok(content) = crate::ipc::get_note(&abs_path).await {
                        tab_mgr.open(&abs_path, &rel_path, &content);
                        state.active_view.set(crate::app::ActiveView::Editor);
                    }
                }
                Err(e) => {
                    create_error.set(e);
                }
            }
        });
    };

    // Handler: create new folder
    let handle_create_folder = move |_| {
        let name = new_name.get();
        if name.trim().is_empty() {
            create_error.set("Name cannot be empty".into());
            return;
        }
        create_error.set(String::new());

        spawn_local(async move {
            match crate::ipc::create_folder("", &name).await {
                Ok(_rel_path) => {
                    show_new_folder.set(false);
                    new_name.set(String::new());
                    // Refresh note list + folder list
                    if let Ok(notes) = crate::ipc::list_notes().await {
                        state.note_list.set(notes);
                    }
                    if let Ok(folders) = crate::ipc::list_folders().await {
                        state.folder_list.set(folders);
                    }
                }
                Err(e) => {
                    create_error.set(e);
                }
            }
        });
    };

    // Cancel dialog
    let handle_cancel = move |_| {
        show_new_note.set(false);
        show_new_folder.set(false);
        new_name.set(String::new());
        create_error.set(String::new());
    };

    // Subscribe to vault file-system events emitted by the Tauri watcher.
    // On any change or removal, refresh all file lists so the tree stays in sync.
    // Also reload the editor content if the changed file is currently open.
    // The listener is registered once on mount (Effect runs once with no deps).
    {
        let note_list = state.note_list;
        let all_files = state.all_files;
        let folder_list = state.folder_list;
        let current_note = state.current_note;
        let note_content = state.note_content;
        let tab_mgr = tab_mgr;
        Effect::new(move |_| {
            spawn_local(async move {
                crate::ipc::listen_vault_events(move |changed_path| {
                    let note_list = note_list;
                    let all_files = all_files;
                    let folder_list = folder_list;
                    let changed_path = changed_path.clone();
                    spawn_local(async move {
                        // Refresh sidebar file lists.
                        if let Ok(notes) = crate::ipc::list_notes().await {
                            note_list.set(notes);
                        }
                        if let Ok(files) = crate::ipc::list_all_files().await {
                            all_files.set(files);
                        }
                        if let Ok(folders) = crate::ipc::list_folders().await {
                            folder_list.set(folders);
                        }

                        // Reload editor content if the changed file is the active tab.
                        // Normalise the path received from the watcher: on Windows,
                        // notify may emit `\\?\C:\...` UNC prefixed paths while the
                        // tab stores the plain `C:\...` form.
                        let normalised = changed_path
                            .strip_prefix(r"\\?\")
                            .unwrap_or(&changed_path)
                            .to_string();

                        // Check if the active tab matches the changed path.
                        let active_matches = current_note
                            .get_untracked()
                            .map(|cn| cn == normalised)
                            .unwrap_or(false);

                        if active_matches {
                            // Guard: do not reload if the user has unsaved edits,
                            // otherwise we would overwrite their in-progress work.
                            // This also prevents a reload loop when Forgedsidian
                            // itself saves a file and the watcher picks it up.
                            let tab_is_modified = tab_mgr.tabs.get_untracked()
                                .iter()
                                .find(|t| t.file_path == normalised)
                                .map(|t| t.modified)
                                .unwrap_or(false);

                            if !tab_is_modified {
                                if let Ok(new_body) = crate::ipc::get_note(&normalised).await {
                                    note_content.set(new_body.clone());
                                    tab_mgr.mark_active_saved(new_body);
                                }
                            }
                        }
                    });
                }).await;
            });
        });
    }

    view! {
        <div class="forge-sidebar__inner">
            <div class="forge-sidebar__search">
                <input
                    type="text"
                    id="forge-search"
                    name="forge-search"
                    placeholder="Search notes..."
                    on:input=on_search_input
                    prop:value=move || state.search_query.get()
                />
            </div>
            <div class="forge-sidebar__vault-switch" style="display:flex;gap:4px;padding:0 8px 4px 8px;">
                <button
                    class="forge-btn forge-btn--small"
                    on:click=handle_switch_vault
                    style="flex:1;padding:4px 8px;font-size:12px;cursor:pointer;background:var(--trl-abyss-light);color:var(--trl-text);border:1px solid var(--trl-abyss-mid);border-radius:4px;"
                >
                    {move || {
                        let vp = state.vault_path.get();
                        if vp.is_empty() {
                            "Open vault...".to_string()
                        } else {
                            let name = file_name_from_path(&vp);
                            format!("{} (switch)", name)
                        }
                    }}
                </button>
                {move || {
                    if state.vault_path.get().is_empty() {
                        view! { <></> }.into_any()
                    } else {
                        view! {
                            <button
                                class="forge-btn forge-btn--small"
                                on:click=handle_reload
                                title="Reload note list"
                                style="padding:4px 8px;font-size:12px;cursor:pointer;background:var(--trl-abyss-light);color:var(--trl-text);border:1px solid var(--trl-abyss-mid);border-radius:4px;"
                            >
                                "Reload"
                            </button>
                        }.into_any()
                    }
                }}
            </div>

            // ── Sidebar tab switcher: Files | Tags ─────────────────────────────────
            {move || {
                if state.vault_path.get().is_empty() {
                    return view! { <></> }.into_any();
                }
                let active_style = "flex:1;padding:4px 8px;font-size:11px;cursor:pointer;\
                                    background:var(--trl-abyss);color:var(--trl-cyan);border:none;\
                                    border-bottom:2px solid var(--trl-cyan);font-weight:600;";
                let inactive_style = "flex:1;padding:4px 8px;font-size:11px;cursor:pointer;\
                                      background:none;color:var(--trl-text-tertiary);border:none;\
                                      border-bottom:2px solid transparent;";
                view! {
                    <div style="display:flex;border-bottom:1px solid var(--trl-abyss);margin:0 0 4px 0;">
                        <button
                            style=move || {
                                if sidebar_tab.get() == SidebarTab::Files {
                                    active_style.to_string()
                                } else {
                                    inactive_style.to_string()
                                }
                            }
                            on:click=move |_| sidebar_tab.set(SidebarTab::Files)
                        >
                            "Files"
                        </button>
                        <button
                            style=move || {
                                if sidebar_tab.get() == SidebarTab::Tags {
                                    active_style.to_string()
                                } else {
                                    inactive_style.to_string()
                                }
                            }
                            on:click=move |_| sidebar_tab.set(SidebarTab::Tags)
                        >
                            "Tags"
                        </button>
                    </div>
                }.into_any()
            }}

            // Action buttons: New Note + New Folder
            {move || {
                if sidebar_tab.get() == SidebarTab::Tags {
                    return view! { <></> }.into_any();
                }
                if state.vault_path.get().is_empty() {
                    view! { <></> }.into_any()
                } else {
                    view! {
                        <div class="forge-sidebar__actions" style="display:flex;gap:4px;padding:0 8px 8px 8px;">
                            <button
                                class="forge-sidebar__action-btn"
                                style="flex:1;padding:3px 6px;font-size:11px;cursor:pointer;background:var(--trl-abyss-light);color:var(--trl-cyan);border:1px solid var(--trl-cyan);border-radius:3px;"
                                on:click=move |_| {
                                    new_name.set(String::new());
                                    create_error.set(String::new());
                                    show_new_folder.set(false);
                                    show_new_note.set(true);
                                }
                            >
                                "+ Note"
                            </button>
                            <button
                                class="forge-sidebar__action-btn"
                                style="flex:1;padding:3px 6px;font-size:11px;cursor:pointer;background:var(--trl-abyss-light);color:var(--trl-cyan);border:1px solid var(--trl-cyan);border-radius:3px;"
                                on:click=move |_| {
                                    new_name.set(String::new());
                                    create_error.set(String::new());
                                    show_new_note.set(false);
                                    show_new_folder.set(true);
                                }
                            >
                                "+ Folder"
                            </button>
                        </div>
                    }.into_any()
                }
            }}

            // Inline create dialog
            {move || {
                if sidebar_tab.get() == SidebarTab::Tags {
                    return view! { <></> }.into_any();
                }
                let showing_note = show_new_note.get();
                let showing_folder = show_new_folder.get();

                if !showing_note && !showing_folder {
                    return view! { <></> }.into_any();
                }

                let label = if showing_note { "New note name:" } else { "New folder name:" };
                let placeholder = if showing_note { "my-note.md" } else { "my-folder" };

                view! {
                    <div class="forge-sidebar__create-dialog" style="padding:6px 8px;background:var(--trl-abyss);border:1px solid var(--trl-abyss-light);border-radius:4px;margin:0 8px 8px 8px;">
                        <label style="font-size:11px;color:var(--trl-text-secondary);display:block;margin-bottom:4px;">{label}</label>
                        <input
                            type="text"
                            placeholder=placeholder
                            prop:value=move || new_name.get()
                            on:input=move |e| new_name.set(event_target_value(&e))
                            on:keydown=move |e: leptos::ev::KeyboardEvent| {
                                if e.key() == "Enter" {
                                    if show_new_note.get() {
                                        handle_create_note(());
                                    } else {
                                        handle_create_folder(());
                                    }
                                } else if e.key() == "Escape" {
                                    show_new_note.set(false);
                                    show_new_folder.set(false);
                                    new_name.set(String::new());
                                    create_error.set(String::new());
                                }
                            }
                            style="width:100%;padding:4px 6px;font-size:12px;background:var(--trl-abyss-deep);color:var(--trl-text);border:1px solid var(--trl-abyss-mid);border-radius:3px;margin-bottom:4px;box-sizing:border-box;"
                        />
                        {move || {
                            let err = create_error.get();
                            if err.is_empty() {
                                view! { <></> }.into_any()
                            } else {
                                view! {
                                    <p style="font-size:10px;color:var(--trl-error);margin:0 0 4px 0;">{err}</p>
                                }.into_any()
                            }
                        }}
                        <div style="display:flex;gap:4px;">
                            <button
                                style="flex:1;padding:3px;font-size:11px;cursor:pointer;background:var(--trl-success);color:#fff;border:none;border-radius:3px;"
                                on:click=move |_| {
                                    if show_new_note.get() {
                                        handle_create_note(());
                                    } else {
                                        handle_create_folder(());
                                    }
                                }
                            >
                                "Create"
                            </button>
                            <button
                                style="flex:1;padding:3px;font-size:11px;cursor:pointer;background:var(--trl-abyss-mid);color:var(--trl-text);border:none;border-radius:3px;"
                                on:click=handle_cancel
                            >
                                "Cancel"
                            </button>
                        </div>
                    </div>
                }.into_any()
            }}

            <nav class="forge-sidebar__notes">
                {move || {
                    // Tags mode — delegate entirely to TagsPanel.
                    if sidebar_tab.get() == SidebarTab::Tags {
                        let on_click = on_note_click.clone();
                        return view! {
                            <crate::components::tags_panel::TagsPanel on_note_click=on_click />
                        }.into_any();
                    }

                    let vault_open = !state.vault_path.get().is_empty();
                    let query = state.search_query.get();
                    let results = state.search_results.get();
                    let all_notes = state.note_list.get();

                    if !vault_open {
                        view! {
                            <p class="forge-sidebar__empty">"No vault open"</p>
                        }
                        .into_any()
                    } else if !query.is_empty() {
                        // Search mode.
                        if results.is_empty() {
                            view! {
                                <p class="forge-sidebar__empty">"No results"</p>
                            }
                            .into_any()
                        } else {
                            let on_click = on_note_click.clone();
                            view! {
                                <ul class="forge-sidebar__list">
                                    <For each=move || results.clone() key=|path| path.clone() let:path>
                                        {
                                            let on_click = on_click.clone();
                                            let display = note_display_name(&path);
                                            view! {
                                                <li class="forge-sidebar__item">
                                                    <button
                                                        class="forge-sidebar__button"
                                                        on:click=move |_| on_click(path.clone())
                                                    >
                                                        {display.clone()}
                                                    </button>
                                                </li>
                                            }
                                        }
                                    </For>
                                </ul>
                            }
                            .into_any()
                        }
                    } else if all_notes.is_empty() {
                        view! {
                            <p class="forge-sidebar__empty">"No notes in vault"</p>
                        }
                        .into_any()
                    } else {
                        // Hierarchical folder tree (lazy rendering).
                        let all_files = state.all_files.get();
                        let folders = state.folder_list.get();
                        let show_hidden = opts.show_hidden_files.get();
                        let show_ext = opts.show_extensions.get();
                        let compact = opts.compact_sidebar.get();
                        let total_notes = all_notes.len();
                        // Use all_files for tree if available, fallback to notes only.
                        let tree_paths_raw = if all_files.is_empty() { all_notes.clone() } else { all_files.clone() };
                        // Filter hidden files (dot-prefixed) unless show_hidden is on.
                        let tree_paths: Vec<String> = if show_hidden {
                            tree_paths_raw
                        } else {
                            tree_paths_raw
                                .into_iter()
                                .filter(|p| {
                                    // Exclude if any path component starts with '.'
                                    !split_path(p).iter().any(|seg| seg.starts_with('.'))
                                })
                                .collect()
                        };
                        let folders: Vec<String> = if show_hidden {
                            folders
                        } else {
                            folders
                                .into_iter()
                                .filter(|p| {
                                    !split_path(p).iter().any(|seg| seg.starts_with('.'))
                                })
                                .collect()
                        };
                        let total_files = tree_paths.len();
                        let on_click = on_note_click.clone();

                        // Reload handler for after folder deletion.
                        let on_deleted = move || {
                            spawn_local(async move {
                                if let Ok(notes) = crate::ipc::list_notes().await {
                                    state.note_list.set(notes);
                                }
                                if let Ok(files) = crate::ipc::list_all_files().await {
                                    state.all_files.set(files);
                                }
                                if let Ok(folders) = crate::ipc::list_folders().await {
                                    state.folder_list.set(folders);
                                }
                            });
                        };

                        // Reload handler for after drag-and-drop move.
                        let on_moved = move || {
                            spawn_local(async move {
                                if let Ok(notes) = crate::ipc::list_notes().await {
                                    state.note_list.set(notes);
                                }
                                if let Ok(files) = crate::ipc::list_all_files().await {
                                    state.all_files.set(files);
                                }
                                if let Ok(folders) = crate::ipc::list_folders().await {
                                    state.folder_list.set(folders);
                                }
                            });
                        };

                        view! {
                            <div class="forge-sidebar__header">
                                <span class="forge-sidebar__count">
                                    {format!("{} notes | {} files", total_notes, total_files)}
                                </span>
                            </div>
                            <FolderTree
                                all_paths=tree_paths
                                folder_paths=folders
                                on_file_click=on_click
                                on_folder_deleted=on_deleted
                                on_item_moved=on_moved
                                show_extensions=show_ext
                                compact=compact
                            />
                        }
                        .into_any()
                    }
                }}
            </nav>
        </div>
    }
}
