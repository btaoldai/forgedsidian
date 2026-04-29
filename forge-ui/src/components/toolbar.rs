//! `<Toolbar />` — view switcher bar (Editor / Graph / Canvas).
//!
//! Renders a horizontal pill bar at the top of the main content area.
//! Each button sets `AppState::active_view` via the shared `RwSignal`.
//! Hides itself when no vault is open.

use crate::app::{ActiveView, AppState};
use leptos::prelude::*;

/// Top toolbar — view navigation + hamburger menu trigger.
#[component]
pub fn Toolbar() -> impl IntoView {
    let state = use_context::<AppState>().expect("AppState must be provided by <App />");

    // Hide toolbar when no vault is open.
    let toolbar_style = move || {
        if state.vault_path.get().is_empty() {
            "display:none;".to_string()
        } else {
            String::new()
        }
    };

    let set_editor = move |_| state.active_view.set(ActiveView::Editor);
    let set_graph  = move |_| state.active_view.set(ActiveView::Graph);
    let set_canvas = move |_| state.active_view.set(ActiveView::Canvas);

    let is_editor = move || state.active_view.get() == ActiveView::Editor;
    let is_graph  = move || state.active_view.get() == ActiveView::Graph;
    let is_canvas = move || state.active_view.get() == ActiveView::Canvas;

    view! {
        <div class="forge-toolbar" style=toolbar_style>
            <div class="forge-toolbar__views">
                <button
                    class="forge-toolbar__view-btn"
                    class:active=is_editor
                    title="Edit notes (Ctrl+1)"
                    on:click=set_editor
                >
                    <span class="forge-toolbar__icon">"E"</span>
                    <span class="forge-toolbar__label">"Editor"</span>
                </button>
                <button
                    class="forge-toolbar__view-btn"
                    class:active=is_graph
                    title="Knowledge graph (Ctrl+2)"
                    on:click=set_graph
                >
                    <span class="forge-toolbar__icon">"G"</span>
                    <span class="forge-toolbar__label">"Graph"</span>
                </button>
                <button
                    class="forge-toolbar__view-btn"
                    class:active=is_canvas
                    title="Spatial canvas (Ctrl+3)"
                    on:click=set_canvas
                >
                    <span class="forge-toolbar__icon">"C"</span>
                    <span class="forge-toolbar__label">"Canvas"</span>
                </button>
            </div>

            // Vault name display (center)
            <div class="forge-toolbar__spacer"></div>
            <div class="forge-toolbar__vault-name">
                {move || {
                    let vp = state.vault_path.get();
                    if vp.is_empty() {
                        String::new()
                    } else {
                        crate::components::folder_tree::file_name_from_path(&vp).to_string()
                    }
                }}
            </div>
            <div class="forge-toolbar__spacer"></div>

            // Hamburger menu button
            <button
                class="forge-toolbar__menu-btn"
                title="Options"
                on:click=move |_| {
                    state.show_options.update(|v| *v = !*v);
                }
            >
                {"\u{2630}"}
            </button>
        </div>
    }
}
