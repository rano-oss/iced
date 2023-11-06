mod data_device;
mod input_method;
mod layer;
mod output;
mod popup;
mod seat;
mod session_lock;
mod window;

use crate::{time::Instant, window::Id};
use sctk::reexports::client::protocol::{
    wl_output::WlOutput, wl_seat::WlSeat, wl_surface::WlSurface,
};

pub use data_device::*;
pub use input_method::*;
pub use layer::*;
pub use output::*;
pub use popup::*;
pub use seat::*;
pub use session_lock::*;
pub use window::*;

/// wayland events
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// layer surface event
    Layer(LayerEvent, WlSurface, Id),
    /// popup event
    Popup(PopupEvent, WlSurface, Id),
    /// output event
    Output(OutputEvent, WlOutput),
    /// window event
    Window(WindowEvent, WlSurface, Id),
    /// Seat Event
    Seat(SeatEvent, WlSeat),
    /// Data Device event
    DataSource(DataSourceEvent),
    /// Dnd Offer events
    DndOffer(DndOfferEvent),
    /// Selection Offer events
    SelectionOffer(SelectionOfferEvent),
    /// Session lock events
    SessionLock(SessionLockEvent),
    /// Frame events
    Frame(Instant, WlSurface, Id),
    /// Input Method
    InputMethod(InputMethodEvent),
    /// Input Method Keyboard Event
    InputMethodKeyboard(InputMethodKeyboardEvent),
    // /// Input Method Popup Event
    // InputMethodPopup(InputMethodPopupEvent)
}
