use std::ops::Range;

use solito_terminal::{ScreenCell, ScreenLine};
use unicode_width::UnicodeWidthChar;

use crate::{
    pipeline::rect::RectSpec,
    util::color::{ThemeColor, srgb_to_linear_rgba},
};

use super::TerminalView;

#[derive(Default, PartialEq, Eq)]
pub(super) struct ImePreedit {
    text: String,
    cursor_range: Option<(usize, usize)>,
}

pub(super) struct PreeditLayout {
    pub(super) col: usize,
    text_range: Range<usize>,
    width: usize,
    cursor: Option<Range<usize>>,
}

impl ImePreedit {
    pub(super) fn new(text: String, cursor_range: Option<(usize, usize)>) -> Self {
        // Winit reports UTF-8 byte offsets, not character or terminal-cell indices.
        let cursor_range = cursor_range.filter(|&(start, end)| text.get(start..end).is_some());
        Self { text, cursor_range }
    }

    pub(super) fn is_active(&self) -> bool {
        !self.text.is_empty()
    }

    pub(super) fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.text.chars()
    }

    fn char_width(ch: char) -> usize {
        UnicodeWidthChar::width(ch).unwrap_or(1).clamp(1, 2)
    }

    fn cell(ch: char, is_wide_continuation: bool) -> ScreenCell {
        let mut cell = ScreenCell::default();
        cell.ch = ch;
        cell.is_wide_continuation = is_wide_continuation;
        cell
    }

    fn columns(text: &str) -> usize {
        text.chars().map(Self::char_width).sum()
    }

    fn layout(&self, cursor_col: usize, cols: usize) -> PreeditLayout {
        let cols = cols.max(1);
        let width = Self::columns(&self.text);
        // Shift a short composition left at the edge; scroll long ones around the active clause.
        let col = cursor_col.min(cols.saturating_sub(width));
        let cursor = self.cursor_range.map(|(start, end)| {
            Self::columns(&self.text[..start])..Self::columns(&self.text[..end])
        });
        let scroll = if width > cols {
            cursor
                .as_ref()
                .map_or(0, |cursor| cursor.start.saturating_sub(cols / 2))
        } else {
            0
        };
        let mut skipped = 0;
        let mut start = 0;
        for (byte, ch) in self.text.char_indices() {
            if skipped >= scroll {
                start = byte;
                break;
            }
            skipped += Self::char_width(ch);
            start = byte + ch.len_utf8();
        }
        let mut visible_width = 0;
        let mut end = start;
        for ch in self.text[start..].chars() {
            let next_width = visible_width + Self::char_width(ch);
            if next_width > cols - col {
                break;
            }
            visible_width = next_width;
            end += ch.len_utf8();
        }
        PreeditLayout {
            col,
            text_range: start..end,
            width: visible_width,
            cursor: cursor.map(|cursor| {
                cursor.start.saturating_sub(skipped).min(visible_width)
                    ..cursor.end.saturating_sub(skipped).min(visible_width)
            }),
        }
    }

    pub(super) fn overlay_line(&self, original: &ScreenLine, layout: &PreeditLayout) -> ScreenLine {
        let blank = Self::cell(' ', false);
        let mut line = original.clone();
        let end = layout.col + layout.width;
        let len = line.len().max(end);
        line.resize(len, blank);
        if layout.col > 0
            && line
                .get(layout.col)
                .is_some_and(|cell| cell.is_wide_continuation)
        {
            line[layout.col - 1] = blank;
        }
        if line.get(end).is_some_and(|cell| cell.is_wide_continuation) {
            line[end] = blank;
        }
        let mut col = layout.col;
        for ch in self.text[layout.text_range.clone()].chars() {
            line[col] = Self::cell(ch, false);
            col += 1;
            if Self::char_width(ch) == 2 {
                line[col] = Self::cell(' ', true);
                col += 1;
            }
        }
        line
    }
}

impl TerminalView {
    pub(crate) fn set_ime_preedit(&mut self, text: String, cursor_range: Option<(usize, usize)>) {
        let preedit = ImePreedit::new(text, cursor_range);
        if self.ime_preedit != preedit {
            self.ime_preedit = preedit;
            self.text_damage.add_row(self.snapshot.cursor_row);
        }
    }

    pub(super) fn preedit_layout(&self) -> PreeditLayout {
        let width = self
            .glyphs
            .text_buffer
            .size()
            .0
            .unwrap_or(self.glyphs.cell_width);
        let cols = ((width / self.glyphs.cell_width) as usize).max(1);
        self.ime_preedit.layout(self.snapshot.cursor_col, cols)
    }

    pub(crate) fn ime_cursor_rect(&self) -> (f32, f32, f32, f32) {
        let (start, end) = self.viewport.visible_range(self.row_count());
        let visible_row = self
            .snapshot
            .cursor_row
            .clamp(start, end.saturating_sub(1).max(start))
            - start;
        let layout = self.preedit_layout();
        let col = if self.ime_preedit.is_active() {
            layout.col + layout.cursor.as_ref().map_or(0, |cursor| cursor.start)
        } else {
            self.snapshot.cursor_col
        };
        (
            Self::PADDING_X + col as f32 * self.glyphs.cell_width,
            Self::terminal_row_y(visible_row, self.config.line_height),
            self.glyphs.cell_width,
            self.config.line_height,
        )
    }

    pub(crate) fn ime_rects(&self) -> Vec<RectSpec> {
        let (start, end) = self.viewport.visible_range(self.row_count());
        if !self.ime_preedit.is_active() || !(start..end).contains(&self.snapshot.cursor_row) {
            return Vec::new();
        }
        let layout = self.preedit_layout();
        let x = Self::PADDING_X + layout.col as f32 * self.glyphs.cell_width;
        let y = Self::terminal_row_y(self.snapshot.cursor_row - start, self.config.line_height);
        let width = layout.width as f32 * self.glyphs.cell_width;
        let mut rects = vec![
            RectSpec::new(
                x,
                y,
                width,
                self.config.line_height,
                srgb_to_linear_rgba(ThemeColor::TAB_ACTIVE),
            ),
            RectSpec::new(
                x,
                y + self.config.line_height - 2.0,
                width,
                1.0,
                ThemeColor::WHITE_ALPHA,
            ),
        ];
        if let Some(cursor) = layout.cursor {
            let cursor_x = x + cursor.start as f32 * self.glyphs.cell_width;
            if cursor.is_empty() {
                rects.push(RectSpec::new(
                    cursor_x,
                    y,
                    1.0,
                    self.config.line_height,
                    srgb_to_linear_rgba(ThemeColor::ACCENT),
                ));
            } else {
                rects.push(RectSpec::new(
                    cursor_x,
                    y + self.config.line_height - 3.0,
                    cursor.len() as f32 * self.glyphs.cell_width,
                    2.0,
                    srgb_to_linear_rgba(ThemeColor::ACCENT),
                ));
            }
        }
        rects
    }
}

#[cfg(test)]
mod tests {
    use super::ImePreedit;
    use solito_terminal::{TerminalSize, TerminalState};

    #[test]
    fn japanese_byte_offsets_become_cell_offsets() {
        let preedit = ImePreedit::new("日本語abc".into(), Some((6, 10)));
        let layout = preedit.layout(5, 80);
        assert_eq!(layout.col, 5);
        assert_eq!(layout.width, 9);
        assert_eq!(layout.cursor, Some(4..7));
    }

    #[test]
    fn preedit_preserves_the_terminal_snapshot_and_uses_wide_cells() {
        let mut terminal = TerminalState::new(TerminalSize::new(20, 2));
        terminal.apply_terminal_output(b"prompt: old");
        let snapshot = terminal.snapshot();
        let preedit = ImePreedit::new("日本語".into(), Some((0, 9)));
        let line = preedit.overlay_line(&snapshot.lines[0], &preedit.layout(8, 20));
        assert_eq!(
            line.iter()
                .filter(|cell| !cell.is_wide_continuation)
                .map(|cell| cell.ch)
                .collect::<String>(),
            "prompt: 日本語"
        );
        assert!(line[9].is_wide_continuation);
        assert_eq!(
            snapshot.lines[0]
                .iter()
                .map(|cell| cell.ch)
                .collect::<String>(),
            "prompt: old"
        );
    }

    #[test]
    fn composition_at_the_right_edge_remains_visible() {
        let preedit = ImePreedit::new("日本語".into(), Some((6, 9)));
        let layout = preedit.layout(9, 10);
        assert_eq!(layout.col, 4);
        assert_eq!(layout.width, 6);
        assert_eq!(layout.cursor, Some(4..6));
    }

    #[test]
    fn long_composition_scrolls_to_the_active_clause() {
        let preedit = ImePreedit::new("あいうえおかきくけこ".into(), Some((24, 27)));
        let layout = preedit.layout(3, 6);
        assert_eq!(layout.col, 0);
        assert!(layout.width <= 6);
        assert!(preedit.text[layout.text_range.clone()].contains('け'));
        assert!(layout.cursor.unwrap().end <= layout.width);
    }

    #[test]
    fn invalid_byte_ranges_and_tiny_windows_do_not_split_characters() {
        let preedit = ImePreedit::new("あ".into(), Some((1, 2)));
        assert!(preedit.cursor_range.is_none());
        let layout = preedit.layout(0, 1);
        assert_eq!(layout.width, 0);
        assert!(layout.text_range.is_empty());
    }

    #[test]
    fn overlay_clears_wide_characters_crossing_its_edges() {
        let mut terminal = TerminalState::new(TerminalSize::new(20, 2));
        terminal.apply_terminal_output("あいう".as_bytes());
        let snapshot = terminal.snapshot();
        let preedit = ImePreedit::new("ab".into(), Some((2, 2)));
        let line = preedit.overlay_line(&snapshot.lines[0], &preedit.layout(1, 20));
        assert_eq!(
            line.iter()
                .filter(|cell| !cell.is_wide_continuation)
                .map(|cell| cell.ch)
                .collect::<String>(),
            " ab う"
        );
        assert_eq!(snapshot.lines[0][0].ch, 'あ');
        assert_eq!(snapshot.lines[0][2].ch, 'い');
    }
}
