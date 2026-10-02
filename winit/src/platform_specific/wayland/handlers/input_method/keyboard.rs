use std::{
    env,
    fmt::Debug,
    num::NonZeroU32,
    sync::{Arc, Mutex},
    time::Duration,
};

use cctk::sctk::reexports::calloop::{
    LoopHandle, RegistrationToken,
    timer::{TimeoutAction, Timer},
};
use cctk::sctk::{
    reexports::client::{
        Connection, Dispatch, Proxy, QueueHandle, WEnum,
        protocol::wl_keyboard::{self, WlKeyboard},
    },
    seat::keyboard::{KeyEvent, Modifiers, RepeatInfo},
};

use xkbcommon::xkb;

use wayland_protocols_experimental::keyboard_filter::v3::client::{
    xx_keyboard_filter_manager_v1::XxKeyboardFilterManagerV1,
    xx_keyboard_filter_v1::XxKeyboardFilterV1,
};

use crate::platform_specific::wayland::event_loop::state::SctkState;
use crate::platform_specific::wayland::sctk_event::{
    InputMethodKeyboardEventVariant, SctkEvent,
};

pub(crate) struct RepeatedKey {
    pub(crate) key: KeyEvent,
    pub(crate) is_first: bool,
}

pub type RepeatCallback =
    Box<dyn FnMut(&mut SctkState, &WlKeyboard, KeyEvent) + 'static>;

pub(crate) struct RepeatData {
    pub(crate) current_repeat: Option<RepeatedKey>,
    pub(crate) repeat_info: RepeatInfo,
    pub(crate) loop_handle: LoopHandle<'static, SctkState>,
    pub(crate) callback: RepeatCallback,
    pub(crate) repeat_token: Option<RegistrationToken>,
}

impl Drop for RepeatData {
    fn drop(&mut self) {
        if let Some(token) = self.repeat_token.take() {
            self.loop_handle.remove(token);
        }
    }
}

pub struct InputMethodKeyboardData {
    xkb_context: Mutex<xkb::Context>,
    user_specified_rmlvo: bool,
    xkb_state: Mutex<Option<xkb::State>>,
    xkb_compose: Mutex<Option<xkb::compose::State>>,
    pub(crate) repeat_data: Arc<Mutex<Option<RepeatData>>>,
}

impl Debug for InputMethodKeyboardData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InputMethodKeyboardData")
            .finish_non_exhaustive()
    }
}

// SAFETY: The state does not share state with any other rust types.
unsafe impl Send for InputMethodKeyboardData {}
// SAFETY: The state is guarded by a mutex since libxkbcommon has no internal synchronization.
unsafe impl Sync for InputMethodKeyboardData {}

impl InputMethodKeyboardData {
    pub fn new(
        loop_handle: LoopHandle<'static, SctkState>,
        callback: RepeatCallback,
    ) -> Self {
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

        InputMethodKeyboardData {
            xkb_context: Mutex::new(xkb_context),
            user_specified_rmlvo: false,
            xkb_state: Mutex::new(None),
            xkb_compose: Mutex::new(xkb_compose),
            repeat_data: Arc::new(Mutex::new(Some(RepeatData {
                current_repeat: None,
                repeat_info: RepeatInfo::Disable,
                loop_handle: loop_handle.clone(),
                callback,
                repeat_token: None,
            }))),
        }
    }

    pub fn update_modifiers(&self) -> Modifiers {
        let guard = self.xkb_state.lock().unwrap();
        let Some(state) = guard.as_ref() else {
            return Modifiers::default();
        };
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

// Dispatch for WlKeyboard with InputMethodKeyboardData
// This handles keyboard events for the IM-bound keyboard
impl Dispatch<WlKeyboard, InputMethodKeyboardData> for SctkState {
    fn event(
        state: &mut SctkState,
        keyboard: &WlKeyboard,
        event: <WlKeyboard as Proxy>::Event,
        udata: &InputMethodKeyboardData,
        _conn: &Connection,
        _qh: &QueueHandle<SctkState>,
    ) {
        log::debug!(target: "im_kbd", "IM keyboard event: {:?}", std::mem::discriminant(&event));
        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => match format {
                WEnum::Value(format) => match format {
                    wl_keyboard::KeymapFormat::NoKeymap => {
                        log::warn!(target: "sctk", "non-xkb compatible keymap");
                    }
                    wl_keyboard::KeymapFormat::XkbV1 => {
                        if udata.user_specified_rmlvo {
                            return;
                        }

                        let context = udata.xkb_context.lock().unwrap();

                        #[allow(unused_unsafe)]
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
                                let xkb_state = xkb::State::new(&keymap);
                                *udata.xkb_state.lock().unwrap() =
                                    Some(xkb_state);
                            }
                            Ok(None) => {
                                log::error!(target: "sctk", "invalid keymap");
                            }
                            Err(err) => {
                                log::error!(target: "sctk", "{}", err);
                            }
                        }
                    }
                    _ => {
                        log::warn!(target: "sctk", "unknown keymap format variant");
                    }
                },
                WEnum::Unknown(value) => {
                    log::warn!(target: "sctk", "unknown keymap format 0x{:x}", value)
                }
            },

            wl_keyboard::Event::Key {
                serial,
                time,
                key,
                state: key_state,
            } => match key_state {
                WEnum::Value(key_state) => {
                    let state_guard = udata.xkb_state.lock().unwrap();

                    if let Some(guard) = state_guard.as_ref() {
                        let keysym = guard.key_get_one_sym((key + 8).into());
                        let utf8 = if key_state
                            == wl_keyboard::KeyState::Pressed
                        {
                            let mut compose = udata.xkb_compose.lock().unwrap();

                            match compose.as_mut() {
                                Some(compose) => match compose.feed(keysym) {
                                    xkb::FeedResult::Ignored => None,
                                    xkb::FeedResult::Accepted => match compose
                                        .status()
                                    {
                                        xkb::Status::Composed => compose.utf8(),
                                        xkb::Status::Nothing => Some(
                                            guard
                                                .key_get_utf8((key + 8).into()),
                                        ),
                                        _ => None,
                                    },
                                },
                                None => {
                                    Some(guard.key_get_utf8((key + 8).into()))
                                }
                            }
                        } else if key_state
                            == wl_keyboard::KeyState::Repeated
                        {
                            // No compose feed on repeats — reuse the mapped char.
                            Some(guard.key_get_utf8((key + 8).into()))
                        } else {
                            None
                        };

                        drop(state_guard);

                        let event = KeyEvent {
                            time,
                            raw_code: key,
                            keysym: keysym.into(),
                            utf8,
                        };

                        match key_state {
                            wl_keyboard::KeyState::Released => {
                                if let Some(repeat_data) =
                                    udata.repeat_data.lock().unwrap().as_mut()
                                {
                                    if Some(event.raw_code)
                                        == repeat_data
                                            .current_repeat
                                            .as_ref()
                                            .map(|r| r.key.raw_code)
                                    {
                                        repeat_data.current_repeat = None;
                                        if let Some(token) =
                                            repeat_data.repeat_token.take()
                                        {
                                            repeat_data
                                                .loop_handle
                                                .remove(token);
                                        }
                                    }
                                }
                                state.sctk_events.push(SctkEvent::InputMethodKeyboardEvent {
                                    variant: InputMethodKeyboardEventVariant::Release(
                                        event,
                                        udata.update_modifiers(),
                                        serial,
                                    ),
                                });
                            }
                            wl_keyboard::KeyState::Repeated => {
                                // Compositor-owned repeat (wl_keyboard v10+).
                                // Must carry a real serial so the IME can
                                // filter/passthrough after preedit clears.
                                state.sctk_events.push(
                                    SctkEvent::InputMethodKeyboardEvent {
                                        variant:
                                            InputMethodKeyboardEventVariant::Repeat(
                                                event,
                                                udata.update_modifiers(),
                                                serial,
                                            ),
                                    },
                                );
                            }
                            wl_keyboard::KeyState::Pressed => {
                                // Push the press event first, before repeat setup
                                state
                                    .sctk_events
                                    .push(SctkEvent::InputMethodKeyboardEvent {
                                    variant:
                                        InputMethodKeyboardEventVariant::Press(
                                            event.clone(),
                                            udata.update_modifiers(),
                                            serial,
                                        ),
                                });

                                // With keyboard-filter, the compositor owns key
                                // repeat (wl_keyboard Repeated / press+release).
                                // Client-side synthetic repeats use serial 0 and
                                // cannot be passthrough'd after preedit ends.
                                if state.keyboard_filter.is_some() {
                                    return;
                                }

                                if let Some(repeat_data) =
                                    udata.repeat_data.lock().unwrap().as_mut()
                                {
                                    let loop_handle =
                                        &mut repeat_data.loop_handle;
                                    let state_guard =
                                        udata.xkb_state.lock().unwrap();
                                    let key_repeats = state_guard
                                        .as_ref()
                                        .map(|guard| {
                                            guard.get_keymap().key_repeats(
                                                (event.raw_code + 8).into(),
                                            )
                                        })
                                        .unwrap_or_default();
                                    if key_repeats {
                                        if let Some(token) =
                                            repeat_data.repeat_token.take()
                                        {
                                            loop_handle.remove(token);
                                        }

                                        let _ = repeat_data
                                            .current_repeat
                                            .replace(RepeatedKey {
                                                key: event.clone(),
                                                is_first: true,
                                            });

                                        let (delay, rate) =
                                            match repeat_data.repeat_info {
                                                RepeatInfo::Disable => return,
                                                RepeatInfo::Repeat {
                                                    delay,
                                                    rate,
                                                } => (delay, rate),
                                            };
                                        let gap = Duration::from_micros(
                                            1_000_000 / rate.get() as u64,
                                        );
                                        let timer = Timer::from_duration(
                                            Duration::from_millis(delay as u64),
                                        );
                                        let repeat_data2 =
                                            udata.repeat_data.clone();

                                        let kbd = keyboard.clone();
                                        if let Ok(token) = loop_handle.insert_source(
                                            timer,
                                            move |_, _, loop_state| {
                                                let mut repeat_data =
                                                    repeat_data2.lock().unwrap();
                                                let repeat_data = match repeat_data.as_mut() {
                                                    Some(repeat_data) => repeat_data,
                                                    None => return TimeoutAction::Drop,
                                                };

                                                let callback = &mut repeat_data.callback;
                                                let key = &mut repeat_data.current_repeat;
                                                if key.is_none() {
                                                    return TimeoutAction::Drop;
                                                }
                                                let key = key.as_mut().unwrap();
                                                key.key.time += if key.is_first {
                                                    key.is_first = false;
                                                    delay
                                                } else {
                                                    gap.as_millis() as u32
                                                };
                                                callback(loop_state, &kbd, key.key.clone());
                                                TimeoutAction::ToDuration(gap)
                                            },
                                        ) {
                                            repeat_data.repeat_token = Some(token);
                                        }
                                    }
                                }
                            }
                            _ => {
                                // Handle server-side key repeat
                                state
                                    .sctk_events
                                    .push(SctkEvent::InputMethodKeyboardEvent {
                                    variant:
                                        InputMethodKeyboardEventVariant::Repeat(
                                            event,
                                            udata.update_modifiers(),
                                            serial,
                                        ),
                                });
                            }
                        }
                    };
                }
                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: compositor sends invalid key state: {:x}", keyboard.id(), unknown);
                }
            },

            wl_keyboard::Event::Modifiers {
                serial: _,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
            } => {
                let mut guard = udata.xkb_state.lock().unwrap();

                let xkb_state = match guard.as_mut() {
                    Some(state) => state,
                    None => return,
                };

                let _ = xkb_state.update_mask(
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    0,
                    0,
                    group,
                );

                // Update the currently repeating key if any.
                if let Some(repeat_data) =
                    udata.repeat_data.lock().unwrap().as_mut()
                {
                    if let Some(mut event) = repeat_data.current_repeat.take() {
                        event.key.utf8 = {
                            let mut compose = udata.xkb_compose.lock().unwrap();
                            match compose.as_mut() {
                                Some(compose) => match compose
                                    .feed(event.key.keysym.into())
                                {
                                    xkb::FeedResult::Ignored => None,
                                    xkb::FeedResult::Accepted => match compose
                                        .status()
                                    {
                                        xkb::Status::Composed => compose.utf8(),
                                        xkb::Status::Nothing => {
                                            Some(xkb_state.key_get_utf8(
                                                (event.key.raw_code + 8).into(),
                                            ))
                                        }
                                        _ => None,
                                    },
                                },
                                None => Some(xkb_state.key_get_utf8(
                                    (event.key.raw_code + 8).into(),
                                )),
                            }
                        };
                        repeat_data.current_repeat = Some(event);
                    }
                }

                drop(guard);

                let modifiers = udata.update_modifiers();
                state.sctk_events.push(SctkEvent::InputMethodKeyboardEvent {
                    variant: InputMethodKeyboardEventVariant::Modifiers(
                        modifiers,
                    ),
                });
            }

            wl_keyboard::Event::RepeatInfo { rate, delay } => {
                let info = if rate != 0 {
                    RepeatInfo::Repeat {
                        rate: NonZeroU32::new(rate as u32).unwrap(),
                        delay: delay as u32,
                    }
                } else {
                    RepeatInfo::Disable
                };

                if let Some(repeat_data) =
                    udata.repeat_data.lock().unwrap().as_mut()
                {
                    repeat_data.repeat_info = info;
                }
            }

            wl_keyboard::Event::Enter { .. }
            | wl_keyboard::Event::Leave { .. } => {
                // Ignored for IM keyboard — interceptor handles enter/leave semantics
            }

            _ => {
                log::warn!(target: "sctk", "unhandled input method keyboard event");
            }
        }
    }
}

// Dispatch for XxKeyboardFilterManagerV1
impl Dispatch<XxKeyboardFilterManagerV1, cctk::sctk::globals::GlobalData>
    for SctkState
{
    fn event(
        _state: &mut SctkState,
        _: &XxKeyboardFilterManagerV1,
        _: <XxKeyboardFilterManagerV1 as Proxy>::Event,
        _: &cctk::sctk::globals::GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        // No events for the manager
    }
}

// Dispatch for XxKeyboardFilterV1
impl Dispatch<XxKeyboardFilterV1, ()> for SctkState {
    fn event(
        _state: &mut SctkState,
        _: &XxKeyboardFilterV1,
        _: <XxKeyboardFilterV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        // No events for keyboard_filter (it's all requests)
    }
}
