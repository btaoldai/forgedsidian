//! Axis-aligned bounding-box (AABB) hit testing.

use crate::item::{CanvasItem, Rect};

/// Returns `true` if the point `(px, py)` lies inside `rect`.
pub fn point_in_rect(px: f64, py: f64, rect: &Rect) -> bool {
    px >= rect.x && px <= rect.x + rect.width && py >= rect.y && py <= rect.y + rect.height
}

/// Find the topmost canvas item under the cursor at `(px, py)`.
///
/// Items are tested in reverse z-index order (highest z-index first).
pub fn hit_test(items: &[CanvasItem], px: f64, py: f64) -> Option<&CanvasItem> {
    let mut sorted: Vec<&CanvasItem> = items.iter().collect();
    sorted.sort_by(|a, b| b.z_index.cmp(&a.z_index));
    sorted
        .into_iter()
        .find(|item| point_in_rect(px, py, &item.bounds))
}
