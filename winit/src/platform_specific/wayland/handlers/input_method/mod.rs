pub mod keyboard;

use std::sync::{Arc, Mutex};

use cctk::sctk::{
    globals::GlobalData,
    reexports::client::{
        Connection, Dispatch, Proxy, QueueHandle, WEnum,
        globals::{BindError, GlobalList},
        protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
    },
};
use wayland_protocols_experimental::input_method::v1::client::{
    xx_input_method_manager_v2::XxInputMethodManagerV2,
    xx_input_method_v1::{self, XxInputMethodV1},
    xx_input_popup_positioner_v1::XxInputPopupPositionerV1,
    xx_input_popup_surface_v2::{self, XxInputPopupSurfaceV2},
};

use crate::platform_specific::wayland::{
    event_loop::state::{Common, CommonSurface, SctkState, send_event},
    sctk_event::{
        InputMethodEventVariant, InputMethodPopupEventVariant, SctkEvent,
    },
};

/// Data for InputMethod dispatch
#[derive(Debug, Clone)]
pub struct InputMethod {
    pub(crate) inner: Arc<Mutex<Inner>>,
}

#[derive(Debug, Default)]
pub(crate) struct Inner {
    pub(crate) serial: u32,
    pub(crate) activated: bool,
}

#[derive(Debug)]
pub struct InputMethodManager {
    manager: XxInputMethodManagerV2,
}

/// State for the input method popup surface
#[derive(Debug, Clone)]
pub struct InputMethodPopup {
    pub wl_surface: WlSurface,
    pub popup_role: Option<XxInputPopupSurfaceV2>,
    pub id: crate::core::window::Id,
}

impl InputMethodManager {
    pub fn new(
        globals: &GlobalList,
        queue_handle: &QueueHandle<SctkState>,
    ) -> Result<Self, BindError> {
        let manager = globals.bind(queue_handle, 1..=3, GlobalData)?;
        Ok(Self { manager })
    }

    pub fn input_method(
        &self,
        seat: &WlSeat,
        queue_handle: &QueueHandle<SctkState>,
        _loop_handle: cctk::sctk::reexports::calloop::LoopHandle<
            'static,
            SctkState,
        >,
    ) -> XxInputMethodV1 {
        let data = InputMethod {
            inner: Arc::new(Mutex::new(Inner::default())),
        };
        self.manager.get_input_method(seat, queue_handle, data)
    }

    /// Create a positioner for the popup surface
    pub fn get_positioner(
        &self,
        queue_handle: &QueueHandle<SctkState>,
    ) -> XxInputPopupPositionerV1 {
        self.manager.get_positioner(queue_handle, ())
    }
}

// Dispatch for XxInputMethodManagerV2
impl Dispatch<XxInputMethodManagerV2, GlobalData> for SctkState {
    fn event(
        _state: &mut SctkState,
        _: &XxInputMethodManagerV2,
        _: <XxInputMethodManagerV2 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        // No events for the manager
    }
}

// Dispatch for XxInputMethodV1
impl Dispatch<XxInputMethodV1, InputMethod> for SctkState {
    fn event(
        state: &mut SctkState,
        im: &XxInputMethodV1,
        event: <XxInputMethodV1 as Proxy>::Event,
        data: &InputMethod,
        _conn: &Connection,
        qh: &QueueHandle<SctkState>,
    ) {
        match event {
            xx_input_method_v1::Event::Activate { .. } => {
                data.inner.lock().unwrap().activated = true;
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Activate,
                });
            }
            xx_input_method_v1::Event::Deactivate => {
                data.inner.lock().unwrap().activated = false;
                // Destroy the popup surface entirely on deactivate
                if let Some(popup) = state.input_method_popup.take() {
                    if let Some(role) = popup.popup_role {
                        role.destroy();
                    }
                    let id = popup.id;
                    // Send Done event BEFORE destroying so sctk_event can look up the surface
                    send_event(
                        &state.events_sender,
                        &state.proxy,
                        SctkEvent::InputMethodPopupEvent {
                            variant: InputMethodPopupEventVariant::Done,
                            id: popup.wl_surface.clone(),
                        },
                    );
                    popup.wl_surface.destroy();
                    state.id_map.retain(|_, v| *v != id);
                }
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Deactivate,
                });
            }
            xx_input_method_v1::Event::SurroundingText {
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
            xx_input_method_v1::Event::TextChangeCause { cause } => {
                if let WEnum::Value(cause) = cause {
                    state.sctk_events.push(SctkEvent::InputMethodEvent {
                        variant: InputMethodEventVariant::TextChangeCause(
                            cause as u32,
                        ),
                    });
                }
            }
            xx_input_method_v1::Event::ContentType { hint, purpose } => {
                if let (WEnum::Value(hint), WEnum::Value(purpose)) =
                    (hint, purpose)
                {
                    state.sctk_events.push(SctkEvent::InputMethodEvent {
                        variant: InputMethodEventVariant::ContentType {
                            hint: hint.bits(),
                            purpose: purpose as u32,
                        },
                    });
                }
            }
            xx_input_method_v1::Event::Done => {
                let mut inner = data.inner.lock().unwrap();
                inner.serial += 1;
                let activated = inner.activated;
                drop(inner);

                // If activated and popup doesn't exist yet, create it now
                if activated {
                    if state.input_method_popup.is_none() {
                        if let Some(settings) =
                            state.input_method_popup_settings.clone()
                        {
                            // Get positioner from manager (avoids borrow conflict)
                            let positioner = state
                                .input_method_manager
                                .as_ref()
                                .map(|m| m.get_positioner(qh));
                            if let Some(positioner) = positioner {
                                let (id, wl_surface) =
                                    state.get_input_method_popup(settings);
                                // Size hint for constraint calculations - should be generous
                                positioner.set_size(800, 600);
                                // Position popup at bottom-left of cursor rect, growing down-right
                                use wayland_protocols_experimental::input_method::v1::client::xx_input_popup_positioner_v1::{Anchor, Gravity, ConstraintAdjustment};
                                positioner.set_anchor(Anchor::BottomLeft);
                                positioner.set_gravity(Gravity::BottomRight);
                                positioner.set_offset(0, 0);
                                positioner.set_constraint_adjustment(
                                    ConstraintAdjustment::FlipY
                                        | ConstraintAdjustment::SlideX
                                        | ConstraintAdjustment::SlideY
                                        | ConstraintAdjustment::ResizeX
                                        | ConstraintAdjustment::ResizeY,
                                );
                                let popup_surface = im.get_input_popup_surface(
                                    &wl_surface,
                                    &positioner,
                                    qh,
                                    InputMethodPopup {
                                        wl_surface: wl_surface.clone(),
                                        popup_role: None,
                                        id,
                                    },
                                );
                                positioner.destroy();
                                if let Some(popup_state) =
                                    state.input_method_popup.as_mut()
                                {
                                    popup_state.popup_role =
                                        Some(popup_surface);
                                }
                            }
                        }
                    }
                }

                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Done,
                });
            }
            xx_input_method_v1::Event::SetAvailableActions { .. } => {
                // TODO: handle available actions
            }
            xx_input_method_v1::Event::Unavailable => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Unavailable,
                });
            }
            _ => {}
        }
    }
}

// Dispatch for XxInputPopupPositionerV1 — no events
impl Dispatch<XxInputPopupPositionerV1, ()> for SctkState {
    fn event(
        _state: &mut SctkState,
        _: &XxInputPopupPositionerV1,
        _: <XxInputPopupPositionerV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        // Positioner has no events
    }
}

// Dispatch for XxInputPopupSurfaceV2
impl Dispatch<XxInputPopupSurfaceV2, InputMethodPopup> for SctkState {
    fn event(
        state: &mut SctkState,
        popup: &XxInputPopupSurfaceV2,
        event: <XxInputPopupSurfaceV2 as Proxy>::Event,
        data: &InputMethodPopup,
        _conn: &Connection,
        _qh: &QueueHandle<SctkState>,
    ) {
        match event {
            xx_input_popup_surface_v2::Event::StartConfigure {
                serial,
                width,
                height,
                ..
            } => {
                let wl_surface = &data.wl_surface;
                // Ack the configure
                popup.ack_configure(serial);
                // Ensure frame status is Ready so rendering will happen
                use crate::platform_specific::wayland::event_loop::state::receive_frame;
                receive_frame(&mut state.frame_status, wl_surface);
                state.request_redraw(wl_surface);
                // Send a configure event so process() can trigger a redraw
                send_event(
                    &state.events_sender,
                    &state.proxy,
                    SctkEvent::InputMethodPopupEvent {
                        variant: InputMethodPopupEventVariant::Configure {
                            width: width as i32,
                            height: height as i32,
                        },
                        id: wl_surface.clone(),
                    },
                );
            }
            xx_input_popup_surface_v2::Event::Repositioned { .. } => {
                // Handle repositioned token if needed
            }
            _ => {}
        }
    }
}

impl SctkState {
    pub fn get_input_method_popup(
        &mut self,
        settings: iced_runtime::platform_specific::wayland::input_method::InputMethodPopupSettings,
    ) -> (crate::core::window::Id, WlSurface) {
        use crate::platform_specific::wayland::event_loop::state::receive_frame;

        let id = settings.id;
        let wl_surface =
            self.compositor_state.create_surface(&self.queue_handle);
        _ = self.id_map.insert(wl_surface.id(), id.clone());

        let common = Arc::new(Mutex::new(Common::from(
            winit::dpi::LogicalSize::new(settings.size.0, settings.size.1),
        )));

        wl_surface.commit();
        self.input_method_popup = Some(InputMethodPopup {
            wl_surface: wl_surface.clone(),
            popup_role: None,
            id,
        });

        // Set frame status to Ready so the first redraw will actually render
        receive_frame(&mut self.frame_status, &wl_surface);
        self.request_redraw(&wl_surface);

        // Send Created event to register with iced's rendering pipeline
        send_event(
            &self.events_sender,
            &self.proxy,
            SctkEvent::InputMethodPopupEvent {
                variant: InputMethodPopupEventVariant::Created(
                    self.queue_handle.clone(),
                    CommonSurface::InputMethodPopup(wl_surface.clone()),
                    id,
                    common,
                    self.connection.display(),
                ),
                id: wl_surface.clone(),
            },
        );

        (id, wl_surface)
    }
}
