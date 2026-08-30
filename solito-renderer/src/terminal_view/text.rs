use glyphon::{Attrs, AttrsList, BufferLine, Color, Family, Shaping};
use solito_terminal::ScreenCell;
use std::collections::{BTreeSet, HashMap};

use crate::{RendererConfig, terminal_view::glyph::GlyphonResources, util::color::ThemeColor};

use super::{TerminalView, text_damage::TextDamage};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct GridTextStyle {
    color: Option<[u8; 4]>,
    letter_spacing: f32,
}

struct GridMetrics<'a> {
    cell_width: f32,
    font_size: f32,
    glyph_widths: &'a HashMap<char, f32>,
}

impl TerminalView {
    pub(super) fn set_text_buffer_size(
        glyphs: &mut GlyphonResources,
        width: u32,
        height: u32,
        config: &RendererConfig,
    ) {
        glyphs.text_buffer.set_size(
            Some(Self::terminal_content_width(width) as f32),
            Some(Self::terminal_content_height(height, config.line_height) as f32),
        );
        glyphs
            .text_buffer
            .shape_until_scroll(&mut glyphs.font_system, false);
    }

    pub(super) fn terminal_content_height(height: u32, line_height: f32) -> u32 {
        height.saturating_sub(line_height.ceil() as u32)
    }

    pub(super) fn terminal_content_width(width: u32) -> u32 {
        let horizontal_padding = (Self::PADDING_X * 2.0).ceil() as u32;

        width.saturating_sub(horizontal_padding).max(1)
    }

    pub(super) fn invalidate_all_text(&mut self) {
        self.text_damage.mark_all();
    }

    pub(crate) fn update_text_buffer(&mut self) {
        match std::mem::take(&mut self.text_damage) {
            TextDamage::None => {}
            TextDamage::Rows(rows) => self.update_text_rows(&rows),
            TextDamage::All => self.set_text_to_buffer(),
        }
    }

    fn set_text_to_buffer(&mut self) {
        let font_family = self.config.font_family.clone();
        let spans = self.visible_text_spans(font_family.as_str());
        let attrs = Self::text_attrs(None, font_family.as_str());

        self.glyphs.text_buffer.set_rich_text(
            spans
                .iter()
                .map(|(text, attrs)| (text.as_str(), attrs.clone())),
            &attrs,
            Shaping::Advanced,
            None,
        );
        // cosmic-text 0.19 no longer shapes implicitly when text is replaced.
        // Glyphon can only prepare glyphs after the buffer has layout runs.
        self.glyphs
            .text_buffer
            .shape_until_scroll(&mut self.glyphs.font_system, false);
    }

    pub(super) fn text_attrs(color: Option<[u8; 4]>, font_family: &str) -> Attrs {
        let attrs = Attrs::new().family(Family::Name(font_family));

        match color {
            Some([r, g, b, a]) => attrs.color(Color::rgba(r, g, b, a)),
            None => attrs,
        }
    }

    fn update_text_rows(&mut self, rows: &BTreeSet<usize>) {
        if self.snapshot.lines.is_empty() {
            self.set_text_to_buffer();
            return;
        }

        let row_count = self.row_count();
        self.viewport.clamp(row_count);
        let (start, end) = self.viewport.visible_range(row_count);
        let visible_rows = rows.range(start..end).copied().collect::<Vec<_>>();

        if visible_rows.is_empty() {
            return;
        }

        self.ensure_glyph_widths(start, end);

        let font_family = self.config.font_family.clone();
        let default_attrs = Self::text_attrs(None, font_family.as_str());
        let cursor_text_color = Self::cursor_text_color(self.caret_color());
        let (cursor_row, cursor_col) = if self.snapshot.cursor_visible {
            (self.snapshot.cursor_row, self.snapshot.cursor_col)
        } else {
            (usize::MAX, usize::MAX)
        };
        let tab_bar_offset = usize::from(self.has_tab_bar());
        let grid = GridMetrics {
            cell_width: self.glyphs.cell_width,
            font_size: self.config.font_size,
            glyph_widths: &self.glyphs.glyph_widths,
        };
        let mut updated = false;

        for absolute_row in visible_rows {
            let buffer_row = tab_bar_offset + absolute_row - start;
            let Some(buffer_line) = self.glyphs.text_buffer.lines.get_mut(buffer_row) else {
                self.set_text_to_buffer();
                return;
            };
            let spans = Self::text_spans_for_lines(
                std::slice::from_ref(&self.snapshot.lines[absolute_row]),
                absolute_row,
                cursor_row,
                cursor_col,
                cursor_text_color,
                font_family.as_str(),
                &grid,
            );

            updated |= Self::set_buffer_line(buffer_line, spans, &default_attrs);
        }

        if updated {
            self.glyphs
                .text_buffer
                .shape_until_scroll(&mut self.glyphs.font_system, false);
        }
    }

    fn set_buffer_line<'a>(
        buffer_line: &mut BufferLine,
        spans: Vec<(String, Attrs<'a>)>,
        default_attrs: &Attrs<'a>,
    ) -> bool {
        let mut text = String::new();
        let mut attrs_list = AttrsList::new(default_attrs);

        for (span, attrs) in spans {
            let start = text.len();
            text.push_str(span.as_str());
            let end = text.len();
            if attrs != *default_attrs {
                attrs_list.add_span(start..end, &attrs);
            }
        }

        buffer_line.set_text(text, buffer_line.ending(), attrs_list)
    }

    fn visible_text_spans<'a>(&mut self, font_family: &'a str) -> Vec<(String, Attrs<'a>)> {
        let cell_width = self.glyphs.cell_width;
        let mut spans = Self::tab_bar_spans_for(&self.tab_bar, font_family, cell_width);
        let has_tab_bar = !spans.is_empty();

        if self.snapshot.lines.is_empty() {
            return spans;
        }

        let row_count = self.row_count();
        self.viewport.clamp(row_count);
        let (start, end) = self.viewport.visible_range(row_count);

        if has_tab_bar {
            spans.push(("\n".to_string(), Self::text_attrs(None, font_family)));
        }

        self.ensure_glyph_widths(start, end);
        let (cursor_row, cursor_col) = if self.snapshot.cursor_visible {
            (self.snapshot.cursor_row, self.snapshot.cursor_col)
        } else {
            (usize::MAX, usize::MAX)
        };
        spans.extend(Self::text_spans_for_lines(
            &self.snapshot.lines[start..end],
            start,
            cursor_row,
            cursor_col,
            Self::cursor_text_color(self.caret_color()),
            font_family,
            &GridMetrics {
                cell_width,
                font_size: self.config.font_size,
                glyph_widths: &self.glyphs.glyph_widths,
            },
        ));

        spans
    }

    fn ensure_glyph_widths(&mut self, start: usize, end: usize) {
        let chars = self.snapshot.lines[start..end]
            .iter()
            .flat_map(|line| line.iter().map(|cell| cell.ch))
            .collect::<Vec<_>>();

        for ch in chars {
            if self.glyphs.glyph_widths.contains_key(&ch) {
                continue;
            }

            let mut encoded: [u8; 4] = [0; 4];
            let text = ch.encode_utf8(&mut encoded);
            let width = GlyphonResources::measure_text_width(
                &mut self.glyphs.font_system,
                &self.config,
                text,
            );
            self.glyphs.glyph_widths.insert(ch, width);
        }
    }

    fn cursor_text_color(caret_color: [f32; 4]) -> [u8; 4] {
        const RED_LUMINANCE_WEIGHT: f32 = 0.2126;
        const GREEN_LUMINANCE_WEIGHT: f32 = 0.7152;
        const BLUE_LUMINANCE_WEIGHT: f32 = 0.0722;
        const LIGHT_BACKGROUND_THRESHOLD: f32 = 0.5;

        let luminance = RED_LUMINANCE_WEIGHT * caret_color[0]
            + GREEN_LUMINANCE_WEIGHT * caret_color[1]
            + BLUE_LUMINANCE_WEIGHT * caret_color[2];

        if luminance > LIGHT_BACKGROUND_THRESHOLD {
            ThemeColor::BLACK
        } else {
            ThemeColor::WHITE
        }
    }

    fn text_spans_for_lines<'a>(
        lines: &[Vec<ScreenCell>],
        first_row: usize,
        cursor_row: usize,
        cursor_col: usize,
        cursor_text_color: [u8; 4],
        font_family: &'a str,
        grid: &GridMetrics<'_>,
    ) -> Vec<(String, Attrs<'a>)> {
        let mut spans = Vec::new();
        let mut current_text = String::new();
        let mut current_style = GridTextStyle::default();

        for (line_index, line) in lines.iter().enumerate() {
            let absolute_row = first_row + line_index;
            for (cell_col, cell) in line.iter().enumerate() {
                let color: Option<[u8; 4]> = Self::cell_text_color(
                    absolute_row,
                    cell_col,
                    cursor_row,
                    cursor_col,
                    cursor_text_color,
                    cell,
                );
                let glyph_width = grid
                    .glyph_widths
                    .get(&cell.ch)
                    .copied()
                    .unwrap_or(grid.cell_width);
                let style = GridTextStyle {
                    color,
                    letter_spacing: Self::grid_letter_spacing(
                        grid.cell_width,
                        glyph_width,
                        grid.font_size,
                    ),
                };

                if current_text.is_empty() {
                    current_style = style;
                } else if current_style != style {
                    Self::push_text_span(&mut spans, &mut current_text, current_style, font_family);
                    current_style = style;
                }

                current_text.push(cell.ch);
            }

            if line_index + 1 < lines.len() {
                current_text.push('\n');
            }
        }

        Self::push_text_span(&mut spans, &mut current_text, current_style, font_family);

        spans
    }

    fn cell_text_color(
        absolute_row: usize,
        cell_col: usize,
        cursor_row: usize,
        cursor_col: usize,
        cursor_text_color: [u8; 4],
        cell: &ScreenCell,
    ) -> Option<[u8; 4]> {
        if absolute_row == cursor_row && cell_col == cursor_col {
            Some(cursor_text_color)
        } else {
            cell.foreground_rgba()
        }
    }

    fn push_text_span<'a>(
        spans: &mut Vec<(String, Attrs<'a>)>,
        text: &mut String,
        style: GridTextStyle,
        font_family: &'a str,
    ) {
        if !text.is_empty() {
            let attrs = Self::text_attrs(style.color, font_family);
            let attrs = if style.letter_spacing == 0.0 {
                attrs
            } else {
                attrs.letter_spacing(style.letter_spacing)
            };
            spans.push((std::mem::take(text), attrs));
        }
    }

    fn grid_letter_spacing(cell_width: f32, glyph_width: f32, font_size: f32) -> f32 {
        let correction = cell_width - glyph_width;
        if correction.abs() < 0.01 {
            0.0
        } else {
            correction / font_size
        }
    }
}

#[cfg(test)]
mod tests {
    use glyphon::{Buffer, Color, FontSystem, Metrics, Shaping, Wrap};
    use solito_terminal::ScreenCell;
    use std::collections::HashMap;

    use crate::{
        RendererConfig,
        terminal_view::{TerminalView, glyph::GlyphonResources},
        util::color::ThemeColor,
    };

    use super::GridMetrics;

    fn color([r, g, b, a]: [u8; 4]) -> Color {
        Color::rgba(r, g, b, a)
    }

    #[test]
    fn cursor_text_color_contrasts_with_caret_color() {
        assert_eq!(
            TerminalView::cursor_text_color([1.0, 1.0, 1.0, 1.0]),
            ThemeColor::BLACK
        );
        assert_eq!(
            TerminalView::cursor_text_color([0.0, 0.0, 0.0, 1.0]),
            ThemeColor::WHITE
        );
    }

    #[test]
    fn text_spans_override_cursor_cell_color() {
        let mut cell = ScreenCell::default();
        cell.ch = 'A';
        let glyph_widths = HashMap::from([('A', 10.0)]);

        let spans = TerminalView::text_spans_for_lines(
            &[vec![cell]],
            0,
            0,
            0,
            ThemeColor::BLACK,
            RendererConfig::DEFAULT_FONT_FAMILY,
            &GridMetrics {
                cell_width: 10.0,
                font_size: RendererConfig::DEFAULT_FONT_SIZE,
                glyph_widths: &glyph_widths,
            },
        );

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].0, "A");
        assert_eq!(spans[0].1.color_opt, Some(color(ThemeColor::BLACK)));
    }

    #[test]
    fn text_attrs_include_foreground_color() {
        let attrs =
            TerminalView::text_attrs(Some([1, 2, 3, 4]), RendererConfig::DEFAULT_FONT_FAMILY);

        assert_eq!(attrs.color_opt, Some(Color::rgba(1, 2, 3, 4)));
    }

    #[test]
    fn text_spans_preserve_wide_continuation_as_grid_cell() {
        let mut ascii_a = ScreenCell::default();
        ascii_a.ch = 'A';
        let mut wide = ScreenCell::default();
        wide.ch = 'あ';
        let mut continuation = ScreenCell::default();
        continuation.ch = ' ';
        continuation.is_wide_continuation = true;
        let mut ascii_b = ScreenCell::default();
        ascii_b.ch = 'B';
        let glyph_widths = HashMap::from([('A', 10.0), ('あ', 10.0), (' ', 10.0), ('B', 10.0)]);

        let spans = TerminalView::text_spans_for_lines(
            &[vec![ascii_a, wide, continuation, ascii_b]],
            0,
            usize::MAX,
            usize::MAX,
            ThemeColor::BLACK,
            RendererConfig::DEFAULT_FONT_FAMILY,
            &GridMetrics {
                cell_width: 10.0,
                font_size: RendererConfig::DEFAULT_FONT_SIZE,
                glyph_widths: &glyph_widths,
            },
        );
        let text = spans
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<String>();

        assert_eq!(text, "Aあ B");
    }

    #[test]
    fn grid_spacing_places_ascii_after_wide_character_at_expected_column() {
        let config = RendererConfig::default();
        let mut font_system = FontSystem::new();
        let cell_width = GlyphonResources::measure_font_width(&mut font_system, &config);
        let mut glyph_widths = HashMap::new();

        for ch in ['A', 'あ', ' ', 'B'] {
            let mut encoded: [u8; 4] = [0; 4];
            let text = ch.encode_utf8(&mut encoded);
            glyph_widths.insert(
                ch,
                GlyphonResources::measure_text_width(&mut font_system, &config, text),
            );
        }

        let mut ascii_a = ScreenCell::default();
        ascii_a.ch = 'A';
        let mut wide = ScreenCell::default();
        wide.ch = 'あ';
        let mut continuation = ScreenCell::default();
        continuation.ch = ' ';
        continuation.is_wide_continuation = true;
        let mut ascii_b = ScreenCell::default();
        ascii_b.ch = 'B';

        let spans = TerminalView::text_spans_for_lines(
            &[vec![ascii_a, wide, continuation, ascii_b]],
            0,
            usize::MAX,
            usize::MAX,
            ThemeColor::BLACK,
            config.font_family.as_str(),
            &GridMetrics {
                cell_width,
                font_size: config.font_size,
                glyph_widths: &glyph_widths,
            },
        );
        let text = spans
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<String>();
        let b_start = text.find('B').expect("B must be present");
        let attrs = TerminalView::text_attrs(None, config.font_family.as_str());
        let mut buffer = Buffer::new(
            &mut font_system,
            Metrics::new(config.font_size, config.line_height),
        );
        buffer.set_size(Some(1000.0), Some(100.0));
        buffer.set_wrap(Wrap::None);
        buffer.set_rich_text(
            spans
                .iter()
                .map(|(text, attrs)| (text.as_str(), attrs.clone())),
            &attrs,
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut font_system, false);

        let run = buffer.layout_runs().next().expect("layout run must exist");
        let b = run
            .glyphs
            .iter()
            .find(|glyph| glyph.start == b_start)
            .expect("B glyph must exist");
        let expected_x = 3.0 * cell_width;

        assert!(
            (b.x - expected_x).abs() < 0.05,
            "B x={} expected={expected_x} cell_width={cell_width}",
            b.x
        );
    }

    #[test]
    fn updating_one_buffer_line_preserves_other_line_layouts() {
        let config = RendererConfig::default();
        let mut font_system = FontSystem::new();
        let attrs = TerminalView::text_attrs(None, config.font_family.as_str());
        let mut buffer = Buffer::new(
            &mut font_system,
            Metrics::new(config.font_size, config.line_height),
        );
        buffer.set_size(Some(1000.0), Some(100.0));
        buffer.set_wrap(Wrap::None);
        buffer.set_rich_text(
            [("unchanged\nold", attrs.clone())],
            &attrs,
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut font_system, false);

        assert_eq!(buffer.lines.len(), 2);
        assert!(!buffer.lines[0].needs_reshaping());
        assert!(!buffer.lines[1].needs_reshaping());

        assert!(TerminalView::set_buffer_line(
            &mut buffer.lines[1],
            vec![("new".to_string(), attrs.clone())],
            &attrs,
        ));

        assert!(!buffer.lines[0].needs_reshaping());
        assert!(buffer.lines[1].needs_reshaping());

        buffer.shape_until_scroll(&mut font_system, false);

        assert_eq!(buffer.lines[0].text(), "unchanged");
        assert_eq!(buffer.lines[1].text(), "new");
        assert!(!buffer.lines[1].needs_reshaping());
    }
}
