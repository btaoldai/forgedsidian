//! Canvas-specific errors.

use crate::item::ItemId;
use thiserror::Error;

/// Top-level error type for `forge-canvas`.
#[derive(Debug, Error)]
pub enum CanvasError {
    /// An operation targeted an item that does not exist on the canvas.
    #[error("canvas item {id:?} not found")]
    ItemNotFound { id: ItemId },
}
