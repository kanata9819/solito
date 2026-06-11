use glyphon::Attrs;

use crate::pipeline::rect::RectSpec;
use crate::terminal_view::TerminalView;
use crate::terminal_view::glyph::GlyphonResources;
use crate::util::color::ThemeColor;

pub struct TabView {}
impl TabView {
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
        let active_index: usize = active_index.min(titles.len().saturating_sub(1));

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
    pub(crate) fn tab_bar_rects(&mut self, width: u32) -> Vec<RectSpec> {
        let cell_width: f32 =
            GlyphonResources::measure_font_width(&mut self.glyphs.font_system, &self.config)
                .max(1.0);

        Self::tab_bar_rects_for(&self.tab_bar, width, cell_width, self.config.line_height)
    }

    pub(super) fn tab_bar_height_for(line_height: f32) -> f32 {
        line_height
    }

    pub(super) fn tab_bar_spans_for<'a>(
        tab_bar: &TabBarSnapshot,
        font_family: &'a str,
        cell_width: f32,
    ) -> Vec<(String, Attrs<'a>)> {
        let mut spans: Vec<(String, Attrs<'a>)> = Vec::new();

        if tab_bar.titles().len() <= 1 {
            return spans;
        }

        for (index, title) in tab_bar.titles().iter().enumerate() {
            if index > 0 {
                spans.push((
                    " ".repeat(Self::tab_text_gap_chars(cell_width)),
                    Self::text_attrs(Some(TabView::TAB_SEPARATOR_COLOR), font_family),
                ));
            }

            let active: bool = index == tab_bar.active_index();
            let text: String = Self::padded_tab_title(title);

            let color: [u8; 4] = if active {
                TabView::TAB_ACTIVE_TEXT_COLOR
            } else {
                TabView::TAB_INACTIVE_TEXT_COLOR
            };

            spans.push((text, Self::text_attrs(Some(color), font_family)));
        }

        spans
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
        let bottom_slant: f32 = TabView::TAB_SLANT * (1.0 - (strip_y + strip_height) / tab_height);
        let strip_slant: f32 = TabView::TAB_SLANT * strip_height / tab_height;

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
        _width: u32,
        cell_width: f32,
        line_height: f32,
    ) -> Vec<RectSpec> {
        let mut rects: Vec<RectSpec> = Vec::new();

        if tab_bar.titles().len() <= 1 {
            return rects;
        }

        let mut x: f32 = Self::PADDING_X;
        let tab_y: f32 = 6.0;
        let tab_height: f32 = Self::tab_bar_height_for(line_height) + 2.0;
        let tab_text_gap_width: f32 = Self::tab_text_gap_chars(cell_width) as f32 * cell_width;

        for (index, title) in tab_bar.titles().iter().enumerate() {
            if index > 0 {
                x += (tab_text_gap_width - TabView::TAB_SLANT).max(0.0);
            }

            let tab_width: f32 = Self::tab_title_width(title, cell_width);
            let slanted_width: f32 = tab_width + TabView::TAB_SLANT;
            let active: bool = index == tab_bar.active_index();
            let background: [f32; 4] = if active {
                TabView::TAB_ACTIVE_BACKGROUND
            } else {
                TabView::TAB_INACTIVE_BACKGROUND
            };

            rects.push(RectSpec::slanted(
                x,
                tab_y,
                slanted_width,
                tab_height,
                background,
                TabView::TAB_SLANT,
            ));

            if active {
                rects.push(Self::tab_strip_rect(
                    x,
                    tab_y,
                    tab_height,
                    0.0,
                    TabView::TAB_TOP_GLOW_HEIGHT,
                    slanted_width,
                    TabView::TAB_ACTIVE_TOP_GLOW,
                ));
                rects.push(Self::tab_strip_rect(
                    x,
                    tab_y,
                    tab_height,
                    tab_height - TabView::TAB_UNDERLINE_HEIGHT,
                    TabView::TAB_UNDERLINE_HEIGHT,
                    slanted_width,
                    TabView::TAB_ACTIVE_UNDERLINE,
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
            " ".repeat(TabView::TAB_TEXT_PADDING),
            title,
            " ".repeat(TabView::TAB_TEXT_PADDING)
        )
    }

    fn tab_text_gap_chars(cell_width: f32) -> usize {
        let gap_width: f32 = cell_width * TabView::TAB_GAP_CHARS as f32 + TabView::TAB_SLANT * 2.0;

        (gap_width / cell_width).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        RendererConfig,
        terminal_view::{TerminalView, tab_bar::TabView},
    };

    use super::TabBarSnapshot;

    #[test]
    fn clamps_active_index_to_existing_tabs() {
        let snapshot: TabBarSnapshot = TabBarSnapshot::new(vec!["Tab 1".to_string()], 99);

        assert_eq!(snapshot.active_index(), 0);
    }

    #[test]
    fn tab_bar_text_gap_matches_next_tab_rect() {
        let snapshot: TabBarSnapshot =
            TabBarSnapshot::new(vec!["Tab 1".to_string(), "Tab 2".to_string()], 0);
        let cell_width = 10.0;
        let spans =
            TerminalView::tab_bar_spans_for(&snapshot, RendererConfig::DEFAULT_FONT_FAMILY, 10.0);
        let rects = TerminalView::tab_bar_rects_for(
            &snapshot,
            220,
            cell_width,
            RendererConfig::DEFAULT_LINE_HEIGHT,
        );
        let second_tab_text_x: f32 =
            TerminalView::PADDING_X + (spans[0].0.len() + spans[1].0.len()) as f32 * cell_width;

        assert_eq!(second_tab_text_x, rects[3].x);
    }

    #[test]
    fn tab_bar_rects_hide_for_single_tab() {
        let snapshot: TabBarSnapshot = TabBarSnapshot::new(vec!["Tab 1".to_string()], 0);

        assert!(
            TerminalView::tab_bar_rects_for(
                &snapshot,
                220,
                10.0,
                RendererConfig::DEFAULT_LINE_HEIGHT,
            )
            .is_empty()
        );
    }

    #[test]
    fn tab_bar_rects_include_background_tabs_and_active_accents() {
        let snapshot: TabBarSnapshot =
            TabBarSnapshot::new(vec!["Tab 1".to_string(), "Tab 2".to_string()], 0);
        let rects = TerminalView::tab_bar_rects_for(
            &snapshot,
            220,
            10.0,
            RendererConfig::DEFAULT_LINE_HEIGHT,
        );

        assert_eq!(rects.len(), 4);
        assert_eq!(rects[0].x, TerminalView::PADDING_X);
        assert_eq!(rects[0].width, 100.0);
        assert_eq!(
            rects[0].height,
            TerminalView::tab_bar_height_for(RendererConfig::DEFAULT_LINE_HEIGHT) + 2.0
        );
        assert_eq!(rects[0].color, TabView::TAB_ACTIVE_BACKGROUND);
        assert_eq!(rects[0].slant, TabView::TAB_SLANT);
        assert_eq!(rects[2].height, TabView::TAB_UNDERLINE_HEIGHT);
        assert_eq!(rects[2].color, TabView::TAB_ACTIVE_UNDERLINE);
        assert_eq!(
            rects[2].slant,
            TabView::TAB_SLANT * TabView::TAB_UNDERLINE_HEIGHT / rects[0].height
        );
        assert_eq!(rects[3].x, 130.0);
        assert_eq!(rects[3].color, TabView::TAB_INACTIVE_BACKGROUND);
    }
}
