//! Interact with the wayland input method popup surface.

use iced_runtime::{
    Action, Task,
    platform_specific::{self, wayland},
    task,
};

pub use wayland::input_method_popup::InputMethodPopupSettings;

/// Create an input method popup surface.
pub fn get_popup<Message>(settings: InputMethodPopupSettings) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethodPopup(
            wayland::input_method_popup::Action::Popup { settings },
        )),
    ))
}

/// Show the input method popup.
pub fn show_input_method_popup<Message>() -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethodPopup(
            wayland::input_method_popup::Action::ShowPopup,
        )),
    ))
}

/// Hide the input method popup.
pub fn hide_input_method_popup<Message>() -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethodPopup(
            wayland::input_method_popup::Action::HidePopup,
        )),
    ))
}
