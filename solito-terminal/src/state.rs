use crate::TerminalSize;
use crate::screen::Screen;
use crate::screen::buffer::ScreenSnapshot;
use vte::Parser;

pub struct TerminalState {
    parser: Parser,
    screen: Screen,
}

impl TerminalState {
    pub fn new(size: TerminalSize) -> Self {
        Self {
            parser: Parser::new(),
            screen: Screen::new(size),
        }
    }

    pub fn apply_terminal_output(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(16 * 1024) {
            self.parser.advance(&mut self.screen, chunk);
            self.screen.limit_history();
        }
    }

    pub fn resize(&mut self, size: TerminalSize) {
        self.screen.resize(size);
        self.screen.limit_history();
    }

    pub fn snapshot(&self) -> ScreenSnapshot {
        self.screen.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalState;
    use crate::{TerminalSize, screen::buffer::ScreenCell};

    fn terminal(cols: usize, rows: usize) -> TerminalState {
        TerminalState::new(TerminalSize::new(cols, rows))
    }

    fn line_text(line: &[ScreenCell]) -> String {
        line.iter()
            .filter(|cell| !cell.is_wide_continuation)
            .map(|cell| cell.ch)
            .collect()
    }

    #[test]
    fn snapshots_share_unchanged_rows_and_keep_old_text() {
        let mut state = terminal(12, 3);
        state.apply_terminal_output(b"first\r\nsecond");
        let before = state.snapshot();
        let same = state.snapshot();
        assert!(before.lines[0].shares_storage_with(&same.lines[0]));
        state.apply_terminal_output(b"\x1b[2;1HX");
        let after = state.snapshot();
        assert!(before.lines[0].shares_storage_with(&after.lines[0]));
        assert!(!before.lines[1].shares_storage_with(&after.lines[1]));
        assert_eq!(line_text(&before.lines[1]), "second");
        assert_eq!(line_text(&after.lines[1]), "Xecond");
    }

    #[test]
    fn history_is_bounded_and_saved_cursor_tracks_retained_rows() {
        let mut state = terminal(12, 3);
        for _ in 0..10_002 {
            state.apply_terminal_output(b"row\r\n");
        }
        state.apply_terminal_output(b"\x1b7");
        let before = state.snapshot();
        state.apply_terminal_output(b"new\r\nnew\r\n\x1b8");
        let after = state.snapshot();
        assert_eq!(after.lines.len(), 10_003);
        assert_eq!(after.history_start, 2);
        assert_eq!(after.cursor_row, before.cursor_row - 2);
        assert_eq!(before.history_start, 0);
        assert!(before.lines[2].shares_storage_with(&after.lines[0]));
    }

    #[test]
    fn alternate_screen_does_not_accumulate_history() {
        let mut state = terminal(12, 3);
        state.apply_terminal_output(b"main\x1b[?1049h");
        for _ in 0..50 {
            state.apply_terminal_output(b"alt\r\n");
        }
        assert_eq!(state.snapshot().lines.len(), 3);
        state.apply_terminal_output(b"\x1b[?1049l");
        assert_eq!(line_text(&state.snapshot().lines[0]), "main");
    }

    #[test]
    fn cursor_movement_stays_inside_visible_screen() {
        let mut state = terminal(8, 3);
        state.apply_terminal_output(b"history\r\nA\r\nB\r\nC");
        state.apply_terminal_output(b"\x1b[999A");
        assert_eq!(state.snapshot().cursor_row, 1);
        state.apply_terminal_output(b"\x1b[999B\x1b[999C");
        assert_eq!(
            (state.snapshot().cursor_row, state.snapshot().cursor_col),
            (3, 7)
        );
        state.apply_terminal_output(b"\x1b[999;999HZ");
        let snapshot = state.snapshot();
        assert_eq!((snapshot.cursor_row, snapshot.cursor_col), (3, 7));
        assert_eq!(snapshot.lines[3][7].ch, 'Z');
    }

    #[test]
    fn erasing_display_preserves_cursor_and_drawing_style() {
        for background in [b"".as_slice(), b"\x1b[44m"] {
            let mut state = terminal(8, 4);
            state.apply_terminal_output(b"old\x1b[31m\x1b[3;4H");
            state.apply_terminal_output(background);
            state.apply_terminal_output(b"\x1b[2J");
            let snapshot = state.snapshot();
            assert_eq!((snapshot.cursor_row, snapshot.cursor_col), (2, 3));
            state.apply_terminal_output(b"X");
            assert_eq!(
                state.snapshot().lines[2][3].foreground_rgba(),
                Some([197, 15, 31, 255])
            );
        }
    }

    #[test]
    fn erasing_visible_display_preserves_scrollback() {
        let mut state = terminal(8, 2);
        state.apply_terminal_output(b"history\r\none\r\ntwo\x1b[2J");
        let snapshot = state.snapshot();
        assert_eq!(line_text(&snapshot.lines[0]), "history");
        assert!(
            snapshot.lines[1..]
                .iter()
                .all(|line| line.iter().all(|cell| cell.ch == ' '))
        );
    }

    #[test]
    fn line_erase_clears_both_halves_of_wide_character() {
        let mut state = terminal(8, 2);
        state.apply_terminal_output("AあB\x1b[1;3H\x1b[K".as_bytes());
        assert_eq!(line_text(&state.snapshot().lines[0]), "A");
    }

    #[test]
    fn tab_uses_configured_stops_and_resize_preserves_cleared_stops() {
        let mut state = terminal(20, 3);
        state.apply_terminal_output(b"\x1b[3g\t");
        assert_eq!(state.snapshot().cursor_col, 19);
        state.resize(TerminalSize::new(24, 3));
        state.apply_terminal_output(b"\r\t");
        assert_eq!(state.snapshot().cursor_col, 23);
    }

    #[test]
    fn line_feed_preserves_column_but_wrap_returns_to_start() {
        let mut state = terminal(4, 4);
        state.apply_terminal_output(b"ab\nZ");
        assert_eq!(line_text(&state.snapshot().lines[1]), "  Z");
        state.apply_terminal_output(b"XY");
        assert_eq!(line_text(&state.snapshot().lines[2]), "Y");
    }

    #[test]
    fn popup_background_does_not_color_untouched_gap() {
        let mut state = terminal(30, 4);
        // A short line is followed by a popup drawn farther to the right.
        state.apply_terminal_output(b"left\x1b[K\x1b[48;2;30;30;30m\x1b[1;20HX");
        let snapshot = state.snapshot();
        assert!(
            snapshot.lines[0][4..19]
                .iter()
                .all(|cell| cell.background_rgba().is_none())
        );
        assert_eq!(
            snapshot.lines[0][19].background_rgba(),
            Some([30, 30, 30, 255])
        );
        assert_eq!(line_text(&snapshot.lines[0]), "left               X");
    }

    #[test]
    fn background_colors_and_resets_reach_the_snapshot() {
        let mut state = terminal(20, 4);
        state.apply_terminal_output(
            b"\x1b[44mA\x1b[104mB\x1b[48;5;196mC\x1b[48;2;12;34;56mD\x1b[49mE\x1b[44mF\x1b[0mG",
        );
        let snapshot = state.snapshot();
        let colors: Vec<_> = snapshot.lines[0]
            .iter()
            .map(ScreenCell::background_rgba)
            .collect();
        assert_eq!(
            colors,
            vec![
                Some([0, 55, 218, 255]),
                Some([59, 120, 255, 255]),
                Some([255, 0, 0, 255]),
                Some([12, 34, 56, 255]),
                None,
                Some([0, 55, 218, 255]),
                None
            ]
        );
    }

    #[test]
    fn background_erasure_fills_empty_cells_without_moving_cursor() {
        for sequence in [b"\x1b[3X".as_slice(), b"\x1b[K", b"\x1b[J"] {
            let mut state = terminal(5, 3);
            state.apply_terminal_output(b"\x1b[2;3H\x1b[48;2;12;34;56m");
            state.apply_terminal_output(sequence);
            let snapshot = state.snapshot();
            assert_eq!((snapshot.cursor_row, snapshot.cursor_col), (1, 2));
            assert!(snapshot.lines[1][2..5].iter().all(|cell|
                cell.ch == ' ' && cell.background_rgba() == Some([12,34,56,255])));
        }
    }

    #[test]
    fn overwriting_selection_restores_default_background() {
        let mut state = terminal(5, 3);
        state.apply_terminal_output(b"\x1b[44mhello\r\x1b[49mhello");
        assert!(
            state.snapshot().lines[0]
                .iter()
                .all(|cell| cell.background_rgba().is_none())
        );
    }

    #[test]
    fn applies_cursor_position_and_overwrite() {
        let mut state = terminal(10, 4);

        state.apply_terminal_output(b"abc\r\nxyz\x1b[1;2HQ");
        let snapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[0]), "aQc");
        assert_eq!(line_text(&snapshot.lines[1]), "xyz");
    }

    #[test]
    fn applies_clear_line_to_end() {
        let mut state = terminal(10, 4);

        state.apply_terminal_output(b"abcdef\x1b[1;3H\x1b[K");
        let snapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[0]), "ab");
    }

    #[test]
    fn applies_cursor_next_line_and_resets_the_column() {
        let mut state = terminal(10, 4);

        state.apply_terminal_output(b"abc\x1b[2EZ");
        let snapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[0]), "abc");
        assert_eq!(line_text(&snapshot.lines[2]), "Z");
        assert_eq!((snapshot.cursor_row, snapshot.cursor_col), (2, 1));
    }

    #[test]
    fn applies_cursor_previous_line_and_resets_the_column() {
        let mut state = terminal(10, 4);

        state.apply_terminal_output(b"top\x1b[3Ebottom\x1b[2FZ");
        let snapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[1]), "Z");
        assert_eq!((snapshot.cursor_row, snapshot.cursor_col), (1, 1));
    }

    #[test]
    fn applies_extended_cursor_and_character_editing_commands() {
        let mut state = terminal(10, 4);

        state.apply_terminal_output(b"abcd\x1b[1;3H\x1b[2@Z\x1b[2aQ\x1b[2dR");
        let snapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[0]), "abZ cQ");
        assert_eq!(line_text(&snapshot.lines[1]), "      R");
        assert_eq!((snapshot.cursor_row, snapshot.cursor_col), (1, 7));
    }

    #[test]
    fn applies_line_editing_and_scrolling_commands() {
        let mut state = terminal(8, 4);

        state.apply_terminal_output(b"A\r\nB\r\nC\r\nD\x1b[2S");
        let snapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[0]), "C");
        assert_eq!(line_text(&snapshot.lines[1]), "D");
        assert!(snapshot.lines[2].is_empty());
        assert!(snapshot.lines[3].is_empty());
    }

    #[test]
    fn applies_tab_repeat_and_modes() {
        let mut state = terminal(8, 4);

        state.apply_terminal_output(b"A\x1b[3b\x1b[4h\x1b[1;2HX\x1b[4l\x1b[3g\tZ\x1b[?25l");
        let snapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[0]), "AXAAA  Z");
        assert!(!snapshot.cursor_visible);
    }

    #[test]
    fn restores_the_primary_screen_after_alternate_screen_mode() {
        let mut state = terminal(10, 4);

        state.apply_terminal_output(b"main\x1b[?1049halt");
        assert_eq!(line_text(&state.snapshot().lines[0]), "alt");

        state.apply_terminal_output(b"\x1b[?1049l");
        let snapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[0]), "main");
        assert_eq!((snapshot.cursor_row, snapshot.cursor_col), (0, 4));
    }

    #[test]
    fn applies_escape_cursor_and_line_controls() {
        let mut state = terminal(8, 4);

        state.apply_terminal_output(b"ab\x1b7\x1bD!\x1b8Z\x1bEX");
        let snapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[0]), "abZ");
        assert_eq!(line_text(&snapshot.lines[1]), "X !");
        assert_eq!((snapshot.cursor_row, snapshot.cursor_col), (1, 1));
    }

    #[test]
    fn applies_reverse_index_and_terminal_reset() {
        let mut state = terminal(8, 4);

        state.apply_terminal_output(b"A\r\nB\x1bMZ");
        assert_eq!(line_text(&state.snapshot().lines[0]), "AZ");

        state.apply_terminal_output(b"\x1bc");
        let snapshot = state.snapshot();
        assert_eq!(snapshot.lines.len(), 1);
        assert!(snapshot.lines[0].is_empty());
        assert_eq!((snapshot.cursor_row, snapshot.cursor_col), (0, 0));
    }

    #[test]
    fn erases_characters_without_moving_or_shifting() {
        let mut state = terminal(10, 4);

        state.apply_terminal_output(b"abcdefgh\x1b[1;3H\x1b[3X");
        let snapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[0]), "ab   fgh");
        assert_eq!(snapshot.cursor_col, 2);
    }

    #[test]
    fn erase_characters_empty_or_zero_defaults_to_one() {
        for sequence in [
            b"abcdef\x1b[1;3H\x1b[X".as_slice(),
            b"abcdef\x1b[1;3H\x1b[0X".as_slice(),
        ] {
            let mut state = terminal(10, 4);

            state.apply_terminal_output(sequence);
            let snapshot = state.snapshot();

            assert_eq!(line_text(&snapshot.lines[0]), "ab def");
            assert_eq!(snapshot.cursor_col, 2);
        }
    }

    #[test]
    fn erase_characters_clears_both_cells_of_a_wide_character() {
        for cursor_col in [2, 3] {
            let mut state = terminal(10, 4);
            let sequence = format!("AあB\x1b[1;{cursor_col}H\x1b[X");

            state.apply_terminal_output(sequence.as_bytes());
            let snapshot = state.snapshot();

            assert_eq!(line_text(&snapshot.lines[0]), "A  B");
            assert!(
                snapshot.lines[0]
                    .iter()
                    .all(|cell| !cell.is_wide_continuation)
            );
        }
    }

    #[test]
    fn wraps_wide_characters_without_splitting_cells() {
        let mut state = terminal(4, 4);

        state.apply_terminal_output("abcあz".as_bytes());
        let snapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[0]), "abc");
        assert_eq!(line_text(&snapshot.lines[1]), "あz");
    }

    #[test]
    fn exposes_cursor_position_in_snapshot() {
        let mut state = terminal(10, 4);

        state.apply_terminal_output(b"abc\r\nxy");
        let snapshot = state.snapshot();

        assert_eq!(snapshot.cursor_row, 1);
        assert_eq!(snapshot.cursor_col, 2);
    }

    #[test]
    fn applies_foreground_color_to_cells() {
        let mut state = terminal(10, 4);

        state.apply_terminal_output(b"a\x1b[31mR\x1b[39mW");
        let snapshot = state.snapshot();

        assert_eq!(snapshot.lines[0][0].foreground_rgba(), None);
        assert_eq!(
            snapshot.lines[0][1].foreground_rgba(),
            Some([197, 15, 31, 255])
        );
        assert_eq!(snapshot.lines[0][2].foreground_rgba(), None);
    }

    #[test]
    fn applies_cursor_color_from_osc() {
        let mut state = terminal(10, 4);

        state.apply_terminal_output(b"\x1b]12;#00ff80\x07");
        let snapshot = state.snapshot();

        assert_eq!(snapshot.cursor_color, Some([0, 255, 128, 255]));
    }

    #[test]
    fn resets_cursor_color_from_osc() {
        let mut state = terminal(10, 4);

        state.apply_terminal_output(b"\x1b]12;#00ff80\x07\x1b]112\x07");
        let snapshot = state.snapshot();

        assert_eq!(snapshot.cursor_color, None);
    }
}
