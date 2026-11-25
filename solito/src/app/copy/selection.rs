use solito_renderer::{CopyModeSelection, CopyModeSelectionKind};
use solito_terminal::ScreenSnapshot;

pub(super) fn selected_text(
    selection: CopyModeSelection,
    screen: &ScreenSnapshot,
) -> Option<String> {
    let start = selection.anchor.min(selection.cursor);
    let end = selection.anchor.max(selection.cursor);
    let whole_lines = selection.kind == CopyModeSelectionKind::Line;

    let mut text = String::new();
    for row in start.row..=end.row {
        let Some(line) = screen.lines.get(row) else {
            break;
        };
        if row != start.row && !line.wrapped_from_previous() {
            text.push('\n');
        }
        let first_col = if !whole_lines && row == start.row {
            start.col
        } else {
            0
        };
        let end_col = if !whole_lines && row == end.row {
            end.col.saturating_add(1)
        } else {
            line.len()
        };
        text.extend(
            line.iter()
                .skip(first_col)
                .take(end_col.saturating_sub(first_col))
                .filter(|cell| !cell.is_wide_continuation)
                .map(|cell| cell.ch),
        );
    }

    (!text.is_empty()).then_some(text)
}
