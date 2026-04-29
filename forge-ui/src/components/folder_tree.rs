//! Hierarchical folder tree for vault navigation.
//!
//! Uses a **lazy rendering** strategy: only the top level is built initially.
//! Children are computed on-demand when a folder is expanded.
//! This keeps the initial render fast even with 16k+ files.
//!
//! Includes folder deletion with confirmation dialog and drag-and-drop
//! reordering (move files/folders via HTML5 Drag API).

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::{BTreeMap, HashSet};
use wasm_bindgen::JsCast;

/// Split a path on both `/` and `\` for cross-platform WASM compatibility.
pub fn split_path(path: &str) -> Vec<&str> {
    path.split(|c| c == '/' || c == '\\')
        .filter(|s| !s.is_empty())
        .collect()
}

/// Extract file name from a path (last component).
pub fn file_name_from_path(path: &str) -> &str {
    let parts = split_path(path);
    parts.last().copied().unwrap_or("(unnamed)")
}

/// A single item at one level of the tree.
#[derive(Clone, Debug)]
pub enum TreeItem {
    Folder(String),
    File(String),
}

/// Build ONE level of the tree from flat paths at the given depth.
fn build_level(paths: &[String], folder_paths: &[String], depth: usize) -> Vec<TreeItem> {
    let mut folders: BTreeMap<String, bool> = BTreeMap::new();
    let mut files: Vec<String> = Vec::new();

    for path in paths {
        let components = split_path(path);
        if depth >= components.len() {
            continue;
        }
        if depth == components.len() - 1 {
            if components[depth] == ".empty" {
                continue;
            }
            files.push(path.clone());
        } else {
            let folder_name = components[depth].to_string();
            folders.entry(folder_name).or_insert(true);
        }
    }

    for fp in folder_paths {
        let normalised = fp.replace('\\', "/");
        let components: Vec<&str> = normalised.split('/').filter(|s| !s.is_empty()).collect();
        if depth < components.len() {
            let folder_name = components[depth].to_string();
            folders.entry(folder_name).or_insert(true);
        }
    }

    let mut result: Vec<TreeItem> = Vec::new();
    for (name, _) in folders {
        result.push(TreeItem::Folder(name));
    }
    files.sort();
    for path in files {
        result.push(TreeItem::File(path));
    }
    result
}

/// Filter paths to children of a folder at given depth.
fn filter_paths_for_folder(paths: &[String], folder_name: &str, depth: usize) -> Vec<String> {
    paths
        .iter()
        .filter(|path| {
            let components = split_path(path);
            components.len() > depth && components[depth] == folder_name
        })
        .cloned()
        .collect()
}

/// Filter folder paths that contain the folder at given depth.
fn filter_folder_paths(folder_paths: &[String], folder_name: &str, depth: usize) -> Vec<String> {
    folder_paths
        .iter()
        .filter(|fp| {
            let normalised = fp.replace('\\', "/");
            let components: Vec<&str> = normalised.split('/').filter(|s| !s.is_empty()).collect();
            components.len() > depth && components[depth] == folder_name
        })
        .cloned()
        .collect()
}

/// Drag-and-drop payload: what is being dragged and whether it's a file or folder.
#[derive(Clone, Debug)]
pub struct DragPayload {
    /// Relative path of the item being dragged.
    pub path: String,
    /// True if the item is a folder, false if it's a file.
    pub is_folder: bool,
}

/// Hierarchical folder tree with lazy rendering, folder deletion, and drag-and-drop.
#[component]
pub fn FolderTree(
    all_paths: Vec<String>,
    folder_paths: Vec<String>,
    on_file_click: impl Fn(String) + 'static + Copy + Send,
    /// Called after a folder is deleted (triggers reload).
    on_folder_deleted: impl Fn() + 'static + Copy + Send,
    /// Called after a file or folder is moved (triggers reload).
    on_item_moved: impl Fn() + 'static + Copy + Send,
    /// Whether to show file extensions in the tree.
    #[prop(default = false)]
    show_extensions: bool,
    /// Compact mode: reduced padding.
    #[prop(default = false)]
    compact: bool,
) -> impl IntoView {
    let expanded = RwSignal::new(HashSet::<String>::new());

    // Delete confirmation state
    let delete_target = RwSignal::new(Option::<String>::None);
    let delete_confirmed = RwSignal::new(false);
    let delete_error = RwSignal::new(String::new());

    // Drag-and-drop state
    let drag_payload = RwSignal::new(Option::<DragPayload>::None);
    let drop_target_folder = RwSignal::new(Option::<String>::None);
    let drop_error = RwSignal::new(String::new());

    let top_level = build_level(&all_paths, &folder_paths, 0);

    view! {
        // Delete confirmation dialog (overlay)
        {move || {
            let target = delete_target.get();
            if let Some(folder_path) = target {
                let folder_display = folder_path.clone();
                let folder_for_delete = folder_path.clone();

                view! {
                    <div style="position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.6);z-index:1000;display:flex;align-items:center;justify-content:center;">
                        <div style="background:var(--trl-abyss);border:1px solid var(--trl-abyss-mid);border-radius:8px;padding:20px;max-width:400px;width:90%;">
                            <h3 style="margin:0 0 12px 0;color:var(--trl-error);font-size:15px;">"Delete folder"</h3>
                            <p style="font-size:13px;color:var(--trl-text);margin:0 0 8px 0;">
                                "Are you sure you want to delete "
                                <strong style="color:var(--trl-error);">{folder_display.clone()}</strong>
                                " and ALL its contents?"
                            </p>
                            <p style="font-size:12px;color:var(--trl-alert);margin:0 0 16px 0;">
                                "This action cannot be undone."
                            </p>

                            <label style="display:flex;align-items:center;gap:8px;font-size:13px;color:var(--trl-text);cursor:pointer;margin-bottom:16px;">
                                <input
                                    type="checkbox"
                                    prop:checked=move || delete_confirmed.get()
                                    on:change=move |_| delete_confirmed.update(|v| *v = !*v)
                                    style="width:16px;height:16px;accent-color:var(--trl-error);"
                                />
                                "I confirm the deletion of this folder"
                            </label>

                            {move || {
                                let err = delete_error.get();
                                if err.is_empty() {
                                    view! { <></> }.into_any()
                                } else {
                                    view! {
                                        <p style="font-size:12px;color:var(--trl-error);margin:0 0 8px 0;">{err}</p>
                                    }.into_any()
                                }
                            }}

                            <div style="display:flex;gap:8px;">
                                <button
                                    style="flex:1;padding:6px;font-size:13px;cursor:pointer;background:var(--trl-error);color:#fff;border:none;border-radius:4px;opacity:0.5;"
                                    style:opacity=move || if delete_confirmed.get() { "1" } else { "0.5" }
                                    prop:disabled=move || !delete_confirmed.get()
                                    on:click=move |_| {
                                        if !delete_confirmed.get() { return; }
                                        let path = folder_for_delete.clone();
                                        spawn_local(async move {
                                            match crate::ipc::delete_folder(&path, true).await {
                                                Ok(msg) => {
                                                    delete_target.set(None);
                                                    delete_confirmed.set(false);
                                                    delete_error.set(String::new());
                                                    leptos::logging::log!("Deleted: {}", msg);
                                                    on_folder_deleted();
                                                }
                                                Err(e) => {
                                                    delete_error.set(e);
                                                }
                                            }
                                        });
                                    }
                                >
                                    "Delete"
                                </button>
                                <button
                                    style="flex:1;padding:6px;font-size:13px;cursor:pointer;background:var(--trl-abyss-mid);color:var(--trl-text);border:none;border-radius:4px;"
                                    on:click=move |_| {
                                        delete_target.set(None);
                                        delete_confirmed.set(false);
                                        delete_error.set(String::new());
                                    }
                                >
                                    "Cancel"
                                </button>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }
        }}

        // Drop error toast (auto-dismisses)
        {move || {
            let err = drop_error.get();
            if err.is_empty() {
                view! { <></> }.into_any()
            } else {
                // Auto-dismiss after 3 seconds
                let err_clone = err.clone();
                spawn_local(async move {
                    crate::components::editor::sleep_ms(3000).await;
                    // Only clear if the error hasn't changed
                    if drop_error.get_untracked() == err_clone {
                        drop_error.set(String::new());
                    }
                });
                view! {
                    <div style="position:fixed;bottom:16px;right:16px;background:var(--trl-error);color:#fff;padding:8px 16px;border-radius:6px;font-size:13px;z-index:1001;box-shadow:0 4px 12px rgba(0,0,0,0.3);">
                        {err}
                    </div>
                }.into_any()
            }
        }}

        <ul
            class="forge-tree"
            class:forge-tree--drop-root=move || {
                drop_target_folder.get().as_deref() == Some("")
            }
            on:dragover=move |ev: web_sys::DragEvent| {
                // ALWAYS preventDefault to allow drops — the "forbidden" cursor
                // appears whenever dragover doesn't call preventDefault.
                ev.prevent_default();
                if let Some(dt) = ev.data_transfer() {
                    dt.set_drop_effect("move");
                }
                // Only set root as drop target if nothing more specific is targeted
                if drop_target_folder.get_untracked().is_none() {
                    drop_target_folder.set(Some(String::new()));
                }
            }
            on:dragleave=move |_: web_sys::DragEvent| {
                if drop_target_folder.get_untracked().as_deref() == Some("") {
                    drop_target_folder.set(None);
                }
            }
            on:drop=move |ev: web_sys::DragEvent| {
                ev.prevent_default();
                let was_root = drop_target_folder.get_untracked().as_deref() == Some("");
                drop_target_folder.set(None);

                // Only handle root-level drops (folder drops are handled by the folder nodes)
                if !was_root {
                    return;
                }

                if let Some(payload) = drag_payload.get_untracked() {
                    drag_payload.set(None);

                    let from = payload.path.clone();
                    let is_folder = payload.is_folder;

                    // Don't move if already at root
                    if !from.contains('/') && !from.contains('\\') {
                        return;
                    }

                    spawn_local(async move {
                        let result = if is_folder {
                            crate::ipc::move_folder(&from, "").await
                        } else {
                            crate::ipc::move_file(&from, "").await
                        };

                        match result {
                            Ok(new_path) => {
                                leptos::logging::log!("Moved {} -> {}", from, new_path);
                                on_item_moved();
                            }
                            Err(e) => {
                                drop_error.set(format!("Move failed: {}", e));
                            }
                        }
                    });
                }
            }
        >
            {top_level.into_iter().map(|item| {
                let paths = all_paths.clone();
                let fps = folder_paths.clone();
                view! {
                    <LazyTreeNode
                        item=item
                        all_paths=paths
                        folder_paths=fps
                        expanded=expanded
                        on_file_click=on_file_click
                        delete_target=delete_target
                        drag_payload=drag_payload
                        drop_target_folder=drop_target_folder
                        drop_error=drop_error
                        on_item_moved=on_item_moved
                        depth=0
                        path_prefix="".to_string()
                        show_extensions=show_extensions
                        compact=compact
                    />
                }
            }).collect_view()}
        </ul>
    }
}

/// Lazy recursive tree node with drag-and-drop support.
#[component]
fn LazyTreeNode(
    item: TreeItem,
    all_paths: Vec<String>,
    folder_paths: Vec<String>,
    expanded: RwSignal<HashSet<String>>,
    on_file_click: impl Fn(String) + 'static + Copy + Send,
    delete_target: RwSignal<Option<String>>,
    drag_payload: RwSignal<Option<DragPayload>>,
    drop_target_folder: RwSignal<Option<String>>,
    drop_error: RwSignal<String>,
    on_item_moved: impl Fn() + 'static + Copy + Send,
    depth: usize,
    path_prefix: String,
    #[prop(default = false)]
    show_extensions: bool,
    #[prop(default = false)]
    compact: bool,
) -> impl IntoView {
    match item {
        TreeItem::File(path) => {
            let raw_name = file_name_from_path(&path);
            let name = if show_extensions {
                raw_name.to_string()
            } else {
                // Strip common extensions for display
                raw_name
                    .rsplit_once('.')
                    .map_or(raw_name.to_string(), |(stem, _)| stem.to_string())
            };
            let path_clone = path.clone();
            let path_for_drag = path.clone();
            let path_for_drop = path.clone();
            let pad = if compact { depth * 10 } else { depth * 14 };
            view! {
                <li
                    class="forge-tree__file"
                    style=format!("padding-left: {}px; user-select: none;", pad)
                    draggable="true"
                    on:dragstart=move |ev: web_sys::DragEvent| {
                        drag_payload.set(Some(DragPayload {
                            path: path_for_drag.clone(),
                            is_folder: false,
                        }));
                        if let Some(dt) = ev.data_transfer() {
                            let _ = dt.set_data("text/plain", &path_for_drag);
                            dt.set_effect_allowed("move");
                        }
                    }
                    on:dragend=move |_: web_sys::DragEvent| {
                        drag_payload.set(None);
                        drop_target_folder.set(None);
                    }
                    on:dragover=move |ev: web_sys::DragEvent| {
                        // preventDefault required — stop_propagation intentionally
                        // removed so the event bubbles to parent folder and root
                        // handlers (needed for correct cursor in WebView2).
                        ev.prevent_default();
                        if let Some(dt) = ev.data_transfer() {
                            dt.set_drop_effect("move");
                        }
                    }
                    on:drop=move |ev: web_sys::DragEvent| {
                        ev.prevent_default();
                        // stop_propagation on drop: prevent parent folder handler
                        // from also firing for the same drag operation.
                        ev.stop_propagation();
                        drop_target_folder.set(None);

                        if let Some(payload) = drag_payload.get_untracked() {
                            drag_payload.set(None);
                            let from = payload.path.clone();
                            let is_folder = payload.is_folder;
                            // Move into the same folder as the target file
                            let dest = extract_parent(&path_for_drop);

                            if extract_parent(&from) == dest {
                                return; // Already in the same folder
                            }

                            spawn_local(async move {
                                let result = if is_folder {
                                    crate::ipc::move_folder(&from, &dest).await
                                } else {
                                    crate::ipc::move_file(&from, &dest).await
                                };
                                match result {
                                    Ok(new_path) => {
                                        leptos::logging::log!("Moved {} -> {}", from, new_path);
                                        on_item_moved();
                                    }
                                    Err(e) => {
                                        drop_error.set(format!("Move failed: {}", e));
                                    }
                                }
                            });
                        }
                    }
                >
                    <button
                        class="forge-tree__file-btn"
                        draggable="false"
                        on:click=move |_| on_file_click(path_clone.clone())
                    >
                        {name}
                    </button>
                </li>
            }
            .into_any()
        }

        TreeItem::Folder(name) => {
            let folder_path = if path_prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", path_prefix, name)
            };

            let folder_path_check = folder_path.clone();
            let folder_path_toggle = folder_path.clone();
            let folder_path_children = folder_path.clone();
            let folder_path_children2 = folder_path.clone();
            let folder_path_delete = folder_path.clone();
            let folder_path_drag = folder_path.clone();
            let folder_path_drop = folder_path.clone();
            let folder_path_drop_check = folder_path.clone();
            let folder_path_drop_leave = folder_path.clone();

            let is_expanded = move || expanded.get().contains(&folder_path_check);

            let is_drop_target = move || {
                drop_target_folder.get().as_deref() == Some(folder_path_drop_check.as_str())
            };

            let toggle_expand = move |_| {
                let mut exp = expanded.get();
                let p = folder_path_toggle.clone();
                if exp.contains(&p) {
                    exp.remove(&p);
                } else {
                    exp.insert(p);
                }
                expanded.set(exp);
            };

            let child_paths = filter_paths_for_folder(&all_paths, &name, depth);
            let child_folder_paths = filter_folder_paths(&folder_paths, &name, depth);

            let pad = if compact { depth * 10 } else { depth * 14 };
            view! {
                <li
                    class="forge-tree__folder"
                    style=format!("padding-left: {}px; user-select: none;", pad)
                    draggable="true"
                    on:dragstart=move |ev: web_sys::DragEvent| {
                        drag_payload.set(Some(DragPayload {
                            path: folder_path_drag.clone(),
                            is_folder: true,
                        }));
                        if let Some(dt) = ev.data_transfer() {
                            let _ = dt.set_data("text/plain", &folder_path_drag);
                            dt.set_effect_allowed("move");
                        }
                        ev.stop_propagation();
                    }
                    on:dragend=move |_: web_sys::DragEvent| {
                        drag_payload.set(None);
                        drop_target_folder.set(None);
                    }
                    on:dragover=move |ev: web_sys::DragEvent| {
                        // ALWAYS preventDefault — required to allow drops.
                        ev.prevent_default();
                        ev.stop_propagation();
                        if let Some(dt) = ev.data_transfer() {
                            dt.set_drop_effect("move");
                        }
                        // Set this folder as drop target for visual highlight
                        drop_target_folder.set(Some(folder_path_drop.clone()));
                    }
                    on:dragleave=move |ev: web_sys::DragEvent| {
                        // Only clear if we're truly leaving this element (not entering a child)
                        if let Some(related) = ev.related_target() {
                            if let Some(current) = ev.current_target() {
                                let current_el: &web_sys::Node = current.unchecked_ref();
                                let related_node: &web_sys::Node = related.unchecked_ref();
                                if current_el.contains(Some(related_node)) {
                                    return;
                                }
                            }
                        }
                        if drop_target_folder.get_untracked().as_deref() == Some(folder_path_drop_leave.as_str()) {
                            drop_target_folder.set(None);
                        }
                    }
                    on:drop=move |ev: web_sys::DragEvent| {
                        ev.prevent_default();
                        ev.stop_propagation();
                        drop_target_folder.set(None);

                        if let Some(payload) = drag_payload.get_untracked() {
                            drag_payload.set(None);

                            let from = payload.path.clone();
                            let dest = folder_path_children2.clone();
                            let is_folder = payload.is_folder;

                            // Don't move if already in this folder
                            let parent = extract_parent(&from);
                            if parent == dest {
                                return;
                            }

                            // Don't move a folder into itself
                            if is_folder && (dest == from || dest.starts_with(&format!("{}/", from))) {
                                drop_error.set("Cannot move a folder into itself".to_string());
                                return;
                            }

                            spawn_local(async move {
                                let result = if is_folder {
                                    crate::ipc::move_folder(&from, &dest).await
                                } else {
                                    crate::ipc::move_file(&from, &dest).await
                                };

                                match result {
                                    Ok(new_path) => {
                                        leptos::logging::log!("Moved {} -> {}", from, new_path);
                                        on_item_moved();
                                    }
                                    Err(e) => {
                                        drop_error.set(format!("Move failed: {}", e));
                                    }
                                }
                            });
                        }
                    }
                >
                    <div
                        class="forge-tree__folder-row"
                        class:forge-tree__folder-row--drop-target=is_drop_target
                        style="display:flex;align-items:center;"
                    >
                        <button
                            class="forge-tree__folder-btn"
                            style="flex:1;"
                            draggable="false"
                            on:click=toggle_expand
                        >
                            <span class="forge-tree__toggle" style="pointer-events:none;">
                                {move || if is_expanded() { "\u{25BC}" } else { "\u{25B6}" }}
                            </span>
                            <span class="forge-tree__folder-name" style="pointer-events:none;">{name.clone()}</span>
                        </button>
                        <button
                            class="forge-tree__delete-btn"
                            title="Delete folder"
                            draggable="false"
                            style="background:transparent;border:none;color:var(--trl-text-tertiary);cursor:pointer;padding:2px 6px;font-size:12px;opacity:0.3;transition:opacity 0.15s;"
                            on:click=move |e: web_sys::MouseEvent| {
                                e.stop_propagation();
                                delete_target.set(Some(folder_path_delete.clone()));
                            }
                        >
                            "x"
                        </button>
                    </div>

                    {move || {
                        if !expanded.get().contains(&folder_path_children) {
                            return view! { <></> }.into_any();
                        }

                        let child_depth = depth + 1;
                        let children = build_level(&child_paths, &child_folder_paths, child_depth);
                        let fp = folder_path_children.clone();

                        view! {
                            <ul class="forge-tree__children">
                                {children.into_iter().map(|child| {
                                    let cp = match &child {
                                        TreeItem::Folder(n) => filter_paths_for_folder(&child_paths, n, child_depth),
                                        TreeItem::File(_) => vec![],
                                    };
                                    let cfp = match &child {
                                        TreeItem::Folder(n) => filter_folder_paths(&child_folder_paths, n, child_depth),
                                        TreeItem::File(_) => vec![],
                                    };
                                    let pfx = fp.clone();
                                    view! {
                                        <LazyTreeNode
                                            item=child
                                            all_paths=cp
                                            folder_paths=cfp
                                            expanded=expanded
                                            on_file_click=on_file_click
                                            delete_target=delete_target
                                            drag_payload=drag_payload
                                            drop_target_folder=drop_target_folder
                                            drop_error=drop_error
                                            on_item_moved=on_item_moved
                                            depth=child_depth
                                            path_prefix=pfx
                                            show_extensions=show_extensions
                                            compact=compact
                                        />
                                    }
                                }).collect_view()}
                            </ul>
                        }
                        .into_any()
                    }}
                </li>
            }
            .into_any()
        }
    }
}

/// Extract parent folder from a path. Returns empty string for root-level items.
fn extract_parent(path: &str) -> String {
    let normalised = path.replace('\\', "/");
    match normalised.rfind('/') {
        Some(idx) => normalised[..idx].to_string(),
        None => String::new(),
    }
}
