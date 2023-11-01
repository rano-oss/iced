use sctk::seat::keyboard::{KeyEvent, Modifiers};

#[derive(Debug, Clone)]
pub enum KeyboardEvent {
    Press(KeyEvent),
    Repeat(KeyEvent),
    Release(KeyEvent),
    Modifiers(Modifiers),
}

#[derive(Debug, Clone)]
pub enum InputMethodEventVariant {
    Activate,
    Deactivate,
}
