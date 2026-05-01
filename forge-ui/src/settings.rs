//! Persistent settings via localStorage.
//!
//! Saves and restores `AppOptions` graph fields so they survive app restarts.
//! Uses the Tauri webview's localStorage (persistent across sessions).

use crate::app::AppOptions;
use leptos::prelude::*;

const STORAGE_KEY: &str = "forgexalith_settings";

/// JSON-serializable snapshot of the settings we persist.
#[derive(serde::Serialize, serde::Deserialize)]
struct PersistedSettings {
    graph_node_size: f32,
    graph_repulsion: f32,
    graph_attraction: f32,
    graph_edge_thickness: f32,
    graph_camera_smoothing: f32,
    font_size: u32,
    auto_save: bool,
    auto_save_delay_ms: u32,
    word_wrap: bool,
    show_extensions: bool,
    show_hidden_files: bool,
    compact_sidebar: bool,
    /// Theme preference: "dark", "light", or "system". Defaults to "dark" if
    /// absent (backward-compatible with pre-Phase-21 saved settings).
    #[serde(default = "default_theme")]
    theme: String,
}

fn default_theme() -> String {
    "dark".to_string()
}

impl PersistedSettings {
    /// Snapshot the current signal values.
    fn from_opts(opts: &AppOptions) -> Self {
        Self {
            graph_node_size: opts.graph_node_size.get_untracked(),
            graph_repulsion: opts.graph_repulsion.get_untracked(),
            graph_attraction: opts.graph_attraction.get_untracked(),
            graph_edge_thickness: opts.graph_edge_thickness.get_untracked(),
            graph_camera_smoothing: opts.graph_camera_smoothing.get_untracked(),
            font_size: opts.font_size.get_untracked(),
            auto_save: opts.auto_save.get_untracked(),
            auto_save_delay_ms: opts.auto_save_delay_ms.get_untracked(),
            word_wrap: opts.word_wrap.get_untracked(),
            show_extensions: opts.show_extensions.get_untracked(),
            show_hidden_files: opts.show_hidden_files.get_untracked(),
            compact_sidebar: opts.compact_sidebar.get_untracked(),
            theme: opts.theme.get_untracked(),
        }
    }

    /// Clamp numeric fields to safe bounds to prevent DoS via tampered localStorage.
    fn sanitize(&mut self) {
        self.graph_node_size = self.graph_node_size.clamp(1.0, 8.0);
        self.graph_repulsion = self.graph_repulsion.clamp(0.1, 4.0);
        self.graph_attraction = self.graph_attraction.clamp(0.1, 4.0);
        self.graph_edge_thickness = self.graph_edge_thickness.clamp(0.5, 5.0);
        self.graph_camera_smoothing = self.graph_camera_smoothing.clamp(0.01, 0.5);
        self.font_size = self.font_size.clamp(8, 32);
        self.auto_save_delay_ms = self.auto_save_delay_ms.clamp(200, 10_000);
        // Validate theme value — reject tampered values.
        if !matches!(self.theme.as_str(), "dark" | "light" | "system") {
            self.theme = "dark".to_string();
        }
    }

    /// Apply the persisted values to the given signals (after sanitization).
    fn apply_to(&self, opts: &AppOptions) {
        opts.graph_node_size.set(self.graph_node_size);
        opts.graph_repulsion.set(self.graph_repulsion);
        opts.graph_attraction.set(self.graph_attraction);
        opts.graph_edge_thickness.set(self.graph_edge_thickness);
        opts.graph_camera_smoothing.set(self.graph_camera_smoothing);
        opts.font_size.set(self.font_size);
        opts.auto_save.set(self.auto_save);
        opts.auto_save_delay_ms.set(self.auto_save_delay_ms);
        opts.word_wrap.set(self.word_wrap);
        opts.show_extensions.set(self.show_extensions);
        opts.show_hidden_files.set(self.show_hidden_files);
        opts.compact_sidebar.set(self.compact_sidebar);
        opts.theme.set(self.theme.clone());
    }
}

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// Load persisted settings and apply them to the given `AppOptions`.
///
/// Silently does nothing if no saved settings exist or if parsing fails.
pub fn load(opts: &AppOptions) {
    let Some(storage) = local_storage() else { return };
    let Some(json) = storage.get_item(STORAGE_KEY).ok().flatten() else { return };
    match serde_json::from_str::<PersistedSettings>(&json) {
        Ok(mut saved) => {
            saved.sanitize();
            saved.apply_to(opts);
            leptos::logging::log!("[settings] loaded persisted settings");
        }
        Err(e) => {
            leptos::logging::warn!("[settings] failed to parse saved settings: {e}");
        }
    }
}

/// Save the current `AppOptions` to localStorage.
fn save(opts: &AppOptions) {
    let Some(storage) = local_storage() else { return };
    let snap = PersistedSettings::from_opts(opts);
    match serde_json::to_string(&snap) {
        Ok(json) => {
            let _ = storage.set_item(STORAGE_KEY, &json);
        }
        Err(e) => {
            leptos::logging::warn!("[settings] failed to serialize settings: {e}");
        }
    }
}

/// Set up a reactive Effect that saves settings whenever any option changes.
///
/// Call once after `AppOptions` is provided as context.
pub fn watch_and_persist(opts: &AppOptions) {
    let opts = opts.clone();
    Effect::new(move |_| {
        // Read all signals to subscribe to them.
        let _ = opts.graph_node_size.get();
        let _ = opts.graph_repulsion.get();
        let _ = opts.graph_attraction.get();
        let _ = opts.graph_edge_thickness.get();
        let _ = opts.graph_camera_smoothing.get();
        let _ = opts.font_size.get();
        let _ = opts.auto_save.get();
        let _ = opts.auto_save_delay_ms.get();
        let _ = opts.word_wrap.get();
        let _ = opts.show_extensions.get();
        let _ = opts.show_hidden_files.get();
        let _ = opts.compact_sidebar.get();
        let _ = opts.theme.get();
        // Save after reading (this runs on every change).
        save(&opts);
    });
}
