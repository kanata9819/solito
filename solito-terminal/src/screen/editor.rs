use super::buffer::{CellStyle, ScreenBuffer, ScreenCell, ScreenSnapshot};
use super::cursor::CursorPosition;
use decodesc::{CsiMessage, EraseMode, OscMessage};
use unicode_width::UnicodeWidthChar;

pub(super) struct ScreenEditor {
    screen_buffer: ScreenBuffer,
}

impl ScreenEditor {
    pub(super) fn new(cols: usize, rows: usize) -> Self {
        let screen_buffer: ScreenBuffer = ScreenBuffer::new(cols, rows);
        Self { screen_buffer }
    }

    pub(super) fn set_cols(&mut self, cols: usize) {
        self.screen_buffer.set_cols(cols);
    }

    pub(super) fn set_rows(&mut self, rows: usize) {
        self.screen_buffer.set_rows(rows);
    }

    pub(super) fn clear_screen(&mut self) {
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
        let row: usize = self.screen_buffer.cursor.get_current_row();
        let col: usize = self.screen_buffer.cursor.get_current_col();
        let line: &mut Vec<ScreenCell> = &mut self.screen_buffer.lines[row];

        match mode {
            EraseMode::ToStart => {
                let end: usize = col.saturating_add(1).min(line.len());
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
            CsiMessage::DeleteCharacters(amount) => self.delete_characters(usize::from(amount)),
            CsiMessage::SaveCursor => self.save_cursor_position(),
            CsiMessage::RestoreCursor => self.restore_cursor_position(),
            CsiMessage::SelectGraphicRendition(params) => self.apply_graphics_rendition(&params),
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

    pub(super) fn apply_osc(&mut self, message: OscMessage) {
        match message {
            OscMessage::SetCursorColor(color) => self.screen_buffer.cursor_color = Some(color),
            OscMessage::ResetCursorColor => self.screen_buffer.cursor_color = None,
            OscMessage::SetIconName(_)
            | OscMessage::SetWindowTitle(_)
            | OscMessage::SetIconAndWindowTitle(_)
            | OscMessage::Unknown { .. } => {}
        }
    }

    pub(super) fn apply_print(&mut self, c: char) {
        self.put_char(c);
    }

    pub(super) fn apply_execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\r' => self.carriage_return(),
            0x08 | 0x7f => self.backspace(),
            b'\t' => self.tab(),
            _ => {}
        }
    }

    fn put_char(&mut self, c: char) {
        if c == '\u{7f}' || c == '\u{8}' {
            return;
        }

        let char_width: usize = UnicodeWidthChar::width(c).unwrap_or(1).clamp(1, 2);

        if self.screen_buffer.pending_wrap {
            // If the previous character reached the right edge,
            // finalize line wrap (auto-wrap) at the next print timing.
            self.wrap_line();
        }

        if char_width == 2 {
            let current_col: usize = self.screen_buffer.cursor.get_current_col();
            // If char width is over the buffer edge, start at next line.
            if current_col + 1 >= self.screen_buffer.cols() {
                self.wrap_line();
            }
        }

        // Fill with spaces up to the cursor position to allow overwriting at arbitrary positions.
        self.screen_buffer.ensure_cursor_col();

        let row: usize = self.screen_buffer.cursor.get_current_row();
        let col: usize = self.screen_buffer.cursor.get_current_col();

        let line: &mut Vec<ScreenCell> = &mut self.screen_buffer.lines[row];
        if col == line.len() {
            line.push(ScreenCell::new(c, self.screen_buffer.style));
        } else {
            line[col] = ScreenCell::new(c, self.screen_buffer.style);
        }

        if char_width == 2 {
            let continuation_col: usize = col + 1;
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

    fn new_line(&mut self) {
        self.advance_to_next_line();
    }

    fn wrap_line(&mut self) {
        self.advance_to_next_line();
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
        let col: usize = self.screen_buffer.cursor.get_current_col();
        let row: usize = self.screen_buffer.cursor.get_current_row();
        let line: &mut Vec<ScreenCell> = &mut self.screen_buffer.lines[row];

        if col < line.len() {
            line.remove(col);
        }
    }

    fn tab(&mut self) {
        // Move to the next tab stop on an 8-column boundary.
        let next_tab: usize = ((self.screen_buffer.cursor.get_current_col() / 8) + 1) * 8;
        self.move_cursor_to_col(next_tab.min(self.screen_buffer.cols().saturating_sub(1)));
    }

    pub(super) fn snapshot(&self) -> ScreenSnapshot {
        self.screen_buffer.snapshot()
    }

    fn move_cursor_up(&mut self, amount: usize) {
        let next_row: usize = self
            .screen_buffer
            .cursor
            .get_current_row()
            .saturating_sub(amount);

        self.screen_buffer.cursor.move_to_row(next_row);
        self.screen_buffer.pending_wrap = false;
    }

    fn move_cursor_down(&mut self, amount: usize) {
        let next_row: usize = self.screen_buffer.cursor.get_current_row() + amount;
        self.screen_buffer.cursor.move_to_row(next_row);
        self.screen_buffer.pending_wrap = false;
        self.screen_buffer.ensure_cursor_line();
    }

    fn move_cursor_forward(&mut self, amount: usize) {
        let next_col: usize = self.screen_buffer.cursor.get_current_col() + amount;
        self.move_cursor_to_col(next_col);
    }

    fn move_cursor_backward(&mut self, amount: usize) {
        let next_col: usize = self
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
        let cursor_row: usize = self.screen_buffer.cursor.get_current_row();
        let cursor_col: usize = self.screen_buffer.cursor.get_current_col();
        let last_row: usize = cursor_row.min(self.screen_buffer.lines.len().saturating_sub(1));

        for row in 0..=last_row {
            // On the cursor row, erase up to the cursor position.
            // In the rows above, erase to the end of the line by filling spaces.
            let end: usize = if row == cursor_row {
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

        let cursor_row: usize = self.screen_buffer.cursor.get_current_row();
        let cursor_col: usize = self.screen_buffer.cursor.get_current_col();

        // Fully clear lines below the cursor.
        for row in cursor_row + 1..self.screen_buffer.lines.len() {
            self.screen_buffer.lines[row].clear();
        }

        // On the cursor row, delete content to the right of the cursor.
        let line: &mut Vec<ScreenCell> = &mut self.screen_buffer.lines[cursor_row];
        if cursor_col < line.len() {
            line.truncate(cursor_col);
        }
    }

    fn delete_characters(&mut self, amount: usize) {
        self.screen_buffer.ensure_cursor_line();

        let row: usize = self.screen_buffer.cursor.get_current_row();
        let col: usize = self.screen_buffer.cursor.get_current_col();
        let line: &mut Vec<ScreenCell> = &mut self.screen_buffer.lines[row];

        for _ in 0..amount {
            if col < line.len() {
                line.remove(col);
            }
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
            self.move_cursor_to(CursorPosition {
                row: pos.row,
                col: pos.col,
            });
        }
    }

    fn advance_to_next_line(&mut self) {
        // Newline moves to the next line head and expands lines if required.
        self.screen_buffer.cursor.move_down();
        self.screen_buffer.cursor.reset_col();
        self.screen_buffer.pending_wrap = false;
        self.screen_buffer.ensure_cursor_line();
    }

    fn apply_graphics_rendition(&mut self, params: &[u16]) {
        const SGR_RESET: u16 = 0;
        const SGR_FAINT_ON: u16 = 2;
        const SGR_FAINT_OFF: u16 = 22;
        const SGR_FG_DEFAULT: u16 = 39;
        const SGR_FG_EXTENDED: u16 = 38;
        const SGR_BG_EXTENDED: u16 = 48;
        const SGR_UNDERLINE_COLOR_EXTENDED: u16 = 58;
        const SGR_FG_LOW_START: u16 = 30;
        const SGR_FG_LOW_END: u16 = 37;
        const SGR_FG_HIGH_START: u16 = 90;
        const SGR_FG_HIGH_END: u16 = 97;
        const SGR_FG_BRIGHT_OFFSET: u16 = 8;

        let mut index: usize = 0;
        while index < params.len() {
            let code: u16 = params[index];
            match code {
                SGR_RESET => self.screen_buffer.style = CellStyle::default(),
                SGR_FAINT_ON => self.screen_buffer.style.faint = true,
                SGR_FAINT_OFF => self.screen_buffer.style.faint = false,
                SGR_FG_LOW_START..=SGR_FG_LOW_END => {
                    self.screen_buffer.style.fg_rgba =
                        Some(ansi_16_color(usize::from(code - SGR_FG_LOW_START)));
                }
                SGR_FG_HIGH_START..=SGR_FG_HIGH_END => {
                    self.screen_buffer.style.fg_rgba = Some(ansi_16_color(usize::from(
                        (code - SGR_FG_HIGH_START) + SGR_FG_BRIGHT_OFFSET,
                    )));
                }
                SGR_FG_DEFAULT => self.screen_buffer.style.fg_rgba = None,
                SGR_FG_EXTENDED => {
                    let consumed: usize =
                        apply_sgr_foreground(&mut self.screen_buffer.style, &params[index..]);
                    index += consumed;
                    continue;
                }
                SGR_BG_EXTENDED | SGR_UNDERLINE_COLOR_EXTENDED => {
                    index += sgr_skip_count(&params[index..]);
                    continue;
                }
                _ => {}
            }

            index += 1;
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn cursor_position(&self) -> CursorPosition {
        CursorPosition {
            row: self.screen_buffer.cursor.get_current_row(),
            col: self.screen_buffer.cursor.get_current_col(),
        }
    }
}

fn sgr_skip_count(params: &[u16]) -> usize {
    match params.get(1).copied() {
        Some(5) => 3,
        Some(2) => 5,
        _ => 1,
    }
}

fn apply_sgr_foreground(style: &mut CellStyle, params: &[u16]) -> usize {
    match params.get(1).copied() {
        Some(5) => {
            if let Some(index) = params.get(2).copied() {
                style.fg_rgba = Some(ansi_256_color(usize::from(index)));
            }
            3
        }
        Some(2) => {
            let r: u8 = params.get(2).copied().unwrap_or(0).min(255) as u8;
            let g: u8 = params.get(3).copied().unwrap_or(0).min(255) as u8;
            let b: u8 = params.get(4).copied().unwrap_or(0).min(255) as u8;
            style.fg_rgba = Some([r, g, b, 255]);
            5
        }
        _ => 1,
    }
}

fn ansi_16_color(index: usize) -> [u8; 4] {
    const ANSI_16: [[u8; 4]; 16] = [
        [12, 12, 12, 255],
        [197, 15, 31, 255],
        [19, 161, 14, 255],
        [193, 156, 0, 255],
        [0, 55, 218, 255],
        [136, 23, 152, 255],
        [58, 150, 221, 255],
        [204, 204, 204, 255],
        [118, 118, 118, 255],
        [231, 72, 86, 255],
        [22, 198, 12, 255],
        [249, 241, 165, 255],
        [59, 120, 255, 255],
        [180, 0, 158, 255],
        [97, 214, 214, 255],
        [242, 242, 242, 255],
    ];

    ANSI_16[index.min(15)]
}

fn ansi_256_color(index: usize) -> [u8; 4] {
    match index {
        0..=15 => ansi_16_color(index),
        16..=231 => {
            let n: usize = index - 16;
            let r: usize = n / 36;
            let g: usize = (n % 36) / 6;
            let b: usize = n % 6;
            [cube_component(r), cube_component(g), cube_component(b), 255]
        }
        232..=255 => {
            let gray: u8 = (8 + (index - 232) * 10).min(255) as u8;
            [gray, gray, gray, 255]
        }
        _ => ansi_16_color(7),
    }
}

fn cube_component(level: usize) -> u8 {
    if level == 0 {
        0
    } else {
        (55 + level * 40).min(255) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube() {
        let level = 0;
        let cube = cube_component(level);
        assert_eq!(cube, 0);
    }
}
