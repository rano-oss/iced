use crate::keyboard::Key;

/// input method events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMethodEvent {
    /// A new text input is interacting with the application
    Activate,
    /// A text input is not interacting with the application anymore
    Deactivate,
    /// The surrounding plain text around the cursor, excluding the preedit text
    SurroundingText {
        /// plain text
        text: String,
        /// Cursor position
        cursor: u32,
        /// Anchor position
        anchor: u32,
    },
    /// indicates the cause of surrounding text change
    TextChangeCause(u32),
    /// content purpose and hint
    ContentType {
        /// Content hint flags
        hint: u32,
        /// Content purpose
        purpose: u32,
    },
    /// apply state
    Done,
}

/// Input method keyboard events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMethodKeyboardEvent {
    /// A key is pressed
    Press(KeyEvent, Key, Modifiers, u32),
    /// A key is released
    Release(KeyEvent, Key, Modifiers, u32),
    /// A key is repeated
    Repeat(KeyEvent, Key, Modifiers),
    /// Modifiers are updated
    Modifiers(Modifiers, RawModifiers),
}

/// Data associated with a key press or release event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    /// Time at which the keypress occurred.
    pub time: u32,

    /// The raw value of the key.
    pub raw_code: u32,

    /// The interpreted symbol of the key (keysym value).
    pub keysym: u32,

    /// UTF-8 interpretation of the entered text.
    ///
    /// This will always be [`None`] on release events.
    pub utf8: Option<String>,
}

/// The state of keyboard modifiers
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
    pub logo: bool,
    /// The "Num lock" key
    pub num_lock: bool,
}

/// Raw modifiers
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RawModifiers {
    /// Modifiers depressed
    pub mods_depressed: u32,
    /// Modifiers latched
    pub mods_latched: u32,
    /// Modifiers locked
    pub mods_locked: u32,
    /// Modifiers group
    pub group: u32,
}
