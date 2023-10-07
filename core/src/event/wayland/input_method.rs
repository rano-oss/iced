/// input method events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMethodEvent {
    /// A new text input is interacting with the application
    Activate,
    /// A text input is not interacting with the application anymore
    Deactivate,
}
