mod movement;
mod selection;

use solito_renderer::{
    CopyModePosition, CopyModeSelection, CopyModeSelectionKind, CopyModeSnapshot,
};
use solito_terminal::ScreenSnapshot;

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
    history_start: usize,
}

impl CopyMode {
    pub(super) fn is_active(&self) -> bool {
        self.snapshot.active
    }

    pub(super) fn enter(&mut self, screen: &ScreenSnapshot) {
        self.history_start = screen.history_start;
        self.snapshot = CopyModeSnapshot {
            active: true,
            cursor: movement::clamp_position(
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

        let cursor = movement::move_position(screen, self.snapshot.cursor, direction);
        self.snapshot.cursor = cursor;

        if let Some(selection) = self.snapshot.selection.as_mut() {
            selection.cursor = cursor;
        }
    }

    pub(super) fn toggle_cell_selection(&mut self) {
        self.toggle_selection(CopyModeSelectionKind::Cell);
    }

    pub(super) fn toggle_line_selection(&mut self) {
        self.toggle_selection(CopyModeSelectionKind::Line);
    }

    fn toggle_selection(&mut self, kind: CopyModeSelectionKind) {
        if !self.snapshot.active {
            return;
        }

        if self.snapshot.selection.map(|selection| selection.kind) == Some(kind) {
            self.snapshot.selection = None;
            return;
        }

        self.snapshot.selection = Some(CopyModeSelection {
            anchor: self.snapshot.cursor,
            cursor: self.snapshot.cursor,
            kind,
        });
    }

    pub(super) fn selected_text(&self, screen: &ScreenSnapshot) -> Option<String> {
        selection::selected_text(self.snapshot.selection?, screen)
    }

    pub(super) fn renderer_snapshot(&mut self, screen: &ScreenSnapshot) -> CopyModeSnapshot {
        if !self.snapshot.active {
            return CopyModeSnapshot::default();
        }

        let removed = screen.history_start.saturating_sub(self.history_start);
        self.history_start = screen.history_start;
        self.snapshot.cursor.row = self.snapshot.cursor.row.saturating_sub(removed);
        if let Some(selection) = self.snapshot.selection.as_mut() {
            if selection.anchor.row < removed || selection.cursor.row < removed {
                // Do not silently copy unrelated text after selected history expires.
                self.snapshot.selection = None;
            } else {
                selection.anchor.row -= removed;
            }
        }
        self.snapshot.cursor = movement::clamp_position(screen, self.snapshot.cursor);

        if let Some(selection) = self.snapshot.selection.as_mut() {
            selection.anchor = movement::clamp_position(screen, selection.anchor);
            selection.cursor = self.snapshot.cursor;
        }

        self.snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::{CopyMode, CopyModeMove};
    use solito_renderer::{CopyModePosition, CopyModeSelectionKind};
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
            cursor_visible: true,
            history_start: 0,
        }
    }

    #[test]
    fn selection_follows_retained_history_and_expires_when_removed() {
        let mut original = screen(&["old", "keep", "last"], 1, 0);
        let mut mode = CopyMode::default();
        mode.enter(&original);
        mode.toggle_cell_selection();
        original.lines.remove(0);
        original.history_start = 1;
        assert_eq!(mode.renderer_snapshot(&original).cursor.row, 0);
        assert_eq!(mode.selected_text(&original), Some("k".to_string()));
        original.lines.remove(0);
        original.history_start = 2;
        assert!(mode.renderer_snapshot(&original).selection.is_none());
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
    fn cell_selection_orders_a_reversed_range() {
        let screen = screen(&["abc"], 0, 2);
        let mut copy_mode = CopyMode::default();

        copy_mode.enter(&screen);
        copy_mode.toggle_cell_selection();
        copy_mode.move_cursor(&screen, CopyModeMove::Left);
        copy_mode.move_cursor(&screen, CopyModeMove::Left);

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
        assert_eq!(copy_mode.snapshot.cursor, CopyModePosition::new(0, 5));

        copy_mode.move_cursor(&screen, CopyModeMove::WordEnd);
        assert_eq!(copy_mode.snapshot.cursor, CopyModePosition::new(0, 7));

        copy_mode.move_cursor(&screen, CopyModeMove::PreviousWord);
        assert_eq!(copy_mode.snapshot.cursor, CopyModePosition::new(0, 5));
    }

    #[test]
    fn line_edge_motion_moves_to_line_bounds() {
        let screen = screen(&["abc"], 0, 1);
        let mut copy_mode = CopyMode::default();

        copy_mode.enter(&screen);
        copy_mode.move_cursor(&screen, CopyModeMove::EndOfLine);
        assert_eq!(copy_mode.snapshot.cursor, CopyModePosition::new(0, 2));

        copy_mode.move_cursor(&screen, CopyModeMove::StartOfLine);
        assert_eq!(copy_mode.snapshot.cursor, CopyModePosition::new(0, 0));
    }

    #[test]
    fn first_and_last_line_motion_preserve_column_when_possible() {
        let screen = screen(&["abc", "de", "efg"], 1, 1);
        let mut copy_mode = CopyMode::default();

        copy_mode.enter(&screen);
        copy_mode.move_cursor(&screen, CopyModeMove::LastLine);
        assert_eq!(copy_mode.snapshot.cursor, CopyModePosition::new(2, 1));

        copy_mode.move_cursor(&screen, CopyModeMove::FirstLine);
        assert_eq!(copy_mode.snapshot.cursor, CopyModePosition::new(0, 1));
    }

    #[test]
    fn toggling_selection_switches_kind_and_clears_the_same_kind() {
        let screen = screen(&["abc"], 0, 1);
        let mut copy_mode = CopyMode::default();
        copy_mode.enter(&screen);

        copy_mode.toggle_cell_selection();
        assert_eq!(
            copy_mode.snapshot.selection.map(|selection| selection.kind),
            Some(CopyModeSelectionKind::Cell)
        );

        copy_mode.toggle_line_selection();
        assert_eq!(
            copy_mode.snapshot.selection.map(|selection| selection.kind),
            Some(CopyModeSelectionKind::Line)
        );

        copy_mode.toggle_line_selection();
        assert!(copy_mode.snapshot.selection.is_none());
    }
}
