use super::context::State;
use crate::renderer::screen::input_buffer::TerminalOutputHandler;

pub(crate) trait TerminalOutputSink {
    fn apply_terminal_output(&mut self, bytes: &[u8]);
}

impl TerminalOutputSink for State {
    fn apply_terminal_output(&mut self, bytes: &[u8]) {
        self.buffer.apply_terminal_output(bytes);
    }
}
