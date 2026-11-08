//! Small xkb helpers shared by the IM keyboard path.
//! Mirrors the private helpers inside smithay-client-toolkit's seat keyboard.

use std::{env, os::unix::io::OwnedFd};

use cctk::sctk::seat::keyboard::Modifiers;
use xkbcommon::xkb;

/// Compose table from `LC_ALL` / `LC_CTYPE` / `LANG` (same order as SCTK).
pub fn compose_state_from_env(
    context: &xkb::Context,
) -> Option<xkb::compose::State> {
    let locale = env::var_os("LC_ALL")
        .and_then(|v| if v.is_empty() { None } else { Some(v) })
        .or_else(|| env::var_os("LC_CTYPE"))
        .and_then(|v| if v.is_empty() { None } else { Some(v) })
        .or_else(|| env::var_os("LANG"))
        .and_then(|v| if v.is_empty() { None } else { Some(v) })
        .unwrap_or_else(|| "C".into());
    let locale = locale.to_str()?;
    let table = xkb::compose::Table::new_from_locale(
        context,
        locale.as_ref(),
        xkb::compose::COMPILE_NO_FLAGS,
    )
    .ok()?;
    Some(xkb::compose::State::new(
        &table,
        xkb::compose::COMPILE_NO_FLAGS,
    ))
}

pub fn modifiers_from_state(state: &xkb::State) -> Modifiers {
    Modifiers {
        ctrl: state
            .mod_name_is_active(xkb::MOD_NAME_CTRL, xkb::STATE_MODS_EFFECTIVE),
        alt: state
            .mod_name_is_active(xkb::MOD_NAME_ALT, xkb::STATE_MODS_EFFECTIVE),
        shift: state.mod_name_is_active(
            xkb::MOD_NAME_SHIFT,
            xkb::STATE_MODS_EFFECTIVE,
        ),
        caps_lock: state.mod_name_is_active(
            xkb::MOD_NAME_CAPS,
            xkb::STATE_MODS_EFFECTIVE,
        ),
        logo: state
            .mod_name_is_active(xkb::MOD_NAME_LOGO, xkb::STATE_MODS_EFFECTIVE),
        num_lock: state
            .mod_name_is_active(xkb::MOD_NAME_NUM, xkb::STATE_MODS_EFFECTIVE),
    }
}

/// UTF-8 for a key, optionally feeding the compose state (press vs repeat).
pub fn utf8_for_keysym(
    compose: &mut Option<xkb::compose::State>,
    xkb_state: &xkb::State,
    keysym: xkb::Keysym,
    raw_code: u32,
    feed_compose: bool,
) -> Option<String> {
    let keycode = (raw_code + 8).into();
    if !feed_compose {
        return Some(xkb_state.key_get_utf8(keycode));
    }
    match compose.as_mut() {
        Some(compose) => match compose.feed(keysym) {
            xkb::FeedResult::Ignored => None,
            xkb::FeedResult::Accepted => match compose.status() {
                xkb::Status::Composed => compose.utf8(),
                xkb::Status::Nothing => Some(xkb_state.key_get_utf8(keycode)),
                _ => None,
            },
        },
        None => Some(xkb_state.key_get_utf8(keycode)),
    }
}

/// Load an XKB v1 keymap from a Wayland keymap fd into a fresh [`xkb::State`].
pub fn state_from_keymap_fd(
    context: &xkb::Context,
    fd: OwnedFd,
    size: u32,
) -> Option<xkb::State> {
    #[allow(unused_unsafe)]
    let keymap = unsafe {
        xkb::Keymap::new_from_fd(
            context,
            fd,
            size as usize,
            xkb::KEYMAP_FORMAT_TEXT_V1,
            xkb::COMPILE_NO_FLAGS,
        )
    };
    match keymap {
        Ok(Some(keymap)) => Some(xkb::State::new(&keymap)),
        Ok(None) => {
            log::error!(target: "sctk", "invalid keymap");
            None
        }
        Err(err) => {
            log::error!(target: "sctk", "{err}");
            None
        }
    }
}
