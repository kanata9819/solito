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
        self.parser.advance(&mut self.screen, bytes);
    }

    pub fn resize(&mut self, size: TerminalSize) {
        self.screen.resize(size);
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
