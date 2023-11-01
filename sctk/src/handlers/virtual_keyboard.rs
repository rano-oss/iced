use std::{fmt::Debug, marker::PhantomData, sync::Mutex};

use iced_futures::core::event::wayland::KeyEvent;
use sctk::reexports::client::{
    delegate_dispatch,
    globals::{BindError, GlobalList},
    protocol::{wl_keyboard, wl_seat::WlSeat},
    Connection, Dispatch, Proxy, QueueHandle,
};

use sctk::seat::keyboard::Modifiers;
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

use sctk::globals::GlobalData;
use xkbcommon::xkb;

use crate::event_loop::state::SctkState;

#[derive(Debug)]
pub struct VirtualKeyboardManager<T> {
    manager: ZwpVirtualKeyboardManagerV1,
    _phantom: PhantomData<T>,
}

pub struct VirtualKeyboard {
    xkb_state: Mutex<Option<xkb::State>>,
}

// SAFETY: The state does not share state with any other rust types.
unsafe impl Send for VirtualKeyboard {}
// SAFETY: The state is guarded by a mutex since libxkbcommon has no internal synchronization.
unsafe impl Sync for VirtualKeyboard {}

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
        let data = VirtualKeyboard {
            xkb_state: Mutex::new(None),
        };
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

impl<T> SctkState<T>
where
    T: 'static + Debug,
{
    pub fn press_key(&mut self, key: KeyEvent) {
        let seat = self.seats.first().expect("seat not present"); //TODO: Handle this better
        if let Some(vk) = seat.virtual_keyboard.as_ref() {
            vk.key(
                key.time,
                key.raw_code,
                wl_keyboard::KeyState::Pressed.into(),
            );
        }
    }

    pub fn release_key(&mut self, key: KeyEvent) {
        let seat = self.seats.first().expect("seat not present"); //TODO: Handle this better
        if let Some(vk) = seat.virtual_keyboard.as_ref() {
            vk.key(
                key.time,
                key.raw_code,
                wl_keyboard::KeyState::Released.into(),
            );
        }
    }

    // pub fn update_modifiers(&mut self, modifiers: Modifiers){
    //     let seat = self.seats.first().expect("seat not present"); //TODO: Handle this better
    //     xkb::State::serialize_mods(, )
    //     if let Some(vk) = seat.virtual_keyboard.as_ref() {
    //         let xkb_state = vk.data()
    //         vk.modifiers(, , , );
    //     }
    // }
}
