//! Convert a copy-mode selection into clipboard text.

use solito_renderer::{CopyModeSelection, CopyModeSelectionKind};
use solito_terminal::{ScreenCell, ScreenSnapshot};

pub(super) fn selected_text(
    selection: CopyModeSelection,
    screen: &ScreenSnapshot,
) -> Option<String> {
    let rows = match selection.kind {
        CopyModeSelectionKind::Line => selected_lines(selection, screen),
        CopyModeSelectionKind::Cell => selected_cells(selection, screen),
    };
    let text = rows.join("\n");

    (!text.is_empty()).then_some(text)
}

fn selected_lines(selection: CopyModeSelection, screen: &ScreenSnapshot) -> Vec<String> {
    let start_row = selection.anchor.row.min(selection.cursor.row);
    let end_row = selection.anchor.row.max(selection.cursor.row);

    (start_row..=end_row)
        .map(|row| line_text(screen.lines.get(row), 0, usize::MAX))
        .collect()
}

fn selected_cells(selection: CopyModeSelection, screen: &ScreenSnapshot) -> Vec<String> {
    let start = selection.anchor.min(selection.cursor);
    let end = selection.anchor.max(selection.cursor);

    (start.row..=end.row)
        .map(|row| {
            let line_len = screen.lines.get(row).map_or(0, Vec::len);
            let start_col = if row == start.row { start.col } else { 0 };
            let end_col = if row == end.row {
                end.col.saturating_add(1).min(line_len)
            } else {
                line_len
            };

            line_text(screen.lines.get(row), start_col.min(line_len), end_col)
        })
        .collect()
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
