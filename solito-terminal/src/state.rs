use crate::screen::Screen;
use crate::screen::buffer::ScreenSnapshot;
use vte::Parser;

pub struct TerminalState {
    parser: Parser,
    screen: Screen,
}

impl TerminalState {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            parser: Parser::new(),
            screen: Screen::new(cols, rows),
        }
    }

    pub fn apply_terminal_output(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.screen, bytes);
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.screen.resize(cols, rows);
    }

    pub fn snapshot(&self) -> ScreenSnapshot {
        self.screen.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalState;
    use crate::screen::buffer::{ScreenCell, ScreenSnapshot};

    fn line_text(line: &[ScreenCell]) -> String {
        line.iter()
            .filter(|cell| !cell.is_wide_continuation)
            .map(|cell| cell.ch)
            .collect()
    }

    #[test]
    fn applies_cursor_position_and_overwrite() {
        let mut state: TerminalState = TerminalState::new(10, 4);

        state.apply_terminal_output(b"abc\r\nxyz\x1b[1;2HQ");
        let snapshot: ScreenSnapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[0]), "aQc");
        assert_eq!(line_text(&snapshot.lines[1]), "xyz");
    }

    #[test]
    fn applies_clear_line_to_end() {
        let mut state: TerminalState = TerminalState::new(10, 4);

        state.apply_terminal_output(b"abcdef\x1b[1;3H\x1b[K");
        let snapshot: ScreenSnapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[0]), "ab");
    }

    #[test]
    fn wraps_wide_characters_without_splitting_cells() {
        let mut state: TerminalState = TerminalState::new(4, 4);

        state.apply_terminal_output("abcあz".as_bytes());
        let snapshot: ScreenSnapshot = state.snapshot();

        assert_eq!(line_text(&snapshot.lines[0]), "abc");
        assert_eq!(line_text(&snapshot.lines[1]), "あz");
    }

    #[test]
    fn exposes_cursor_position_in_snapshot() {
        let mut state: TerminalState = TerminalState::new(10, 4);

        state.apply_terminal_output(b"abc\r\nxy");
        let snapshot: ScreenSnapshot = state.snapshot();

        assert_eq!(snapshot.cursor_row, 1);
        assert_eq!(snapshot.cursor_col, 2);
    }

    #[test]
    fn applies_foreground_color_to_cells() {
        let mut state: TerminalState = TerminalState::new(10, 4);

        state.apply_terminal_output(b"a\x1b[31mR\x1b[39mW");
        let snapshot: ScreenSnapshot = state.snapshot();

        assert_eq!(snapshot.lines[0][0].foreground_rgba(), None);
        assert_eq!(
            snapshot.lines[0][1].foreground_rgba(),
            Some([197, 15, 31, 255])
        );
        assert_eq!(snapshot.lines[0][2].foreground_rgba(), None);
    }

    #[test]
    fn applies_cursor_color_from_osc() {
        let mut state: TerminalState = TerminalState::new(10, 4);

        state.apply_terminal_output(b"\x1b]12;#00ff80\x07");
        let snapshot: ScreenSnapshot = state.snapshot();

        assert_eq!(snapshot.cursor_color, Some([0, 255, 128, 255]));
    }

    #[test]
    fn resets_cursor_color_from_osc() {
        let mut state: TerminalState = TerminalState::new(10, 4);

        state.apply_terminal_output(b"\x1b]12;#00ff80\x07\x1b]112\x07");
        let snapshot: ScreenSnapshot = state.snapshot();

        assert_eq!(snapshot.cursor_color, None);
    }
}
