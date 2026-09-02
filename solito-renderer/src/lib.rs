//! GPU rendering for immutable terminal snapshots.
//!
//! This crate does not parse PTY bytes or ANSI escape sequences.

mod state;
mod terminal_view;
mod util;

mod pass;
mod pipeline;

pub use solito_config::renderer::{RendererConfig, WindowBackdrop};
pub use solito_terminal::TerminalSize;
pub use state::renderer::Renderer;
pub use terminal_view::{
    CopyModePosition, CopyModeSelection, CopyModeSelectionKind, CopyModeSnapshot, TabBarSnapshot,
};

pub fn estimate_term_size(width: u32, height: u32, config: &RendererConfig) -> TerminalSize {
    terminal_view::TerminalView::estimate_terminal_size(width, height, config)
}
