//! Keyboard-filter protocol Dispatch stubs.
//! Key handling uses the SCTK seat keyboard bound via `bind_to_input_method`.

use cctk::sctk::reexports::client::{
    Connection, Dispatch, Proxy, QueueHandle,
};

use wayland_protocols_experimental::keyboard_filter::v3::client::{
    xx_keyboard_filter_manager_v1::XxKeyboardFilterManagerV1,
    xx_keyboard_filter_v1::XxKeyboardFilterV1,
};

use crate::platform_specific::wayland::event_loop::state::SctkState;

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
