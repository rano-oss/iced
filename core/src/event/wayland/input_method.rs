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
        cause: ChangeCause,
    },
    ContentType {
        hint: ContentHint,
        purpose: ContentPurpose,
    },
    Done,
    AvailableActions {
        available_actions: Vec<u8>,
    },
}
