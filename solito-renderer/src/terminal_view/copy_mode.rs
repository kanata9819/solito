use solito_terminal::ScreenCell;

use crate::{pipeline::rect::RectSpec, terminal_view::TerminalView, util::color::ThemeColor};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyModePosition {
    pub row: usize,
    pub col: usize,
}

impl CopyModePosition {
    pub const fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyModeSelectionKind {
    Cell,
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopyModeSelection {
    pub anchor: CopyModePosition,
    pub cursor: CopyModePosition,
    pub kind: CopyModeSelectionKind,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyModeSnapshot {
    pub active: bool,
    pub cursor: CopyModePosition,
    pub selection: Option<CopyModeSelection>,
}

impl TerminalView {
    const COPY_MODE_SELECTION_COLOR: [f32; 4] = ThemeColor::BLUE_500_ALPHA;
    const COPY_MODE_CURSOR_COLOR: [f32; 4] = ThemeColor::YELLOW_400_ALPHA;

    pub(crate) fn set_copy_mode(&mut self, copy_mode: CopyModeSnapshot) {
        self.copy_mode = copy_mode;

        if self.copy_mode.active {
            let row_count = self.row_count();
            self.viewport
                .scroll_to_include(self.copy_mode.cursor.row, row_count);
        }

        self.set_text_to_buffer();
    }

    pub(crate) fn copy_mode_active(&self) -> bool {
        self.copy_mode.active
    }

    pub(crate) fn copy_mode_rects(&mut self) -> Vec<RectSpec> {
        if !self.copy_mode.active {
            return Vec::new();
        }

        let row_count = self.row_count();
        self.viewport.clamp(row_count);
        let (visible_start, visible_end): (usize, usize) = self.viewport.visible_range(row_count);
        Self::copy_mode_rects_for(
            &self.copy_mode,
            &self.snapshot.lines,
            visible_start,
            visible_end,
            self.glyphs.cell_width,
            self.config.line_height,
            self.has_tab_bar(),
        )
    }

    fn copy_mode_rects_for(
        copy_mode: &CopyModeSnapshot,
        lines: &[Vec<ScreenCell>],
        visible_start: usize,
        visible_end: usize,
        cell_width: f32,
        line_height: f32,
        has_tab_bar: bool,
    ) -> Vec<RectSpec> {
        let mut rects = Vec::new();

        if let Some(selection) = copy_mode.selection {
            for row in visible_start..visible_end {
                if let Some((start_col, end_col)) =
                    Self::selected_cols_for_row(selection, row, lines)
                    && start_col < end_col
                {
                    let visible_row = row.saturating_sub(visible_start);
                    rects.push(RectSpec::new(
                        Self::PADDING_X + start_col as f32 * cell_width,
                        Self::terminal_row_y(visible_row, line_height, has_tab_bar),
                        (end_col - start_col) as f32 * cell_width,
                        line_height,
                        Self::COPY_MODE_SELECTION_COLOR,
                    ));
                }
            }
        }

        if copy_mode.cursor.row >= visible_start && copy_mode.cursor.row < visible_end {
            let visible_row = copy_mode.cursor.row - visible_start;
            let cursor_col = copy_mode
                .cursor
                .col
                .min(Self::display_col_count(lines, copy_mode.cursor.row) - 1);

            rects.push(RectSpec::new(
                Self::PADDING_X + cursor_col as f32 * cell_width,
                Self::terminal_row_y(visible_row, line_height, has_tab_bar),
                cell_width,
                line_height,
                Self::COPY_MODE_CURSOR_COLOR,
            ));
        }

        rects
    }

    fn selected_cols_for_row(
        selection: CopyModeSelection,
        row: usize,
        lines: &[Vec<ScreenCell>],
    ) -> Option<(usize, usize)> {
        match selection.kind {
            CopyModeSelectionKind::Line => {
                let (start_row, end_row): (usize, usize) =
                    Self::ordered_rows(selection.anchor, selection.cursor);

                if row < start_row || row > end_row {
                    return None;
                }

                Some((0, Self::display_col_count(lines, row)))
            }
            CopyModeSelectionKind::Cell => {
                let (start, end): (CopyModePosition, CopyModePosition) =
                    Self::ordered_positions(selection.anchor, selection.cursor);

                if row < start.row || row > end.row {
                    return None;
                }

                let col_count = Self::display_col_count(lines, row);
                let start_col = if row == start.row {
                    start.col.min(col_count - 1)
                } else {
                    0
                };
                let end_col = if row == end.row {
                    end.col.min(col_count - 1) + 1
                } else {
                    col_count
                };

                Some((start_col, end_col))
            }
        }
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
}
