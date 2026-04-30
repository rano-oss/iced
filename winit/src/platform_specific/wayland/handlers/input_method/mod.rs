pub mod keyboard;

use std::sync::{Arc, Mutex};

use cctk::sctk::reexports::calloop::LoopHandle;
use cctk::sctk::{
    globals::GlobalData,
    reexports::client::{
        Connection, Dispatch, Proxy, QueueHandle,
        globals::{BindError, GlobalList},
        protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
    },
    seat::keyboard::Modifiers,
};
use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_keyboard_grab_v2::ZwpInputMethodKeyboardGrabV2,
    zwp_input_method_manager_v2::ZwpInputMethodManagerV2,
    zwp_input_method_v2::{self, ZwpInputMethodV2},
    zwp_input_popup_surface_v2::{self, ZwpInputPopupSurfaceV2},
};

use crate::platform_specific::wayland::{
    event_loop::state::{SctkState, send_event},
    sctk_event::{InputMethodEventVariant, SctkEvent},
};

use self::keyboard::InputMethodKeyboardData;

#[derive(Debug, Clone)]
pub struct InputMethod {
    pub(crate) inner: Arc<Mutex<Inner>>,
}

#[derive(Debug, Default)]
pub(crate) struct Inner {
    pub(crate) serial: u32,
}

#[derive(Debug)]
pub struct InputMethodManager {
    manager: ZwpInputMethodManagerV2,
}

/// State for the input method popup surface
#[derive(Debug, Clone)]
pub struct InputMethodPopup {
    pub wl_surface: WlSurface,
    pub popup_role: Option<ZwpInputPopupSurfaceV2>,
}

impl InputMethodManager {
    pub fn new(
        globals: &GlobalList,
        queue_handle: &QueueHandle<SctkState>,
    ) -> Result<Self, BindError> {
        let manager = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(Self { manager })
    }

    pub fn input_method(
        &self,
        seat: &WlSeat,
        queue_handle: &QueueHandle<SctkState>,
        loop_handle: LoopHandle<'static, SctkState>,
    ) -> ZwpInputMethodV2 {
        let mut data = InputMethod {
            inner: Arc::new(Mutex::new(Inner::default())),
        };
        let im =
            self.manager
                .get_input_method(seat, queue_handle, data.clone());
        let _ = data.grab_keyboard_with_repeat(
            queue_handle,
            &im,
            None,
            loop_handle,
            Box::new(move |state, _kbd: &ZwpInputMethodKeyboardGrabV2, e| {
                state.sctk_events.push(SctkEvent::InputMethodKeyboardEvent {
                    variant: crate::platform_specific::wayland::sctk_event::InputMethodKeyboardEventVariant::Repeat(e),
                });
            }),
        )
        .expect("Input method keyboard grab failed");
        im
    }
}

impl SctkState {
    pub fn get_input_method_popup(
        &mut self,
        settings: iced_runtime::platform_specific::wayland::input_method_popup::InputMethodPopupSettings,
    ) -> (iced_runtime::core::window::Id, WlSurface) {
        let wl_surface =
            self.compositor_state.create_surface(&self.queue_handle);
        wl_surface.commit();
        self.input_method_popup = Some(InputMethodPopup {
            wl_surface: wl_surface.clone(),
            popup_role: None,
        });
        (settings.id, wl_surface)
    }

    pub fn show_input_method_popup(&mut self) {
        let seat = self.seats.first().expect("seat not present");
        let popup_state = self
            .input_method_popup
            .as_mut()
            .expect("Input Method popup not present");
        if popup_state.popup_role.is_none() {
            popup_state.popup_role = seat.input_method.as_ref().map(|im| {
                im.get_input_popup_surface(
                    &popup_state.wl_surface,
                    &self.queue_handle,
                    popup_state.clone(),
                )
            });
        }
    }

    pub fn hide_input_method_popup(&mut self) {
        if let Some(popup_state) = self.input_method_popup.as_mut() {
            if let Some(popup_role) = popup_state.popup_role.take() {
                popup_role.destroy();
            }
        }
    }
}

// Dispatch for ZwpInputMethodManagerV2
impl Dispatch<ZwpInputMethodManagerV2, GlobalData> for SctkState {
    fn event(
        _state: &mut SctkState,
        _: &ZwpInputMethodManagerV2,
        _: <ZwpInputMethodManagerV2 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        // No events for the manager
    }
}

// Dispatch for ZwpInputMethodV2
impl Dispatch<ZwpInputMethodV2, InputMethod> for SctkState {
    fn event(
        state: &mut SctkState,
        _im: &ZwpInputMethodV2,
        event: <ZwpInputMethodV2 as Proxy>::Event,
        data: &InputMethod,
        _conn: &Connection,
        _qh: &QueueHandle<SctkState>,
    ) {
        match event {
            zwp_input_method_v2::Event::Activate => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Activate,
                });
            }
            zwp_input_method_v2::Event::Deactivate => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Deactivate,
                });
            }
            zwp_input_method_v2::Event::SurroundingText {
                text,
                cursor,
                anchor,
            } => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::SurroundingText {
                        text,
                        cursor,
                        anchor,
                    },
                });
            }
            zwp_input_method_v2::Event::TextChangeCause { cause } => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::TextChangeCause(
                        cause.into(),
                    ),
                });
            }
            zwp_input_method_v2::Event::ContentType { hint, purpose } => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::ContentType {
                        hint: hint.into(),
                        purpose: purpose.into(),
                    },
                });
            }
            zwp_input_method_v2::Event::Done => {
                let mut inner = data.inner.lock().unwrap();
                inner.serial += 1;
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Done,
                });
            }
            _ => {}
        }
    }
}

// Dispatch for ZwpInputPopupSurfaceV2
impl Dispatch<ZwpInputPopupSurfaceV2, InputMethodPopup> for SctkState {
    fn event(
        _state: &mut SctkState,
        _popup: &ZwpInputPopupSurfaceV2,
        event: <ZwpInputPopupSurfaceV2 as Proxy>::Event,
        _data: &InputMethodPopup,
        _conn: &Connection,
        _qh: &QueueHandle<SctkState>,
    ) {
        match event {
            zwp_input_popup_surface_v2::Event::TextInputRectangle {
                x: _,
                y: _,
                width: _,
                height: _,
            } => {
                // Could be used to position the popup
            }
            _ => {}
        }
    }
}
