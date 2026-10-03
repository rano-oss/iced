//! Interact with the wayland input method protocol.

use iced_runtime::{
    Action, Task,
    platform_specific::{self, wayland},
    task,
};

use crate::core::window::Id as SurfaceId;
pub use wayland::input_method::InputMethodPopupSettings;
pub use wayland::input_method::{
    Anchor, ConstraintAdjustment, Gravity, PopupPositionMode, PopupPositioner,
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

/// Set size of the input method popup with default positioner settings.
pub fn set_size<Message>(
    id: SurfaceId,
    width: u32,
    height: u32,
) -> Task<Message> {
    set_size_with_positioner(id, width, height, PopupPositioner::default())
}

/// Set size of the input method popup with custom positioner settings.
///
/// Use the re-exported `Anchor`, `Gravity`, and `ConstraintAdjustment` types
/// from this module to configure positioning behavior.
pub fn set_size_with_positioner<Message>(
    id: SurfaceId,
    width: u32,
    height: u32,
    positioner: PopupPositioner,
) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::Size {
                id,
                width,
                height,
                positioner,
            },
        )),
    ))
}

/// Reset the popup's last tracked positioner size.
/// Call this when the popup is hidden so subsequent size requests start fresh.
pub fn reset_popup_size<Message>() -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::ResetPopupSize,
        )),
    ))
}

/// Set how the compositor positions the input method popup.
pub fn set_popup_position_mode<Message>(
    mode: PopupPositionMode,
) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::SetPopupPositionMode { mode },
        )),
    ))
}
