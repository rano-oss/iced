//! Input method actions

use iced_core::window::Id;
use std::fmt;

pub use wayland_protocols_experimental::input_method::v1::client::xx_input_popup_positioner_v1::{
    Anchor, Gravity, ConstraintAdjustment,
};
pub use wayland_protocols_experimental::input_method::v1::client::xx_input_popup_surface_v2::PopupPositionMode;

/// Positioner settings for popup repositioning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupPositioner {
    /// Anchor point on the cursor rectangle
    pub anchor: Anchor,
    /// Direction the popup grows from the anchor
    pub gravity: Gravity,
    /// Offset from anchor point (x, y)
    pub offset: (i32, i32),
    /// Constraint adjustment flags
    pub constraint_adjustment: ConstraintAdjustment,
}

impl Default for PopupPositioner {
    fn default() -> Self {
        Self {
            anchor: Anchor::BottomLeft,
            gravity: Gravity::BottomRight,
            offset: (0, 0),
            constraint_adjustment: ConstraintAdjustment::FlipY
                | ConstraintAdjustment::SlideX
                | ConstraintAdjustment::SlideY
                | ConstraintAdjustment::ResizeX
                | ConstraintAdjustment::ResizeY,
        }
    }
}

/// Input method popup settings
#[derive(Debug, Clone)]
pub struct InputMethodPopupSettings {
    /// Id of the popup
    pub id: Id,
    /// The initial size of the window.
    pub size: (u32, u32),
}

impl Default for InputMethodPopupSettings {
    fn default() -> Self {
        Self {
            id: Id::unique(),
            size: (256, 256),
        }
    }
}

/// Input method action
#[derive(Clone)]
pub enum Action {
    /// Set the preedit string
    SetPreeditString {
        /// The preedit string
        string: String,
        /// Cursor begin position (bytes)
        cursor_begin: i32,
        /// Cursor end position (bytes)
        cursor_end: i32,
    },
    /// Commit a string to the text input
    CommitString(String),
    /// Commit pending state
    Commit,
    /// Filter a key event: consume (true) or passthrough (false)
    FilterKey(u32, bool),
    /// Create and show input method popup
    Popup {
        /// settings
        settings: InputMethodPopupSettings,
    },
    /// Reposition the popup with new size and positioner settings
    Size {
        /// id of the popup
        id: Id,
        /// width
        width: u32,
        /// height
        height: u32,
        /// Positioner settings (anchor, gravity, offset, constraints)
        positioner: PopupPositioner,
    },
    /// Reset the popup's last tracked positioner size.
    /// Call this when the popup is hidden so the next show starts fresh.
    ResetPopupSize,
    /// Set how the compositor positions the popup relative to the text input cursor.
    SetPopupPositionMode {
        /// Positioning mode
        mode: PopupPositionMode,
    },
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetPreeditString {
                string,
                cursor_begin,
                cursor_end,
            } => f
                .debug_struct("SetPreeditString")
                .field("string", string)
                .field("cursor_begin", cursor_begin)
                .field("cursor_end", cursor_end)
                .finish(),
            Self::CommitString(s) => {
                f.debug_tuple("CommitString").field(s).finish()
            }
            Self::Commit => write!(f, "Commit"),
            Self::FilterKey(serial, consumed) => f
                .debug_struct("FilterKey")
                .field("serial", serial)
                .field("consumed", consumed)
                .finish(),
            Self::Popup { settings } => {
                f.debug_tuple("Popup").field(settings).finish()
            }
            Self::Size {
                id,
                width,
                height,
                positioner,
            } => f
                .debug_struct("Size")
                .field("id", id)
                .field("width", width)
                .field("height", height)
                .field("positioner", positioner)
                .finish(),
            Self::ResetPopupSize => write!(f, "ResetPopupSize"),
            Self::SetPopupPositionMode { mode } => f
                .debug_struct("SetPopupPositionMode")
                .field("mode", mode)
                .finish(),
        }
    }
}
