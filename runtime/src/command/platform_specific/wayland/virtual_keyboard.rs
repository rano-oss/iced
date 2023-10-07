use std::{fmt, marker::PhantomData};

use iced_futures::MaybeSend;
use sctk::seat::keyboard::KeyEvent;

/// Virtual keyboard action
pub struct Action<T> {
    /// The inner action
    pub inner: ActionInner,
    /// The phantom data
    _phantom: PhantomData<T>,
}

impl<T> From<ActionInner> for Action<T> {
    fn from(inner: ActionInner) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<T> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// Virtual keyboard Actions
pub enum ActionInner {
    /// create a window and receive a message with its Id
    KeyPressed {
        /// popup
        key_event: KeyEvent,
    },
}

impl<T> Action<T> {
    /// Maps the output of a virtual keyboard [`Action`] using the provided closure.
    pub fn map<A>(
        self,
        _: impl Fn(T) -> A + 'static + MaybeSend + Sync,
    ) -> Action<A>
    where
        T: 'static,
    {
        Action::from(self.inner)
    }
}

impl fmt::Debug for ActionInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyPressed { key_event } => {
                f.debug_tuple("Key event").field(key_event).finish()
            }
        }
    }
}
