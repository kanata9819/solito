use super::context::State;
use crate::{renderer::terminal_view::TerminalView, terminal::ScreenSnapshot};
use winit::dpi::PhysicalSize;

pub(crate) trait TerminalViewRenderer {
    fn set_terminal_snapshot(&mut self, snapshot: ScreenSnapshot);
    fn terminal_size(&self) -> (usize, usize);
    fn terminal_size_for(&self, size: PhysicalSize<u32>) -> (usize, usize);
}

impl TerminalViewRenderer for State {
    fn set_terminal_snapshot(&mut self, snapshot: ScreenSnapshot) {
        self.terminal_view.set_snapshot(snapshot);
    }

    fn terminal_size(&self) -> (usize, usize) {
        let width = self.window_surface.config.width;
        let height = self.window_surface.config.height;

        self.terminal_size_for(PhysicalSize::new(width, height))
    }

    fn terminal_size_for(&self, size: PhysicalSize<u32>) -> (usize, usize) {
        (
            TerminalView::visible_cols(size.width),
            TerminalView::visible_rows(size.height),
        )
    }
}
