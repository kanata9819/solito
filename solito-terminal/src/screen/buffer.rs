use super::cursor::Cursor;
use super::lines::ScreenLines;
use crate::TerminalSize;
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CellStyle {
    pub(super) faint: bool,
    pub(super) fg_rgba: Option<[u8; 4]>,
    pub(super) bg_rgba: Option<[u8; 4]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenCell {
    pub ch: char,
    pub(super) style: CellStyle,
    pub is_wide_continuation: bool,
}

impl ScreenCell {
    pub(super) fn new(ch: char, style: CellStyle) -> Self {
        Self {
            ch,
            style,
            is_wide_continuation: false,
        }
    }

    pub(super) fn blank(style: CellStyle) -> Self {
        Self {
            ch: ' ',
            style,
            is_wide_continuation: false,
        }
    }

    pub(super) fn wide_continuation(style: CellStyle) -> Self {
        Self {
            is_wide_continuation: true,
            ..Self::blank(style)
        }
    }

    pub fn background_rgba(&self) -> Option<[u8; 4]> {
        self.style.bg_rgba
    }

    pub fn foreground_rgba(&self) -> Option<[u8; 4]> {
        self.style.fg_rgba
    }
}

/// A shared row. Snapshots clone the Arc; writing copies only a shared row.
#[derive(Clone, Debug, Default)]
pub struct ScreenLine {
    cells: std::sync::Arc<Vec<ScreenCell>>,
    /// Auto-wrap continued this row onto the next one. Resizing rejoins such
    /// rows, as ConPTY does with its own buffer.
    wrapped: bool,
}

impl ScreenLine {
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.cells, &other.cells)
    }

    pub fn is_wrapped(&self) -> bool {
        self.wrapped
    }

    pub(super) fn set_wrapped(&mut self, wrapped: bool) {
        self.wrapped = wrapped;
    }
}

impl std::ops::Deref for ScreenLine {
    type Target = Vec<ScreenCell>;
    fn deref(&self) -> &Self::Target {
        &self.cells
    }
}

impl std::ops::DerefMut for ScreenLine {
    fn deref_mut(&mut self) -> &mut Self::Target {
        std::sync::Arc::make_mut(&mut self.cells)
    }
}

impl PartialEq for ScreenLine {
    fn eq(&self, other: &Self) -> bool {
        self.wrapped == other.wrapped
            && (self.shares_storage_with(other) || self.cells == other.cells)
    }
}
impl Eq for ScreenLine {}
impl From<Vec<ScreenCell>> for ScreenLine {
    fn from(cells: Vec<ScreenCell>) -> Self {
        Self {
            cells: std::sync::Arc::new(cells),
            wrapped: false,
        }
    }
}
impl FromIterator<ScreenCell> for ScreenLine {
    fn from_iter<T: IntoIterator<Item = ScreenCell>>(iter: T) -> Self {
        Self::from(iter.into_iter().collect::<Vec<_>>())
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScreenSnapshot {
    pub lines: ScreenLines,
    pub history_start: usize,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub cursor_color: Option<[u8; 4]>,
    pub cursor_visible: bool,
}

#[derive(Clone, Default, Debug)]
pub(super) struct ScreenBuffer {
    cols: usize,
    rows: usize,
    pub(super) lines: ScreenLines,
    history_start: usize,
    pub(super) cursor: Cursor,
    pub(super) pending_wrap: bool,
    pub(super) style: CellStyle,
    pub(super) cursor_color: Option<[u8; 4]>,
    pub(super) scroll_region: (usize, usize),
    pub(super) origin_mode: bool,
    pub(super) insert_mode: bool,
    pub(super) auto_wrap: bool,
    pub(super) cursor_visible: bool,
    pub(super) tab_stops: Vec<bool>,
}

impl ScreenBuffer {
    pub(super) fn new(size: TerminalSize) -> Self {
        Self {
            cols: size.cols,
            rows: size.rows,
            lines: vec![ScreenLine::default()].into(),
            history_start: 0,
            cursor: Cursor::default(),
            pending_wrap: false,
            style: CellStyle::default(),
            cursor_color: None,
            scroll_region: (0, size.rows.saturating_sub(1)),
            origin_mode: false,
            insert_mode: false,
            auto_wrap: true,
            cursor_visible: true,
            tab_stops: default_tab_stops(size.cols),
        }
    }

    /// `reflow` rewraps soft-wrapped rows to the new width and keeps the top
    /// row anchored, matching how ConPTY resizes its own buffer. Without it,
    /// rows are clipped, which suits the alternate screen that apps redraw.
    pub(super) fn resize(&mut self, size: TerminalSize, reflow: bool) {
        if self.cols == size.cols && self.rows == size.rows {
            return;
        }
        if reflow {
            self.resize_with_reflow(size);
        } else {
            self.resize_clipped(size);
        }
    }

    fn resize_with_reflow(&mut self, size: TerminalSize) {
        let old_top = self.get_viewport_top();
        let (mut top, cursor) = if self.cols == size.cols {
            let cursor = (self.cursor.get_current_row(), self.cursor.get_current_col());
            (old_top, cursor)
        } else {
            self.rewrap(size.cols, old_top)
        };
        let (cursor_row, cursor_col) = cursor;
        // ConPTY grows downward from a fixed top row. It scrolls only to keep
        // the cursor and any text below it visible, such as the rest of a
        // right-aligned prompt that the narrower width split in two.
        let last_text = (cursor_row..self.lines.len())
            .rev()
            .find(|&row| !self.lines[row].is_empty())
            .unwrap_or(cursor_row);
        top = top
            .max((last_text + 1).saturating_sub(size.rows))
            .min(cursor_row);
        let target = top + size.rows;
        while self.lines.len() > target {
            self.lines.remove(self.lines.len() - 1);
        }
        if top > 0 {
            while self.lines.len() < target {
                self.lines.push_back(ScreenLine::default());
            }
        }
        self.cursor.move_to_row(cursor_row);
        self.cursor.move_to_col(cursor_col);
        self.apply_size(size);
    }

    /// Rejoin soft-wrapped rows and split them at `cols`. Returns the row now
    /// holding the start of `old_top` and the new cursor position.
    fn rewrap(&mut self, cols: usize, old_top: usize) -> (usize, (usize, usize)) {
        let old_cols = self.cols;
        let cursor_row = self.cursor.get_current_row();
        let cursor_col = self.cursor.get_current_col();
        let blank = ScreenCell::blank(CellStyle::default());
        let mut out: Vec<ScreenLine> = Vec::with_capacity(self.lines.len());
        let mut new_top = None;
        let mut new_cursor = None;
        let mut row = 0;
        while row < self.lines.len() {
            let start = row;
            while row + 1 < self.lines.len() && self.lines[row].is_wrapped() {
                row += 1;
            }
            let end = row;
            row += 1;
            let holds_cursor = (start..=end).contains(&cursor_row);
            if start == end && !holds_cursor && self.lines[start].len() <= cols {
                // Most rows fit as they are and keep sharing their storage.
                if start == old_top {
                    new_top = Some(out.len());
                }
                let mut line = self.lines[start].clone();
                line.set_wrapped(false);
                out.push(line);
                continue;
            }
            let mut cells = Vec::new();
            let mut top_offset = None;
            let mut cursor_offset = None;
            for source in start..=end {
                if source == old_top {
                    top_offset = Some(cells.len());
                }
                if source == cursor_row {
                    cursor_offset = Some(cells.len() + cursor_col);
                }
                let row_start = cells.len();
                cells.extend_from_slice(&self.lines[source]);
                // A wide glyph that did not fit left one empty cell; it is not text.
                let wide_pushed =
                    self.lines.get(source + 1).is_some_and(|next| {
                        next.get(1).is_some_and(|cell| cell.is_wide_continuation)
                    }) && self.lines[source].len() + 1 == old_cols;
                if source < end && !wide_pushed {
                    cells.resize(row_start + old_cols, blank);
                }
            }
            while cells.last() == Some(&blank) {
                cells.pop();
            }
            let first = out.len();
            let positions = split_cells(cells, cols, &mut out);
            if let Some(offset) = top_offset {
                new_top = Some(first + positions.locate(offset, cols).0);
            }
            if let Some(offset) = cursor_offset {
                let (row, col) = positions.locate(offset, cols);
                new_cursor = Some((first + row, col));
            }
        }
        self.lines = out.into();
        let last = self.lines.len().saturating_sub(1);
        let cursor = new_cursor.unwrap_or((cursor_row.min(last), cursor_col.min(cols - 1)));
        (new_top.unwrap_or(0), cursor)
    }

    fn resize_clipped(&mut self, size: TerminalSize) {
        // Remove rows below the cursor before pushing its row into scrollback.
        let top = self.get_viewport_top();
        while self.lines.len() > top + size.rows
            && self.lines.len() > self.cursor.get_current_row() + 1
        {
            self.lines.remove(self.lines.len() - 1);
        }
        let top = self.lines.len().saturating_sub(size.rows);
        for row in top..self.lines.len() {
            if self.lines[row].len() > size.cols {
                let line = &mut self.lines[row];
                line.truncate(size.cols);
                if let Some(last) = line.last_mut()
                    && last.ch.width() == Some(2)
                {
                    *last = ScreenCell::blank(CellStyle::default());
                }
            }
        }
        self.apply_size(size);
    }

    /// Adopt the new grid after rows have been rearranged for it.
    fn apply_size(&mut self, size: TerminalSize) {
        let full_height = self.scroll_region == (0, self.rows - 1);
        self.cols = size.cols;
        self.rows = size.rows;
        if full_height {
            self.scroll_region = (0, size.rows - 1);
        } else {
            self.scroll_region.0 = self.scroll_region.0.min(size.rows - 1);
            self.scroll_region.1 = self
                .scroll_region
                .1
                .clamp(self.scroll_region.0, size.rows - 1);
        }
        // Preserve existing stops, including ones explicitly cleared by the application.
        let old_cols = self.tab_stops.len();
        self.tab_stops.resize(size.cols, false);
        for col in old_cols..size.cols {
            self.tab_stops[col] = col % 8 == 0;
        }
        self.cursor
            .move_to_col(self.cursor.get_current_col().min(size.cols - 1));
        let top = self.get_viewport_top();
        self.cursor.move_to_row(
            self.cursor
                .get_current_row()
                .clamp(top, top + self.rows - 1),
        );
        self.pending_wrap = false;
    }

    pub(super) fn limit_history(&mut self, history_limit: usize) {
        let remove = self
            .lines
            .len()
            .saturating_sub(self.rows.saturating_add(history_limit));
        if remove == 0 {
            return;
        }
        self.lines.discard_prefix(remove);
        self.history_start += remove;
        self.cursor.discard_rows(remove);
    }

    pub(super) fn snapshot(&self) -> ScreenSnapshot {
        ScreenSnapshot {
            lines: self.lines.clone(),
            history_start: self.history_start,
            cursor_row: self.cursor.get_current_row(),
            cursor_col: self.cursor.get_current_col(),
            cursor_color: self.cursor_color,
            cursor_visible: self.cursor_visible,
        }
    }

    pub(super) fn cols(&self) -> usize {
        self.cols
    }

    pub(super) fn rows(&self) -> usize {
        self.rows
    }

    pub(super) fn ensure_cursor_line(&mut self) {
        while self.lines.len() <= self.cursor.get_current_row() {
            self.lines.push_back(ScreenLine::default());
        }
    }

    pub(super) fn ensure_cursor_col(&mut self) {
        self.ensure_cursor_line();
        let row = self.cursor.get_current_row();
        let col = self.cursor.get_current_col();
        if self.lines[row].len() < col {
            // Moving to a column does not paint the cells skipped along the way.
            // Only explicitly printed or erased cells receive the active style.
            self.lines[row].resize(col, ScreenCell::blank(CellStyle::default()));
        }
    }

    pub(super) fn get_viewport_top(&self) -> usize {
        self.lines.len().saturating_sub(self.rows)
    }
}

/// Where split rows begin, for mapping offsets in a joined row.
struct SplitRows {
    starts: Vec<usize>,
    content: usize,
}

impl SplitRows {
    /// Row and column of `offset`. Offsets past the text, like a cursor after
    /// a prompt, stay on the last row at the matching column.
    fn locate(&self, offset: usize, cols: usize) -> (usize, usize) {
        if offset < self.content {
            let row = self.starts.partition_point(|&start| start <= offset) - 1;
            return (row, offset - self.starts[row]);
        }
        let row = self.starts.len() - 1;
        let col = self.content - self.starts[row] + (offset - self.content);
        (row, col.min(cols - 1))
    }
}

/// Split one joined row into rows of `cols` cells, marking all but the last as wrapped.
/// A wide glyph that would straddle the edge moves to the next row, as when printed.
fn split_cells(cells: Vec<ScreenCell>, cols: usize, out: &mut Vec<ScreenLine>) -> SplitRows {
    let content = cells.len();
    let mut starts = vec![0];
    let mut current = Vec::with_capacity(cols.min(content));
    let mut index = 0;
    while index < content {
        let width = if cells
            .get(index + 1)
            .is_some_and(|cell| cell.is_wide_continuation)
        {
            2
        } else {
            1
        };
        if current.len() + width > cols && !current.is_empty() {
            let mut line = ScreenLine::from(std::mem::take(&mut current));
            line.set_wrapped(true);
            out.push(line);
            starts.push(index);
        }
        current.extend_from_slice(&cells[index..index + width]);
        index += width;
    }
    out.push(ScreenLine::from(current));
    SplitRows { starts, content }
}

fn default_tab_stops(cols: usize) -> Vec<bool> {
    (0..cols).map(|col| col % 8 == 0).collect()
}
