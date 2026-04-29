//! `<TabBar />` — horizontal tab strip for switching between open files.
//!
//! Renders one tab per open file. The active tab is highlighted.
//! Each tab shows a close button and a modified indicator (dot).
//! Middle-click on a tab closes it.

use crate::tab_manager::TabManager;
use leptos::prelude::*;

/// Tab bar component — renders all open tabs as a horizontal strip.
///
/// Placed between the toolbar and the editor/view area in the main content.
#[component]
pub fn TabBar() -> impl IntoView {
    let tab_mgr = use_context::<TabManager>().expect("TabManager not found");

    view! {
        <div class="forge-tab-bar">
            {move || {
                let tabs = tab_mgr.tabs.get();
                let active_id = tab_mgr.active_tab_id.get();

                if tabs.is_empty() {
                    return view! { <></> }.into_any();
                }

                view! {
                    <div class="forge-tab-bar__inner">
                        {tabs.into_iter().map(|tab| {
                            let tab_id = tab.id;
                            let is_active = active_id == Some(tab_id);
                            let title = tab.title.clone();
                            let modified = tab.modified;
                            let rel_path = tab.rel_path.clone();

                            view! {
                                <div
                                    class="forge-tab"
                                    class:forge-tab--active=is_active
                                    class:forge-tab--modified=modified
                                    title=rel_path
                                    on:mousedown=move |ev: web_sys::MouseEvent| {
                                        // Middle-click (button 1) closes the tab.
                                        if ev.button() == 1 {
                                            ev.prevent_default();
                                            tab_mgr.close(tab_id);
                                        }
                                    }
                                    on:click=move |_| {
                                        tab_mgr.activate(tab_id);
                                    }
                                >
                                    <span class="forge-tab__title">
                                        {if modified {
                                            format!("{} \u{2022}", title)
                                        } else {
                                            title.clone()
                                        }}
                                    </span>
                                    <button
                                        class="forge-tab__close"
                                        title="Close tab"
                                        on:click=move |ev: web_sys::MouseEvent| {
                                            ev.stop_propagation();
                                            tab_mgr.close(tab_id);
                                        }
                                    >
                                        "\u{00D7}"
                                    </button>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}
