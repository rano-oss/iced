use std::{
    borrow::Borrow,
    env,
    fmt::Debug,
    marker::PhantomData,
    num::NonZeroU32,
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Duration,
};

use cctk::sctk::{
    globals::GlobalData,
    reexports::calloop::{timer::Timer, LoopHandle, RegistrationToken},
    seat::keyboard::{KeyEvent, Modifiers, RepeatInfo},
};
use wayland_client::{
    delegate_dispatch,
    globals::{BindError, GlobalList},
    protocol::{
        wl_keyboard,
        wl_seat::{self, WlSeat},
        wl_surface,
    },
    Dispatch, Proxy, QueueHandle, WEnum,
};
use wayland_protocols::wp::input_method::v3::client::{
    wp_input_method_manager_v3::WpInputMethodManagerV3,
    wp_input_method_v3::{self, WpInputMethodV3},
};
use xkbcommon::xkb;
use xkeysym::{KeyCode, Keysym};

use crate::{
    event_loop::state::{SctkState, TOKEN_CTR},
    sctk_event::{InputMethodEventVariant, SctkEvent},
    wayland::calloop::timer::TimeoutAction,
};

use super::seat;

pub(crate) struct RepeatedKey {
    pub(crate) key: KeyEvent,
    /// Whether this is the first event of the repeat sequence.
    pub(crate) is_first: bool,
    pub(crate) surface: wl_surface::WlSurface,
}

pub(crate) struct RepeatData<T> {
    pub(crate) current_repeat: Option<RepeatedKey>,
    pub(crate) repeat_info: RepeatInfo,
    pub(crate) loop_handle: LoopHandle<'static, T>,
    pub(crate) repeat_token: Option<RegistrationToken>,
}

impl<T> Drop for RepeatData<T> {
    fn drop(&mut self) {
        if let Some(token) = self.repeat_token.take() {
            self.loop_handle.remove(token);
        }
    }
}

pub struct KeyboardData<T> {
    first_event: AtomicBool,
    xkb_context: Mutex<xkb::Context>,
    xkb_state: Mutex<Option<xkb::State>>,
    xkb_compose: Mutex<Option<xkb::compose::State>>,
    repeat_data: Arc<Mutex<RepeatData<T>>>,
    focus: Mutex<Option<wl_surface::WlSurface>>,
    _phantom_data: PhantomData<T>,
}

impl<T> Debug for KeyboardData<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyboardData").finish_non_exhaustive()
    }
}

impl<T> KeyboardData<T> {
    pub fn new(loop_handle: LoopHandle<'static, T>) -> Self {
        let xkb_context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let xkb_compose = if let Some(locale) = env::var_os("LC_ALL")
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .or_else(|| env::var_os("LC_CTYPE"))
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .or_else(|| env::var_os("LANG"))
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .unwrap_or_else(|| "C".into())
            .to_str()
        {
            // TODO: Pending new release of xkbcommon to use new_from_locale with OsStr
            if let Ok(table) = xkb::compose::Table::new_from_locale(
                &xkb_context,
                locale.as_ref(),
                xkb::compose::COMPILE_NO_FLAGS,
            ) {
                let compose_state = xkb::compose::State::new(
                    &table,
                    xkb::compose::COMPILE_NO_FLAGS,
                );
                Some(compose_state)
            } else {
                None
            }
        } else {
            None
        };
        let udata = KeyboardData {
            first_event: AtomicBool::new(false),
            xkb_context: Mutex::new(xkb_context),
            xkb_state: Mutex::new(None),
            xkb_compose: Mutex::new(xkb_compose),
            repeat_data: Arc::new(Mutex::new(RepeatData {
                current_repeat: None,
                repeat_info: RepeatInfo::Disable,
                loop_handle: loop_handle.clone(),
                repeat_token: None,
            })),
            focus: Mutex::new(None),
            _phantom_data: PhantomData,
        };

        udata
    }

    fn update_modifiers(&self) -> Modifiers {
        let guard = self.xkb_state.lock().unwrap();
        let state = guard.as_ref().unwrap();

        Modifiers {
            ctrl: state.mod_name_is_active(
                xkb::MOD_NAME_CTRL,
                xkb::STATE_MODS_EFFECTIVE,
            ),
            alt: state.mod_name_is_active(
                xkb::MOD_NAME_ALT,
                xkb::STATE_MODS_EFFECTIVE,
            ),
            shift: state.mod_name_is_active(
                xkb::MOD_NAME_SHIFT,
                xkb::STATE_MODS_EFFECTIVE,
            ),
            caps_lock: state.mod_name_is_active(
                xkb::MOD_NAME_CAPS,
                xkb::STATE_MODS_EFFECTIVE,
            ),
            logo: state.mod_name_is_active(
                xkb::MOD_NAME_LOGO,
                xkb::STATE_MODS_EFFECTIVE,
            ),
            num_lock: state.mod_name_is_active(
                xkb::MOD_NAME_NUM,
                xkb::STATE_MODS_EFFECTIVE,
            ),
        }
    }
}

// SAFETY: The state does not share state with any other rust types.
unsafe impl<T> Send for KeyboardData<T> {}
// SAFETY: The state is guarded by a mutex since libxkbcommon has no internal synchronization.
unsafe impl<T> Sync for KeyboardData<T> {}

#[derive(Debug)]
pub struct InputMethodManager {
    manager: WpInputMethodManagerV3,
}

#[derive(Debug, Default)]
pub(crate) struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub struct InputMethod {
    keyboard_data: KeyboardData<SctkState>,
    pub cursor_rectangle: Mutex<Rectangle>,
    seat: WlSeat,
    pub serial: Mutex<u32>,
}

impl InputMethodManager {
    pub fn new(
        globals: &GlobalList,
        queue_handle: &QueueHandle<SctkState>,
    ) -> Result<Self, BindError> {
        let manager = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(Self { manager })
    }

    pub fn new_input_method(
        &self,
        seat: &WlSeat,
        app_id: String,
        queue_handle: &QueueHandle<SctkState>,
        loop_handle: LoopHandle<'static, SctkState>,
    ) -> WpInputMethodV3 {
        let keyboard_data = KeyboardData::new(loop_handle.clone());
        let data = InputMethod {
            keyboard_data,
            cursor_rectangle: Mutex::new(Rectangle::default()),
            seat: seat.clone(),
            serial: 0.into(),
        };
        self.manager
            .get_input_method(seat, app_id, queue_handle, data)
    }
}

impl Dispatch<WpInputMethodManagerV3, GlobalData, SctkState>
    for InputMethodManager
{
    fn event(
        state: &mut SctkState,
        proxy: &WpInputMethodManagerV3,
        event: <WpInputMethodManagerV3 as wayland_client::Proxy>::Event,
        data: &GlobalData,
        conn: &wayland_client::Connection,
        qhandle: &QueueHandle<SctkState>,
    ) {
        // No events.
    }
}

impl Dispatch<WpInputMethodV3, InputMethod, SctkState> for InputMethodManager {
    fn event(
        state: &mut SctkState,
        proxy: &WpInputMethodV3,
        event: <WpInputMethodV3 as wayland_client::Proxy>::Event,
        data: &InputMethod,
        conn: &wayland_client::Connection,
        qhandle: &QueueHandle<SctkState>,
    ) {
        let seat_id = data.seat.clone();
        match event {
            wp_input_method_v3::Event::Activate { app_id } => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Activate { app_id },
                    seat_id,
                });
            }
            wp_input_method_v3::Event::Deactivate => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Deactivate,
                    seat_id,
                })
            }
            wp_input_method_v3::Event::TextInputDestroyed { app_id } => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::TextInputDestroyed {
                        app_id,
                    },
                    seat_id,
                })
            }
            wp_input_method_v3::Event::SurroundingText {
                text,
                cursor,
                anchor,
            } => state.sctk_events.push(SctkEvent::InputMethodEvent {
                variant: InputMethodEventVariant::SurroundingText {
                    text,
                    cursor,
                    anchor,
                },
                seat_id,
            }),
            wp_input_method_v3::Event::TextChangeCause { cause } => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::TextChangeCause {
                        cause: cause.into_result().unwrap(),
                    },
                    seat_id,
                })
            }
            wp_input_method_v3::Event::ContentType { hint, purpose } => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::ContentType {
                        hint: hint.into_result().unwrap(),
                        purpose: purpose.into_result().unwrap(),
                    },
                    seat_id,
                })
            }
            wp_input_method_v3::Event::Done => {
                *data.serial.lock().unwrap() += 1;
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Done,
                    seat_id,
                })
            }
            wp_input_method_v3::Event::AvailableActions {
                available_actions,
            } => state.sctk_events.push(SctkEvent::InputMethodEvent {
                variant: InputMethodEventVariant::AvailableActions {
                    available_actions,
                },
                seat_id,
            }),
            wp_input_method_v3::Event::CursorRectangle {
                x,
                y,
                width,
                height,
            } => {
                if let Some(im_popup) = &state.input_method_popup {
                    // update positioner
                    im_popup
                        .data
                        .positioner
                        .set_anchor_rect(x, y, width, height);
                    im_popup.popup.reposition(
                        &im_popup.data.positioner,
                        TOKEN_CTR
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    )
                }
                let mut rectangle_guard = data.cursor_rectangle.lock().unwrap();
                rectangle_guard.x = x;
                rectangle_guard.y = y;
                rectangle_guard.width = width;
                rectangle_guard.height = height;
            }
            wp_input_method_v3::Event::Keymap { format, fd, size } => {
                let udata = &data.keyboard_data;
                match format {
                    WEnum::Value(format) => match format {
                        wl_keyboard::KeymapFormat::NoKeymap => {
                            log::warn!(target: "sctk", "non-xkb compatible keymap");
                        }

                        wl_keyboard::KeymapFormat::XkbV1 => {
                            let context = udata.xkb_context.lock().unwrap();

                            match unsafe {
                                xkb::Keymap::new_from_fd(
                                    &context,
                                    fd,
                                    size as usize,
                                    xkb::KEYMAP_FORMAT_TEXT_V1,
                                    xkb::COMPILE_NO_FLAGS,
                                )
                            } {
                                Ok(Some(keymap)) => {
                                    let state = xkb::State::new(&keymap);
                                    {
                                        let mut state_guard =
                                            udata.xkb_state.lock().unwrap();
                                        *state_guard = Some(state);
                                    }
                                }
                                Ok(None) => {
                                    log::error!(target: "sctk", "invalid keymap");
                                }

                                Err(err) => {
                                    log::error!(target: "sctk", "{}", err);
                                }
                            }
                        }

                        _ => unreachable!(),
                    },

                    WEnum::Unknown(value) => {
                        log::warn!(target: "sctk", "unknown keymap format 0x{:x}", value)
                    }
                }
            }
            wp_input_method_v3::Event::RepeatInfo { rate, delay } => {
                let udata = &data.keyboard_data;
                let info = if rate != 0 {
                    RepeatInfo::Repeat {
                        rate: NonZeroU32::new(rate as u32).unwrap(),
                        delay: delay as u32,
                    }
                } else {
                    RepeatInfo::Disable
                };

                let mut repeat_data = udata.repeat_data.lock().unwrap();
                repeat_data.repeat_info = info;
            }
            wp_input_method_v3::Event::Key {
                serial,
                time,
                key,
                state: key_state,
            } => {
                let udata = &data.keyboard_data;
                match key_state {
                    WEnum::Value(key_state) => {
                        let state_guard = udata.xkb_state.lock().unwrap();

                        if let Some(guard) = state_guard.as_ref() {
                            // We must add 8 to the keycode for any functions we pass the raw keycode into per
                            // wl_keyboard protocol.
                            let keycode = KeyCode::new(key + 8);
                            let keysym = guard.key_get_one_sym(keycode);
                            let utf8 = if key_state
                                == wl_keyboard::KeyState::Pressed
                            {
                                let mut compose =
                                    udata.xkb_compose.lock().unwrap();

                                match compose.as_mut() {
                                    Some(compose) => match compose.feed(keysym)
                                    {
                                        xkb::FeedResult::Ignored => None,
                                        xkb::FeedResult::Accepted => {
                                            match compose.status() {
                                                xkb::Status::Composed => {
                                                    compose.utf8()
                                                }
                                                xkb::Status::Nothing => Some(
                                                    guard.key_get_utf8(keycode),
                                                ),
                                                _ => None,
                                            }
                                        }
                                    },

                                    // No compose
                                    None => Some(guard.key_get_utf8(keycode)),
                                }
                            } else {
                                None
                            };

                            // Drop guard before calling user code.
                            drop(state_guard);

                            let event = KeyEvent {
                                time,
                                raw_code: key,
                                keysym,
                                utf8,
                            };

                            match key_state {
                                wl_keyboard::KeyState::Released => {
                                    {
                                        let mut repeat_data =
                                            udata.repeat_data.lock().unwrap();
                                        if Some(event.raw_code)
                                            == repeat_data
                                                .current_repeat
                                                .as_ref()
                                                .map(|r| r.key.raw_code)
                                        {
                                            repeat_data.current_repeat = None;
                                        }
                                    }
                                    // TODO: release key data
                                    state.sctk_events.push(
                                        SctkEvent::InputMethodEvent {
                                            variant:
                                                InputMethodEventVariant::Release(
                                                    event
                                                ),
                                            seat_id
                                        },
                                    );
                                }

                                wl_keyboard::KeyState::Pressed => {
                                    {
                                        let mut repeat_data =
                                            udata.repeat_data.lock().unwrap();
                                        let state_guard =
                                            udata.xkb_state.lock().unwrap();
                                        let key_repeats = state_guard
                                            .as_ref()
                                            .map(|guard| {
                                                guard.get_keymap().key_repeats(
                                                    KeyCode::new(
                                                        event.raw_code + 8,
                                                    ),
                                                )
                                            })
                                            .unwrap_or_default();
                                        if key_repeats {
                                            // Cancel the previous timer / repeat.
                                            if let Some(token) =
                                                repeat_data.repeat_token.take()
                                            {
                                                &repeat_data
                                                    .loop_handle
                                                    .remove(token);
                                            }

                                            let surface = match udata
                                                .focus
                                                .lock()
                                                .unwrap()
                                                .as_ref()
                                                .cloned()
                                            {
                                                Some(surface) => surface,

                                                None => {
                                                    log::warn!(
                                                "wl_keyboard::key with no focused surface");
                                                    return;
                                                }
                                            };

                                            // Update the current repeat key.
                                            let _ = repeat_data
                                                .current_repeat
                                                .replace(RepeatedKey {
                                                    key: event.clone(),
                                                    is_first: true,
                                                    surface,
                                                });

                                            let (delay, rate) =
                                                match repeat_data.repeat_info {
                                                    RepeatInfo::Disable => {
                                                        return
                                                    }
                                                    RepeatInfo::Repeat {
                                                        delay,
                                                        rate,
                                                    } => (delay, rate),
                                                };
                                            let gap = Duration::from_micros(
                                                1_000_000 / rate.get() as u64,
                                            );
                                            let timer = Timer::from_duration(
                                                Duration::from_millis(
                                                    delay as u64,
                                                ),
                                            );
                                            let repeat_data2 =
                                                udata.repeat_data.clone();

                                            // Start the timer.
                                            let im = proxy.clone();
                                            let seat_id_2 = seat_id.clone();
                                            if let Ok(token) = &repeat_data.loop_handle.insert_source(
                                                timer,
                                                move |_, _, state| {
                                                    let mut repeat_data =
                                                        repeat_data2.lock().unwrap();

                                                    let key = &mut repeat_data.current_repeat;
                                                    if key.is_none() {
                                                        return TimeoutAction::Drop;
                                                    }
                                                    let key = key.as_mut().unwrap();
                                                    // If surface was closed while focused, no `Leave`
                                                    // event occurred.
                                                    if !key.surface.is_alive() {
                                                        return TimeoutAction::Drop;
                                                    }
                                                    key.key.time += if key.is_first {
                                                        key.is_first = false;
                                                        delay
                                                    } else {
                                                        gap.as_millis() as u32
                                                    };
                                                    if let Some(my_seat) = state
                                                        .seats
                                                        .iter_mut()
                                                        .find(|s| s.seat == seat_id_2)
                                                    {
                                                        state.sctk_events.push(SctkEvent::InputMethodEvent { variant: InputMethodEventVariant::Repeat(key.key.clone()), seat_id: seat_id_2.clone() });
                                                    }
                                                    TimeoutAction::ToDuration(gap)
                                                },
                                            ) {
                                                repeat_data.repeat_token = Some(*token);
                                            }
                                        }
                                    }
                                    state.sctk_events.push(
                                        SctkEvent::InputMethodEvent {
                                            variant:
                                                InputMethodEventVariant::Press(
                                                    event,
                                                ),
                                            seat_id,
                                        },
                                    );
                                }

                                _ => unreachable!(),
                            }
                        };
                    }

                    WEnum::Unknown(unknown) => {
                        log::warn!(target: "sctk", "{}: compositor sends invalid key state: {:x}", proxy.id(), unknown);
                    }
                }
            }
            wp_input_method_v3::Event::Modifiers {
                serial,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
            } => {
                let udata = &data.keyboard_data;
                let mut guard = udata.xkb_state.lock().unwrap();

                let xkb_state = match guard.as_mut() {
                    Some(state) => state,
                    None => return,
                };

                // Apply the new xkb state with the new modifiers.
                let _ = xkb_state.update_mask(
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    0,
                    0,
                    group,
                );

                // Update the currently repeating key if any.
                let mut repeat_data = udata.repeat_data.lock().unwrap();
                if let Some(mut event) = repeat_data.current_repeat.take() {
                    // Apply new modifiers to get new utf8.
                    event.key.utf8 = {
                        let mut compose = udata.xkb_compose.lock().unwrap();

                        match compose.as_mut() {
                            Some(compose) => match compose
                                .feed(event.key.keysym)
                            {
                                xkb::FeedResult::Ignored => None,
                                xkb::FeedResult::Accepted => match compose
                                    .status()
                                {
                                    xkb::Status::Composed => compose.utf8(),
                                    xkb::Status::Nothing => Some(
                                        xkb_state.key_get_utf8(KeyCode::new(
                                            event.key.raw_code + 8,
                                        )),
                                    ),
                                    _ => None,
                                },
                            },

                            // No compose.
                            None => Some(xkb_state.key_get_utf8(KeyCode::new(
                                event.key.raw_code + 8,
                            ))),
                        }
                    };

                    // Update the stored event.
                    repeat_data.current_repeat = Some(event);
                }

                // Drop guard before calling user code.
                drop(guard);

                // Always issue the modifiers update for the user.
                let modifiers = udata.update_modifiers();
                //TODO: Modifiers are forwarded here
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Modifiers(modifiers),
                    seat_id,
                })
            }
            _ => unreachable!(),
        }
    }
}

delegate_dispatch!(SctkState: [WpInputMethodManagerV3: GlobalData] => InputMethodManager);
delegate_dispatch!(SctkState: [WpInputMethodV3: InputMethod] => InputMethodManager);
