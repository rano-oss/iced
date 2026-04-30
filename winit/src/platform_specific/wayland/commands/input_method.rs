//! Interact with the wayland input method protocol.

use iced_runtime::{
    Action, Task,
    platform_specific::{self, wayland},
    task,
};

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

/// Filter a key event, indicating whether it was consumed by the IME.
pub fn filter_key<Message>(serial: u32, consumed: bool) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::FilterKey(serial, consumed),
        )),
    ))
}
