use cctk::wayland_client::protocol::wl_keyboard::KeyState;
use iced_core::{layout::Limits, window::Id};
use wayland_protocols::{
    wp::text_input::v3::client::wp_text_input_v3::{
        Action as IMAction, CommitMode, PreeditColorHint, PreeditStyle,
        PreeditUnderline,
    },
    xdg::shell::client::xdg_positioner::{Anchor, Gravity},
};

/// Positioner of a popup
#[derive(Debug, Clone)]
pub struct InputMethodPositioner {
    /// size of the popup (if it is None, the popup will be autosized)
    pub size: Option<(u32, u32)>,
    /// Limits of the popup size
    pub size_limits: Limits,
    /// the anchor location on the popup
    pub anchor: Anchor,
    /// the gravity of the popup
    pub gravity: Gravity,
    /// the constraint adjustment,
    /// Specify how the window should be positioned if the originally intended position caused the surface to be constrained, meaning at least partially outside positioning boundaries set by the compositor. The adjustment is set by constructing a bitmask describing the adjustment to be made when the surface is constrained on that axis.
    /// If no bit for one axis is set, the compositor will assume that the child surface should not change its position on that axis when constrained.
    ///
    /// If more than one bit for one axis is set, the order of how adjustments are applied is specified in the corresponding adjustment descriptions.
    ///
    /// The default adjustment is none.
    pub constraint_adjustment: u32,
    /// offset of the popup
    pub offset: (i32, i32),
    /// whether the popup is reactive
    pub reactive: bool,
}

/// Popup creation details
#[derive(Debug, Clone)]
pub struct InputMethodPopupSettings {
    /// XXX must be unique, id of the popup
    pub id: Id,
    /// positioner of the popup
    pub positioner: InputMethodPositioner,
}

#[derive(Debug, Clone)]
pub enum Action {
    Commit,
    SetPreeditString {
        text: String,
        cursor_begin: i32,
        cursor_end: i32,
    },
    SetString {
        text: String,
    },
    DeleteSurroundingText {
        before_length: u32,
        after_length: u32,
    },
    SetAction {
        action: IMAction,
    },
    SetLanguage {
        language: String,
    },
    SetPreeditCommitMode {
        mode: CommitMode,
    },
    SetPreeditStyle {
        begin: i32,
        end: i32,
        underline: PreeditUnderline,
        style: PreeditStyle,
        color: PreeditColorHint,
    },
    /// create a window and receive a message with its Id
    Popup {
        /// popup
        popup: InputMethodPopupSettings,
    },
    /// destroy the popup
    Destroy {
        /// id of the popup
        id: Id,
    },
    /// set the size of the popup
    Size {
        /// id of the popup
        id: Id,
        /// width
        width: u32,
        /// height
        height: u32,
    },
    ForwardKey {
        key_state: KeyState,
    },
}
