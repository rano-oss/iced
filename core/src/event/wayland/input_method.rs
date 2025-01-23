use cctk::wayland_client::WEnum;
use wayland_protocols::wp::text_input::v3::client::wp_text_input_v3::{
    ChangeCause, ContentHint, ContentPurpose,
};

/// Input method events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMethodEvent {
    Activate {
        app_id: String,
    },
    Deactivate,
    TextInputDestroyed {
        app_id: String,
    },
    SurroundingText {
        text: String,
        cursor: u32,
        anchor: u32,
    },
    TextChangeCause {
        cause: WEnum<ChangeCause>,
    },
    ContentType {
        hint: WEnum<ContentHint>,
        purpose: WEnum<ContentPurpose>,
    },
    Done,
    // Keymap {
    //     format: WEnum<KeymapFormat>,
    //     fd: OwnedFd,
    //     size: u32,
    // },
    // RepeatInfo {
    //     rate: i32,
    //     delay: i32,
    // },
    // Key {
    //     serial: u32,
    //     time: u32,
    //     key: u32,
    //     state: WEnum<KeyState>,
    // },
    // Modifiers {
    //     serial: u32,
    //     mods_depressed: u32,
    //     mods_latched: u32,
    //     mods_locked: u32,
    //     group: u32,
    // },
    AvailableActions {
        available_actions: Vec<u8>,
    },
    // CursorRectangle {
    //     x: i32,
    //     y: i32,
    //     width: i32,
    //     height: i32,
    // },
}
