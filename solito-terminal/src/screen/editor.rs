use super::buffer::{CellStyle, ScreenBuffer, ScreenCell, ScreenSnapshot};
use super::cursor::CursorPosition;
use super::sgr;
use crate::TerminalSize;
use decodesc::{CsiMessage, EraseMode, OscMessage};
use unicode_width::UnicodeWidthChar;

pub(crate) struct Screen {
    screen_buffer: ScreenBuffer,
}

impl Screen {
    pub(crate) fn new(size: TerminalSize) -> Self {
        let screen_buffer = ScreenBuffer::new(size);
        Self { screen_buffer }
    }

    pub(crate) fn resize(&mut self, size: TerminalSize) {
        self.screen_buffer.resize(size);
    }

    fn clear_screen(&mut self) {
        // Reset screen contents and move the cursor back to top-left.
        self.screen_buffer.lines.clear();
        self.screen_buffer.lines.push(Vec::new());
        self.screen_buffer.cursor.reset_row();
        self.screen_buffer.cursor.reset_col();
        self.screen_buffer.pending_wrap = false;
        self.screen_buffer.style = CellStyle::default();
    }

    fn erase_line(&mut self, mode: EraseMode) {
        self.screen_buffer.ensure_cursor_line();
        let row = self.screen_buffer.cursor.get_current_row();
        let col = self.screen_buffer.cursor.get_current_col();
        let line = &mut self.screen_buffer.lines[row];

        match mode {
            EraseMode::ToStart => {
                let end = col.saturating_add(1).min(line.len());
                for cell in line.iter_mut().take(end) {
                    *cell = ScreenCell::blank(CellStyle::default());
                }
            }
            EraseMode::All => line.clear(),
            EraseMode::ToEnd => {
                if col < line.len() {
                    line.truncate(col);
                }
            }
        }
    }

    pub(super) fn move_cursor_to(&mut self, position: CursorPosition) {
        self.screen_buffer.cursor.move_to(CursorPosition {
            row: position.row + self.screen_buffer.get_viewport_top(),
            col: position.col,
        });
        self.screen_buffer.pending_wrap = false;
    }

    pub(super) fn apply_csi(&mut self, message: CsiMessage) {
        match message {
            CsiMessage::CursorUp(amount) => self.move_cursor_up(usize::from(amount)),
            CsiMessage::CursorDown(amount) => self.move_cursor_down(usize::from(amount)),
            CsiMessage::CursorForward(amount) => self.move_cursor_forward(usize::from(amount)),
            CsiMessage::CursorBackward(amount) => self.move_cursor_backward(usize::from(amount)),
            CsiMessage::CursorHorizontalAbsolute(col) => {
                self.move_cursor_to_col(usize::from(col).saturating_sub(1))
            }
            CsiMessage::CursorPosition { row, col } => self.move_cursor_to(CursorPosition {
                row: usize::from(row).saturating_sub(1),
                col: usize::from(col).saturating_sub(1),
            }),
            CsiMessage::EraseDisplay(mode) => self.erase_display(mode),
            CsiMessage::EraseLine(mode) => self.erase_line(mode),
            CsiMessage::EraseCharacters(amount) => self.erase_characters(usize::from(amount)),
            CsiMessage::DeleteCharacters(amount) => self.delete_characters(usize::from(amount)),
            CsiMessage::SaveCursor => self.save_cursor_position(),
            CsiMessage::RestoreCursor => self.restore_cursor_position(),
            CsiMessage::SelectGraphicRendition(params) => {
                sgr::apply(&mut self.screen_buffer.style, &params)
            }
            CsiMessage::Unknown { .. }
            | CsiMessage::CursorNextLine(_)
            | CsiMessage::CursorPreviousLine(_)
            | CsiMessage::ScrollUp(_)
            | CsiMessage::ScrollDown(_)
            | CsiMessage::DeviceStatusReport(_)
            | CsiMessage::ShowCursor
            | CsiMessage::HideCursor => {}
        }
    }

    pub(super) fn apply_osc(&mut self, message: &OscMessage) {
        match message {
            OscMessage::SetCursorColor(color) => self.screen_buffer.cursor_color = Some(*color),
            OscMessage::ResetCursorColor => self.screen_buffer.cursor_color = None,
            OscMessage::SetIconName(_)
            | OscMessage::SetWindowTitle(_)
            | OscMessage::SetIconAndWindowTitle(_)
            | OscMessage::Unknown { .. } => {}
        }
    }

    pub(super) fn apply_execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.advance_to_next_line(),
            b'\r' => self.carriage_return(),
            0x08 | 0x7f => self.backspace(),
            b'\t' => self.tab(),
            _ => {}
        }
    }

    pub(super) fn put_char(&mut self, c: char) {
        if c == '\u{7f}' || c == '\u{8}' {
            return;
        }

        let char_width = UnicodeWidthChar::width(c).unwrap_or(1).clamp(1, 2);

        if self.screen_buffer.pending_wrap {
            // If the previous character reached the right edge,
            // finalize line wrap (auto-wrap) at the next print timing.
            self.advance_to_next_line();
        }

        if char_width == 2 {
            let current_col = self.screen_buffer.cursor.get_current_col();
            // If char width is over the buffer edge, start at next line.
            if current_col + 1 >= self.screen_buffer.cols() {
                self.advance_to_next_line();
            }
        }

        // Fill with spaces up to the cursor position to allow overwriting at arbitrary positions.
        self.screen_buffer.ensure_cursor_col();

        let row = self.screen_buffer.cursor.get_current_row();
        let col = self.screen_buffer.cursor.get_current_col();

        let line = &mut self.screen_buffer.lines[row];
        if col == line.len() {
            line.push(ScreenCell::new(c, self.screen_buffer.style));
        } else {
            line[col] = ScreenCell::new(c, self.screen_buffer.style);
        }

        if char_width == 2 {
            let continuation_col = col + 1;
            if continuation_col == line.len() {
                line.push(ScreenCell::wide_continuation(self.screen_buffer.style));
            } else {
                line[continuation_col] = ScreenCell::wide_continuation(self.screen_buffer.style);
            }
        }

        self.screen_buffer
            .cursor
            .move_to_col(col.saturating_add(char_width));

        if self.screen_buffer.cursor.get_current_col() >= self.screen_buffer.cols() {
            // Do not line-break immediately after crossing the edge;
            // wrap on the next print to match common terminal auto-wrap behavior.
            self.screen_buffer
                .cursor
                .move_to_col(self.screen_buffer.cols().saturating_sub(1));

            self.screen_buffer.pending_wrap = true;
        }
    }

    fn carriage_return(&mut self) {
        self.screen_buffer.cursor.reset_col();
        self.screen_buffer.pending_wrap = false;
    }

    fn backspace(&mut self) {
        self.screen_buffer.pending_wrap = false;
        if self.screen_buffer.cursor.get_current_col() == 0 {
            return;
        }

        self.screen_buffer.cursor.move_left();
        let col = self.screen_buffer.cursor.get_current_col();
        let row = self.screen_buffer.cursor.get_current_row();
        let line = &mut self.screen_buffer.lines[row];

        if col < line.len() {
            line.remove(col);
        }
    }

    fn tab(&mut self) {
        // Move to the next tab stop on an 8-column boundary.
        let next_tab = ((self.screen_buffer.cursor.get_current_col() / 8) + 1) * 8;
        self.move_cursor_to_col(next_tab.min(self.screen_buffer.cols().saturating_sub(1)));
    }

    pub(crate) fn snapshot(&self) -> ScreenSnapshot {
        self.screen_buffer.snapshot()
    }

    fn move_cursor_up(&mut self, amount: usize) {
        let next_row = self
            .screen_buffer
            .cursor
            .get_current_row()
            .saturating_sub(amount);

        self.screen_buffer.cursor.move_to_row(next_row);
        self.screen_buffer.pending_wrap = false;
    }

    fn move_cursor_down(&mut self, amount: usize) {
        let next_row = self.screen_buffer.cursor.get_current_row() + amount;
        self.screen_buffer.cursor.move_to_row(next_row);
        self.screen_buffer.pending_wrap = false;
        self.screen_buffer.ensure_cursor_line();
    }

    fn move_cursor_forward(&mut self, amount: usize) {
        let next_col = self.screen_buffer.cursor.get_current_col() + amount;
        self.move_cursor_to_col(next_col);
    }

    fn move_cursor_backward(&mut self, amount: usize) {
        let next_col = self
            .screen_buffer
            .cursor
            .get_current_col()
            .saturating_sub(amount);

        self.screen_buffer.cursor.move_to_col(next_col);
        self.screen_buffer.pending_wrap = false;
    }

    fn move_cursor_to_col(&mut self, col: usize) {
        self.screen_buffer.cursor.move_to_col(col);
        self.screen_buffer.pending_wrap = false;
    }

    fn erase_display(&mut self, mode: EraseMode) {
        match mode {
            EraseMode::ToStart => self.erase_display_before_cursor(),
            EraseMode::All => self.clear_screen(),
            EraseMode::ToEnd => self.erase_display_after_cursor(),
        }
    }

    fn erase_display_before_cursor(&mut self) {
        let cursor_row = self.screen_buffer.cursor.get_current_row();
        let cursor_col = self.screen_buffer.cursor.get_current_col();
        let last_row = cursor_row.min(self.screen_buffer.lines.len().saturating_sub(1));

        for row in 0..=last_row {
            // On the cursor row, erase up to the cursor position.
            // In the rows above, erase to the end of the line by filling spaces.
            let end = if row == cursor_row {
                cursor_col
                    .saturating_add(1)
                    .min(self.screen_buffer.lines[row].len())
            } else {
                self.screen_buffer.lines[row].len()
            };

            for ch in self.screen_buffer.lines[row].iter_mut().take(end) {
                *ch = ScreenCell::blank(CellStyle::default());
            }
        }
    }

    fn erase_display_after_cursor(&mut self) {
        self.screen_buffer.ensure_cursor_line();

        let cursor_row = self.screen_buffer.cursor.get_current_row();
        let cursor_col = self.screen_buffer.cursor.get_current_col();

        // Fully clear lines below the cursor.
        for row in cursor_row + 1..self.screen_buffer.lines.len() {
            self.screen_buffer.lines[row].clear();
        }

        // On the cursor row, delete content to the right of the cursor.
        let line = &mut self.screen_buffer.lines[cursor_row];
        if cursor_col < line.len() {
            line.truncate(cursor_col);
        }
    }

    fn delete_characters(&mut self, amount: usize) {
        self.screen_buffer.ensure_cursor_line();

        let row = self.screen_buffer.cursor.get_current_row();
        let col = self.screen_buffer.cursor.get_current_col();
        let line = &mut self.screen_buffer.lines[row];

        for _ in 0..amount {
            if col < line.len() {
                line.remove(col);
            }
        }
    }

    fn erase_characters(&mut self, amount: usize) {
        self.screen_buffer.ensure_cursor_line();

        let row = self.screen_buffer.cursor.get_current_row();
        let col = self.screen_buffer.cursor.get_current_col();
        let cols = self.screen_buffer.cols();
        let line = &mut self.screen_buffer.lines[row];

        if amount == 0 || col >= cols || col >= line.len() {
            return;
        }

        let mut start = col;
        let mut end = col.saturating_add(amount).min(cols).min(line.len());

        if line[start].is_wide_continuation {
            start = start.saturating_sub(1);
        }
        if line.get(end).is_some_and(|cell| cell.is_wide_continuation) {
            end += 1;
        }

        for cell in &mut line[start..end] {
            *cell = ScreenCell::blank(CellStyle::default());
        }
    }

    fn save_cursor_position(&mut self) {
        self.screen_buffer
            .cursor
            .save_cursor_position(CursorPosition {
                row: self.screen_buffer.cursor.get_current_row(),
                col: self.screen_buffer.cursor.get_current_col(),
            });
    }

    fn restore_cursor_position(&mut self) {
        if let Some(pos) = self.screen_buffer.cursor.get_saved_cursor_position() {
            self.move_cursor_to(pos);
        }
    }

    fn advance_to_next_line(&mut self) {
        // Newline moves to the next line head and expands lines if required.
        self.screen_buffer.cursor.move_down();
        self.screen_buffer.cursor.reset_col();
        self.screen_buffer.pending_wrap = false;
        self.screen_buffer.ensure_cursor_line();
    }
}
