use std::marker::PhantomData;

use sctk::reexports::client::globals::{BindError, GlobalList};
use sctk::reexports::client::protocol::wl_seat::WlSeat;
use sctk::reexports::client::Dispatch;
use sctk::reexports::client::{
    delegate_dispatch, Connection, Proxy, QueueHandle,
};

use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

use sctk::globals::GlobalData;

use crate::event_loop::state::SctkState;

#[derive(Debug)]
pub struct VirtualKeyboardManager<T> {
    manager: ZwpVirtualKeyboardManagerV1,
    _phantom: PhantomData<T>,
}

pub struct VirtualKeyboard {}

impl<T: 'static> VirtualKeyboardManager<T> {
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

    pub fn virtual_keyboard(
        &self,
        seat: &WlSeat,
        queue_handle: &QueueHandle<SctkState<T>>,
    ) -> ZwpVirtualKeyboardV1 {
        let data = VirtualKeyboard {};
        self.manager
            .create_virtual_keyboard(seat, queue_handle, data)
    }
}

impl<T: 'static> Dispatch<ZwpVirtualKeyboardManagerV1, GlobalData, SctkState<T>>
    for VirtualKeyboardManager<T>
{
    fn event(
        _: &mut SctkState<T>,
        _: &ZwpVirtualKeyboardManagerV1,
        _: <ZwpVirtualKeyboardManagerV1 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        // Ignore zwp_virtual_keyboard_manager events
    }
}

impl<T: 'static> Dispatch<ZwpVirtualKeyboardV1, VirtualKeyboard, SctkState<T>>
    for VirtualKeyboardManager<T>
{
    fn event(
        _: &mut SctkState<T>,
        _: &ZwpVirtualKeyboardV1,
        _: <ZwpVirtualKeyboardV1 as Proxy>::Event,
        _: &VirtualKeyboard,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        // virtual keyboard has no events
    }
}

delegate_dispatch!(@<T: 'static> SctkState<T>: [ZwpVirtualKeyboardManagerV1: GlobalData] => VirtualKeyboardManager<T>);
delegate_dispatch!(@<T: 'static> SctkState<T>: [ZwpVirtualKeyboardV1: VirtualKeyboard] => VirtualKeyboardManager<T>);
