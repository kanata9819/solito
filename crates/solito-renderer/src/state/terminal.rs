use super::context::State;
use solito_terminal::ScreenSnapshot;
use winit::dpi::PhysicalSize;

pub trait TerminalViewRenderer {
    fn set_terminal_snapshot(&mut self, snapshot: ScreenSnapshot);
    fn terminal_size(&mut self) -> (usize, usize);
    fn terminal_size_for(&mut self, size: PhysicalSize<u32>) -> (usize, usize);
}

impl TerminalViewRenderer for State {
    fn set_terminal_snapshot(&mut self, snapshot: ScreenSnapshot) {
        self.terminal_view.set_snapshot(snapshot);
    }

    fn terminal_size(&mut self) -> (usize, usize) {
        let width: u32 = self.window_surface.config.width;
        let height: u32 = self.window_surface.config.height;

        self.terminal_size_for(PhysicalSize::new(width, height))
    }

    fn terminal_size_for(&mut self, size: PhysicalSize<u32>) -> (usize, usize) {
        (
            self.terminal_view.visible_cols(size.width),
            self.terminal_view.visible_rows(size.height),
        )
    }
}
