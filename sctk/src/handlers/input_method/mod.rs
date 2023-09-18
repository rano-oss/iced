use std::marker::PhantomData;

use sctk::reexports::client::globals::{BindError, GlobalList};
use sctk::reexports::client::protocol::wl_surface::WlSurface;
use sctk::reexports::client::Dispatch;
use sctk::reexports::client::{
    delegate_dispatch, Connection, Proxy, QueueHandle,
};
use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_keyboard_grab_v2;
use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_v2;
use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_popup_surface_v2;
use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_keyboard_grab_v2::ZwpInputMethodKeyboardGrabV2,
    zwp_input_method_manager_v2::ZwpInputMethodManagerV2,
    zwp_input_method_v2::ZwpInputMethodV2,
    zwp_input_popup_surface_v2::ZwpInputPopupSurfaceV2,
};

use sctk::globals::GlobalData;

use crate::event_loop::state::SctkState;

use super::wp_fractional_scaling::FractionalScalingManager;

#[derive(Debug)]
pub struct InputMethodManager<T> {
    manager: ZwpInputMethodManagerV2,
    _phantom: PhantomData<T>,
}

impl<T: 'static> InputMethodManager<T> {
    pub fn new(
        globals: &GlobalList,
        queue_handle: &QueueHandle<SctkState<T>>,
    ) -> Result<Self, BindError> {
        let manager = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(Self {
            manager,
            _phantom: PhantomData,
        })
    }
}

impl<T: 'static> Dispatch<ZwpInputMethodManagerV2, GlobalData, SctkState<T>>
    for InputMethodManager<T>
{
    fn event(
        _: &mut SctkState<T>,
        _: &ZwpInputMethodManagerV2,
        _: <ZwpInputMethodManagerV2 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        // No events.
    }
}

pub struct InputMethod {}

impl<T: 'static> Dispatch<ZwpInputMethodV2, InputMethod, SctkState<T>>
    for InputMethodManager<T>
{
    fn event(
        _: &mut SctkState<T>,
        _: &ZwpInputMethodV2,
        event: <ZwpInputMethodV2 as Proxy>::Event,
        _: &InputMethod,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        match event {
            zwp_input_method_v2::Event::Activate => {}
            zwp_input_method_v2::Event::Deactivate => {
                //   data.input_method_context.active = false;
                //     if let Some(popup) =
                //         &data.input_method_context.popup_surface.popup_surface
                //     {
                //         popup.destroy();
                //     }
                //     data.input_method_context.popup_surface.popup_surface = None;
            }
            zwp_input_method_v2::Event::SurroundingText {
                text: _,
                cursor: _,
                anchor: _,
            } => {}
            zwp_input_method_v2::Event::TextChangeCause { cause: _ } => {}
            zwp_input_method_v2::Event::ContentType {
                hint: _,
                purpose: _,
            } => {}
            zwp_input_method_v2::Event::Done => {} //println!("done??")},
            zwp_input_method_v2::Event::Unavailable => {
                panic!("Another input method already present!")
            }
            _ => unreachable!(),
        }
    }
}

pub struct InputMethodKeyboard {}

impl<T: 'static>
    Dispatch<ZwpInputMethodKeyboardGrabV2, InputMethodKeyboard, SctkState<T>>
    for InputMethodManager<T>
{
    fn event(
        _: &mut SctkState<T>,
        _: &ZwpInputMethodKeyboardGrabV2,
        event: <ZwpInputMethodKeyboardGrabV2 as Proxy>::Event,
        _: &InputMethodKeyboard,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        match event {
            zwp_input_method_keyboard_grab_v2::Event::Keymap {
                format: _,
                fd: _,
                size: _,
            } => {}
            zwp_input_method_keyboard_grab_v2::Event::Key {
                serial,
                time,
                key,
                state,
            } => {}
            zwp_input_method_keyboard_grab_v2::Event::Modifiers {
                serial,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
            } => {}
            zwp_input_method_keyboard_grab_v2::Event::RepeatInfo {
                rate,
                delay,
            } => {}
            _ => unreachable!(),
        }
    }
}

pub struct InputMethodPopup {}

impl<T: 'static>
    Dispatch<ZwpInputPopupSurfaceV2, InputMethodPopup, SctkState<T>>
    for InputMethodManager<T>
{
    fn event(
        _: &mut SctkState<T>,
        _: &ZwpInputPopupSurfaceV2,
        event: <ZwpInputPopupSurfaceV2 as Proxy>::Event,
        _: &InputMethodPopup,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        match event {
            zwp_input_popup_surface_v2::Event::TextInputRectangle {
                x,
                y,
                width,
                height,
            } => {}
            _ => unreachable!(),
        }
    }
}

delegate_dispatch!(@<T: 'static> SctkState<T>: [ZwpInputMethodManagerV2: GlobalData] => InputMethodManager<T>);
delegate_dispatch!(@<T: 'static> SctkState<T>: [ZwpInputMethodV2: InputMethod] => InputMethodManager<T>);
delegate_dispatch!(@<T: 'static> SctkState<T>: [ZwpInputMethodKeyboardGrabV2: InputMethodKeyboard] => InputMethodManager<T>);
delegate_dispatch!(@<T: 'static> SctkState<T>: [ZwpInputPopupSurfaceV2: InputMethodPopup] => InputMethodManager<T>);
