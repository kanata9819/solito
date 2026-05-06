use decodesc::{DecodedEvent, VteEvent, decode};
use vte::{Params, Perform};

use crate::terminal::screen::buffer::ScreenSnapshot;
use crate::terminal::screen::editor::ScreenEditor;

pub struct Screen {
    buffer_editor: ScreenEditor,
}

impl Screen {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            buffer_editor: ScreenEditor::new(cols, rows),
        }
    }

    pub fn set_cols(&mut self, cols: usize) {
        self.buffer_editor.set_cols(cols);
    }

    pub fn set_rows(&mut self, rows: usize) {
        self.buffer_editor.set_rows(rows);
    }

    pub fn cursor_position_1_based(&self) -> (usize, usize) {
        self.buffer_editor.buffer().cursor_position_1_based()
    }

    pub fn snapshot(&self) -> ScreenSnapshot {
        self.buffer_editor.snapshot()
    }
}

impl Perform for Screen {
    fn print(&mut self, c: char) {
        self.buffer_editor.apply_print(c);
    }

    fn execute(&mut self, byte: u8) {
        self.buffer_editor.apply_execute(byte);
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        let _ = decode(VteEvent::Osc {
            params,
            bell_terminated,
        });
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        if let Some(DecodedEvent { csi: Some(csi), .. }) = decode(VteEvent::Csi {
            params,
            intermediates,
            ignore,
            action,
        }) {
            self.buffer_editor.apply_csi(csi);
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        let _ = decode(VteEvent::Esc {
            intermediates,
            ignore,
            byte,
        });
    }
}
