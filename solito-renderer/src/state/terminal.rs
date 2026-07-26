use super::context::State;
use crate::terminal_view::{CopyModeSnapshot, TabBarSnapshot};
use solito_terminal::ScreenSnapshot;
use winit::dpi::PhysicalSize;

impl State {
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

    pub fn terminal_size(&self) -> (usize, usize) {
        let width: u32 = self.window_surface.config.width;
        let height: u32 = self.window_surface.config.height;

        self.terminal_size_for(PhysicalSize::new(width, height))
    }

    pub fn terminal_size_for(&self, size: PhysicalSize<u32>) -> (usize, usize) {
        (
            self.terminal_view.visible_cols(size.width),
            self.terminal_view.visible_rows(size.height),
        )
    }
}
