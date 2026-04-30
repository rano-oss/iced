use std::{
    env,
    fmt::Debug,
    num::NonZeroU32,
    sync::{Arc, Mutex},
    time::Duration,
};
pub use xkeysym::Keysym;

use cctk::sctk::reexports::calloop::{
    LoopHandle, RegistrationToken,
    timer::{TimeoutAction, Timer},
};
use cctk::sctk::{
    reexports::client::{
        Connection, Dispatch, Proxy, QueueHandle, WEnum, protocol::wl_keyboard,
    },
    seat::keyboard::{KeyEvent, KeyboardError, Modifiers, RMLVO, RepeatInfo},
};

use xkbcommon::xkb;

use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_keyboard_grab_v2::{self, ZwpInputMethodKeyboardGrabV2},
    zwp_input_method_v2::ZwpInputMethodV2,
};

use super::InputMethod;
use crate::platform_specific::wayland::event_loop::state::SctkState;
use crate::platform_specific::wayland::sctk_event::{
    InputMethodKeyboardEventVariant, SctkEvent,
};

pub(crate) struct RepeatedKey {
    pub(crate) key: KeyEvent,
    pub(crate) is_first: bool,
}

pub type RepeatCallback = Box<
    dyn FnMut(&mut SctkState, &ZwpInputMethodKeyboardGrabV2, KeyEvent)
        + 'static,
>;

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

impl InputMethod {
    pub fn grab_keyboard_with_repeat(
        &mut self,
        qh: &QueueHandle<SctkState>,
        input_method: &ZwpInputMethodV2,
        rmlvo: Option<RMLVO>,
        loop_handle: LoopHandle<'static, SctkState>,
        callback: RepeatCallback,
    ) -> Result<ZwpInputMethodKeyboardGrabV2, KeyboardError> {
        let udata = match rmlvo {
            Some(rmlvo) => InputMethodKeyboardData::from_rmlvo(rmlvo)?,
            None => InputMethodKeyboardData::new(),
        };

        let kbd_data = &udata;
        let _ = kbd_data.repeat_data.lock().unwrap().replace(RepeatData {
            current_repeat: None,
            repeat_info: RepeatInfo::Disable,
            loop_handle: loop_handle.clone(),
            callback,
            repeat_token: None,
        });
        kbd_data.init_compose();

        Ok(input_method.grab_keyboard(qh, udata))
    }
}

/// Wrapper around a libxkbcommon keymap
#[allow(missing_debug_implementations)]
pub struct Keymap<'a>(pub(crate) &'a xkb::Keymap);

impl<'a> Keymap<'a> {
    pub fn as_string(&self) -> String {
        self.0.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1)
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
    pub fn new() -> Self {
        let xkb_context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let udata = InputMethodKeyboardData {
            xkb_context: Mutex::new(xkb_context),
            xkb_state: Mutex::new(None),
            user_specified_rmlvo: false,
            xkb_compose: Mutex::new(None),
            repeat_data: Arc::new(Mutex::new(None)),
        };
        udata.init_compose();
        udata
    }

    pub fn from_rmlvo(rmlvo: RMLVO) -> Result<Self, KeyboardError> {
        let xkb_context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap = xkb::Keymap::new_from_names(
            &xkb_context,
            &rmlvo.rules.unwrap_or_default(),
            &rmlvo.model.unwrap_or_default(),
            &rmlvo.layout.unwrap_or_default(),
            &rmlvo.variant.unwrap_or_default(),
            rmlvo.options,
            xkb::COMPILE_NO_FLAGS,
        );

        if keymap.is_none() {
            return Err(KeyboardError::InvalidKeymap);
        }

        let xkb_state = Some(xkb::State::new(&keymap.unwrap()));

        let udata = InputMethodKeyboardData {
            xkb_context: Mutex::new(xkb_context),
            xkb_state: Mutex::new(xkb_state),
            user_specified_rmlvo: true,
            xkb_compose: Mutex::new(None),
            repeat_data: Arc::new(Mutex::new(None)),
        };
        udata.init_compose();
        Ok(udata)
    }

    fn init_compose(&self) {
        let xkb_context = self.xkb_context.lock().unwrap();

        if let Some(locale) = env::var_os("LC_ALL")
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
                *self.xkb_compose.lock().unwrap() = Some(compose_state);
            }
        }
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

/// Raw modifiers from the compositor
#[derive(Debug, Clone, Copy, Default)]
pub struct RawModifiers {
    pub mods_depressed: u32,
    pub mods_latched: u32,
    pub mods_locked: u32,
    pub group: u32,
}

fn send_event(state: &SctkState, event: SctkEvent) {
    use crate::platform_specific::wayland::event_loop::state::send_event as do_send;
    do_send(&state.events_sender, &state.proxy, event);
}

impl Dispatch<ZwpInputMethodKeyboardGrabV2, InputMethodKeyboardData>
    for SctkState
{
    fn event(
        state: &mut SctkState,
        keyboard: &ZwpInputMethodKeyboardGrabV2,
        event: <ZwpInputMethodKeyboardGrabV2 as Proxy>::Event,
        udata: &InputMethodKeyboardData,
        conn: &Connection,
        qh: &QueueHandle<SctkState>,
    ) {
        match event {
            zwp_input_method_keyboard_grab_v2::Event::Keymap {
                format,
                fd,
                size,
            } => match format {
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
                    _ => unreachable!(),
                },
                WEnum::Unknown(value) => {
                    log::warn!(target: "sctk", "unknown keymap format 0x{:x}", value)
                }
            },

            zwp_input_method_keyboard_grab_v2::Event::Key {
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
                                    }
                                }
                                state.sctk_events.push(SctkEvent::InputMethodKeyboardEvent {
                                    variant: InputMethodKeyboardEventVariant::Release(event, serial),
                                });
                            }
                            wl_keyboard::KeyState::Pressed => {
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

                                        let _ = repeat_data.current_repeat.replace(
                                            RepeatedKey {
                                                key: event.clone(),
                                                is_first: true,
                                            },
                                        );

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
                                state
                                    .sctk_events
                                    .push(SctkEvent::InputMethodKeyboardEvent {
                                    variant:
                                        InputMethodKeyboardEventVariant::Press(
                                            event, serial,
                                        ),
                                });
                            }
                            _ => unreachable!(),
                        }
                    };
                }
                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: compositor sends invalid key state: {:x}", keyboard.id(), unknown);
                }
            },

            zwp_input_method_keyboard_grab_v2::Event::Modifiers {
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

            zwp_input_method_keyboard_grab_v2::Event::RepeatInfo {
                rate,
                delay,
            } => {
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

            _ => unreachable!(),
        }
    }
}
