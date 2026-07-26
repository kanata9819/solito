//! Keep renderer snapshots and window chrome in sync with application state.

use super::SolitoApplication;
use solito_renderer::TabBarSnapshot;
use solito_terminal::ScreenSnapshot;

impl SolitoApplication {
    pub(super) fn drain_terminal_output(&mut self) {
        if self.tabs.drain_outputs() {
            self.refresh_active_terminal();
        }
    }

    pub(super) fn refresh_active_terminal(&mut self) {
        if let (Some(renderer), Some(snapshot)) = (&mut self.renderer, self.tabs.active_snapshot())
        {
            let copy_mode = self.copy_mode.renderer_snapshot(&snapshot);
            renderer.set_terminal_snapshot(snapshot);
            renderer.set_copy_mode(copy_mode);
        }
    }

    pub(super) fn show_active_terminal_at_bottom(&mut self) {
        if let (Some(renderer), Some(snapshot)) = (&mut self.renderer, self.tabs.active_snapshot())
        {
            let copy_mode = self.copy_mode.renderer_snapshot(&snapshot);
            renderer.set_terminal_snapshot_at_bottom(snapshot);
            renderer.set_copy_mode(copy_mode);
        }
    }

    pub(super) fn refresh_tab_bar(&mut self) {
        let snapshot = self.tab_bar_snapshot();
        if let Some(renderer) = &mut self.renderer {
            renderer.set_tab_bar(snapshot);
        }
    }

    pub(super) fn tab_bar_snapshot(&self) -> TabBarSnapshot {
        TabBarSnapshot::new(self.tabs.titles(), self.tabs.active_index())
    }

    pub(super) fn refresh_copy_mode(&mut self, snapshot: &ScreenSnapshot) {
        if let Some(renderer) = &mut self.renderer {
            renderer.set_copy_mode(self.copy_mode.renderer_snapshot(snapshot));
        }
    }

    pub(super) fn leave_copy_mode(&mut self) {
        if !self.copy_mode.is_active() {
            return;
        }

        self.copy_mode.exit();
        if let Some(renderer) = &mut self.renderer {
            renderer.set_copy_mode(Default::default());
        }
        self.update_window_title();
    }

    pub(super) fn update_window_title(&self) {
        let title = if self.copy_mode.is_active() {
            "Solito - Copy Mode"
        } else {
            "Solito"
        };

        if let Some(window) = &self.window {
            window.set_title(title);
        }
    }
}
