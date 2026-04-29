//! `<OptionsMenu />` — slide-in options panel triggered by the hamburger button.
//!
//! Reads and writes `AppOptions` from `AppState` context.
//! All options are reactive and take effect immediately.

use crate::app::{AppOptions, AppState};
use leptos::prelude::*;

/// Overlay options panel — slides in from the right.
#[component]
pub fn OptionsMenu() -> impl IntoView {
    let state = use_context::<AppState>().expect("AppState must be provided by <App />");
    let opts = use_context::<AppOptions>().expect("AppOptions must be provided by <App />");

    let close = move |_| state.show_options.set(false);

    view! {
        // Backdrop — click to close
        <div
            class="forge-options__backdrop"
            on:click=close
        ></div>

        // Panel
        <div class="forge-options__panel">
            <div class="forge-options__header">
                <h3>"Options"</h3>
                <button
                    class="forge-options__close"
                    on:click=close
                    title="Close"
                >
                    {"\u{2715}"}
                </button>
            </div>

            <div class="forge-options__body">
                // --- Auto-save ---
                <div class="forge-options__group">
                    <h4>"Editor"</h4>

                    <label class="forge-options__toggle">
                        <span>"Auto-save"</span>
                        <input
                            type="checkbox"
                            prop:checked=move || opts.auto_save.get()
                            on:change=move |_| opts.auto_save.update(|v| *v = !*v)
                        />
                    </label>

                    <label class="forge-options__range">
                        <span>{move || format!("Auto-save delay: {}ms", opts.auto_save_delay_ms.get())}</span>
                        <input
                            type="range"
                            min="200"
                            max="3000"
                            step="100"
                            prop:value=move || opts.auto_save_delay_ms.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                    opts.auto_save_delay_ms.set(v);
                                }
                            }
                        />
                    </label>

                    <label class="forge-options__toggle">
                        <span>"Word wrap"</span>
                        <input
                            type="checkbox"
                            prop:checked=move || opts.word_wrap.get()
                            on:change=move |_| opts.word_wrap.update(|v| *v = !*v)
                        />
                    </label>

                    <label class="forge-options__select">
                        <span>"Default mode"</span>
                        <select
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                opts.default_edit_mode.set(val == "edit");
                            }
                        >
                            <option value="preview" selected=move || !opts.default_edit_mode.get()>"Preview"</option>
                            <option value="edit" selected=move || opts.default_edit_mode.get()>"Edit"</option>
                        </select>
                    </label>
                </div>

                // --- Appearance ---
                <div class="forge-options__group">
                    <h4>"Appearance"</h4>

                    <label class="forge-options__select">
                        <span>"Theme"</span>
                        <select
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                opts.theme.set(val);
                            }
                        >
                            <option value="dark" selected=move || opts.theme.get() == "dark">"Dark"</option>
                            <option value="light" selected=move || opts.theme.get() == "light">"Light"</option>
                            <option value="system" selected=move || opts.theme.get() == "system">"System"</option>
                        </select>
                    </label>

                    <label class="forge-options__range">
                        <span>{move || format!("Font size: {}px", opts.font_size.get())}</span>
                        <input
                            type="range"
                            min="11"
                            max="22"
                            step="1"
                            prop:value=move || opts.font_size.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                    opts.font_size.set(v);
                                }
                            }
                        />
                    </label>
                </div>

                // --- Graph View ---
                // Slider internals stay the same; labels remap to 1%-100%.
                <div class="forge-options__group">
                    <h4>"Graph View"</h4>

                    <label class="forge-options__range">
                        <span>{move || {
                            let v = opts.graph_node_size.get();
                            let pct = (v - 1.0) / 7.0 * 99.0 + 1.0;
                            format!("Node size: {:.0}%", pct)
                        }}</span>
                        <input
                            type="range"
                            min="100"
                            max="800"
                            step="10"
                            prop:value=move || (opts.graph_node_size.get() * 100.0).to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                    opts.graph_node_size.set(v / 100.0);
                                }
                            }
                        />
                    </label>

                    <label class="forge-options__range">
                        <span>{move || {
                            let v = opts.graph_repulsion.get();
                            let pct = (v - 0.1) / 3.9 * 99.0 + 1.0;
                            format!("Repulsion: {:.0}%", pct)
                        }}</span>
                        <input
                            type="range"
                            min="10"
                            max="400"
                            step="5"
                            prop:value=move || (opts.graph_repulsion.get() * 100.0).to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                    opts.graph_repulsion.set(v / 100.0);
                                }
                            }
                        />
                    </label>

                    <label class="forge-options__range">
                        <span>{move || {
                            let v = opts.graph_attraction.get();
                            let pct = (v - 0.1) / 3.9 * 99.0 + 1.0;
                            format!("Attraction: {:.0}%", pct)
                        }}</span>
                        <input
                            type="range"
                            min="10"
                            max="400"
                            step="5"
                            prop:value=move || (opts.graph_attraction.get() * 100.0).to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                    opts.graph_attraction.set(v / 100.0);
                                }
                            }
                        />
                    </label>

                    <label class="forge-options__range">
                        <span>{move || {
                            let v = opts.graph_edge_thickness.get();
                            let pct = (v - 0.5) / 4.5 * 99.0 + 1.0;
                            format!("Edge thickness: {:.0}%", pct)
                        }}</span>
                        <input
                            type="range"
                            min="50"
                            max="500"
                            step="10"
                            prop:value=move || (opts.graph_edge_thickness.get() * 100.0).to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                    opts.graph_edge_thickness.set(v / 100.0);
                                }
                            }
                        />
                    </label>

                    <label class="forge-options__range">
                        <span>{move || {
                            let v = opts.graph_camera_smoothing.get();
                            let pct = (v - 0.01) / 0.49 * 99.0 + 1.0;
                            format!("Camera smoothing: {:.0}%", pct)
                        }}</span>
                        <input
                            type="range"
                            min="1"
                            max="50"
                            step="1"
                            prop:value=move || (opts.graph_camera_smoothing.get() * 100.0).to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                    opts.graph_camera_smoothing.set(v / 100.0);
                                }
                            }
                        />
                    </label>

                    <button
                        class="forge-btn forge-btn--small"
                        style="margin-top:8px;padding:4px 12px;font-size:12px;cursor:pointer;background:var(--trl-abyss-light);color:var(--trl-text);border:1px solid var(--trl-abyss-mid);border-radius:4px;width:100%;"
                        on:click=move |_| {
                            opts.graph_node_size.set(2.5);
                            opts.graph_repulsion.set(1.0);
                            opts.graph_attraction.set(1.0);
                            opts.graph_edge_thickness.set(2.0);
                            opts.graph_camera_smoothing.set(0.12);
                        }
                    >
                        "Reset to defaults"
                    </button>
                </div>

                // --- Sidebar ---
                <div class="forge-options__group">
                    <h4>"Sidebar"</h4>

                    <label class="forge-options__toggle">
                        <span>"Show file extensions"</span>
                        <input
                            type="checkbox"
                            prop:checked=move || opts.show_extensions.get()
                            on:change=move |_| opts.show_extensions.update(|v| *v = !*v)
                        />
                    </label>

                    <label class="forge-options__toggle">
                        <span>"Show hidden files"</span>
                        <input
                            type="checkbox"
                            prop:checked=move || opts.show_hidden_files.get()
                            on:change=move |_| opts.show_hidden_files.update(|v| *v = !*v)
                        />
                    </label>

                    <label class="forge-options__toggle">
                        <span>"Compact mode"</span>
                        <input
                            type="checkbox"
                            prop:checked=move || opts.compact_sidebar.get()
                            on:change=move |_| opts.compact_sidebar.update(|v| *v = !*v)
                        />
                    </label>
                </div>
            </div>
        </div>
    }
}
