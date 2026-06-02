use solito_renderer::{
    CopyModePosition, CopyModeSelection, CopyModeSelectionKind, CopyModeSnapshot,
};
use solito_terminal::{ScreenCell, ScreenSnapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CopyModeMove {
    Left,
    Down,
    Up,
    Right,
    StartOfLine,
    EndOfLine,
    NextWord,
    PreviousWord,
    WordEnd,
    FirstLine,
    LastLine,
    PageUp,
    PageDown,
}

#[derive(Default)]
pub(super) struct CopyMode {
    snapshot: CopyModeSnapshot,
}

impl CopyMode {
    pub(super) fn is_active(&self) -> bool {
        self.snapshot.active
    }

    pub(super) fn enter(&mut self, screen: &ScreenSnapshot) {
        self.snapshot = CopyModeSnapshot {
            active: true,
            cursor: Self::clamp_position(
                screen,
                CopyModePosition::new(screen.cursor_row, screen.cursor_col),
            ),
            selection: None,
        };
    }

    pub(super) fn exit(&mut self) {
        self.snapshot = CopyModeSnapshot::default();
    }

    pub(super) fn move_cursor(&mut self, screen: &ScreenSnapshot, direction: CopyModeMove) {
        if !self.snapshot.active {
            return;
        }

        let cursor: CopyModePosition = Self::move_position(screen, self.snapshot.cursor, direction);
        self.snapshot.cursor = cursor;

        if let Some(selection) = self.snapshot.selection.as_mut() {
            selection.cursor = cursor;
        }
    }

    pub(super) fn toggle_cell_selection(&mut self) {
        if !self.snapshot.active {
            return;
        }

        if matches!(
            self.snapshot.selection,
            Some(CopyModeSelection {
                kind: CopyModeSelectionKind::Cell,
                ..
            })
        ) {
            self.snapshot.selection = None;
            return;
        }

        self.snapshot.selection = Some(CopyModeSelection {
            anchor: self.snapshot.cursor,
            cursor: self.snapshot.cursor,
            kind: CopyModeSelectionKind::Cell,
        });
    }

    pub(super) fn toggle_line_selection(&mut self) {
        if !self.snapshot.active {
            return;
        }

        if matches!(
            self.snapshot.selection,
            Some(CopyModeSelection {
                kind: CopyModeSelectionKind::Line,
                ..
            })
        ) {
            self.snapshot.selection = None;
            return;
        }

        self.snapshot.selection = Some(CopyModeSelection {
            anchor: self.snapshot.cursor,
            cursor: self.snapshot.cursor,
            kind: CopyModeSelectionKind::Line,
        });
    }

    pub(super) fn selected_text(&self, screen: &ScreenSnapshot) -> Option<String> {
        let selection: CopyModeSelection = self.snapshot.selection?;
        let mut rows: Vec<String> = Vec::new();

        match selection.kind {
            CopyModeSelectionKind::Line => {
                let (start_row, end_row): (usize, usize) =
                    Self::ordered_rows(selection.anchor, selection.cursor);

                for row in start_row..=end_row {
                    rows.push(Self::line_text(screen.lines.get(row), 0, usize::MAX));
                }
            }
            CopyModeSelectionKind::Cell => {
                let (start, end): (CopyModePosition, CopyModePosition) =
                    Self::ordered_positions(selection.anchor, selection.cursor);

                for row in start.row..=end.row {
                    let line_len: usize = Self::line_len(screen, row);
                    let start_col: usize = if row == start.row { start.col } else { 0 };
                    let end_col: usize = if row == end.row {
                        end.col.saturating_add(1).min(line_len)
                    } else {
                        line_len
                    };

                    rows.push(Self::line_text(
                        screen.lines.get(row),
                        start_col.min(line_len),
                        end_col,
                    ));
                }
            }
        }

        let text: String = rows.join("\n");

        if text.is_empty() { None } else { Some(text) }
    }

    pub(super) fn renderer_snapshot(&mut self, screen: &ScreenSnapshot) -> CopyModeSnapshot {
        if !self.snapshot.active {
            return CopyModeSnapshot::default();
        }

        self.snapshot.cursor = Self::clamp_position(screen, self.snapshot.cursor);

        if let Some(selection) = self.snapshot.selection.as_mut() {
            selection.anchor = Self::clamp_position(screen, selection.anchor);
            selection.cursor = self.snapshot.cursor;
        }

        self.snapshot
    }

    fn move_position(
        screen: &ScreenSnapshot,
        position: CopyModePosition,
        direction: CopyModeMove,
    ) -> CopyModePosition {
        let row_count: usize = screen.lines.len().max(1);
        let mut row: usize = position.row.min(row_count - 1);
        let mut col: usize = position.col.min(Self::last_col(screen, row));

        match direction {
            CopyModeMove::Left => col = col.saturating_sub(1),
            CopyModeMove::Right => {
                col = col.saturating_add(1).min(Self::last_col(screen, row));
            }
            CopyModeMove::Up => {
                row = row.saturating_sub(1);
                col = col.min(Self::last_col(screen, row));
            }
            CopyModeMove::Down => {
                row = row.saturating_add(1).min(row_count - 1);
                col = col.min(Self::last_col(screen, row));
            }
            CopyModeMove::StartOfLine => col = 0,
            CopyModeMove::EndOfLine => col = Self::last_col(screen, row),
            CopyModeMove::NextWord => return Self::next_word_start(screen, position),
            CopyModeMove::PreviousWord => return Self::previous_word_start(screen, position),
            CopyModeMove::WordEnd => return Self::word_end(screen, position),
            CopyModeMove::FirstLine => {
                row = 0;
                col = col.min(Self::last_col(screen, row));
            }
            CopyModeMove::LastLine => {
                row = row_count - 1;
                col = col.min(Self::last_col(screen, row));
            }
            CopyModeMove::PageUp => {
                row = row.saturating_sub(Self::page_rows(screen));
                col = col.min(Self::last_col(screen, row));
            }
            CopyModeMove::PageDown => {
                row = row
                    .saturating_add(Self::page_rows(screen))
                    .min(row_count - 1);
                col = col.min(Self::last_col(screen, row));
            }
        }

        CopyModePosition::new(row, col)
    }

    fn next_word_start(screen: &ScreenSnapshot, position: CopyModePosition) -> CopyModePosition {
        let row_count: usize = screen.lines.len().max(1);
        let mut row: usize = position.row.min(row_count - 1);
        let mut col: usize = position.col.min(Self::last_col(screen, row));

        if Self::is_nonblank(screen, row, col) {
            while col < Self::line_len(screen, row) && Self::is_nonblank(screen, row, col) {
                col += 1;
            }
        }

        while row < row_count {
            while col < Self::line_len(screen, row) && !Self::is_nonblank(screen, row, col) {
                col += 1;
            }

            if col < Self::line_len(screen, row) {
                return CopyModePosition::new(row, col);
            }

            row += 1;
            col = 0;
        }

        Self::last_position(screen)
    }

    fn previous_word_start(
        screen: &ScreenSnapshot,
        position: CopyModePosition,
    ) -> CopyModePosition {
        let row_count: usize = screen.lines.len().max(1);
        let mut row: usize = position.row.min(row_count - 1);
        let mut col: usize = position.col.min(Self::last_col(screen, row));

        if col > 0 {
            col -= 1;
        } else if row > 0 {
            row -= 1;
            col = Self::last_col(screen, row);
        } else {
            return CopyModePosition::new(row, 0);
        }

        loop {
            while !Self::is_nonblank(screen, row, col) {
                if col > 0 {
                    col -= 1;
                } else if row > 0 {
                    row -= 1;
                    col = Self::last_col(screen, row);
                } else {
                    return CopyModePosition::new(0, 0);
                }
            }

            while col > 0 && Self::is_nonblank(screen, row, col - 1) {
                col -= 1;
            }

            return CopyModePosition::new(row, col);
        }
    }

    fn word_end(screen: &ScreenSnapshot, position: CopyModePosition) -> CopyModePosition {
        let row_count: usize = screen.lines.len().max(1);
        let mut row: usize = position.row.min(row_count - 1);
        let mut col: usize = position.col.min(Self::last_col(screen, row));

        if Self::is_nonblank(screen, row, col)
            && col < Self::last_col(screen, row)
            && Self::is_nonblank(screen, row, col + 1)
        {
            while col < Self::last_col(screen, row) && Self::is_nonblank(screen, row, col + 1) {
                col += 1;
            }

            return CopyModePosition::new(row, col);
        }

        let start: CopyModePosition = Self::next_word_start(screen, position);
        row = start.row.min(row_count - 1);
        col = start.col.min(Self::last_col(screen, row));

        while col < Self::last_col(screen, row) && Self::is_nonblank(screen, row, col + 1) {
            col += 1;
        }

        CopyModePosition::new(row, col)
    }

    fn clamp_position(screen: &ScreenSnapshot, position: CopyModePosition) -> CopyModePosition {
        let row_count: usize = screen.lines.len().max(1);
        let row: usize = position.row.min(row_count - 1);
        let col: usize = position.col.min(Self::last_col(screen, row));

        CopyModePosition::new(row, col)
    }

    fn ordered_rows(anchor: CopyModePosition, cursor: CopyModePosition) -> (usize, usize) {
        if anchor.row <= cursor.row {
            (anchor.row, cursor.row)
        } else {
            (cursor.row, anchor.row)
        }
    }

    fn ordered_positions(
        anchor: CopyModePosition,
        cursor: CopyModePosition,
    ) -> (CopyModePosition, CopyModePosition) {
        if (anchor.row, anchor.col) <= (cursor.row, cursor.col) {
            (anchor, cursor)
        } else {
            (cursor, anchor)
        }
    }

    fn last_col(screen: &ScreenSnapshot, row: usize) -> usize {
        Self::line_len(screen, row).saturating_sub(1)
    }

    fn last_position(screen: &ScreenSnapshot) -> CopyModePosition {
        let row_count: usize = screen.lines.len().max(1);
        let row: usize = row_count - 1;

        CopyModePosition::new(row, Self::last_col(screen, row))
    }

    fn page_rows(screen: &ScreenSnapshot) -> usize {
        (screen.lines.len() / 2).clamp(5, 20)
    }

    fn line_len(screen: &ScreenSnapshot, row: usize) -> usize {
        screen.lines.get(row).map(|line| line.len()).unwrap_or(0)
    }

    fn is_nonblank(screen: &ScreenSnapshot, row: usize, col: usize) -> bool {
        screen
            .lines
            .get(row)
            .and_then(|line| line.get(col))
            .is_some_and(|cell| !cell.is_wide_continuation && !cell.ch.is_whitespace())
    }

    fn line_text(line: Option<&Vec<ScreenCell>>, start_col: usize, end_col: usize) -> String {
        let Some(line) = line else {
            return String::new();
        };

        line.iter()
            .enumerate()
            .filter(|(col, cell)| *col >= start_col && *col < end_col && !cell.is_wide_continuation)
            .map(|(_, cell)| cell.ch)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{CopyMode, CopyModeMove};
    use solito_terminal::{ScreenCell, ScreenSnapshot};

    fn screen(lines: &[&str], cursor_row: usize, cursor_col: usize) -> ScreenSnapshot {
        ScreenSnapshot {
            lines: lines
                .iter()
                .map(|line| {
                    line.chars()
                        .map(|ch| {
                            let mut cell = ScreenCell::default();
                            cell.ch = ch;
                            cell
                        })
                        .collect()
                })
                .collect(),
            cursor_row,
            cursor_col,
            cursor_color: None,
        }
    }

    #[test]
    fn cell_selection_copies_selected_range() {
        let screen = screen(&["abc", "def"], 0, 0);
        let mut copy_mode = CopyMode::default();

        copy_mode.enter(&screen);
        copy_mode.toggle_cell_selection();
        copy_mode.move_cursor(&screen, CopyModeMove::Right);
        copy_mode.move_cursor(&screen, CopyModeMove::Right);

        assert_eq!(copy_mode.selected_text(&screen), Some("abc".to_string()));
    }

    #[test]
    fn line_selection_copies_whole_rows() {
        let screen = screen(&["abc", "def"], 0, 1);
        let mut copy_mode = CopyMode::default();

        copy_mode.enter(&screen);
        copy_mode.toggle_line_selection();
        copy_mode.move_cursor(&screen, CopyModeMove::Down);

        assert_eq!(
            copy_mode.selected_text(&screen),
            Some("abc\ndef".to_string())
        );
    }

    #[test]
    fn word_motion_moves_between_nonblank_runs() {
        let screen = screen(&["abc  def ghi"], 0, 0);
        let mut copy_mode = CopyMode::default();

        copy_mode.enter(&screen);
        copy_mode.move_cursor(&screen, CopyModeMove::NextWord);
        assert_eq!(
            copy_mode.snapshot.cursor,
            solito_renderer::CopyModePosition::new(0, 5)
        );

        copy_mode.move_cursor(&screen, CopyModeMove::WordEnd);
        assert_eq!(
            copy_mode.snapshot.cursor,
            solito_renderer::CopyModePosition::new(0, 7)
        );

        copy_mode.move_cursor(&screen, CopyModeMove::PreviousWord);
        assert_eq!(
            copy_mode.snapshot.cursor,
            solito_renderer::CopyModePosition::new(0, 5)
        );
    }

    #[test]
    fn line_edge_motion_moves_to_line_bounds() {
        let screen = screen(&["abc"], 0, 1);
        let mut copy_mode = CopyMode::default();

        copy_mode.enter(&screen);
        copy_mode.move_cursor(&screen, CopyModeMove::EndOfLine);
        assert_eq!(
            copy_mode.snapshot.cursor,
            solito_renderer::CopyModePosition::new(0, 2)
        );

        copy_mode.move_cursor(&screen, CopyModeMove::StartOfLine);
        assert_eq!(
            copy_mode.snapshot.cursor,
            solito_renderer::CopyModePosition::new(0, 0)
        );
    }

    #[test]
    fn first_and_last_line_motion_preserve_column_when_possible() {
        let screen = screen(&["abc", "de", "efg"], 1, 1);
        let mut copy_mode = CopyMode::default();

        copy_mode.enter(&screen);
        copy_mode.move_cursor(&screen, CopyModeMove::LastLine);
        assert_eq!(
            copy_mode.snapshot.cursor,
            solito_renderer::CopyModePosition::new(2, 1)
        );

        copy_mode.move_cursor(&screen, CopyModeMove::FirstLine);
        assert_eq!(
            copy_mode.snapshot.cursor,
            solito_renderer::CopyModePosition::new(0, 1)
        );
    }
}
