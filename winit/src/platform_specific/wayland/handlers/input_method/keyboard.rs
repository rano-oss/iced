use std::{env, fmt::Debug, sync::Mutex};

use cctk::sctk::{
    reexports::client::{
        Connection, Dispatch, Proxy, QueueHandle, WEnum,
        protocol::wl_keyboard::{self, WlKeyboard},
    },
    seat::keyboard::{KeyEvent, Modifiers},
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

pub struct InputMethodKeyboardData {
    xkb_context: Mutex<xkb::Context>,
    xkb_state: Mutex<Option<xkb::State>>,
    xkb_compose: Mutex<Option<xkb::compose::State>>,
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
                Some(xkb::compose::State::new(
                    &table,
                    xkb::compose::COMPILE_NO_FLAGS,
                ))
            } else {
                None
            }
        } else {
            None
        };

        Self {
            xkb_context: Mutex::new(xkb_context),
            xkb_state: Mutex::new(None),
            xkb_compose: Mutex::new(xkb_compose),
        }
    }

    pub fn update_modifiers(&self) -> Modifiers {
        let guard = self.xkb_state.lock().unwrap();
        guard
            .as_ref()
            .map(Self::modifiers_from_state)
            .unwrap_or_default()
    }

    fn modifiers_from_state(state: &xkb::State) -> Modifiers {
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

    fn utf8_for_keysym(
        &self,
        xkb_state: &xkb::State,
        keysym: xkb::Keysym,
        raw_code: u32,
        feed_compose: bool,
    ) -> Option<String> {
        if !feed_compose {
            return Some(xkb_state.key_get_utf8((raw_code + 8).into()));
        }
        let mut compose = self.xkb_compose.lock().unwrap();
        match compose.as_mut() {
            Some(compose) => match compose.feed(keysym) {
                xkb::FeedResult::Ignored => None,
                xkb::FeedResult::Accepted => match compose.status() {
                    xkb::Status::Composed => compose.utf8(),
                    xkb::Status::Nothing => {
                        Some(xkb_state.key_get_utf8((raw_code + 8).into()))
                    }
                    _ => None,
                },
            },
            None => Some(xkb_state.key_get_utf8((raw_code + 8).into())),
        }
    }

    fn push_key(
        &self,
        state: &mut SctkState,
        serial: u32,
        time: u32,
        key: u32,
        key_state: wl_keyboard::KeyState,
    ) {
        let state_guard = self.xkb_state.lock().unwrap();
        let Some(guard) = state_guard.as_ref() else {
            return;
        };
        let keysym = guard.key_get_one_sym((key + 8).into());
        let utf8 = match key_state {
            wl_keyboard::KeyState::Pressed => {
                self.utf8_for_keysym(guard, keysym, key, true)
            }
            wl_keyboard::KeyState::Repeated => {
                self.utf8_for_keysym(guard, keysym, key, false)
            }
            _ => None,
        };
        // Read mods while we already hold xkb_state — do not call
        // update_modifiers() here (that would re-lock and deadlock).
        let mods = Self::modifiers_from_state(guard);
        drop(state_guard);

        let event = KeyEvent {
            time,
            raw_code: key,
            keysym: keysym.into(),
            utf8,
        };
        let variant = match key_state {
            wl_keyboard::KeyState::Released => {
                InputMethodKeyboardEventVariant::Release(event, mods, serial)
            }
            wl_keyboard::KeyState::Pressed => {
                InputMethodKeyboardEventVariant::Press(event, mods, serial)
            }
            // Compositor-owned repeat (wl_keyboard v10+ / keyboard-filter).
            // Real serial required so the IME can filter after preedit ends.
            _ => InputMethodKeyboardEventVariant::Repeat(event, mods, serial),
        };
        state
            .sctk_events
            .push(SctkEvent::InputMethodKeyboardEvent { variant });
    }
}

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
                                *udata.xkb_state.lock().unwrap() =
                                    Some(xkb::State::new(&keymap));
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
                    udata.push_key(state, serial, time, key, key_state);
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
                let Some(xkb_state) = guard.as_mut() else {
                    return;
                };
                let _ = xkb_state.update_mask(
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    0,
                    0,
                    group,
                );
                drop(guard);

                state.sctk_events.push(SctkEvent::InputMethodKeyboardEvent {
                    variant: InputMethodKeyboardEventVariant::Modifiers(
                        udata.update_modifiers(),
                    ),
                });
            }

            // Repeat rate is compositor-owned when keyboard-filter is bound.
            wl_keyboard::Event::RepeatInfo { .. }
            | wl_keyboard::Event::Enter { .. }
            | wl_keyboard::Event::Leave { .. } => {}

            _ => {
                log::warn!(target: "sctk", "unhandled input method keyboard event");
            }
        }
    }
}

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
    }
}

impl Dispatch<XxKeyboardFilterV1, ()> for SctkState {
    fn event(
        _state: &mut SctkState,
        _: &XxKeyboardFilterV1,
        _: <XxKeyboardFilterV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
    }
}
