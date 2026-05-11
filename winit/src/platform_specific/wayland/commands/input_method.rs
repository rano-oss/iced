//! Interact with the wayland input method protocol.

use iced_runtime::{
    Action, Task,
    platform_specific::{self, wayland},
    task,
};

use crate::core::window::Id as SurfaceId;
pub use wayland::input_method::InputMethodPopupSettings;

/// Set the preedit string displayed to the user.
pub fn set_preedit_string<Message>(
    string: String,
    cursor_begin: i32,
    cursor_end: i32,
) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::SetPreeditString {
                string,
                cursor_begin,
                cursor_end,
            },
        )),
    ))
}

/// Commit a string to the focused text input.
pub fn commit_string<Message>(string: String) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::CommitString(string),
        )),
    ))
}

/// Commit the current input method state.
pub fn commit<Message>() -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::Commit,
        )),
    ))
}

/// Filter a key event: consume (true) or passthrough (false).
pub fn filter_key<Message>(serial: u32, consumed: bool) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::FilterKey(serial, consumed),
        )),
    ))
}

/// Create an input method popup surface.
pub fn get_input_method_popup<Message>(
    settings: InputMethodPopupSettings,
) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::Popup { settings },
        )),
    ))
}

/// Set size of the input method popup.
pub fn set_size<Message>(
    id: SurfaceId,
    width: u32,
    height: u32,
) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::Size { id, width, height },
        )),
    ))
}

/// Freeze or unfreeze the popup position.
/// When frozen, the compositor will not update the popup position
/// in response to cursor rectangle changes from the text input.
pub fn set_frozen<Message>(frozen: bool) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::SetFrozen { frozen },
        )),
    ))
}
