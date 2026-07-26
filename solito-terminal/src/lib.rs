//! Terminal emulation without windowing or GPU concerns.
//!
//! PTY bytes enter through [`TerminalState`] and leave as a [`ScreenSnapshot`].

mod screen;
mod size;
mod state;

pub use screen::buffer::{ScreenCell, ScreenSnapshot};
pub use size::TerminalSize;
pub use state::TerminalState;
