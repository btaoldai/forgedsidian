//! Canvas items: notes, tasks and shapes.

use forge_core::NoteId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A unique identifier for a canvas item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(Uuid);

impl ItemId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ItemId {
    fn default() -> Self {
        Self::new()
    }
}

/// An axis-aligned bounding box on the infinite canvas.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The type-specific content of a canvas item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemContent {
    /// A linked note card displayed on the canvas.
    Note { note_id: NoteId },
    /// A standalone task card with ABCDE priority.
    Task {
        title: String,
        priority: crate::abcde::Priority,
        done: bool,
    },
    /// A freeform text label.
    Label { text: String },
}

/// A positioned item on the infinite canvas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasItem {
    pub id: ItemId,
    pub bounds: Rect,
    pub content: ItemContent,
    pub z_index: i32,
}
