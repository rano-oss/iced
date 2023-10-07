pub mod keyboard;
use std::fmt::Debug;
use std::marker::PhantomData;

use sctk::reexports::calloop::LoopHandle;
use sctk::reexports::client::globals::{BindError, GlobalList};
use sctk::reexports::client::protocol::wl_seat::WlSeat;
use sctk::reexports::client::Dispatch;
use sctk::reexports::client::{
    delegate_dispatch, Connection, Proxy, QueueHandle,
};
use sctk::seat::keyboard::{KeyEvent, Modifiers};
use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_v2;
use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_popup_surface_v2;
use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_keyboard_grab_v2::ZwpInputMethodKeyboardGrabV2,
    zwp_input_method_manager_v2::ZwpInputMethodManagerV2,
    zwp_input_method_v2::ZwpInputMethodV2,
    zwp_input_popup_surface_v2::ZwpInputPopupSurfaceV2,
};

use sctk::globals::GlobalData;

use crate::delegate_input_method_keyboard;
use crate::event_loop::state::SctkState;
use crate::sctk_event::{
    InputMethodEventVariant, KeyboardEventVariant, SctkEvent,
};

use self::keyboard::InputMethodKeyboardHandler;

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

    pub fn input_method(
        &self,
        seat: &WlSeat,
        queue_handle: &QueueHandle<SctkState<T>>,
        loop_handle: LoopHandle<'static, SctkState<T>>,
    ) -> ZwpInputMethodV2 {
        let mut data = InputMethod {};
        let im =
            self.manager
                .get_input_method(seat, queue_handle, data.clone());
        data.grab_keyboard_with_repeat(
            queue_handle,
            &im,
            None,
            loop_handle,
            Box::new(move |state, _kbd: &ZwpInputMethodKeyboardGrabV2, e| {
                state.sctk_events.push(SctkEvent::InputMethodKeyboardEvent {
                    variant: KeyboardEventVariant::Repeat(e),
                })
            }),
        )
        .expect("Input method keyboard grab failed");
        im
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

#[derive(Clone)]
pub struct InputMethod {}

impl<T: 'static> Dispatch<ZwpInputMethodV2, InputMethod, SctkState<T>>
    for InputMethodManager<T>
{
    fn event(
        state: &mut SctkState<T>,
        _: &ZwpInputMethodV2,
        event: <ZwpInputMethodV2 as Proxy>::Event,
        _: &InputMethod,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        match event {
            zwp_input_method_v2::Event::Activate => {
                println!("Activate");
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Activate,
                })
            }
            zwp_input_method_v2::Event::Deactivate => {
                println!("DeActivate");
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Deactivate,
                })
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
            zwp_input_method_v2::Event::Done => {}
            zwp_input_method_v2::Event::Unavailable => {
                panic!("Another input method already present!")
            }
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
                x: _,
                y: _,
                width: _,
                height: _,
            } => {}
            _ => unreachable!(),
        }
    }
}

delegate_dispatch!(@<T: 'static> SctkState<T>: [ZwpInputMethodManagerV2: GlobalData] => InputMethodManager<T>);
delegate_dispatch!(@<T: 'static> SctkState<T>: [ZwpInputMethodV2: InputMethod] => InputMethodManager<T>);
delegate_dispatch!(@<T: 'static> SctkState<T>: [ZwpInputPopupSurfaceV2: InputMethodPopup] => InputMethodManager<T>);

impl<T: 'static> InputMethodKeyboardHandler for SctkState<T> {
    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &ZwpInputMethodKeyboardGrabV2,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.sctk_events.push(SctkEvent::InputMethodKeyboardEvent {
            variant: KeyboardEventVariant::Press(event),
        });
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &ZwpInputMethodKeyboardGrabV2,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.sctk_events.push(SctkEvent::InputMethodKeyboardEvent {
            variant: KeyboardEventVariant::Release(event),
        });
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &ZwpInputMethodKeyboardGrabV2,
        _serial: u32,
        modifiers: Modifiers,
    ) {
        self.sctk_events.push(SctkEvent::InputMethodKeyboardEvent {
            variant: KeyboardEventVariant::Modifiers(modifiers),
        });
    }
}

delegate_input_method_keyboard!(@<T: 'static> SctkState<T>);
