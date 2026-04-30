//! Input method popup actions

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

/// Input Method Popup Actions
#[derive(Clone)]
pub enum Action {
    /// Create and show input method popup
    Popup {
        /// settings
        settings: InputMethodPopupSettings,
    },
    /// Show input method popup
    ShowPopup,
    /// Hide input method popup
    HidePopup,
    /// Set size of the input method popup
    Size {
        /// id of the popup
        id: Id,
        /// width
        width: u32,
        /// height
        height: u32,
    },
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Popup { settings } => {
                f.debug_tuple("Popup").field(settings).finish()
            }
            Self::ShowPopup => write!(f, "ShowPopup"),
            Self::HidePopup => write!(f, "HidePopup"),
            Self::Size { id, width, height } => f
                .debug_struct("Size")
                .field("id", id)
                .field("width", width)
                .field("height", height)
                .finish(),
        }
    }
}
