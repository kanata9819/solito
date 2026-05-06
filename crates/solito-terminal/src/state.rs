use crate::screen::buffer::ScreenSnapshot;
use crate::screen::core::Screen;
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

    pub fn set_width(&mut self, cols: usize) {
        self.screen.set_cols(cols.max(1));
    }

    pub fn set_height(&mut self, rows: usize) {
        self.screen.set_rows(rows.max(1));
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
}
