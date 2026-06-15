mod state;
mod terminal_view;
mod util;

mod pass;
mod pipeline;

pub use solito_config::renderer::{RendererConfig, WindowBackdrop};
pub use state::context::{State, TerminalViewRenderer, WindowRenderer};
pub use terminal_view::{
    CopyModePosition, CopyModeSelection, CopyModeSelectionKind, CopyModeSnapshot, TabBarSnapshot,
};

pub fn estimate_terminal_size(width: u32, height: u32, config: &RendererConfig) -> (usize, usize) {
    terminal_view::TerminalView::estimate_terminal_size(width, height, config)
}
