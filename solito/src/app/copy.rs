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

    pub(super) fn has_selection(&self) -> bool {
        self.snapshot.selection.is_some()
    }

    pub(super) fn clear_selection(&mut self) -> bool {
        self.snapshot.selection.take().is_some()
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

    pub(super) fn exit(&mut self) -> bool {
        let changed = self.snapshot != CopyModeSnapshot::default();
        self.snapshot = CopyModeSnapshot::default();
        changed
    }

    pub(super) fn move_cursor(&mut self, screen: &ScreenSnapshot, direction: CopyModeMove) {
        if !self.snapshot.active {
            return;
        }

        self.update_snapshot(screen);
        let cursor = movement::move_position(screen, self.snapshot.cursor, direction);
        self.snapshot.cursor = cursor;

        if let Some(selection) = self.snapshot.selection.as_mut() {
            selection.cursor = cursor;
        }
    }

    pub(super) fn toggle_cell_selection(&mut self, screen: &ScreenSnapshot) {
        self.toggle_selection(screen, CopyModeSelectionKind::Cell);
    }

    pub(super) fn toggle_line_selection(&mut self, screen: &ScreenSnapshot) {
        self.toggle_selection(screen, CopyModeSelectionKind::Line);
    }

    pub(super) fn select_cells(
        &mut self,
        screen: &ScreenSnapshot,
        history_anchor: CopyModePosition,
        history_cursor: CopyModePosition,
    ) -> bool {
        let previous = self.snapshot;
        self.update_snapshot(screen);
        if let Some(anchor) = position_in_snapshot(screen, history_anchor)
            && let Some(cursor) = position_in_snapshot(screen, history_cursor)
        {
            self.history_start = screen.history_start;
            self.snapshot.cursor = cursor;
            self.snapshot.selection = Some(CopyModeSelection {
                anchor,
                cursor,
                kind: CopyModeSelectionKind::Cell,
            });
        }
        self.snapshot != previous
    }

    fn toggle_selection(&mut self, screen: &ScreenSnapshot, kind: CopyModeSelectionKind) {
        if !self.snapshot.active {
            return;
        }

        self.update_snapshot(screen);
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

    pub(super) fn selected_text(&mut self, screen: &ScreenSnapshot) -> Option<String> {
        selection::selected_text(self.update_snapshot(screen).selection?, screen)
    }

    pub(super) fn update_snapshot(&mut self, screen: &ScreenSnapshot) -> CopyModeSnapshot {
        if !self.snapshot.active && self.snapshot.selection.is_none() {
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

fn position_in_snapshot(
    screen: &ScreenSnapshot,
    position: CopyModePosition,
) -> Option<CopyModePosition> {
    let row = position.row.checked_sub(screen.history_start)?;
    (row < screen.lines.len())
        .then(|| movement::clamp_position(screen, CopyModePosition::new(row, position.col)))
}

#[cfg(test)]
mod tests {
    use super::{CopyMode, CopyModeMove};
    use solito_renderer::{CopyModePosition, CopyModeSelectionKind};
    use solito_terminal::{ScreenCell, ScreenSnapshot, TerminalSize, TerminalState};

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
        mode.toggle_cell_selection(&original);
        original.lines.remove(0);
        original.history_start = 1;
        assert_eq!(mode.selected_text(&original), Some("k".to_string()));
        assert_eq!(mode.update_snapshot(&original).cursor.row, 0);
        original.lines.remove(0);
        original.history_start = 2;
        assert_eq!(mode.selected_text(&original), None);
        assert!(mode.update_snapshot(&original).selection.is_none());
    }

    #[test]
    fn moving_before_redraw_uses_the_cursor_after_history_eviction() {
        let mut original = screen(&["old", "first", "second", "third"], 1, 0);
        let mut mode = CopyMode::default();
        mode.enter(&original);
        mode.toggle_cell_selection(&original);
        original.lines.remove(0);
        original.history_start = 1;

        mode.move_cursor(&original, CopyModeMove::Down);

        assert_eq!(mode.selected_text(&original).as_deref(), Some("first\ns"));
    }

    #[test]
    fn cell_selection_copies_selected_range() {
        let screen = screen(&["abc", "def"], 0, 0);
        let mut copy_mode = CopyMode::default();

        copy_mode.enter(&screen);
        copy_mode.toggle_cell_selection(&screen);
        copy_mode.move_cursor(&screen, CopyModeMove::Right);
        copy_mode.move_cursor(&screen, CopyModeMove::Right);

        assert_eq!(copy_mode.selected_text(&screen), Some("abc".to_string()));
    }

    #[test]
    fn cell_selection_orders_a_reversed_range() {
        let screen = screen(&["abc"], 0, 2);
        let mut copy_mode = CopyMode::default();

        copy_mode.enter(&screen);
        copy_mode.toggle_cell_selection(&screen);
        copy_mode.move_cursor(&screen, CopyModeMove::Left);
        copy_mode.move_cursor(&screen, CopyModeMove::Left);

        assert_eq!(copy_mode.selected_text(&screen), Some("abc".to_string()));
    }

    #[test]
    fn line_selection_copies_whole_rows() {
        let screen = screen(&["abc", "def"], 0, 1);
        let mut copy_mode = CopyMode::default();

        copy_mode.enter(&screen);
        copy_mode.toggle_line_selection(&screen);
        copy_mode.move_cursor(&screen, CopyModeMove::Down);

        assert_eq!(
            copy_mode.selected_text(&screen),
            Some("abc\ndef".to_string())
        );
    }

    #[test]
    fn mouse_selects_characters_in_either_direction_without_entering_copy_mode() {
        let mut terminal = TerminalState::new(TerminalSize::new(10, 2));
        terminal.apply_terminal_output("first\r\na日b".as_bytes());
        let mut screen = terminal.snapshot();
        screen.history_start = 20;
        let mut copy_mode = CopyMode::default();
        let first = CopyModePosition::new(20, 3);
        let wide = CopyModePosition::new(21, 1);
        let continuation = CopyModePosition::new(21, 2);

        assert!(copy_mode.select_cells(&screen, wide, continuation));
        assert_eq!(copy_mode.selected_text(&screen).as_deref(), Some("日"));
        assert!(!copy_mode.is_active());

        for (anchor, cursor) in [(first, continuation), (continuation, first)] {
            assert!(copy_mode.select_cells(&screen, anchor, cursor));
            assert_eq!(copy_mode.selected_text(&screen).as_deref(), Some("st\na日"));
        }

        assert!(copy_mode.clear_selection());
        assert_eq!(copy_mode.selected_text(&screen), None);
    }

    #[test]
    fn mouse_drag_tracks_history_rows_until_its_anchor_expires() {
        let mut screen = screen(&["old", "first", "second", "third"], 0, 0);
        let mut copy_mode = CopyMode::default();
        let anchor = CopyModePosition::new(1, 1);
        let cursor = CopyModePosition::new(2, 0);
        assert!(copy_mode.select_cells(&screen, anchor, anchor));

        screen.lines.remove(0);
        screen.history_start = 1;
        assert!(copy_mode.select_cells(&screen, anchor, cursor));
        assert_eq!(copy_mode.selected_text(&screen).as_deref(), Some("irst\ns"));

        screen.lines.remove(0);
        screen.history_start = 2;
        assert!(copy_mode.select_cells(&screen, anchor, cursor));
        assert_eq!(copy_mode.selected_text(&screen), None);
        assert!(!copy_mode.select_cells(&screen, anchor, anchor));
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
    fn copy_cursor_and_selection_stay_on_whole_wide_characters() {
        let mut terminal = TerminalState::new(TerminalSize::new(10, 2));
        terminal.apply_terminal_output("a日b\r\n終\x1b[1;3H".as_bytes());
        let screen = terminal.snapshot();
        let mut mode = CopyMode::default();
        mode.enter(&screen);
        assert_eq!(mode.snapshot.cursor, CopyModePosition::new(0, 1));

        mode.move_cursor(&screen, CopyModeMove::Right);
        assert_eq!(mode.snapshot.cursor, CopyModePosition::new(0, 3));
        mode.move_cursor(&screen, CopyModeMove::Left);
        mode.toggle_cell_selection(&screen);
        assert_eq!(mode.selected_text(&screen).as_deref(), Some("日"));

        mode.move_cursor(&screen, CopyModeMove::Down);
        mode.move_cursor(&screen, CopyModeMove::EndOfLine);
        assert_eq!(mode.snapshot.cursor, CopyModePosition::new(1, 0));
        assert_eq!(mode.selected_text(&screen).as_deref(), Some("日b\n終"));
    }

    #[test]
    fn word_motions_treat_full_width_letters_and_spaces_as_whole_characters() {
        let mut terminal = TerminalState::new(TerminalSize::new(15, 2));
        terminal.apply_terminal_output("a日本　b\r".as_bytes());
        let screen = terminal.snapshot();
        let mut mode = CopyMode::default();
        mode.enter(&screen);

        mode.move_cursor(&screen, CopyModeMove::WordEnd);
        assert_eq!(mode.snapshot.cursor, CopyModePosition::new(0, 3));
        mode.move_cursor(&screen, CopyModeMove::WordEnd);
        assert_eq!(mode.snapshot.cursor, CopyModePosition::new(0, 7));
        mode.move_cursor(&screen, CopyModeMove::PreviousWord);
        assert_eq!(mode.snapshot.cursor, CopyModePosition::new(0, 0));
        mode.move_cursor(&screen, CopyModeMove::NextWord);
        assert_eq!(mode.snapshot.cursor, CopyModePosition::new(0, 7));
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

        copy_mode.toggle_cell_selection(&screen);
        assert_eq!(
            copy_mode.snapshot.selection.map(|selection| selection.kind),
            Some(CopyModeSelectionKind::Cell)
        );

        copy_mode.toggle_line_selection(&screen);
        assert_eq!(
            copy_mode.snapshot.selection.map(|selection| selection.kind),
            Some(CopyModeSelectionKind::Line)
        );

        copy_mode.toggle_line_selection(&screen);
        assert!(copy_mode.snapshot.selection.is_none());
    }
}
