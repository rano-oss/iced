//! Interact with the popups of your application.
use crate::core::window::Id as SurfaceId;
use iced_runtime::{
    self,
    platform_specific::{
        self,
        wayland::{self, input_method::InputMethodPopupSettings},
    },
    task, Action, Task,
};
use wayland_protocols::wp::text_input::v3::client::wp_text_input_v3::{
    Action as IMAction, CommitMode, PreeditColorHint, PreeditStyle,
    PreeditUnderline,
};

/// TODO: fix this to input method popup request
/// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:request:get_popup>
pub fn get_input_method_popup<Message>(
    popup: InputMethodPopupSettings,
) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::Popup { popup },
        )),
    ))
}

/// <https://wayland.app/protocols/xdg-shell#xdg_popup:request:reposition>
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

/// <https://wayland.app/protocols/xdg-shell#xdg_popup:request:destroy>
pub fn destroy_input_method_popup<Message>(id: SurfaceId) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::Destroy { id },
        )),
    ))
}

pub fn commit<Message>() -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::Commit,
        )),
    ))
}

pub fn set_preedit_string<Message>(
    text: String,
    cursor_begin: i32,
    cursor_end: i32,
) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::SetPreeditString {
                text,
                cursor_begin,
                cursor_end,
            },
        )),
    ))
}

pub fn set_string<Message>(text: String) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::SetString { text },
        )),
    ))
}

pub fn delete_surrounding_text<Message>(
    before_length: u32,
    after_length: u32,
) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::DeleteSurroundingText {
                before_length,
                after_length,
            },
        )),
    ))
}

pub fn set_action<Message>(action: IMAction) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::SetAction { action },
        )),
    ))
}

pub fn set_language<Message>(language: String) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::SetLanguage { language },
        )),
    ))
}

pub fn set_preedit_commit_mode<Message>(mode: CommitMode) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::SetPreeditCommitMode { mode },
        )),
    ))
}

pub fn set_preedit_style<Message>(
    begin: i32,
    end: i32,
    underline: PreeditUnderline,
    style: PreeditStyle,
    color: PreeditColorHint,
) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            wayland::input_method::Action::SetPreeditStyle {
                begin,
                end,
                underline,
                style,
                color,
            },
        )),
    ))
}
