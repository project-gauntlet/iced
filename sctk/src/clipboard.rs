//! Access the clipboard.
pub use iced_runtime::clipboard::Action;

use iced_runtime::command::{self, Command};
use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use crate::core::clipboard::Kind;

/// A buffer for short-term storage and transfer within and between
/// applications.
#[allow(missing_debug_implementations)]
pub struct Clipboard {
    pub(crate) state: State,
}

pub(crate) enum State {
    Connected(Arc<Mutex<smithay_clipboard::Clipboard>>),
    Unavailable,
}

impl Clipboard {
    pub unsafe fn connect(display: *mut c_void) -> Clipboard {
        let context = Arc::new(Mutex::new(smithay_clipboard::Clipboard::new(
            display as *mut _,
        )));

        Clipboard {
            state: State::Connected(context),
        }
    }

    /// Creates a new [`Clipboard`] that isn't associated with a window.
    /// This clipboard will never contain a copied value.
    pub fn unconnected() -> Clipboard {
        Clipboard {
            state: State::Unavailable,
        }
    }

    /// Reads the current content of the [`Clipboard`] as text.
    pub fn read(&self, kind: Kind) -> Option<String> {
        match &self.state {
            State::Connected(clipboard) => {
                let clipboard = clipboard.lock().unwrap();

                match kind {
                    Kind::Standard => clipboard.load(),
                    Kind::Primary => clipboard.load_primary(),
                }.ok()
            }
            State::Unavailable => None,
        }
    }

    /// Writes the given text contents to the [`Clipboard`].
    pub fn write(&mut self, kind: Kind, contents: String) {
        match &mut self.state {
            State::Connected(clipboard) => {
                let clipboard = clipboard.lock().unwrap();
                match kind {
                    Kind::Standard => clipboard.store(contents),
                    Kind::Primary => clipboard.store_primary(contents),
                }
            }
            State::Unavailable => {}
        }
    }
}

impl iced_runtime::core::clipboard::Clipboard for Clipboard {
    fn read(&self, kind: Kind) -> Option<String> {
        self.read(kind)
    }

    fn write(&mut self, kind: Kind, contents: String) {
        self.write(kind, contents)
    }
}

/// Read the current contents of the clipboard.
pub fn read<Message>(
    f: impl Fn(Option<String>) -> Message + 'static,
    kind: Kind
) -> Command<Message> {
    Command::single(command::Action::Clipboard(Action::Read(Box::new(f), kind)))
}

/// Write the given contents to the clipboard.
pub fn write<Message>(contents: String, kind: Kind) -> Command<Message> {
    Command::single(command::Action::Clipboard(Action::Write(contents, kind)))
}
