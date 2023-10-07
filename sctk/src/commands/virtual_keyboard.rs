//! Interact with the virtual keyboard from your application.
use iced_runtime::command::Command;
use iced_runtime::command::{
    self,
    platform_specific::{self, wayland},
};
use sctk::seat::keyboard::KeyEvent;

pub fn virtual_keyboard_key_press<Message>(
    key_event: KeyEvent,
) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::VirtualKeyboard(
            wayland::virtual_keyboard::ActionInner::KeyPressed { key_event }
                .into(),
        )),
    ))
}
