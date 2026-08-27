use super::buffer::{CellStyle, ScreenBuffer, ScreenCell, ScreenSnapshot};
use super::cursor::CursorPosition;
use super::sgr;
use crate::TerminalSize;
use decodesc::{CsiMessage, EraseMode, EscMessage, OscMessage, TabClearMode};
use unicode_width::UnicodeWidthChar;

pub(crate) struct Screen {
    screen_buffer: ScreenBuffer,
    primary_screen: Option<ScreenBuffer>,
    last_printed: Option<char>,
}

impl Screen {
    pub(crate) fn new(size: TerminalSize) -> Self {
        let screen_buffer = ScreenBuffer::new(size);
        Self {
            screen_buffer,
            primary_screen: None,
            last_printed: None,
        }
    }

    pub(crate) fn resize(&mut self, size: TerminalSize) {
        self.screen_buffer.resize(size);
        if let Some(primary_screen) = &mut self.primary_screen {
            primary_screen.resize(size);
        }
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
            CsiMessage::CursorNextLine(amount) => {
                self.move_cursor_down(usize::from(amount));
                self.carriage_return();
            }
            CsiMessage::CursorPreviousLine(amount) => {
                self.move_cursor_up(usize::from(amount));
                self.carriage_return();
            }
            CsiMessage::CursorHorizontalAbsolute(col) => {
                self.move_cursor_to_col(usize::from(col).saturating_sub(1))
            }
            CsiMessage::CursorVerticalAbsolute(row) => {
                self.move_cursor_to_row_absolute(usize::from(row).saturating_sub(1))
            }
            CsiMessage::CursorHorizontalRelative(amount) => {
                self.move_cursor_forward(usize::from(amount))
            }
            CsiMessage::CursorVerticalRelative(amount) => {
                self.move_cursor_down(usize::from(amount))
            }
            CsiMessage::CursorPosition { row, col } => self.move_cursor_to(CursorPosition {
                row: self.cursor_row_from_viewport(usize::from(row).saturating_sub(1)),
                col: usize::from(col).saturating_sub(1),
            }),
            CsiMessage::EraseDisplay(mode) => self.erase_display(mode),
            CsiMessage::EraseLine(mode) => self.erase_line(mode),
            CsiMessage::EraseCharacters(amount) => self.erase_characters(usize::from(amount)),
            CsiMessage::DeleteCharacters(amount) => self.delete_characters(usize::from(amount)),
            CsiMessage::InsertBlankCharacters(amount) => {
                self.insert_blank_characters(usize::from(amount))
            }
            CsiMessage::InsertLines(amount) => self.insert_lines(usize::from(amount)),
            CsiMessage::DeleteLines(amount) => self.delete_lines(usize::from(amount)),
            CsiMessage::ScrollUp(amount) => self.scroll_up(usize::from(amount)),
            CsiMessage::ScrollDown(amount) => self.scroll_down(usize::from(amount)),
            CsiMessage::SetScrollRegion { top, bottom } => {
                self.set_scroll_region(usize::from(top), bottom.map(usize::from))
            }
            CsiMessage::CursorForwardTabulation(amount) => self.tab_forward(usize::from(amount)),
            CsiMessage::CursorBackwardTabulation(amount) => self.tab_backward(usize::from(amount)),
            CsiMessage::TabClear(mode) => self.clear_tab_stop(mode),
            CsiMessage::RepeatPrecedingCharacter(amount) => {
                self.repeat_preceding_character(usize::from(amount))
            }
            CsiMessage::SaveCursor => self.save_cursor_position(),
            CsiMessage::RestoreCursor => self.restore_cursor_position(),
            CsiMessage::SelectGraphicRendition(params) => {
                sgr::apply(&mut self.screen_buffer.style, &params)
            }
            CsiMessage::SetMode { private, modes } => self.set_modes(private, &modes),
            CsiMessage::ResetMode { private, modes } => self.reset_modes(private, &modes),
            CsiMessage::ShowCursor => self.screen_buffer.cursor_visible = true,
            CsiMessage::HideCursor => self.screen_buffer.cursor_visible = false,
            CsiMessage::Unknown { .. } | CsiMessage::DeviceStatusReport(_) => {}
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

    pub(super) fn apply_esc(&mut self, message: EscMessage) {
        match message {
            EscMessage::SaveCursor => self.save_cursor_position(),
            EscMessage::RestoreCursor => self.restore_cursor_position(),
            EscMessage::Index => self.index(),
            EscMessage::NextLine => {
                self.index();
                self.carriage_return();
            }
            EscMessage::ReverseIndex => self.reverse_index(),
            EscMessage::Reset => self.reset(),
            EscMessage::Unknown { .. } => {}
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
        const DEL: char = '\u{7f}';
        const BACKSPACE: char = '\u{8}';
        if c == DEL || c == BACKSPACE {
            return;
        }

        let char_width = UnicodeWidthChar::width(c).unwrap_or(1).clamp(1, 2);

        // If the previous character reached the right edge, wrap before printing this one.
        if self.screen_buffer.pending_wrap {
            if self.screen_buffer.auto_wrap {
                // If the previous character reached the right edge,
                // finalize line wrap (auto-wrap) at the next print timing.
                self.advance_to_next_line();
            } else {
                self.screen_buffer.pending_wrap = false;
            }
        }

        // if current charcter is full width, then status would be true
        let is_wide = char_width == 2;
        if is_wide {
            let current_col = self.screen_buffer.cursor.get_current_col();
            // If char width is over the buffer edge, start at next line.
            let next_col = current_col + 1;
            if next_col >= self.screen_buffer.cols() {
                self.advance_to_next_line();
            }
        }

        // Fill with spaces up to the cursor position to allow overwriting at arbitrary positions.
        self.screen_buffer.ensure_cursor_col();

        let row = self.screen_buffer.cursor.get_current_row();
        let col = self.screen_buffer.cursor.get_current_col();
        let cols = self.screen_buffer.cols();

        if self.screen_buffer.insert_mode {
            for _ in 0..char_width {
                self.screen_buffer.insert_cell(
                    row,
                    col,
                    ScreenCell::blank(self.screen_buffer.style),
                );
            }
            self.screen_buffer.truncate_line(row, cols);
        }

        if col == self.screen_buffer.line_len(row) {
            self.screen_buffer
                .push_cell(row, ScreenCell::new(c, self.screen_buffer.style));
        } else {
            self.screen_buffer
                .replace_cell(row, col, ScreenCell::new(c, self.screen_buffer.style));
        }

        if is_wide {
            let continuation_col = col + 1;
            if continuation_col == self.screen_buffer.line_len(row) {
                // Append it if the cell does not exist yet.
                self.screen_buffer
                    .push_cell(row, ScreenCell::wide_continuation(self.screen_buffer.style));
            } else {
                // Otherwise, replace the existing next cell.
                self.screen_buffer.replace_cell(
                    row,
                    continuation_col,
                    ScreenCell::wide_continuation(self.screen_buffer.style),
                );
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

        // for repeat writing
        self.last_printed = Some(c);
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
        let next_row = if self.screen_buffer.origin_mode {
            let (top, _) = self.scroll_region_bounds();
            self.screen_buffer
                .cursor
                .get_current_row()
                .saturating_sub(amount)
                .max(top)
        } else {
            self.screen_buffer
                .cursor
                .get_current_row()
                .saturating_sub(amount)
        };

        self.screen_buffer.cursor.move_to_row(next_row);
        self.screen_buffer.pending_wrap = false;
    }

    fn move_cursor_down(&mut self, amount: usize) {
        let next_row = if self.screen_buffer.origin_mode {
            let (_, bottom) = self.scroll_region_bounds();
            self.screen_buffer
                .cursor
                .get_current_row()
                .saturating_add(amount)
                .min(bottom)
        } else {
            self.screen_buffer.cursor.get_current_row() + amount
        };
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
        self.screen_buffer
            .cursor
            .move_to_col(col.min(self.screen_buffer.cols().saturating_sub(1)));
        self.screen_buffer.pending_wrap = false;
    }

    fn cursor_row_from_viewport(&self, row: usize) -> usize {
        let row = row.min(self.screen_buffer.rows().saturating_sub(1));
        if self.screen_buffer.origin_mode {
            let (top, bottom) = self.screen_buffer.scroll_region;
            top.saturating_add(row).min(bottom)
        } else {
            row
        }
    }

    fn move_cursor_to_row_absolute(&mut self, row: usize) {
        let row = self.cursor_row_from_viewport(row);
        let viewport_top = self.screen_buffer.get_viewport_top();
        self.screen_buffer.cursor.move_to_row(viewport_top + row);
        self.screen_buffer.pending_wrap = false;
        self.screen_buffer.ensure_cursor_line();
    }

    fn scroll_region_bounds(&mut self) -> (usize, usize) {
        let viewport_top = self.screen_buffer.get_viewport_top();
        let (top, bottom) = self.screen_buffer.scroll_region;
        let top = viewport_top + top;
        let bottom = viewport_top + bottom;
        while self.screen_buffer.lines.len() <= bottom {
            self.screen_buffer.lines.push(Vec::new());
        }
        (top, bottom)
    }

    fn set_scroll_region(&mut self, top: usize, bottom: Option<usize>) {
        let rows = self.screen_buffer.rows();
        let top = top.saturating_sub(1).min(rows.saturating_sub(1));
        let bottom = bottom
            .unwrap_or(rows)
            .saturating_sub(1)
            .clamp(top, rows.saturating_sub(1));
        self.screen_buffer.scroll_region = (top, bottom);
        self.screen_buffer.scroll_region_active = true;
        self.move_cursor_to(CursorPosition { row: 0, col: 0 });
    }

    fn scroll_up(&mut self, amount: usize) {
        let (top, bottom) = self.scroll_region_bounds();
        for _ in 0..amount.min(bottom - top + 1) {
            self.screen_buffer.lines.remove(top);
            self.screen_buffer.lines.insert(bottom, Vec::new());
        }
        self.screen_buffer.pending_wrap = false;
    }

    fn scroll_down(&mut self, amount: usize) {
        let (top, bottom) = self.scroll_region_bounds();
        for _ in 0..amount.min(bottom - top + 1) {
            self.screen_buffer.lines.remove(bottom);
            self.screen_buffer.lines.insert(top, Vec::new());
        }
        self.screen_buffer.pending_wrap = false;
    }

    fn insert_blank_characters(&mut self, amount: usize) {
        self.screen_buffer.ensure_cursor_col();
        let row = self.screen_buffer.cursor.get_current_row();
        let col = self.screen_buffer.cursor.get_current_col();
        let cols = self.screen_buffer.cols();
        let blank = ScreenCell::blank(self.screen_buffer.style);
        let line = &mut self.screen_buffer.lines[row];
        for _ in 0..amount.min(cols.saturating_sub(col)) {
            line.insert(col, blank);
        }
        line.truncate(cols);
        self.screen_buffer.pending_wrap = false;
    }

    fn insert_lines(&mut self, amount: usize) {
        let (top, bottom) = self.scroll_region_bounds();
        let row = self.screen_buffer.cursor.get_current_row();
        if !(top..=bottom).contains(&row) {
            return;
        }
        for _ in 0..amount.min(bottom - row + 1) {
            self.screen_buffer.lines.insert(row, Vec::new());
            self.screen_buffer.lines.remove(bottom + 1);
        }
        self.screen_buffer.pending_wrap = false;
    }

    fn delete_lines(&mut self, amount: usize) {
        let (top, bottom) = self.scroll_region_bounds();
        let row = self.screen_buffer.cursor.get_current_row();
        if !(top..=bottom).contains(&row) {
            return;
        }
        for _ in 0..amount.min(bottom - row + 1) {
            self.screen_buffer.lines.remove(row);
            self.screen_buffer.lines.insert(bottom, Vec::new());
        }
        self.screen_buffer.pending_wrap = false;
    }

    fn tab_forward(&mut self, amount: usize) {
        let mut col = self.screen_buffer.cursor.get_current_col();
        for _ in 0..amount {
            col = self
                .screen_buffer
                .tab_stops
                .iter()
                .enumerate()
                .skip(col.saturating_add(1))
                .find_map(|(index, stop)| stop.then_some(index))
                .unwrap_or(self.screen_buffer.cols().saturating_sub(1));
        }
        self.move_cursor_to_col(col);
    }

    fn tab_backward(&mut self, amount: usize) {
        let mut col = self.screen_buffer.cursor.get_current_col();
        for _ in 0..amount {
            col = self.screen_buffer.tab_stops[..col]
                .iter()
                .rposition(|stop| *stop)
                .unwrap_or(0);
        }
        self.move_cursor_to_col(col);
    }

    fn clear_tab_stop(&mut self, mode: TabClearMode) {
        match mode {
            TabClearMode::Current => {
                let col = self.screen_buffer.cursor.get_current_col();
                if let Some(stop) = self.screen_buffer.tab_stops.get_mut(col) {
                    *stop = false;
                }
            }
            TabClearMode::All => self.screen_buffer.tab_stops.fill(false),
        }
    }

    fn repeat_preceding_character(&mut self, amount: usize) {
        if let Some(c) = self.last_printed {
            for _ in 0..amount {
                self.put_char(c);
            }
        }
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
            self.screen_buffer.cursor.move_to(pos);
            self.screen_buffer.pending_wrap = false;
            self.screen_buffer.ensure_cursor_line();
        }
    }

    fn advance_to_next_line(&mut self) {
        // Newline moves to the next line head and expands lines if required.
        let cursor_row = self.screen_buffer.cursor.get_current_row();
        if self.screen_buffer.scroll_region_active {
            let (top, bottom) = self.scroll_region_bounds();
            if (top..=bottom).contains(&cursor_row) && cursor_row == bottom {
                self.scroll_up(1);
            } else {
                self.screen_buffer.cursor.move_down();
            }
        } else {
            self.screen_buffer.cursor.move_down();
        }
        self.screen_buffer.cursor.reset_col();
        self.screen_buffer.pending_wrap = false;
        self.screen_buffer.ensure_cursor_line();
    }

    fn index(&mut self) {
        let cursor_row = self.screen_buffer.cursor.get_current_row();
        if self.screen_buffer.scroll_region_active {
            let (top, bottom) = self.scroll_region_bounds();
            if (top..=bottom).contains(&cursor_row) && cursor_row == bottom {
                self.scroll_up(1);
            } else {
                self.screen_buffer.cursor.move_down();
            }
        } else {
            self.screen_buffer.cursor.move_down();
        }
        self.screen_buffer.pending_wrap = false;
        self.screen_buffer.ensure_cursor_line();
    }

    fn reverse_index(&mut self) {
        let cursor_row = self.screen_buffer.cursor.get_current_row();
        if self.screen_buffer.scroll_region_active {
            let (top, bottom) = self.scroll_region_bounds();
            if (top..=bottom).contains(&cursor_row) && cursor_row == top {
                self.scroll_down(1);
            } else {
                self.screen_buffer
                    .cursor
                    .move_to_row(cursor_row.saturating_sub(1));
            }
        } else {
            self.screen_buffer
                .cursor
                .move_to_row(cursor_row.saturating_sub(1));
        }
        self.screen_buffer.pending_wrap = false;
    }

    fn reset(&mut self) {
        let size = TerminalSize::new(self.screen_buffer.cols(), self.screen_buffer.rows());
        self.screen_buffer = ScreenBuffer::new(size);
        self.primary_screen = None;
        self.last_printed = None;
    }

    fn set_modes(&mut self, private: bool, modes: &[u16]) {
        for mode in modes {
            if private {
                match mode {
                    6 => self.screen_buffer.origin_mode = true,
                    7 => self.screen_buffer.auto_wrap = true,
                    25 => self.screen_buffer.cursor_visible = true,
                    47 | 1047 => self.enter_alternate_screen(false),
                    1048 => self.save_cursor_position(),
                    1049 => self.enter_alternate_screen(true),
                    _ => {}
                }
            } else if *mode == 4 {
                self.screen_buffer.insert_mode = true;
            }
        }
    }

    fn reset_modes(&mut self, private: bool, modes: &[u16]) {
        for mode in modes {
            if private {
                match mode {
                    6 => self.screen_buffer.origin_mode = false,
                    7 => self.screen_buffer.auto_wrap = false,
                    25 => self.screen_buffer.cursor_visible = false,
                    47 | 1047 => self.leave_alternate_screen(false),
                    1048 => self.restore_cursor_position(),
                    1049 => self.leave_alternate_screen(true),
                    _ => {}
                }
            } else if *mode == 4 {
                self.screen_buffer.insert_mode = false;
            }
        }
    }

    fn enter_alternate_screen(&mut self, save_cursor: bool) {
        if self.primary_screen.is_some() {
            return;
        }
        if save_cursor {
            self.save_cursor_position();
        }
        let size = TerminalSize::new(self.screen_buffer.cols(), self.screen_buffer.rows());
        self.primary_screen = Some(std::mem::replace(
            &mut self.screen_buffer,
            ScreenBuffer::new(size),
        ));
        self.last_printed = None;
    }

    fn leave_alternate_screen(&mut self, restore_cursor: bool) {
        if let Some(primary_screen) = self.primary_screen.take() {
            self.screen_buffer = primary_screen;
            if restore_cursor {
                self.restore_cursor_position();
            }
        }
        self.last_printed = None;
    }
}
