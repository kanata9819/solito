use glyphon::Attrs;

use crate::pipeline::rect::RectSpec;
use crate::terminal_view::TerminalView;
use crate::util::color::ThemeColor;

struct TabStyle;
impl TabStyle {
    pub const TAB_ACTIVE_TEXT_COLOR: [u8; 4] = ThemeColor::WHITE;
    pub const TAB_INACTIVE_TEXT_COLOR: [u8; 4] = ThemeColor::BLUE_GRAY_400;
    pub const TAB_SEPARATOR_COLOR: [u8; 4] = ThemeColor::BLUE_GRAY_700;

    pub const TAB_ACTIVE_BACKGROUND: [f32; 4] = ThemeColor::NAVY_800_ALPHA;
    pub const TAB_INACTIVE_BACKGROUND: [f32; 4] = ThemeColor::NAVY_900_ALPHA;
    pub const TAB_ACTIVE_UNDERLINE: [f32; 4] = ThemeColor::CYAN_400;
    pub const TAB_ACTIVE_TOP_GLOW: [f32; 4] = ThemeColor::CYAN_GLOW;

    pub const TAB_TEXT_PADDING: usize = 2;
    pub const TAB_GAP_CHARS: usize = 1;
    pub const TAB_SLANT: f32 = 10.0;
    pub const TAB_TOP_GLOW_HEIGHT: f32 = 1.0;
    pub const TAB_UNDERLINE_HEIGHT: f32 = 3.0;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TabBarSnapshot {
    titles: Vec<String>,
    active_index: usize,
}

impl TabBarSnapshot {
    pub fn new(titles: Vec<String>, active_index: usize) -> Self {
        let active_index = active_index.min(titles.len().saturating_sub(1));

        Self {
            titles,
            active_index,
        }
    }

    pub(crate) fn titles(&self) -> &[String] {
        &self.titles
    }

    pub(crate) fn active_index(&self) -> usize {
        self.active_index
    }
}

impl TerminalView {
    pub(crate) fn set_tab_bar(&mut self, tab_bar: TabBarSnapshot) {
        self.tab_bar = tab_bar;
        self.mark_text_buffer_dirty();
    }

    pub(crate) fn tab_bar_rects(&self) -> Vec<RectSpec> {
        Self::tab_bar_rects_for(
            &self.tab_bar,
            self.glyphs.cell_width,
            self.config.line_height,
        )
    }

    pub(super) fn tab_bar_spans_for<'a>(
        tab_bar: &TabBarSnapshot,
        font_family: &'a str,
        cell_width: f32,
    ) -> Vec<(String, Attrs<'a>)> {
        let mut spans = Vec::new();

        if tab_bar.titles().len() <= 1 {
            return spans;
        }

        for (index, title) in tab_bar.titles().iter().enumerate() {
            if index > 0 {
                spans.push((
                    " ".repeat(Self::tab_text_gap_chars(cell_width)),
                    Self::text_attrs(Some(TabStyle::TAB_SEPARATOR_COLOR), font_family),
                ));
            }

            let active = index == tab_bar.active_index();
            let text = Self::padded_tab_title(title);

            let color: [u8; 4] = if active {
                TabStyle::TAB_ACTIVE_TEXT_COLOR
            } else {
                TabStyle::TAB_INACTIVE_TEXT_COLOR
            };

            spans.push((text, Self::text_attrs(Some(color), font_family)));
        }

        spans
    }

    pub(super) fn has_tab_bar(&self) -> bool {
        self.tab_bar.titles().len() > 1
    }

    fn tab_strip_rect(
        tab_x: f32,
        tab_y: f32,
        tab_height: f32,
        strip_y: f32,
        strip_height: f32,
        width: f32,
        color: [f32; 4],
    ) -> RectSpec {
        let bottom_slant = TabStyle::TAB_SLANT * (1.0 - (strip_y + strip_height) / tab_height);
        let strip_slant = TabStyle::TAB_SLANT * strip_height / tab_height;

        RectSpec::slanted(
            tab_x + bottom_slant,
            tab_y + strip_y,
            width,
            strip_height,
            color,
            strip_slant,
        )
    }

    fn tab_bar_rects_for(
        tab_bar: &TabBarSnapshot,
        cell_width: f32,
        line_height: f32,
    ) -> Vec<RectSpec> {
        let mut rects = Vec::new();

        if tab_bar.titles().len() <= 1 {
            return rects;
        }

        let mut x = Self::PADDING_X;
        let tab_y = 6.0;
        let tab_height = line_height + 2.0;
        let tab_text_gap_width = Self::tab_text_gap_chars(cell_width) as f32 * cell_width;

        for (index, title) in tab_bar.titles().iter().enumerate() {
            if index > 0 {
                x += (tab_text_gap_width - TabStyle::TAB_SLANT).max(0.0);
            }

            let tab_width = Self::tab_title_width(title, cell_width);
            let slanted_width = tab_width + TabStyle::TAB_SLANT;
            let active = index == tab_bar.active_index();
            let background: [f32; 4] = if active {
                TabStyle::TAB_ACTIVE_BACKGROUND
            } else {
                TabStyle::TAB_INACTIVE_BACKGROUND
            };

            rects.push(RectSpec::slanted(
                x,
                tab_y,
                slanted_width,
                tab_height,
                background,
                TabStyle::TAB_SLANT,
            ));

            if active {
                rects.push(Self::tab_strip_rect(
                    x,
                    tab_y,
                    tab_height,
                    0.0,
                    TabStyle::TAB_TOP_GLOW_HEIGHT,
                    slanted_width,
                    TabStyle::TAB_ACTIVE_TOP_GLOW,
                ));
                rects.push(Self::tab_strip_rect(
                    x,
                    tab_y,
                    tab_height,
                    tab_height - TabStyle::TAB_UNDERLINE_HEIGHT,
                    TabStyle::TAB_UNDERLINE_HEIGHT,
                    slanted_width,
                    TabStyle::TAB_ACTIVE_UNDERLINE,
                ));
            }

            x += slanted_width;
        }

        rects
    }

    fn tab_title_width(title: &str, cell_width: f32) -> f32 {
        Self::padded_tab_title(title).chars().count() as f32 * cell_width
    }

    fn padded_tab_title(title: &str) -> String {
        format!(
            "{}{}{}",
            " ".repeat(TabStyle::TAB_TEXT_PADDING),
            title,
            " ".repeat(TabStyle::TAB_TEXT_PADDING)
        )
    }

    fn tab_text_gap_chars(cell_width: f32) -> usize {
        let gap_width = cell_width * TabStyle::TAB_GAP_CHARS as f32 + TabStyle::TAB_SLANT * 2.0;

        (gap_width / cell_width).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use glyphon::Color;

    use crate::{
        RendererConfig,
        terminal_view::{TerminalView, tab_bar::TabStyle},
        util::color::ThemeColor,
    };

    use super::TabBarSnapshot;

    fn color([r, g, b, a]: [u8; 4]) -> Color {
        Color::rgba(r, g, b, a)
    }

    #[test]
    fn clamps_active_index_to_existing_tabs() {
        let snapshot = TabBarSnapshot::new(vec!["Tab 1".to_string()], 99);

        assert_eq!(snapshot.active_index(), 0);
    }

    #[test]
    fn tab_bar_text_gap_matches_next_tab_rect() {
        let snapshot = TabBarSnapshot::new(vec!["Tab 1".to_string(), "Tab 2".to_string()], 0);
        let cell_width = 10.0;
        let spans =
            TerminalView::tab_bar_spans_for(&snapshot, RendererConfig::DEFAULT_FONT_FAMILY, 10.0);
        let rects = TerminalView::tab_bar_rects_for(
            &snapshot,
            cell_width,
            RendererConfig::DEFAULT_LINE_HEIGHT,
        );
        let second_tab_text_x =
            TerminalView::PADDING_X + (spans[0].0.len() + spans[1].0.len()) as f32 * cell_width;

        assert_eq!(second_tab_text_x, rects[3].x);
    }

    #[test]
    fn tab_bar_rects_hide_for_single_tab() {
        let snapshot = TabBarSnapshot::new(vec!["Tab 1".to_string()], 0);

        assert!(
            TerminalView::tab_bar_rects_for(&snapshot, 10.0, RendererConfig::DEFAULT_LINE_HEIGHT)
                .is_empty()
        );
    }

    #[test]
    fn tab_bar_rects_include_background_tabs_and_active_accents() {
        let snapshot = TabBarSnapshot::new(vec!["Tab 1".to_string(), "Tab 2".to_string()], 0);
        let rects =
            TerminalView::tab_bar_rects_for(&snapshot, 10.0, RendererConfig::DEFAULT_LINE_HEIGHT);

        assert_eq!(rects.len(), 4);
        assert_eq!(rects[0].x, TerminalView::PADDING_X);
        assert_eq!(rects[0].width, 100.0);
        assert_eq!(rects[0].height, RendererConfig::DEFAULT_LINE_HEIGHT + 2.0);
        assert_eq!(rects[0].color, TabStyle::TAB_ACTIVE_BACKGROUND);
        assert_eq!(rects[0].slant, TabStyle::TAB_SLANT);
        assert_eq!(rects[2].height, TabStyle::TAB_UNDERLINE_HEIGHT);
        assert_eq!(rects[2].color, TabStyle::TAB_ACTIVE_UNDERLINE);
        assert_eq!(
            rects[2].slant,
            TabStyle::TAB_SLANT * TabStyle::TAB_UNDERLINE_HEIGHT / rects[0].height
        );
        assert_eq!(rects[3].x, 130.0);
        assert_eq!(rects[3].color, TabStyle::TAB_INACTIVE_BACKGROUND);
    }

    #[test]
    fn tab_bar_spans_mark_active_tab() {
        let snapshot = TabBarSnapshot::new(vec!["Tab 1".to_string(), "Tab 2".to_string()], 0);
        let spans =
            TerminalView::tab_bar_spans_for(&snapshot, RendererConfig::DEFAULT_FONT_FAMILY, 10.0);

        assert_eq!(spans[0].0, "  Tab 1  ");
        assert_eq!(spans[0].1.color_opt, Some(color(ThemeColor::WHITE)));
        assert_eq!(spans[1].0, "   ");
        assert_eq!(spans[2].0, "  Tab 2  ");
        assert_eq!(spans[2].1.color_opt, Some(color(ThemeColor::BLUE_GRAY_400)));
    }
}
