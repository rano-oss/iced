use std::{fmt::Debug, sync::Mutex};

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
use crate::platform_specific::wayland::xkb_utils::{
    compose_state_from_env, modifiers_from_state, state_from_keymap_fd,
    utf8_for_keysym,
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
        let xkb_compose = compose_state_from_env(&xkb_context);
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
            .map(modifiers_from_state)
            .unwrap_or_default()
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
        let mut compose = self.xkb_compose.lock().unwrap();
        let utf8 = match key_state {
            wl_keyboard::KeyState::Pressed => {
                utf8_for_keysym(&mut compose, guard, keysym, key, true)
            }
            wl_keyboard::KeyState::Repeated => {
                utf8_for_keysym(&mut compose, guard, keysym, key, false)
            }
            _ => None,
        };
        drop(compose);
        // Mods while we already hold xkb_state — do not call update_modifiers()
        // (that would re-lock and deadlock).
        let mods = modifiers_from_state(guard);
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
            _ => InputMethodKeyboardEventVariant::Repeat(event, mods, serial),
        };
        state
            .sctk_events
            .push(SctkEvent::InputMethodKeyboardEvent { variant });
        if let Some(popup) = state.input_method_popup.as_ref() {
            let surface = popup.wl_surface.clone();
            state.request_redraw(&surface);
        }
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
        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => match format {
                WEnum::Value(format) => match format {
                    wl_keyboard::KeymapFormat::NoKeymap => {
                        log::warn!(target: "sctk", "non-xkb compatible keymap");
                    }
                    wl_keyboard::KeymapFormat::XkbV1 => {
                        let context = udata.xkb_context.lock().unwrap();
                        if let Some(new_state) =
                            state_from_keymap_fd(&context, fd, size)
                        {
                            *udata.xkb_state.lock().unwrap() = Some(new_state);
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
