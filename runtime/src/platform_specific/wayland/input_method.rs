//! Input method actions

use std::fmt;

/// Input method action
#[derive(Clone)]
pub enum Action {
    /// Set the preedit string
    SetPreeditString {
        /// The preedit string
        string: String,
        /// Cursor begin position (bytes)
        cursor_begin: i32,
        /// Cursor end position (bytes)
        cursor_end: i32,
    },
    /// Commit a string to the text input
    CommitString(String),
    /// Commit pending state
    Commit,
    /// Filter a key event (true = consumed, false = forwarded)
    FilterKey(u32, bool),
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetPreeditString {
                string,
                cursor_begin,
                cursor_end,
            } => f
                .debug_struct("SetPreeditString")
                .field("string", string)
                .field("cursor_begin", cursor_begin)
                .field("cursor_end", cursor_end)
                .finish(),
            Self::CommitString(s) => {
                f.debug_tuple("CommitString").field(s).finish()
            }
            Self::Commit => write!(f, "Commit"),
            Self::FilterKey(serial, consumed) => f
                .debug_struct("FilterKey")
                .field("serial", serial)
                .field("consumed", consumed)
                .finish(),
        }
    }
}
