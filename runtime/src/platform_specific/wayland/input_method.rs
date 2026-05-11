//! Input method actions

use iced_core::window::Id;
use std::fmt;

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
    /// Set size of the input method popup
    Size {
        /// id of the popup
        id: Id,
        /// width
        width: u32,
        /// height
        height: u32,
    },
    /// Freeze or unfreeze the popup position.
    /// When frozen, the compositor will not update the popup position
    /// in response to cursor rectangle changes.
    SetFrozen {
        /// Whether to freeze (true) or unfreeze (false)
        frozen: bool,
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
            Self::Size { id, width, height } => f
                .debug_struct("Size")
                .field("id", id)
                .field("width", width)
                .field("height", height)
                .finish(),
            Self::SetFrozen { frozen } => {
                f.debug_struct("SetFrozen").field("frozen", frozen).finish()
            }
        }
    }
}
