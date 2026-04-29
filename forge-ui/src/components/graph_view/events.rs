//! Event handlers for pointer, wheel, and control buttons.
//!
//! Manages mouse/touch/stylus interactions for pan, zoom, node drag, and click-to-open.
//! Also handles camera control buttons (fit view, zoom in/out, reset layout).

use wasm_bindgen::JsCast;

/// Drag distance threshold (in screen pixels) to distinguish click from drag.
/// If pointer moves < 3px from origin, it's treated as a click; >= 3px is a drag.
pub const DRAG_THRESHOLD: f32 = 3.0;

/// Extract canvas-relative coordinates from a pointer event.
///
/// Converts from window/client coordinates to canvas-local coordinates
/// by subtracting the canvas's bounding rect offset.
///
/// # Arguments
///
/// * `ev` - The PointerEvent from the browser.
///
/// # Returns
///
/// (x, y) in canvas-relative coordinates. Falls back to client coordinates if
/// the target element cannot be resolved.
pub fn canvas_coords(ev: &web_sys::PointerEvent) -> (f32, f32) {
    if let Some(target) = ev.target() {
        if let Some(el) = target.dyn_ref::<web_sys::Element>() {
            let rect = el.get_bounding_client_rect();
            return (
                ev.client_x() as f32 - rect.left() as f32,
                ev.client_y() as f32 - rect.top() as f32,
            );
        }
    }
    (ev.client_x() as f32, ev.client_y() as f32)
}

/// Extract canvas-relative coordinates from a wheel event.
///
/// Converts from window/client coordinates to canvas-local coordinates
/// by subtracting the canvas's bounding rect offset.
///
/// # Arguments
///
/// * `ev` - The WheelEvent from the browser.
///
/// # Returns
///
/// (x, y) in canvas-relative coordinates. Falls back to client coordinates if
/// the target element cannot be resolved.
pub fn canvas_coords_wheel(ev: &web_sys::WheelEvent) -> (f32, f32) {
    if let Some(target) = ev.target() {
        if let Some(el) = target.dyn_ref::<web_sys::Element>() {
            let rect = el.get_bounding_client_rect();
            return (
                ev.client_x() as f32 - rect.left() as f32,
                ev.client_y() as f32 - rect.top() as f32,
            );
        }
    }
    (ev.client_x() as f32, ev.client_y() as f32)
}
