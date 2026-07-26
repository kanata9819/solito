//! Pure cursor movement rules for copy mode.

use super::CopyModeMove;
use solito_renderer::CopyModePosition;
use solito_terminal::ScreenSnapshot;

pub(super) fn move_position(
    screen: &ScreenSnapshot,
    position: CopyModePosition,
    direction: CopyModeMove,
) -> CopyModePosition {
    let row_count = screen.lines.len().max(1);
    let mut row = position.row.min(row_count - 1);
    let mut col = position.col.min(last_col(screen, row));

    match direction {
        CopyModeMove::Left => col = col.saturating_sub(1),
        CopyModeMove::Right => col = col.saturating_add(1).min(last_col(screen, row)),
        CopyModeMove::Up => {
            row = row.saturating_sub(1);
            col = col.min(last_col(screen, row));
        }
        CopyModeMove::Down => {
            row = row.saturating_add(1).min(row_count - 1);
            col = col.min(last_col(screen, row));
        }
        CopyModeMove::StartOfLine => col = 0,
        CopyModeMove::EndOfLine => col = last_col(screen, row),
        CopyModeMove::NextWord => return next_word_start(screen, position),
        CopyModeMove::PreviousWord => return previous_word_start(screen, position),
        CopyModeMove::WordEnd => return word_end(screen, position),
        CopyModeMove::FirstLine => {
            row = 0;
            col = col.min(last_col(screen, row));
        }
        CopyModeMove::LastLine => {
            row = row_count - 1;
            col = col.min(last_col(screen, row));
        }
        CopyModeMove::PageUp => {
            row = row.saturating_sub(page_rows(screen));
            col = col.min(last_col(screen, row));
        }
        CopyModeMove::PageDown => {
            row = row.saturating_add(page_rows(screen)).min(row_count - 1);
            col = col.min(last_col(screen, row));
        }
    }

    CopyModePosition::new(row, col)
}

pub(super) fn clamp_position(
    screen: &ScreenSnapshot,
    position: CopyModePosition,
) -> CopyModePosition {
    let row_count = screen.lines.len().max(1);
    let row = position.row.min(row_count - 1);
    let col = position.col.min(last_col(screen, row));
    CopyModePosition::new(row, col)
}

fn next_word_start(screen: &ScreenSnapshot, position: CopyModePosition) -> CopyModePosition {
    let row_count = screen.lines.len().max(1);
    let mut row = position.row.min(row_count - 1);
    let mut col = position.col.min(last_col(screen, row));

    if is_nonblank(screen, row, col) {
        while col < line_len(screen, row) && is_nonblank(screen, row, col) {
            col += 1;
        }
    }

    while row < row_count {
        while col < line_len(screen, row) && !is_nonblank(screen, row, col) {
            col += 1;
        }

        if col < line_len(screen, row) {
            return CopyModePosition::new(row, col);
        }

        row += 1;
        col = 0;
    }

    last_position(screen)
}

fn previous_word_start(screen: &ScreenSnapshot, position: CopyModePosition) -> CopyModePosition {
    let row_count = screen.lines.len().max(1);
    let mut row = position.row.min(row_count - 1);
    let mut col = position.col.min(last_col(screen, row));

    if col > 0 {
        col -= 1;
    } else if row > 0 {
        row -= 1;
        col = last_col(screen, row);
    } else {
        return CopyModePosition::new(row, 0);
    }

    while !is_nonblank(screen, row, col) {
        if col > 0 {
            col -= 1;
        } else if row > 0 {
            row -= 1;
            col = last_col(screen, row);
        } else {
            return CopyModePosition::new(0, 0);
        }
    }

    while col > 0 && is_nonblank(screen, row, col - 1) {
        col -= 1;
    }

    CopyModePosition::new(row, col)
}

fn word_end(screen: &ScreenSnapshot, position: CopyModePosition) -> CopyModePosition {
    let row_count = screen.lines.len().max(1);
    let mut row = position.row.min(row_count - 1);
    let mut col = position.col.min(last_col(screen, row));

    if is_nonblank(screen, row, col)
        && col < last_col(screen, row)
        && is_nonblank(screen, row, col + 1)
    {
        while col < last_col(screen, row) && is_nonblank(screen, row, col + 1) {
            col += 1;
        }
        return CopyModePosition::new(row, col);
    }

    let start = next_word_start(screen, position);
    row = start.row.min(row_count - 1);
    col = start.col.min(last_col(screen, row));

    while col < last_col(screen, row) && is_nonblank(screen, row, col + 1) {
        col += 1;
    }

    CopyModePosition::new(row, col)
}

fn last_position(screen: &ScreenSnapshot) -> CopyModePosition {
    let row = screen.lines.len().max(1) - 1;
    CopyModePosition::new(row, last_col(screen, row))
}

fn page_rows(screen: &ScreenSnapshot) -> usize {
    (screen.lines.len() / 2).clamp(5, 20)
}

fn line_len(screen: &ScreenSnapshot, row: usize) -> usize {
    screen.lines.get(row).map_or(0, Vec::len)
}

fn last_col(screen: &ScreenSnapshot, row: usize) -> usize {
    line_len(screen, row).saturating_sub(1)
}

fn is_nonblank(screen: &ScreenSnapshot, row: usize, col: usize) -> bool {
    screen
        .lines
        .get(row)
        .and_then(|line| line.get(col))
        .is_some_and(|cell| !cell.is_wide_continuation && !cell.ch.is_whitespace())
}
