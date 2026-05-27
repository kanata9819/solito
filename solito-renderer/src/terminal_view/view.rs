use ::glyphon::{Attrs, Color, Family, Shaping};
use solito_terminal::{ScreenCell, ScreenSnapshot};

use crate::pipeline::rect::RectSpec;
use crate::util;
use crate::{RendererConfig, terminal_view::tab_bar::TabView};

use super::{
    copy_mode::{CopyModePosition, CopyModeSelection, CopyModeSelectionKind, CopyModeSnapshot},
    glyph::GlyphonResources,
    resources::GlyphResources,
    tab_bar::TabBarSnapshot,
    viewport::ViewportState,
};

pub(crate) struct TerminalView {
    pub(crate) glyphs: GlyphResources,
    config: RendererConfig,
    snapshot: ScreenSnapshot,
    tab_bar: TabBarSnapshot,
    copy_mode: CopyModeSnapshot,
    viewport: ViewportState,
}

impl TerminalView {
    pub(crate) const PADDING_X: f32 = 10.0;
    pub(crate) const PADDING_Y: f32 = 10.0;
    pub(crate) const DEFAULT_CARET_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
    const COPY_MODE_SELECTION_COLOR: [f32; 4] = [0.2, 0.45, 0.95, 0.36];
    const COPY_MODE_CURSOR_COLOR: [f32; 4] = [0.95, 0.84, 0.25, 0.55];

    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        swapchain: wgpu::TextureFormat,
        physical_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
        config: RendererConfig,
    ) -> Self {
        let config: RendererConfig = config.sanitized();
        let glyphon: GlyphonResources = GlyphonResources::new(
            device,
            queue,
            swapchain,
            physical_size,
            scale_factor,
            &config,
        );
        let mut glyphs: GlyphResources = GlyphResources::new(glyphon);

        Self::set_text_buffer_size(
            &mut glyphs,
            physical_size.width,
            physical_size.height,
            &config,
        );

        Self {
            glyphs,
            viewport: ViewportState::new(
                Self::terminal_content_height(physical_size.height, config.line_height),
                config.line_height,
            ),
            config,
            snapshot: ScreenSnapshot::default(),
            tab_bar: TabBarSnapshot::default(),
            copy_mode: CopyModeSnapshot::default(),
        }
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32, snapshot: ScreenSnapshot) {
        self.snapshot = snapshot;
        Self::set_text_buffer_size(&mut self.glyphs, width, height, &self.config);
        self.viewport.resize(
            Self::terminal_content_height(height, self.config.line_height),
            self.row_count(),
        );
        self.set_text_to_buffer();
    }

    pub(crate) fn set_tab_bar(&mut self, tab_bar: TabBarSnapshot) {
        self.tab_bar = tab_bar;
        self.set_text_to_buffer();
    }

    pub(crate) fn set_copy_mode(&mut self, copy_mode: CopyModeSnapshot) {
        self.copy_mode = copy_mode;

        if self.copy_mode.active {
            let row_count: usize = self.row_count();
            self.viewport
                .scroll_to_include(self.copy_mode.cursor.row, row_count);
        }

        self.set_text_to_buffer();
    }

    pub(crate) fn set_snapshot(&mut self, snapshot: ScreenSnapshot) {
        self.viewport.reset();
        self.snapshot = snapshot;
        self.set_text_to_buffer();
    }

    pub(crate) fn scroll(&mut self, _x: f32, y: f32) {
        self.viewport.scroll(y, self.row_count());
        self.set_text_to_buffer();
    }

    pub(crate) fn caret_rect(&mut self) -> (f32, f32, f32, f32) {
        let row_count: usize = self.row_count();
        self.viewport.clamp(row_count);
        let (start, end): (usize, usize) = self.viewport.visible_range(row_count);

        if self.snapshot.cursor_row < start || self.snapshot.cursor_row >= end {
            return (Self::PADDING_X, self.terminal_origin_y(), 0.0, 0.0);
        }

        let cell_width: f32 =
            GlyphonResources::measure_font_width(&mut self.glyphs.font_system, &self.config)
                .max(1.0);

        let visible_row: usize = self.snapshot.cursor_row - start;

        let caret_x: f32 = Self::PADDING_X + self.snapshot.cursor_col as f32 * cell_width;
        let caret_y: f32 = if self.tab_bar.titles().len() <= 1 {
            Self::PADDING_Y + visible_row as f32 * self.config.line_height
        } else {
            Self::PADDING_Y + self.config.line_height + visible_row as f32 * self.config.line_height
        };

        (caret_x, caret_y, cell_width, self.config.line_height)
    }

    pub(crate) fn copy_mode_active(&self) -> bool {
        self.copy_mode.active
    }

    pub(crate) fn caret_color(&self) -> [f32; 4] {
        self.snapshot
            .cursor_color
            .map(util::color::rgba_to_f32)
            .unwrap_or(Self::DEFAULT_CARET_COLOR)
    }

    pub(crate) fn tab_bar_rects(&mut self, width: u32) -> Vec<RectSpec> {
        let cell_width: f32 =
            GlyphonResources::measure_font_width(&mut self.glyphs.font_system, &self.config)
                .max(1.0);

        Self::tab_bar_rects_for(&self.tab_bar, width, cell_width, self.config.line_height)
    }

    pub(crate) fn copy_mode_rects(&mut self) -> Vec<RectSpec> {
        if !self.copy_mode.active {
            return Vec::new();
        }

        let row_count: usize = self.row_count();
        self.viewport.clamp(row_count);
        let (visible_start, visible_end): (usize, usize) = self.viewport.visible_range(row_count);
        let cell_width: f32 =
            GlyphonResources::measure_font_width(&mut self.glyphs.font_system, &self.config)
                .max(1.0);

        Self::copy_mode_rects_for(
            &self.copy_mode,
            &self.snapshot.lines,
            visible_start,
            visible_end,
            cell_width,
            self.config.line_height,
            self.has_tab_bar(),
        )
    }

    fn set_text_to_buffer(&mut self) {
        let font_family: String = self.config.font_family.clone();
        let spans: Vec<(String, Attrs<'_>)> = self.visible_text_spans(font_family.as_str());
        let attrs: Attrs<'_> = Self::text_attrs(None, font_family.as_str());

        self.glyphs.text_buffer.set_rich_text(
            &mut self.glyphs.font_system,
            spans
                .iter()
                .map(|(text, attrs)| (text.as_str(), attrs.clone())),
            &attrs,
            Shaping::Advanced,
            None,
        );
    }

    fn visible_text_spans<'a>(&mut self, font_family: &'a str) -> Vec<(String, Attrs<'a>)> {
        let mut spans: Vec<(String, Attrs<'a>)> =
            Self::tab_bar_spans_for(&self.tab_bar, font_family);
        let has_tab_bar: bool = !spans.is_empty();

        if self.snapshot.lines.is_empty() {
            return spans;
        }

        let row_count: usize = self.row_count();
        self.viewport.clamp(row_count);
        let (start, end): (usize, usize) = self.viewport.visible_range(row_count);

        if has_tab_bar {
            spans.push(("\n".to_string(), Self::text_attrs(None, font_family)));
        }

        spans.extend(Self::text_spans_for_lines(
            &self.snapshot.lines[start..end],
            start,
            self.snapshot.cursor_row,
            self.snapshot.cursor_col,
            Self::cursor_text_color(self.caret_color()),
            font_family,
        ));

        spans
    }

    fn tab_bar_spans_for<'a>(
        tab_bar: &TabBarSnapshot,
        font_family: &'a str,
    ) -> Vec<(String, Attrs<'a>)> {
        let mut spans: Vec<(String, Attrs<'a>)> = Vec::new();

        if tab_bar.titles().len() <= 1 {
            return spans;
        }

        for (index, title) in tab_bar.titles().iter().enumerate() {
            if index > 0 {
                spans.push((
                    " ".repeat(TabView::TAB_GAP_CHARS),
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

        for (index, title) in tab_bar.titles().iter().enumerate() {
            if index > 0 {
                x += cell_width * TabView::TAB_GAP_CHARS as f32 + TabView::TAB_SLANT;
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

    fn copy_mode_rects_for(
        copy_mode: &CopyModeSnapshot,
        lines: &[Vec<ScreenCell>],
        visible_start: usize,
        visible_end: usize,
        cell_width: f32,
        line_height: f32,
        has_tab_bar: bool,
    ) -> Vec<RectSpec> {
        let mut rects: Vec<RectSpec> = Vec::new();

        if let Some(selection) = copy_mode.selection {
            for row in visible_start..visible_end {
                if let Some((start_col, end_col)) =
                    Self::selected_cols_for_row(selection, row, lines)
                    && start_col < end_col
                {
                    let visible_row: usize = row.saturating_sub(visible_start);
                    rects.push(RectSpec::new(
                        Self::PADDING_X + start_col as f32 * cell_width,
                        Self::terminal_row_y(visible_row, line_height, has_tab_bar),
                        (end_col - start_col) as f32 * cell_width,
                        line_height,
                        Self::COPY_MODE_SELECTION_COLOR,
                    ));
                }
            }
        }

        if copy_mode.cursor.row >= visible_start && copy_mode.cursor.row < visible_end {
            let visible_row: usize = copy_mode.cursor.row - visible_start;
            let cursor_col: usize = copy_mode
                .cursor
                .col
                .min(Self::display_col_count(lines, copy_mode.cursor.row) - 1);

            rects.push(RectSpec::new(
                Self::PADDING_X + cursor_col as f32 * cell_width,
                Self::terminal_row_y(visible_row, line_height, has_tab_bar),
                cell_width,
                line_height,
                Self::COPY_MODE_CURSOR_COLOR,
            ));
        }

        rects
    }

    fn selected_cols_for_row(
        selection: CopyModeSelection,
        row: usize,
        lines: &[Vec<ScreenCell>],
    ) -> Option<(usize, usize)> {
        match selection.kind {
            CopyModeSelectionKind::Line => {
                let (start_row, end_row): (usize, usize) =
                    Self::ordered_rows(selection.anchor, selection.cursor);

                if row < start_row || row > end_row {
                    return None;
                }

                Some((0, Self::display_col_count(lines, row)))
            }
            CopyModeSelectionKind::Cell => {
                let (start, end): (CopyModePosition, CopyModePosition) =
                    Self::ordered_positions(selection.anchor, selection.cursor);

                if row < start.row || row > end.row {
                    return None;
                }

                let col_count: usize = Self::display_col_count(lines, row);
                let start_col: usize = if row == start.row {
                    start.col.min(col_count - 1)
                } else {
                    0
                };
                let end_col: usize = if row == end.row {
                    end.col.min(col_count - 1) + 1
                } else {
                    col_count
                };

                Some((start_col, end_col))
            }
        }
    }

    fn ordered_rows(anchor: CopyModePosition, cursor: CopyModePosition) -> (usize, usize) {
        if anchor.row <= cursor.row {
            (anchor.row, cursor.row)
        } else {
            (cursor.row, anchor.row)
        }
    }

    fn ordered_positions(
        anchor: CopyModePosition,
        cursor: CopyModePosition,
    ) -> (CopyModePosition, CopyModePosition) {
        if (anchor.row, anchor.col) <= (cursor.row, cursor.col) {
            (anchor, cursor)
        } else {
            (cursor, anchor)
        }
    }

    fn display_col_count(lines: &[Vec<ScreenCell>], row: usize) -> usize {
        lines.get(row).map(|line| line.len()).unwrap_or(0).max(1)
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

    fn padded_tab_title(title: &str) -> String {
        format!(
            "{}{}{}",
            " ".repeat(TabView::TAB_TEXT_PADDING),
            title,
            " ".repeat(TabView::TAB_TEXT_PADDING)
        )
    }

    fn tab_title_width(title: &str, cell_width: f32) -> f32 {
        Self::padded_tab_title(title).chars().count() as f32 * cell_width
    }

    fn text_spans_for_lines<'a>(
        lines: &[Vec<ScreenCell>],
        first_row: usize,
        cursor_row: usize,
        cursor_col: usize,
        cursor_text_color: [u8; 4],
        font_family: &'a str,
    ) -> Vec<(String, Attrs<'a>)> {
        let mut spans: Vec<(String, Attrs<'a>)> = Vec::new();
        let mut current_text: String = String::new();
        let mut current_color: Option<[u8; 4]> = None;

        for (line_index, line) in lines.iter().enumerate() {
            let absolute_row: usize = first_row + line_index;
            for (cell_col, cell) in line.iter().enumerate() {
                if cell.is_wide_continuation {
                    continue;
                }

                let color: Option<[u8; 4]> = Self::cell_text_color(
                    absolute_row,
                    cell_col,
                    cursor_row,
                    cursor_col,
                    cursor_text_color,
                    cell,
                );

                if current_text.is_empty() {
                    current_color = color;
                } else if Self::should_start_new_span(&current_text, current_color, color) {
                    Self::push_text_span(&mut spans, &mut current_text, current_color, font_family);
                    current_color = color;
                }

                current_text.push(Self::cell_char(cell));
            }

            if line_index + 1 < lines.len() {
                current_text.push('\n');
            }
        }

        Self::push_text_span(&mut spans, &mut current_text, current_color, font_family);

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

    fn should_start_new_span(
        current_text: &str,
        current_color: Option<[u8; 4]>,
        next_color: Option<[u8; 4]>,
    ) -> bool {
        !current_text.is_empty() && current_color != next_color
    }

    fn push_text_span<'a>(
        spans: &mut Vec<(String, Attrs<'a>)>,
        text: &mut String,
        color: Option<[u8; 4]>,
        font_family: &'a str,
    ) {
        if !text.is_empty() {
            spans.push((std::mem::take(text), Self::text_attrs(color, font_family)));
        }
    }

    fn cursor_text_color(caret_color: [f32; 4]) -> [u8; 4] {
        const RED_LUMINANCE_WEIGHT: f32 = 0.2126;
        const GREEN_LUMINANCE_WEIGHT: f32 = 0.7152;
        const BLUE_LUMINANCE_WEIGHT: f32 = 0.0722;
        const LIGHT_BACKGROUND_THRESHOLD: f32 = 0.5;

        let luminance: f32 = RED_LUMINANCE_WEIGHT * caret_color[0]
            + GREEN_LUMINANCE_WEIGHT * caret_color[1]
            + BLUE_LUMINANCE_WEIGHT * caret_color[2];

        if luminance > LIGHT_BACKGROUND_THRESHOLD {
            [0, 0, 0, 255]
        } else {
            [255, 255, 255, 255]
        }
    }

    fn text_attrs<'a>(color: Option<[u8; 4]>, font_family: &'a str) -> Attrs<'a> {
        let attrs: Attrs<'a> = Attrs::new().family(Family::Name(font_family));

        match color {
            Some([r, g, b, a]) => attrs.color(Color::rgba(r, g, b, a)),
            None => attrs,
        }
    }

    fn row_count(&self) -> usize {
        self.snapshot.lines.len().max(1)
    }

    fn has_tab_bar(&self) -> bool {
        self.tab_bar.titles().len() > 1
    }

    fn cell_char(cell: &ScreenCell) -> char {
        cell.ch
    }

    pub(crate) fn visible_cols(&mut self, width: u32) -> usize {
        let cell_width: f32 =
            GlyphonResources::measure_font_width(&mut self.glyphs.font_system, &self.config)
                .max(1.0);
        let content_width: u32 = Self::terminal_content_width(width);

        ((content_width as f32 / cell_width).floor() as usize).max(1)
    }

    pub(crate) fn visible_rows(&self, height: u32) -> usize {
        let content_height: u32 = Self::terminal_content_height(height, self.config.line_height);
        ((content_height as f32 / self.config.line_height).floor() as usize).max(1)
    }

    fn terminal_origin_y(&self) -> f32 {
        Self::terminal_origin_y_for(self.config.line_height)
    }

    fn terminal_origin_y_for(line_height: f32) -> f32 {
        Self::PADDING_Y + Self::tab_bar_height_for(line_height)
    }

    fn terminal_row_y(visible_row: usize, line_height: f32, has_tab_bar: bool) -> f32 {
        if has_tab_bar {
            Self::PADDING_Y + line_height + visible_row as f32 * line_height
        } else {
            Self::PADDING_Y + visible_row as f32 * line_height
        }
    }

    fn terminal_content_height(height: u32, line_height: f32) -> u32 {
        height.saturating_sub(Self::tab_bar_height_for(line_height).ceil() as u32)
    }

    fn tab_bar_height_for(line_height: f32) -> f32 {
        line_height
    }

    fn terminal_content_width(width: u32) -> u32 {
        let horizontal_padding: u32 = (Self::PADDING_X * 2.0).ceil() as u32;

        width.saturating_sub(horizontal_padding).max(1)
    }

    fn set_text_buffer_size(
        glyphs: &mut GlyphResources,
        width: u32,
        height: u32,
        config: &RendererConfig,
    ) {
        glyphs.text_buffer.set_size(
            &mut glyphs.font_system,
            Some(Self::terminal_content_width(width) as f32),
            Some(Self::terminal_content_height(height, config.line_height) as f32),
        );
        glyphs
            .text_buffer
            .shape_until_scroll(&mut glyphs.font_system, false);
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalView;
    use crate::terminal_view::TabBarSnapshot;
    use crate::terminal_view::tab_bar::TabView;
    use crate::{RendererConfig, util};
    use glyphon::Color;
    use solito_terminal::ScreenCell;

    #[test]
    fn cell_char_returns_cell_character() {
        let mut cell: ScreenCell = ScreenCell::default();
        cell.ch = 'A';

        assert_eq!(TerminalView::cell_char(&cell), 'A');
    }

    #[test]
    fn text_attrs_include_foreground_color() {
        let attrs =
            TerminalView::text_attrs(Some([1, 2, 3, 4]), RendererConfig::DEFAULT_FONT_FAMILY);

        assert_eq!(attrs.color_opt, Some(Color::rgba(1, 2, 3, 4)));
    }

    #[test]
    fn rgba_to_f32_normalizes_color_channels() {
        assert_eq!(
            util::color::rgba_to_f32([0, 128, 255, 255]),
            [0.0, 128.0 / 255.0, 1.0, 1.0]
        );
    }

    #[test]
    fn caret_color_defaults_to_white() {
        assert_eq!(TerminalView::DEFAULT_CARET_COLOR, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn cursor_text_color_contrasts_with_caret_color() {
        assert_eq!(
            TerminalView::cursor_text_color([1.0, 1.0, 1.0, 1.0]),
            [0, 0, 0, 255]
        );
        assert_eq!(
            TerminalView::cursor_text_color([0.0, 0.0, 0.0, 1.0]),
            [255, 255, 255, 255]
        );
    }

    #[test]
    fn text_spans_override_cursor_cell_color() {
        let mut cell: ScreenCell = ScreenCell::default();
        cell.ch = 'A';

        let spans = TerminalView::text_spans_for_lines(
            &[vec![cell]],
            0,
            0,
            0,
            [0, 0, 0, 255],
            RendererConfig::DEFAULT_FONT_FAMILY,
        );

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].0, "A");
        assert_eq!(spans[0].1.color_opt, Some(Color::rgba(0, 0, 0, 255)));
    }

    #[test]
    fn tab_bar_spans_mark_active_tab() {
        let snapshot: TabBarSnapshot =
            TabBarSnapshot::new(vec!["Tab 1".to_string(), "Tab 2".to_string()], 0);
        let spans = TerminalView::tab_bar_spans_for(&snapshot, RendererConfig::DEFAULT_FONT_FAMILY);

        assert_eq!(spans[0].0, "  Tab 1  ");
        assert_eq!(spans[0].1.color_opt, Some(Color::rgba(255, 255, 255, 255)));
        assert_eq!(spans[1].0, " ");
        assert_eq!(spans[2].0, "  Tab 2  ");
        assert_eq!(spans[2].1.color_opt, Some(Color::rgba(140, 148, 160, 255)));
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
    fn visible_rows_reserve_one_row_for_tab_bar() {
        assert_eq!(
            TerminalView::terminal_content_height(90, RendererConfig::DEFAULT_LINE_HEIGHT),
            60
        );
    }

    #[test]
    fn terminal_content_width_reserves_horizontal_padding() {
        assert_eq!(TerminalView::terminal_content_width(100), 80);
        assert_eq!(TerminalView::terminal_content_width(10), 1);
    }
}
