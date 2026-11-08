use super::CopyModeMove;
use solito_renderer::CopyModePosition;
use solito_terminal::ScreenSnapshot;

pub(super) fn move_position(
    screen: &ScreenSnapshot,
    position: CopyModePosition,
    direction: CopyModeMove,
) -> CopyModePosition {
    let row_count = screen.lines.len().max(1);
    let CopyModePosition { row, col } = position;

    let target = match direction {
        CopyModeMove::Left => CopyModePosition::new(row, col.saturating_sub(1)),
        CopyModeMove::Right => CopyModePosition::new(row, next_glyph_col(screen, row, col)),
        CopyModeMove::Up => CopyModePosition::new(row.saturating_sub(1), col),
        CopyModeMove::Down => CopyModePosition::new(row + 1, col),
        CopyModeMove::StartOfLine => CopyModePosition::new(row, 0),
        CopyModeMove::EndOfLine => CopyModePosition::new(row, last_col(screen, row)),
        CopyModeMove::NextWord => next_word_start(screen, position),
        CopyModeMove::PreviousWord => previous_word_start(screen, position),
        CopyModeMove::WordEnd => word_end(screen, position),
        CopyModeMove::FirstLine => CopyModePosition::new(0, col),
        CopyModeMove::LastLine => CopyModePosition::new(row_count - 1, col),
        CopyModeMove::PageUp => CopyModePosition::new(row.saturating_sub(page_rows(screen)), col),
        CopyModeMove::PageDown => CopyModePosition::new(row + page_rows(screen), col),
    };

    clamp_position(screen, target)
}

pub(super) fn clamp_position(
    screen: &ScreenSnapshot,
    position: CopyModePosition,
) -> CopyModePosition {
    let row_count = screen.lines.len().max(1);
    let row = position.row.min(row_count - 1);
    let col = position.col.min(last_col(screen, row));
    CopyModePosition::new(row, leading_col(screen, row, col))
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
    let CopyModePosition { row, col } = position;
    let next = next_glyph_col(screen, row, col);
    let CopyModePosition { row, mut col } =
        if is_nonblank(screen, row, col) && is_nonblank(screen, row, next) {
            position
        } else {
            next_word_start(screen, position)
        };

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
    screen.lines.get(row).map_or(0, |line| line.len())
}

fn last_col(screen: &ScreenSnapshot, row: usize) -> usize {
    line_len(screen, row).saturating_sub(1)
}

fn is_nonblank(screen: &ScreenSnapshot, row: usize, col: usize) -> bool {
    let col = leading_col(screen, row, col);
    screen
        .lines
        .get(row)
        .and_then(|line| line.get(col))
        .is_some_and(|cell| !cell.ch.is_whitespace())
}

fn leading_col(screen: &ScreenSnapshot, row: usize, col: usize) -> usize {
    col.saturating_sub(usize::from(is_continuation(screen, row, col)))
}

fn next_glyph_col(screen: &ScreenSnapshot, row: usize, col: usize) -> usize {
    let next = col + 1;
    next + usize::from(is_continuation(screen, row, next))
}

fn is_continuation(screen: &ScreenSnapshot, row: usize, col: usize) -> bool {
    screen
        .lines
        .get(row)
        .and_then(|line| line.get(col))
        .is_some_and(|cell| cell.is_wide_continuation)
}
