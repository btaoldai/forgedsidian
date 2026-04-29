//! ABCDE prioritization logic.
//!
//! The ABCDE method (Brian Tracy) assigns a single letter priority to each
//! task.  `A` = must do, `B` = should do, `C` = nice to do, `D` = delegate,
//! `E` = eliminate.

use serde::{Deserialize, Serialize};

/// ABCDE priority level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Priority {
    /// Must do — critical consequences if not done.
    A,
    /// Should do — mild consequences if not done.
    B,
    /// Nice to do — no consequences if not done.
    C,
    /// Delegate — someone else can do this.
    D,
    /// Eliminate — not worth doing at all.
    E,
}

impl Priority {
    /// Returns a short human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Priority::A => "A - Must do",
            Priority::B => "B - Should do",
            Priority::C => "C - Nice to do",
            Priority::D => "D - Delegate",
            Priority::E => "E - Eliminate",
        }
    }

    /// Returns a CSS hex color suitable for the canvas badge.
    pub fn color(self) -> &'static str {
        match self {
            Priority::A => "#ef4444", // red
            Priority::B => "#f97316", // orange
            Priority::C => "#eab308", // yellow
            Priority::D => "#3b82f6", // blue
            Priority::E => "#6b7280", // grey
        }
    }
}
