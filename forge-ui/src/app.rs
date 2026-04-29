//! Root `<App />` component — global reactive state and layout shell.

use crate::tab_manager::TabManager;
use leptos::prelude::*;
use leptos::task::spawn_local;
// wasm_bindgen::JsCast is used via unchecked_ref() in the resize Effect.
#[allow(unused_imports)]
use wasm_bindgen::JsCast;

/// Discriminant for the currently visible content panel in the main UI.
///
/// Only one view is active at a time. The active view is stored in [`AppState::active_view`]
/// and drives the reactive render logic in the [`App`] component. Switching views preserves
/// the editor's note content and cursor position (via [`TabManager`]) but changes the visual
/// presentation.
///
/// # Examples
/// - **Editor**: full-width markdown textarea with live save and wikilink parsing.
/// - **Graph**: 2D force-directed knowledge graph rendered with GPU acceleration.
/// - **Canvas**: spatial outliner (placeholder for future spatial PKM features).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    /// Markdown editor for the current note.
    ///
    /// Shows a textarea with the note's content, auto-save timer, and wikilink highlighting.
    /// Syncs with [`TabManager`] to support multi-file editing across tabs.
    Editor,
    /// Knowledge graph visualization.
    ///
    /// Force-directed node-link diagram showing notes as nodes and wikilinks as edges.
    /// Rendered via [`GpuGraphView`] with configurable physics parameters (repulsion,
    /// attraction, node size, edge thickness, camera smoothing).
    Graph,
    /// Canvas / spatial outliner view.
    ///
    /// Spatial or hierarchical view of the vault structure. Placeholder for future
    /// spatial PKM workflows (e.g., 2D outliner, mind map).
    Canvas,
}

/// User-configurable application settings backed by reactive Leptos signals.
///
/// All settings are wrapped in [`RwSignal`] for reactivity: changes to any field automatically
/// trigger UI updates via Leptos' effect system. Settings are read from and persisted to
/// browser `localStorage` via [`crate::settings::load`] and [`crate::settings::watch_and_persist`].
///
/// Provided via Leptos context in the [`App`] component, making them globally accessible
/// to any component that needs to query or update user preferences.
///
/// # Defaults
/// All settings have sensible defaults (see [`Default::default`]). Note that graph view
/// physics are tuned for vaults of 19–200 notes; larger vaults may benefit from custom
/// adjustment of repulsion and node size to avoid overcrowding.
#[derive(Clone)]
pub struct AppOptions {
    /// Enable debounced auto-save on note edits.
    ///
    /// When true, changes to the editor trigger a write to disk after the debounce delay
    /// (see `auto_save_delay_ms`). Improves responsiveness by avoiding a disk write on
    /// every keystroke.
    pub auto_save: RwSignal<bool>,
    /// Debounce delay in milliseconds before auto-saving the current note.
    ///
    /// Typical range: 300–1000 ms. Lower values = faster persistence (higher CPU/disk
    /// activity); higher values = more batching (risk of loss if app crashes mid-edit).
    /// Default: 500 ms.
    pub auto_save_delay_ms: RwSignal<u32>,
    /// Enable word wrapping in the editor textarea.
    ///
    /// When true, long lines wrap to the viewport width; when false, lines scroll horizontally.
    pub word_wrap: RwSignal<bool>,
    /// Default to edit mode (true) or preview mode (false) when opening a note.
    ///
    /// Affects the initial state of the editor. Users can always toggle via the UI.
    pub default_edit_mode: RwSignal<bool>,
    /// Base font size in pixels for the editor textarea and default text.
    ///
    /// Typical range: 12–18 px. Applied via CSS custom property `--forge-font-size`.
    pub font_size: RwSignal<u32>,
    /// Show file extensions (.md, .txt, etc.) in the sidebar tree.
    pub show_extensions: RwSignal<bool>,
    /// Show hidden files (prefixed with `.`) in the sidebar tree.
    ///
    /// Note: `.forge-index/` is always hidden, regardless of this setting.
    pub show_hidden_files: RwSignal<bool>,
    /// Compact sidebar: reduce padding and margins on tree items for a narrower appearance.
    pub compact_sidebar: RwSignal<bool>,

    // ── Theme ──

    /// Active theme: `"dark"`, `"light"`, or `"system"` (follows OS preference).
    ///
    /// Applied reactively via a `data-theme` attribute on the `<html>` element.
    /// When set to `"system"`, the resolved theme is determined by the
    /// `prefers-color-scheme` media query at CSS level.
    pub theme: RwSignal<String>,

    // ── Graph View Physics ──

    /// Node size multiplier for the knowledge graph visualization.
    ///
    /// 1.0 = default 40px base radius. Higher values enlarge nodes for easier interaction
    /// on large graphs. Default: 2.5 (100% increase to 40px → 100px effective base radius).
    pub graph_node_size: RwSignal<f32>,
    /// Repulsion force multiplier between graph nodes.
    ///
    /// 1.0 = default. Higher values push nodes further apart, reducing overlap but increasing
    /// spread. Tuned for 19–200 nodes. Default: 1.0.
    pub graph_repulsion: RwSignal<f32>,
    /// Attraction force multiplier between connected nodes (edges).
    ///
    /// 1.0 = default. Higher values pull connected nodes closer, tightening clusters.
    /// Default: 1.0.
    pub graph_attraction: RwSignal<f32>,
    /// Edge (wikilink) thickness multiplier in the knowledge graph.
    ///
    /// 1.0 = default 3px. Higher values make edges more visible. Default: 2.0.
    pub graph_edge_thickness: RwSignal<f32>,
    /// Camera smoothing factor (0.0 = instant jump, 1.0 = very slow pan/zoom).
    ///
    /// Controls the lerp factor for pan/zoom animations when interacting with the graph.
    /// 0.12 provides a pleasant glide. Range: [0.0, 1.0].
    pub graph_camera_smoothing: RwSignal<f32>,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            auto_save:          RwSignal::new(true),
            auto_save_delay_ms: RwSignal::new(500),
            word_wrap:          RwSignal::new(true),
            default_edit_mode:  RwSignal::new(false),
            font_size:          RwSignal::new(14),
            show_extensions:    RwSignal::new(false),
            show_hidden_files:  RwSignal::new(false),
            compact_sidebar:    RwSignal::new(false),
            theme:              RwSignal::new("dark".to_string()),
            // Graph defaults — tuned for 19-200 nodes.
            // node_size 2.5 = 250% (old 100% is now the minimum).
            graph_node_size:        RwSignal::new(2.5),
            graph_repulsion:        RwSignal::new(1.0),
            graph_attraction:       RwSignal::new(1.0),
            graph_edge_thickness:   RwSignal::new(2.0),
            graph_camera_smoothing: RwSignal::new(0.12),
        }
    }
}

/// Global application state shared across all components via Leptos context.
///
/// All fields are [`RwSignal`] (Leptos 0.7 API) for fine-grained reactivity and
/// thread-safe cloneability across component boundaries without heap allocation (no `Arc`).
/// Provided in the [`App`] component root and accessible to any descendant via
/// `use_context::<AppState>()`.
///
/// This struct merges:
/// 1. **Vault state** — current vault path, note list, folder structure.
/// 2. **Editor state** — current note path and markdown content (synced with [`TabManager`]).
/// 3. **UI state** — active view, search query, results, loading/error messages, options panel.
///
/// # Typical Flow
/// 1. User clicks "Open vault" → `AppState::vault_path` is set, `is_loading` = true.
/// 2. Async task fetches vault metadata (notes, folders, files) → updates lists and clears loading.
/// 3. User clicks a note → `current_note` + `note_content` updated, view rendered via [`Editor`].
/// 4. User types (auto-save enabled) → note content debounced to backend after 500ms.
/// 5. User switches to Graph view → `active_view` changed, renders [`GpuGraphView`] instead.
#[derive(Clone)]
pub struct AppState {
    /// Absolute path to the currently open vault, or empty string if no vault is open.
    ///
    /// When empty, the sidebar and main content show the welcome screen. When set,
    /// the vault tree is rendered and IPC calls become valid.
    pub vault_path: RwSignal<String>,
    /// The live search query string typed into the search bar.
    ///
    /// Used to filter `note_list` and populate `search_results` via debounced backend queries.
    pub search_query: RwSignal<String>,
    /// Path of the currently active note (relative to vault root), or `None` if no note is selected.
    ///
    /// Synced with [`TabManager::active_tab_id`] by an effect in the [`App`] component.
    /// When updated, triggers re-render of the editor (or placeholder if empty).
    pub current_note: RwSignal<Option<String>>,
    /// Markdown source of the currently active note.
    ///
    /// Updated when the user opens a note or clicks a tab. Used as the textarea value
    /// in the editor component. Stored here for easy access from multiple child components.
    pub note_content: RwSignal<String>,
    /// All markdown note paths in the vault (relative to vault root), sorted alphabetically.
    ///
    /// Fetched on vault open via IPC. Used to populate the sidebar tree.
    /// Includes all `.md` files found during the vault index; does not exclude hidden notes.
    pub note_list: RwSignal<Vec<String>>,
    /// All non-hidden folder paths in the vault (relative to vault root).
    ///
    /// Used to render folder collapse/expand UI in the sidebar. Updated on vault open.
    pub folder_list: RwSignal<Vec<String>>,
    /// All non-hidden file paths in the vault, regardless of type (.md, .txt, .png, etc.).
    ///
    /// Used to show the full file tree when the "show all files" option is enabled.
    /// Typically larger than `note_list`.
    pub all_files: RwSignal<Vec<String>>,
    /// Search results: list of matching note paths (relative to vault root).
    ///
    /// Populated by debounced full-text search queries against the backend index.
    /// Cleared when `search_query` is empty.
    pub search_results: RwSignal<Vec<String>>,
    /// Which view is currently rendered in the main content area.
    ///
    /// One of: `Editor` (markdown textarea), `Graph` (knowledge graph), `Canvas` (spatial view).
    /// Switching views preserves the active note and editor content (stored in [`TabManager`]).
    pub active_view: RwSignal<ActiveView>,
    /// Last error message to display as a transient banner, or `None` if no error.
    ///
    /// Automatically cleared when the user performs an action that succeeds
    /// (e.g., successfully opens a vault, saves a note).
    pub error_msg: RwSignal<Option<String>>,
    /// True while a vault is being opened or indexed (loading state).
    ///
    /// When true, the main content shows a spinner and loading message instead of the
    /// vault. Updated by async `handle_open_vault` closure.
    pub is_loading: RwSignal<bool>,
    /// Human-readable status message displayed during loading (e.g. "Indexing 6808 notes...").
    ///
    /// Updated frequently by async tasks to show progress to the user.
    /// Cleared when `is_loading` is set to false.
    pub loading_status: RwSignal<String>,
    /// True when the options/settings panel is visible (overlay).
    ///
    /// Toggled by the options menu button. When true, [`OptionsMenu`] is rendered
    /// as a modal on top of the main content.
    pub show_options: RwSignal<bool>,
    /// True when the command palette (Ctrl+P) overlay is visible.
    pub show_command_palette: RwSignal<bool>,
    /// Current indexing progress step (1-based), or 0 if not indexing.
    pub indexing_step: RwSignal<u8>,
    /// Total indexing steps (typically 6).
    pub indexing_total: RwSignal<u8>,
    /// Detail message from the current indexing step.
    pub indexing_detail: RwSignal<String>,
}

/// Root Leptos component — initializes global state and renders the Forgedsidian UI shell.
///
/// This is the entry point of the web UI. Responsibilities:
/// 1. Create and provide [`AppState`] context (vault path, note list, active view, etc.).
/// 2. Create and provide [`AppOptions`] context (user settings from `localStorage`).
/// 3. Create and provide [`TabManager`] context (multi-file tab system).
/// 4. Set up global reactive effects:
///    - Sync active tab ↔ `AppState::current_note` and `note_content`.
///    - Apply reactive font size to the document CSS custom property.
///    - Handle sidebar resize (drag handle) and toggle.
///    - Global keyboard shortcuts (Ctrl+W to close tab, Ctrl+Tab to cycle).
/// 5. Render the layout shell: sidebar + resize handle + main content area + options panel.
///
/// # Layout
/// - **Sidebar** (left): collapsible file tree, resizable width, collapse toggle button.
/// - **Resize handle**: draggable divider between sidebar and main content.
/// - **Main content**: toolbar + tab bar + active view (editor, graph, or canvas).
/// - **Options panel**: overlay modal for user settings (toggled by toolbar button).
///
/// # Initialization
/// On mount, settings are loaded from `localStorage` via [`crate::settings::load`], and a
/// watch effect is installed to persist all option changes back to storage.
///
/// # Non-Responsive Elements
/// Some elements (loading spinner, error banner) are rendered conditionally; the component
/// is otherwise always-mounted to preserve Leptos' reactive context and state.
#[component]
pub fn App() -> impl IntoView {
    let state = AppState {
        vault_path:     RwSignal::new(String::new()),
        search_query:   RwSignal::new(String::new()),
        current_note:   RwSignal::new(None),
        note_content:   RwSignal::new(String::new()),
        note_list:      RwSignal::new(Vec::new()),
        folder_list:    RwSignal::new(Vec::new()),
        all_files:      RwSignal::new(Vec::new()),
        search_results: RwSignal::new(Vec::new()),
        active_view:    RwSignal::new(ActiveView::Editor),
        error_msg:      RwSignal::new(None),
        is_loading:     RwSignal::new(false),
        loading_status: RwSignal::new(String::new()),
        show_options:   RwSignal::new(false),
        show_command_palette: RwSignal::new(false),
        indexing_step:   RwSignal::new(0),
        indexing_total:  RwSignal::new(6),
        indexing_detail: RwSignal::new(String::new()),
    };
    provide_context(state.clone());

    let opts = AppOptions::default();
    // Restore persisted settings (localStorage) before providing context.
    crate::settings::load(&opts);
    provide_context(opts.clone());
    // Auto-save settings whenever any option changes.
    crate::settings::watch_and_persist(&opts);

    // Tab manager — multi-file tab system.
    let tab_mgr = TabManager::new();
    provide_context(tab_mgr.clone());

    // Sync active tab -> AppState (current_note + note_content) for backward
    // compatibility with the editor component.
    {
        let state = state.clone();
        let tab_mgr = tab_mgr.clone();
        Effect::new(move |_| {
            let _active_id = tab_mgr.active_tab_id.get();
            // Re-read tabs signal to track content changes too.
            let tabs = tab_mgr.tabs.get();
            if let Some(id) = _active_id {
                if let Some(tab) = tabs.iter().find(|t| t.id == id) {
                    let current = state.current_note.get_untracked();
                    if current.as_deref() != Some(&tab.file_path) {
                        state.note_content.set(tab.content.clone());
                        state.current_note.set(Some(tab.file_path.clone()));
                    }
                }
            } else {
                state.current_note.set(None);
                state.note_content.set(String::new());
            }
        });
    }

    let vault_opened = move || !state.vault_path.get().is_empty();

    // Register indexing progress listener once at startup.
    {
        let state_progress = state.clone();
        spawn_local(async move {
            crate::ipc::listen_indexing_progress(move |p| {
                state_progress.indexing_step.set(p.step);
                state_progress.indexing_total.set(p.total);
                state_progress.loading_status.set(p.label.clone());
                state_progress.indexing_detail.set(p.detail.unwrap_or_default());
            }).await;
        });
    }

    // Trigger native folder picker, then open the vault at the selected path.
    let handle_open_vault = move |_| {
        state.error_msg.set(None);
        state.is_loading.set(true);
        state.indexing_step.set(0);
        state.loading_status.set("Opening folder picker...".into());
        spawn_local(async move {
            match crate::ipc::pick_and_open_vault().await {
                Ok(Some(path)) => {
                    state.loading_status.set("Fetching note list...".into());
                    state.vault_path.set(path.clone());
                    // Fetch note list + folder list for the sidebar.
                    match crate::ipc::list_notes().await {
                        Ok(notes) => {
                            state.loading_status.set(format!("Loaded {} notes", notes.len()));
                            state.note_list.set(notes);
                        }
                        Err(e) => {
                            leptos::logging::warn!("[forge] list_notes failed: {}", &e);
                        }
                    }
                    // Fetch all files (for full tree display) and folders.
                    match crate::ipc::list_all_files().await {
                        Ok(files) => {
                            state.all_files.set(files);
                        }
                        Err(e) => {
                            leptos::logging::warn!("[forge] list_all_files failed: {}", &e);
                        }
                    }
                    match crate::ipc::list_folders().await {
                        Ok(folders) => {
                            state.folder_list.set(folders);
                        }
                        Err(e) => {
                            leptos::logging::warn!("[forge] list_folders failed: {}", &e);
                        }
                    }
                    state.is_loading.set(false);
                    state.indexing_step.set(0);
                }
                Ok(None) => {
                    // User cancelled.
                    state.is_loading.set(false);
                    state.loading_status.set(String::new());
                    state.indexing_step.set(0);
                }
                Err(e) => {
                    state.error_msg.set(Some(e));
                    state.is_loading.set(false);
                    state.indexing_step.set(0);
                }
            }
        });
    };

    // ---- Sidebar resize & collapse state ----
    let sidebar_width = RwSignal::new(280_i32);
    let sidebar_collapsed = RwSignal::new(false);
    let is_dragging = RwSignal::new(false);

    // Provide these to child components that may need them.
    provide_context(sidebar_collapsed);

    // Toggle sidebar collapse.
    let toggle_sidebar = move |_| {
        sidebar_collapsed.update(|v| *v = !*v);
    };

    // Drag start handler — registers global mouse listeners.
    let on_drag_start = move |_: leptos::ev::MouseEvent| {
        is_dragging.set(true);
        // Add body class to prevent text selection while dragging.
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(body) = doc.body() {
                let _ = body.class_list().add_1("resizing");
            }
        }
    };

    // Global mousemove — update sidebar width when dragging.
    Effect::new(move |_| {
        use wasm_bindgen::prelude::*;

        let dragging = is_dragging.get();
        if !dragging {
            return;
        }

        // We set up global event listeners via JS for mousemove and mouseup.
        // These are cleaned up on mouseup.
        let on_move = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |ev: web_sys::MouseEvent| {
            if is_dragging.get_untracked() {
                let new_width = ev.client_x().max(150).min(600);
                sidebar_width.set(new_width);
            }
        });

        let on_up = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |_: web_sys::MouseEvent| {
            is_dragging.set(false);
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                if let Some(body) = doc.body() {
                    let _ = body.class_list().remove_1("resizing");
                }
            }
        });

        if let Some(window) = web_sys::window() {
            let _ = window.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref());
            let _ = window.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref());
        }

        // Leak the closures so they stay alive until mouseup.
        // This is acceptable because they only live for the duration of a drag.
        on_move.forget();
        on_up.forget();
    });

    // Apply reactive font size to the editor area via a CSS custom property.
    Effect::new(move |_| {
        let fs = opts.font_size.get();
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(el) = doc.document_element() {
                let _ = el
                    .unchecked_ref::<web_sys::HtmlElement>()
                    .style()
                    .set_property("--forge-font-size", &format!("{}px", fs));
            }
        }
    });

    // Apply reactive theme via data-theme attribute on <html>.
    // "dark" | "light" -> explicit override; "system" -> remove attribute, let CSS
    // @media (prefers-color-scheme) handle it.
    {
        let opts_theme = opts.clone();
        Effect::new(move |_| {
            let theme = opts_theme.theme.get();
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                if let Some(el) = doc.document_element() {
                    match theme.as_str() {
                        "dark" | "light" => {
                            let _ = el.set_attribute("data-theme", &theme);
                        }
                        _ => {
                            // "system": remove data-theme, CSS media query takes over.
                            let _ = el.remove_attribute("data-theme");
                        }
                    }
                }
            }
        });
    }

    // Global keyboard shortcuts for tab management and command palette.
    {
        use wasm_bindgen::prelude::*;
        let tab_mgr = tab_mgr.clone();
        let state_kbd = state.clone();
        let on_keydown = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
            let ctrl = ev.ctrl_key() || ev.meta_key();
            if !ctrl {
                return;
            }

            match ev.key().as_str() {
                "w" | "W" => {
                    // Ctrl+W — close active tab.
                    ev.prevent_default();
                    if let Some(id) = tab_mgr.active_tab_id.get_untracked() {
                        tab_mgr.close(id);
                    }
                }
                "Tab" => {
                    // Ctrl+Tab / Ctrl+Shift+Tab — cycle tabs.
                    ev.prevent_default();
                    if ev.shift_key() {
                        tab_mgr.prev_tab();
                    } else {
                        tab_mgr.next_tab();
                    }
                }
                "p" | "P" => {
                    // Ctrl+P — toggle command palette.
                    ev.prevent_default();
                    state_kbd.show_command_palette.update(|v| *v = !*v);
                }
                _ => {}
            }
        });

        if let Some(window) = web_sys::window() {
            let _ = window.add_event_listener_with_callback(
                "keydown",
                on_keydown.as_ref().unchecked_ref(),
            );
        }
        on_keydown.forget();
    }

    view! {
        <div class="forge-shell" style="position:relative;">
            {move || {
                state.error_msg.get().map(|msg| {
                    view! {
                        <div class="forge-error-banner" style="background:var(--trl-error);color:#fff;padding:8px 16px;font-size:14px;position:absolute;top:0;left:0;right:0;z-index:50;">
                            <strong>"Error: "</strong>{msg}
                        </div>
                    }
                })
            }}

            // Sidebar with dynamic width
            <aside
                class="forge-sidebar"
                class:collapsed=move || sidebar_collapsed.get()
                style=move || {
                    if sidebar_collapsed.get() {
                        "width:0;".to_string()
                    } else {
                        format!("width:{}px;", sidebar_width.get())
                    }
                }
            >
                <crate::components::sidebar::SidebarContent />
            </aside>

            // Resize handle
            <div
                class="forge-resize-handle"
                class:dragging=move || is_dragging.get()
                on:mousedown=on_drag_start
                style=move || {
                    if sidebar_collapsed.get() {
                        "display:none;".to_string()
                    } else {
                        String::new()
                    }
                }
            />

            // Collapse toggle button
            <button
                class="forge-sidebar-toggle"
                on:click=toggle_sidebar
                style=move || {
                    if sidebar_collapsed.get() {
                        "left:4px;".to_string()
                    } else {
                        format!("left:{}px;", sidebar_width.get() + 8)
                    }
                }
                title=move || {
                    if sidebar_collapsed.get() {
                        "Show sidebar"
                    } else {
                        "Hide sidebar"
                    }
                }
            >
                {move || if sidebar_collapsed.get() { "\u{25B6}" } else { "\u{25C0}" }}
            </button>

            // Main content wrapper — toolbar + tab bar + view area
            <div class="forge-main">
                // Toolbar — always in DOM, hides itself when no vault is open
                <crate::components::toolbar::Toolbar />

                // Tab bar — shows open file tabs (only when vault is open)
                {move || {
                    if vault_opened() {
                        view! { <crate::components::tab_bar::TabBar /> }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }}

                <main class="forge-content">
                    {move || {
                        if state.is_loading.get() {
                            let status = state.loading_status.get();
                            let step = state.indexing_step.get();
                            let total = state.indexing_total.get();
                            let detail = state.indexing_detail.get();
                            let pct = if total > 0 && step > 0 {
                                ((step as f64) / (total as f64) * 100.0) as u32
                            } else {
                                0
                            };
                            let show_bar = step > 0;
                            view! {
                                <div class="forge-content__loading" style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;gap:16px;">
                                    {if show_bar {
                                        view! {
                                            // Progress bar container
                                            <div class="forge-progress" style="width:320px;max-width:80vw;">
                                                <div class="forge-progress__bar" style="width:100%;height:8px;background:var(--trl-abyss-light);border-radius:4px;overflow:hidden;">
                                                    <div
                                                        class="forge-progress__fill"
                                                        style=format!(
                                                            "width:{}%;height:100%;background:var(--trl-cyan);border-radius:4px;transition:width 0.3s ease;",
                                                            pct
                                                        )
                                                    />
                                                </div>
                                                <div style="display:flex;justify-content:space-between;margin-top:6px;">
                                                    <span style="font-size:12px;color:var(--trl-text-tertiary);">
                                                        {format!("Step {}/{}", step, total)}
                                                    </span>
                                                    <span style="font-size:12px;color:var(--trl-text-tertiary);">
                                                        {format!("{}%", pct)}
                                                    </span>
                                                </div>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="forge-spinner" style="width:48px;height:48px;border:4px solid var(--trl-abyss-light);border-top:4px solid var(--trl-cyan);border-radius:50%;animation:spin 1s linear infinite;"></div>
                                        }.into_any()
                                    }}
                                    <p style="color:var(--trl-text-secondary);font-size:14px;font-weight:500;">{status}</p>
                                    {if !detail.is_empty() {
                                        view! {
                                            <p style="color:var(--trl-text-tertiary);font-size:12px;margin-top:-8px;">{detail}</p>
                                        }.into_any()
                                    } else {
                                        view! { <></> }.into_any()
                                    }}
                                </div>
                            }
                            .into_any()
                        } else if vault_opened() {
                            match state.active_view.get() {
                                ActiveView::Editor => {
                                    view! { <crate::components::editor::Editor /> }.into_any()
                                }
                                ActiveView::Graph => {
                                    view! { <crate::components::graph_view::GpuGraphView /> }.into_any()
                                }
                                ActiveView::Canvas => {
                                    view! { <crate::components::canvas_view::CanvasView /> }.into_any()
                                }
                            }
                        } else {
                            view! {
                                <div class="forge-content__placeholder">
                                    <h2>"Welcome to Forgedsidian"</h2>
                                    <p>"A modular knowledge graph platform in Rust."</p>
                                    <button on:click=handle_open_vault class="forge-btn forge-btn--primary">
                                        "Open vault"
                                    </button>
                                </div>
                            }
                            .into_any()
                        }
                    }}
                </main>
            </div>

            // Options panel overlay (conditionally rendered)
            {move || {
                if state.show_options.get() {
                    view! { <crate::components::options_menu::OptionsMenu /> }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            // Command Palette overlay (Ctrl+P)
            {move || {
                if state.show_command_palette.get() {
                    view! { <crate::components::command_palette::CommandPalette /> }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}
        </div>
    }
}
