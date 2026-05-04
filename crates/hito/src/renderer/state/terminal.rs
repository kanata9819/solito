use super::context::State;
use crate::renderer::screen::buffer::ScreenBufferEditor;

pub trait TerminalOutputSink {
    fn print_char(&mut self, char: char);
    fn carriage_return(&mut self);
    fn line_feed(&mut self);
    fn clear_line(&mut self);
    fn move_cursor_to(&mut self, row: u16, col: u16);
}

impl TerminalOutputSink for State {
    fn print_char(&mut self, char: char) {
        self.buffer.push_char(char);
    }

    fn carriage_return(&mut self) {
        self.buffer.reset_col();
    }

    fn line_feed(&mut self) {
        self.buffer.line_feed();
    }

    fn clear_line(&mut self) {
        self.buffer.clear_line();
    }

    fn move_cursor_to(&mut self, row: u16, col: u16) {
        self.buffer.move_cursor_to(row, col);
    }
}
