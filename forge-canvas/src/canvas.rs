//! The infinite canvas state machine.

use crate::{
    error::CanvasError,
    item::{CanvasItem, ItemId},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Viewport transform: pan offset + zoom level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Viewport {
    pub pan_x: f64,
    pub pan_y: f64,
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
        }
    }
}

/// The root canvas state — owns all items and the viewport transform.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Canvas {
    items: HashMap<ItemId, CanvasItem>,
    viewport: Viewport,
}

impl Canvas {
    /// Create an empty canvas.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace an item on the canvas.
    pub fn upsert(&mut self, item: CanvasItem) {
        self.items.insert(item.id, item);
    }

    /// Remove an item by ID.
    ///
    /// # Errors
    /// Returns [`CanvasError::ItemNotFound`] if the ID is unknown.
    pub fn remove(&mut self, id: ItemId) -> Result<CanvasItem, CanvasError> {
        self.items
            .remove(&id)
            .ok_or(CanvasError::ItemNotFound { id })
    }

    /// Return an iterator over all canvas items.
    pub fn items(&self) -> impl Iterator<Item = &CanvasItem> {
        self.items.values()
    }

    /// Update the viewport transform.
    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.viewport = viewport;
    }

    /// Return the current viewport.
    pub fn viewport(&self) -> Viewport {
        self.viewport
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abcde::Priority;
    use crate::item::{ItemContent, Rect};

    /// Create a test label item with a given ID.
    fn create_test_label(id: ItemId, text: &str) -> CanvasItem {
        CanvasItem {
            id,
            bounds: Rect {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 50.0,
            },
            content: ItemContent::Label {
                text: text.to_string(),
            },
            z_index: 1,
        }
    }

    /// Create a test task item with a given ID.
    fn create_test_task(id: ItemId, title: &str, priority: Priority) -> CanvasItem {
        CanvasItem {
            id,
            bounds: Rect {
                x: 50.0,
                y: 60.0,
                width: 200.0,
                height: 80.0,
            },
            content: ItemContent::Task {
                title: title.to_string(),
                priority,
                done: false,
            },
            z_index: 2,
        }
    }

    #[test]
    fn canvas_upsert_inserts_new_item() {
        let mut canvas = Canvas::new();
        let item_id = ItemId::new();
        let item = create_test_label(item_id, "Test Label");

        canvas.upsert(item.clone());

        let items: Vec<_> = canvas.items().collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, item_id);
        assert_eq!(items[0].z_index, 1);
    }

    #[test]
    fn canvas_upsert_replaces_existing_item_with_same_id() {
        let mut canvas = Canvas::new();
        let item_id = ItemId::new();
        let item_v1 = create_test_label(item_id, "Original Label");
        let item_v2 = create_test_label(item_id, "Updated Label");

        canvas.upsert(item_v1);
        assert_eq!(canvas.items().count(), 1);

        canvas.upsert(item_v2);
        assert_eq!(
            canvas.items().count(),
            1,
            "Should not duplicate; still one item"
        );

        let items: Vec<_> = canvas.items().collect();
        match &items[0].content {
            ItemContent::Label { text } => assert_eq!(text, "Updated Label"),
            _ => panic!("Expected Label content"),
        }
    }

    #[test]
    fn canvas_upsert_multiple_different_items() {
        let mut canvas = Canvas::new();
        let id1 = ItemId::new();
        let id2 = ItemId::new();
        let id3 = ItemId::new();

        let item1 = create_test_label(id1, "Label 1");
        let item2 = create_test_task(id2, "Task 1", Priority::A);
        let item3 = create_test_label(id3, "Label 2");

        canvas.upsert(item1);
        canvas.upsert(item2);
        canvas.upsert(item3);

        assert_eq!(
            canvas.items().count(),
            3,
            "All three items should be present"
        );
    }

    #[test]
    fn canvas_remove_existing_item_returns_ok() {
        let mut canvas = Canvas::new();
        let item_id = ItemId::new();
        let item = create_test_label(item_id, "To Remove");

        canvas.upsert(item.clone());
        let result = canvas.remove(item_id);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, item_id);
        assert_eq!(
            canvas.items().count(),
            0,
            "Canvas should be empty after removal"
        );
    }

    #[test]
    fn canvas_remove_non_existent_item_returns_error() {
        let mut canvas = Canvas::new();
        let non_existent_id = ItemId::new();

        let result = canvas.remove(non_existent_id);

        assert!(result.is_err());
        match result.unwrap_err() {
            CanvasError::ItemNotFound { id } => assert_eq!(id, non_existent_id),
        }
    }

    #[test]
    fn canvas_remove_twice_second_fails() {
        let mut canvas = Canvas::new();
        let item_id = ItemId::new();
        let item = create_test_label(item_id, "Remove Me");

        canvas.upsert(item);
        let first_remove = canvas.remove(item_id);
        assert!(first_remove.is_ok());

        let second_remove = canvas.remove(item_id);
        assert!(second_remove.is_err(), "Second removal should fail");
        assert_eq!(canvas.items().count(), 0);
    }

    #[test]
    fn viewport_default_is_zero_pan_unit_zoom() {
        let viewport = Viewport::default();
        assert_eq!(viewport.pan_x, 0.0);
        assert_eq!(viewport.pan_y, 0.0);
        assert_eq!(viewport.zoom, 1.0);
    }

    #[test]
    fn viewport_serde_json_roundtrip() {
        let original = Viewport {
            pan_x: 123.45,
            pan_y: -67.89,
            zoom: 2.5,
        };

        let json = serde_json::to_string(&original).expect("Serialization failed");
        let deserialized: Viewport = serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(deserialized.pan_x, original.pan_x);
        assert_eq!(deserialized.pan_y, original.pan_y);
        assert_eq!(deserialized.zoom, original.zoom);
    }

    #[test]
    fn canvas_serde_json_roundtrip_with_items() {
        let mut canvas = Canvas::new();
        let id1 = ItemId::new();
        let id2 = ItemId::new();

        let item1 = create_test_label(id1, "Roundtrip Label");
        let item2 = create_test_task(id2, "Roundtrip Task", Priority::B);

        canvas.upsert(item1);
        canvas.upsert(item2);
        canvas.set_viewport(Viewport {
            pan_x: 42.0,
            pan_y: 84.0,
            zoom: 1.5,
        });

        let json = serde_json::to_string(&canvas).expect("Serialization failed");
        let restored: Canvas = serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(restored.items().count(), 2, "Items should be restored");
        let restored_viewport = restored.viewport();
        assert_eq!(restored_viewport.pan_x, 42.0);
        assert_eq!(restored_viewport.pan_y, 84.0);
        assert_eq!(restored_viewport.zoom, 1.5);
    }

    #[test]
    fn canvas_set_viewport_updates_state() {
        let mut canvas = Canvas::new();
        let new_viewport = Viewport {
            pan_x: 15.0,
            pan_y: 25.0,
            zoom: 3.0,
        };

        canvas.set_viewport(new_viewport);
        let retrieved = canvas.viewport();

        assert_eq!(retrieved.pan_x, 15.0);
        assert_eq!(retrieved.pan_y, 25.0);
        assert_eq!(retrieved.zoom, 3.0);
    }
}
