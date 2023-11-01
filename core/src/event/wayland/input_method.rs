use sctk::seat::keyboard::Keysym;

/// input method events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMethodEvent {
    /// A new text input is interacting with the application
    Activate,
    /// A text input is not interacting with the application anymore
    Deactivate,
}

/// Input method keyboard events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMethodKeyboardEvent {
    /// A key is pressed
    Press(KeyEvent),
    /// A key is released
    Release(KeyEvent),
    /// A key is repeated
    Repeat(KeyEvent),
    /// Modifiers are updated
    Modifiers(Modifiers),
}

/// Data associated with a key press or release event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    /// Time at which the keypress occurred.
    pub time: u32,

    /// The raw value of the key.
    pub raw_code: u32,

    /// The interpreted symbol of the key.
    ///
    /// This corresponds to one of the assoiated values on the [`Keysym`] type.
    pub keysym: Keysym,

    /// UTF-8 interpretation of the entered text.
    ///
    /// This will always be [`None`] on release events.
    pub utf8: Option<String>,
}

/// The state of keyboard modifiers
///
/// Each field of this indicates whether a specified modifier is active.
///
/// Depending on the modifier, the modifier key may currently be pressed or toggled.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    /// The "control" key
    pub ctrl: bool,

    /// The "alt" key
    pub alt: bool,

    /// The "shift" key
    pub shift: bool,

    /// The "Caps lock" key
    pub caps_lock: bool,

    /// The "logo" key
    ///
    /// Also known as the "windows" or "super" key on a keyboard.
    #[doc(alias = "windows")]
    #[doc(alias = "super")]
    pub logo: bool,

    /// The "Num lock" key
    pub num_lock: bool,
}

impl From<sctk::seat::keyboard::KeyEvent> for KeyEvent {
    fn from(value: sctk::seat::keyboard::KeyEvent) -> Self {
        KeyEvent {
            time: value.time,
            raw_code: value.raw_code,
            keysym: value.keysym,
            utf8: value.utf8,
        }
    }
}

impl From<sctk::seat::keyboard::Modifiers> for Modifiers {
    fn from(value: sctk::seat::keyboard::Modifiers) -> Self {
        Modifiers {
            ctrl: value.ctrl,
            alt: value.alt,
            shift: value.shift,
            caps_lock: value.caps_lock,
            logo: value.logo,
            num_lock: value.num_lock,
        }
    }
}
