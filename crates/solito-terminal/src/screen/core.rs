use crate::screen::buffer::ScreenSnapshot;
use crate::screen::editor::ScreenEditor;

pub struct Screen {
    pub(super) buffer_editor: ScreenEditor,
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

    pub fn snapshot(&self) -> ScreenSnapshot {
        self.buffer_editor.snapshot()
    }
}
