use solito_terminal::ScreenLines;
use solito_terminal::{ScreenSnapshot, TerminalSize};

use crate::RendererConfig;

use super::{
    copy_mode::CopyModeSnapshot, glyph::GlyphonResources, ime::ImePreedit, tab_bar::TabBar,
    text_damage::TextDamage, viewport::ViewportState,
};

pub(crate) struct TerminalView {
    pub(crate) glyphs: GlyphonResources,
    pub(crate) config: RendererConfig,
    pub(super) snapshot: ScreenSnapshot,
    pub(crate) tab_bar: TabBar,
    pub(super) viewport: ViewportState,
    pub(super) copy_mode: CopyModeSnapshot,
    pub(super) ime_preedit: ImePreedit,
    pub(super) text_damage: TextDamage,
    pub(super) text_origin: Option<usize>,
}

impl TerminalView {
    pub(crate) fn set_tab_bar(&mut self, snapshot: super::TabBarSnapshot) {
        self.tab_bar
            .set_snapshot(snapshot, &mut self.glyphs.font_system);
    }

    pub(crate) const PADDING_X: f32 = 16.0;
    pub(crate) const PADDING_Y: f32 = 12.0;

    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        swapchain: wgpu::TextureFormat,
        physical_size: winit::dpi::PhysicalSize<u32>,
        config: RendererConfig,
        font_system: glyphon::FontSystem,
    ) -> Self {
        let config = config.sanitized();
        let mut glyphs = GlyphonResources::new(device, queue, swapchain, &config, font_system);

        Self::set_text_buffer_size(&mut glyphs, physical_size.width, physical_size.height);

        Self {
            glyphs,
            viewport: ViewportState::new(
                Self::terminal_content_height(physical_size.height),
                config.line_height,
            ),
            config,
            snapshot: ScreenSnapshot::default(),
            tab_bar: TabBar::new(physical_size.width),
            copy_mode: CopyModeSnapshot::default(),
            ime_preedit: ImePreedit::default(),
            text_damage: TextDamage::All,
            text_origin: None,
        }
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32, snapshot: ScreenSnapshot) {
        // Anchor scrolled content before adopting output received while hidden.
        self.set_snapshot(snapshot);
        Self::set_text_buffer_size(&mut self.glyphs, width, height);
        self.viewport
            .resize(Self::terminal_content_height(height), self.row_count());
        self.tab_bar.resize(width, &mut self.glyphs.font_system);
        self.invalidate_all_text();
    }

    pub(crate) fn set_snapshot(&mut self, snapshot: ScreenSnapshot) {
        let previous_range = self.viewport.visible_range(self.row_count());
        let keep_start = if self.viewport.is_at_bottom() {
            None
        } else {
            Some(self.viewport.visible_range(self.row_count()).0)
        };
        let removed = snapshot
            .history_start
            .saturating_sub(self.snapshot.history_start);
        let keep_start = keep_start.map(|start| start.saturating_sub(removed));
        if let Some(start) = keep_start {
            self.viewport
                .scroll_to_start(start, snapshot.lines.len().max(1));
        } else {
            self.viewport.clamp(snapshot.lines.len().max(1));
        }

        let next_range = self.viewport.visible_range(snapshot.lines.len().max(1));
        if self.ime_preedit.is_active() {
            // Composition follows the shell cursor even while its blink phase is hidden.
            self.text_damage.add_row(self.snapshot.cursor_row);
            self.text_damage.add_row(snapshot.cursor_row);
        }
        if removed > 0 || previous_range != next_range {
            self.invalidate_all_text();
        } else if self.text_damage != TextDamage::All {
            let damage = TextDamage::between(&self.snapshot, &snapshot, next_range.0..next_range.1);
            self.text_damage.merge(damage);
        }
        self.snapshot = snapshot;
    }

    pub(crate) fn set_snapshot_at_bottom(&mut self, snapshot: ScreenSnapshot) {
        self.set_snapshot(snapshot);
        self.scroll_to_bottom();
    }

    pub(crate) fn scroll_to_bottom(&mut self) -> bool {
        let changed = !self.viewport.is_at_bottom();
        self.viewport.reset();
        if changed {
            self.invalidate_all_text();
        }
        changed
    }

    pub(crate) fn scroll(&mut self, _x: f32, y: f32) {
        let previous_range = self.viewport.visible_range(self.row_count());
        self.viewport.scroll(y, self.row_count());
        if previous_range != self.viewport.visible_range(self.row_count()) {
            self.invalidate_all_text();
        }
    }

    pub(crate) fn visible_cols(&self, width: u32) -> usize {
        let content_width = Self::terminal_content_width(width);

        ((content_width as f32 / self.glyphs.cell_width).floor() as usize).max(1)
    }

    pub(crate) fn cell_at(&self, x: f64, y: f64, size: TerminalSize) -> Option<(usize, usize)> {
        let top = Self::terminal_row_y(0, self.config.line_height);
        Self::cell_at_position(
            x,
            y,
            top,
            self.glyphs.cell_width,
            self.config.line_height,
            size,
        )
    }

    fn cell_at_position(
        x: f64,
        y: f64,
        top: f32,
        width: f32,
        height: f32,
        size: TerminalSize,
    ) -> Option<(usize, usize)> {
        let x = x - f64::from(Self::PADDING_X);
        let y = y - f64::from(top);
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
            return None;
        }
        let col = (x / f64::from(width)) as usize;
        let row = (y / f64::from(height)) as usize;
        (col < size.cols && row < size.rows).then_some((col, row))
    }

    pub(crate) fn visible_rows(&self, height: u32) -> usize {
        let content_height = Self::terminal_content_height(height);
        ((content_height as f32 / self.config.line_height).floor() as usize).max(1)
    }

    pub(crate) fn terminal_row_y(visible_row: usize, line_height: f32) -> f32 {
        TabBar::HEIGHT + Self::PADDING_Y + visible_row as f32 * line_height
    }

    pub(super) fn display_col_count(lines: &ScreenLines, row: usize) -> usize {
        lines.get(row).map_or(1, |line| line.len().max(1))
    }

    pub(super) fn row_count(&self) -> usize {
        self.snapshot.lines.len().max(1)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn gpu_text_updates_cover_scrolling_tabs_selection_and_empty_screen() {
        use super::super::{CopyModePosition, CopyModeSnapshot, TabBarSnapshot};
        use solito_terminal::{ScreenSnapshot, TerminalSize, TerminalState};

        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let config = RendererConfig::default();
        let mut view = TerminalView::new(
            &device,
            &queue,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            winit::dpi::PhysicalSize::new(800, 158),
            config,
            glyphon::FontSystem::new(),
        );
        let mut blank = TerminalState::new(TerminalSize::new(80, 3));
        blank.apply_terminal_output(b"\x1b[3;4H");
        view.set_snapshot(blank.snapshot());
        assert!(view.update_text_buffer());
        assert_eq!(
            view.caret_rect(),
            (
                TerminalView::PADDING_X + 3.0 * view.glyphs.cell_width,
                TerminalView::terminal_row_y(2, view.config.line_height),
                view.glyphs.cell_width,
                view.config.line_height,
            ),
            "moving to an empty row must keep the cursor visible without printing"
        );
        let mut startup = TerminalState::new(TerminalSize::new(80, 3));
        startup.apply_terminal_output(b"Welcome to Nushell");
        view.set_snapshot(startup.snapshot());
        assert!(view.update_text_buffer());
        startup.apply_terminal_output(b"\r\n~> ");
        view.set_snapshot(startup.snapshot());
        assert!(view.update_text_buffer());
        assert!(
            view.glyphs
                .text_buffer
                .layout_runs()
                .any(|run| run.text == "~> "),
            "a prompt appended after an already rendered banner must have glyphs"
        );
        let prompt = startup.snapshot();
        view.set_ime_preedit("日本語".into(), Some((6, 9)));
        assert!(view.update_text_buffer());
        assert_eq!(view.glyphs.text_buffer.lines[1].text(), "~> 日 本 語 ");
        assert!(
            view.glyphs
                .text_buffer
                .layout_runs()
                .any(|run| run.text.contains('日'))
        );
        let candidate = view.ime_cursor_rect();
        assert_eq!(
            candidate.0,
            TerminalView::PADDING_X + 7.0 * view.glyphs.cell_width
        );
        assert_eq!(
            candidate.1,
            TerminalView::terminal_row_y(1, view.config.line_height)
        );
        assert_eq!(view.ime_rects().len(), 3);
        assert_eq!(view.caret_rect().2, 0.0);
        let mut blinking = prompt.clone();
        blinking.cursor_visible = false;
        view.set_snapshot(blinking.clone());
        assert!(view.update_text_buffer());
        assert_eq!(
            view.ime_cursor_rect(),
            candidate,
            "candidate position must not blink"
        );
        assert_eq!(view.glyphs.text_buffer.lines[1].text(), "~> 日 本 語 ");
        blinking.cursor_row = 0;
        blinking.cursor_col = 5;
        view.set_snapshot(blinking);
        assert!(view.update_text_buffer());
        assert_eq!(view.glyphs.text_buffer.lines[1].text(), "~> ");
        assert!(view.glyphs.text_buffer.lines[0].text().contains('日'));
        view.set_ime_preedit(String::new(), None);
        view.set_snapshot(prompt);
        assert!(view.update_text_buffer());
        assert_eq!(
            view.glyphs.text_buffer.lines[0].text(),
            "Welcome to Nushell"
        );
        assert_eq!(view.glyphs.text_buffer.lines[1].text(), "~> ");
        assert!(view.ime_rects().is_empty());
        // Three terminal rows below the fixed toolbar and content padding.
        let mut terminal = TerminalState::new(TerminalSize::new(80, 3));
        terminal.apply_terminal_output("first\r\nsecond\r\n日本語\r\nlast".as_bytes());
        view.set_snapshot(terminal.snapshot());
        assert!(view.update_text_buffer());
        assert!(!view.update_text_buffer());
        assert_eq!(view.glyphs.text_buffer.lines[0].text(), "second");
        assert_eq!(view.glyphs.text_buffer.lines[2].text(), "last");
        view.set_snapshot(terminal.snapshot());
        assert!(!view.update_text_buffer());
        view.resize(900, 158, terminal.snapshot());
        assert!(
            view.update_text_buffer(),
            "resizing must prepare glyphs again"
        );
        assert!(!view.update_text_buffer());

        view.scroll(0.0, 1.0);
        assert!(view.update_text_buffer());
        assert_eq!(view.glyphs.text_buffer.lines[0].text(), "first");
        let scrolled_layout = |view: &TerminalView| {
            view.glyphs
                .text_buffer
                .layout_runs()
                .map(|run| (run.text.to_owned(), run.line_y, run.line_w, run.line_height))
                .collect::<Vec<_>>()
        };
        let before_restore = scrolled_layout(&view);
        let mut background = TerminalState::new(TerminalSize::new(80, 3));
        background.apply_terminal_output("first\r\nsecond\r\n日本語\r\nlast\r\nnew".as_bytes());
        view.resize(900, 158, background.snapshot());
        assert!(view.update_text_buffer());
        assert_eq!(
            scrolled_layout(&view),
            before_restore,
            "restoring with background output must preserve the scrolled text and glyph positions"
        );
        view.resize(900, 158, terminal.snapshot());
        assert!(view.update_text_buffer());
        assert!(view.scroll_to_bottom());
        assert!(view.update_text_buffer());
        assert!(!view.scroll_to_bottom());
        assert!(!view.update_text_buffer());

        view.set_copy_mode(CopyModeSnapshot {
            active: true,
            cursor: CopyModePosition::new(2, 0),
            selection: None,
        });
        assert!(
            !view.update_text_buffer(),
            "copy cursor only updates rectangles"
        );
        view.set_copy_mode(CopyModeSnapshot::default());
        view.set_tab_bar(TabBarSnapshot::new(vec!["one".into(), "two".into()], 1));
        assert!(
            !view.update_text_buffer(),
            "tab changes must not reshape shell text"
        );
        assert!(
            view.tab_bar
                .text_areas()
                .any(|area| area.buffer.lines[0].text().contains("two"))
        );
        assert_eq!(view.glyphs.text_buffer.lines[0].text(), "second");
        assert_eq!(
            view.cell_at(16.0, 56.0, TerminalSize::new(80, 3)),
            Some((0, 0))
        );
        assert_eq!(view.cell_at(16.0, 30.0, TerminalSize::new(80, 3)), None);
        view.tab_bar.dirty = false;
        assert!(view.tab_bar.hover(Some(0)));
        assert!(!view.tab_bar.hover(Some(0)));
        assert!(!view.update_text_buffer());
        assert!(view.tab_bar.focus(false));
        assert!(!view.update_text_buffer());
        view.set_tab_bar(TabBarSnapshot::default());
        view.set_snapshot_at_bottom(ScreenSnapshot::default());
        assert!(view.update_text_buffer());
        assert_eq!(view.glyphs.text_buffer.lines.len(), 1);
        assert_eq!(view.glyphs.text_buffer.lines[0].text(), "");
    }

    #[test]
    fn mouse_cells_exclude_padding_and_tab_bar() {
        let cell = |x, y| {
            super::TerminalView::cell_at_position(
                x,
                y,
                40.0,
                10.0,
                30.0,
                solito_terminal::TerminalSize::new(80, 24),
            )
        };
        assert_eq!(cell(16.0, 40.0), Some((0, 0)));
        assert_eq!(cell(35.9, 70.0), Some((1, 1)));
        assert_eq!(cell(15.0, 40.0), None);
        assert_eq!(cell(16.0, 39.0), None);
        assert_eq!(cell(816.0, 40.0), None);
        assert_eq!(cell(16.0, 760.0), None);
    }
    use super::TerminalView;
    use crate::util::color::ThemeColor;
    use crate::{RendererConfig, util};

    #[test]
    fn rgba_to_f32_normalizes_color_channels() {
        assert_eq!(
            util::color::rgba_to_f32([0, 128, 255, 255]),
            [0.0, 128.0 / 255.0, 1.0, 1.0]
        );
    }

    #[test]
    fn caret_color_defaults_to_white() {
        assert_eq!(TerminalView::DEFAULT_CARET_COLOR, ThemeColor::WHITE_ALPHA);
    }

    #[test]
    fn visible_rows_reserve_toolbar_and_both_padding_edges() {
        assert_eq!(TerminalView::terminal_content_height(158), 90);
    }

    #[test]
    fn terminal_content_width_reserves_horizontal_padding() {
        assert_eq!(TerminalView::terminal_content_width(100), 68);
        assert_eq!(TerminalView::terminal_content_width(10), 1);
    }

    #[test]
    fn terminal_row_y_accounts_for_the_tab_bar() {
        assert_eq!(TerminalView::terminal_row_y(2, 30.0), 116.0);
    }
}
