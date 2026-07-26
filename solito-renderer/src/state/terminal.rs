use super::renderer::Renderer;
use crate::terminal_view::{CopyModeSnapshot, TabBarSnapshot};
use solito_terminal::{ScreenSnapshot, TerminalSize};
use winit::dpi::PhysicalSize;

impl Renderer {
    pub fn set_copy_mode(&mut self, snapshot: CopyModeSnapshot) {
        self.terminal_view.set_copy_mode(snapshot);
    }

    pub fn set_tab_bar(&mut self, snapshot: TabBarSnapshot) {
        self.terminal_view.set_tab_bar(snapshot);
    }

    pub fn set_terminal_snapshot(&mut self, snapshot: ScreenSnapshot) {
        self.terminal_view.set_snapshot(snapshot);
    }

    pub fn set_terminal_snapshot_at_bottom(&mut self, snapshot: ScreenSnapshot) {
        self.terminal_view.set_snapshot_at_bottom(snapshot);
    }

    pub fn terminal_size(&self) -> TerminalSize {
        let width: u32 = self.window_surface.config.width;
        let height: u32 = self.window_surface.config.height;

        self.terminal_size_for(PhysicalSize::new(width, height))
    }

    pub fn terminal_size_for(&self, window_size: PhysicalSize<u32>) -> TerminalSize {
        TerminalSize::new(
            self.terminal_view.visible_cols(window_size.width),
            self.terminal_view.visible_rows(window_size.height),
        )
    }
}
