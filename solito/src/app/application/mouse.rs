use super::SolitoApplication;
use crate::session::runtime::SessionInput;
use solito_renderer::CopyModePosition;
use solito_terminal::MouseMode;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, MouseScrollDelta},
};

#[derive(Default)]
pub(super) struct MouseInputState {
    pub(super) position: Option<PhysicalPosition<f64>>,
    pub(super) selection_drag: Option<SelectionDrag>,
    cell: Option<(usize, usize)>,
    buttons: [bool; 3],
    scroll: [f64; 2],
    context: Option<(usize, MouseMode, bool)>,
}

pub(super) struct SelectionDrag {
    anchor: CopyModePosition,
    origin: PhysicalPosition<f64>,
    started: bool,
}

impl SelectionDrag {
    fn update(&mut self, position: PhysicalPosition<f64>) -> bool {
        const DRAG_THRESHOLD: f64 = 3.0;
        let dx = (position.x - self.origin.x).abs();
        let dy = (position.y - self.origin.y).abs();
        self.started |= dx.max(dy) >= DRAG_THRESHOLD;
        self.started
    }
}

impl SolitoApplication {
    pub(super) fn activate_clicked_tab(
        &mut self,
        state: ElementState,
        button: MouseButton,
    ) -> bool {
        if state != ElementState::Pressed || button != MouseButton::Left {
            return false;
        }
        let Some(position) = self.mouse.position else {
            return false;
        };
        let Some(index) = self
            .renderer
            .as_ref()
            .and_then(|renderer| renderer.tab_at(position.x, position.y))
        else {
            return false;
        };
        if self.tabs.activate(index) {
            self.clear_ime();
            self.leave_copy_mode();
            self.refresh_tab_bar();
            self.show_active_terminal_at_bottom();
            self.update_tab_hover(Some(position));
            self.update_ime_cursor_area();
        }
        // A click on the strip belongs to Solito even when the tab is already active.
        true
    }

    pub(super) fn update_tab_hover(&mut self, position: Option<PhysicalPosition<f64>>) {
        if let Some(renderer) = &mut self.renderer {
            let tab = position.and_then(|p| renderer.tab_at(p.x, p.y));
            self.needs_redraw |= renderer.set_tab_hover(tab);
            if let Some(window) = &self.window {
                window.set_cursor(if tab.is_some() {
                    winit::window::CursorIcon::Pointer
                } else {
                    winit::window::CursorIcon::Default
                });
            }
        }
    }

    fn mouse_mode(&mut self) -> MouseMode {
        let mode = self.tabs.active_mouse_mode();
        let context = (self.tabs.active_index(), mode, self.copy_mode.is_active());
        if self.mouse.context != Some(context) {
            self.mouse = MouseInputState {
                position: self.mouse.position,
                context: Some(context),
                ..Default::default()
            };
        }
        if self.copy_mode.is_active() {
            MouseMode::default()
        } else {
            mode
        }
    }

    fn mouse_cell(&self) -> Option<(usize, usize)> {
        let position = self.mouse.position?;
        self.renderer
            .as_ref()?
            .terminal_cell_at(position.x, position.y)
    }

    fn mouse_history_cell(&self, position: PhysicalPosition<f64>) -> Option<CopyModePosition> {
        self.renderer
            .as_ref()?
            .terminal_history_cell_at(position.x, position.y)
    }

    fn begin_mouse_selection(&mut self) {
        self.mouse.selection_drag = self.mouse.position.and_then(|origin| {
            Some(SelectionDrag {
                anchor: self.mouse_history_cell(origin)?,
                origin,
                started: false,
            })
        });
        if self.copy_mode.clear_selection()
            && let Some(snapshot) = self.tabs.active_snapshot()
        {
            self.refresh_copy_mode(&snapshot);
        }
    }

    pub(super) fn update_mouse_selection(&mut self) {
        let (Some(position), Some(drag)) = (self.mouse.position, &mut self.mouse.selection_drag)
        else {
            return;
        };
        if !drag.update(position) {
            return;
        }
        let anchor = drag.anchor;
        if let Some(cursor) = self.mouse_history_cell(position)
            && let Some(snapshot) = self.tabs.active_snapshot()
            && self.copy_mode.select_cells(&snapshot, anchor, cursor)
        {
            if self.terminal_dirty {
                self.refresh_active_terminal();
            } else {
                self.refresh_copy_mode(&snapshot);
            }
        }
    }

    fn send_mouse(&self, mode: MouseMode, code: u8, released: bool, (col, row): (usize, usize)) {
        let modifiers = (u8::from(self.modifiers.shift_key()) * 4)
            | (u8::from(self.modifiers.alt_key()) * 8)
            | (u8::from(self.modifiers.control_key()) * 16);
        if let Some(bytes) = mode.encode(code | modifiers, released, col, row)
            && let Some(tx) = self.tabs.active_input_tx()
            && let Err(err) = tx.send(SessionInput::write(bytes))
        {
            tracing::error!("mouse input failed: {err}");
        }
    }

    pub(super) fn handle_mouse_motion(&mut self, position: PhysicalPosition<f64>) {
        let mode = self.mouse_mode();
        self.mouse.position = Some(position);
        if mode.tracking == 0 {
            self.update_mouse_selection();
            return;
        }
        let cell = self.mouse_cell();
        if cell != self.mouse.cell
            && let Some(cell) = cell
        {
            let button = self
                .mouse
                .buttons
                .iter()
                .position(|down| *down)
                .unwrap_or(3) as u8;
            self.send_mouse(mode, button | 32, false, cell);
            self.mouse.cell = Some(cell);
        }
    }

    pub(super) fn handle_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        let mode = self.mouse_mode();
        if button == MouseButton::Left && mode.tracking == 0 {
            match state {
                ElementState::Pressed => self.begin_mouse_selection(),
                ElementState::Released => self.mouse.selection_drag = None,
            }
            return;
        }
        let button = match button {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            _ => return,
        };
        let released = state == ElementState::Released;
        if released && !self.mouse.buttons[button] {
            return;
        }
        let cell = self
            .mouse_cell()
            .or_else(|| released.then_some(self.mouse.cell).flatten());
        self.mouse.buttons[button] = !released && cell.is_some() && mode.tracking != 0;
        if let Some(cell) = cell {
            self.mouse.cell = Some(cell);
            self.send_mouse(mode, button as u8, released, cell);
        }
    }

    /// Return true when the application owns scrolling, even for sub-cell deltas.
    pub(super) fn report_mouse_wheel(&mut self, delta: MouseScrollDelta) -> bool {
        let mode = self.mouse_mode();
        if mode.tracking == 0 {
            self.mouse.scroll = [0.0; 2];
            return false;
        }
        let Some(cell) = self.mouse_cell() else {
            return true;
        };
        let (x, y) = match delta {
            MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y)),
            MouseScrollDelta::PixelDelta(p) => {
                let height = f64::from(self.renderer_config.line_height.max(1.0));
                (p.x / height, p.y / height)
            }
        };
        for (axis, delta, positive, negative) in [(0, x, 66, 67), (1, y, 64, 65)] {
            let steps = scroll_steps(&mut self.mouse.scroll[axis], delta);
            for _ in 0..steps.unsigned_abs() {
                self.send_mouse(
                    mode,
                    if steps > 0 { positive } else { negative },
                    false,
                    cell,
                );
            }
        }
        true
    }
}

fn scroll_steps(remainder: &mut f64, delta: f64) -> i32 {
    const MAX_STEPS_PER_EVENT: f64 = 256.0;
    if !delta.is_finite() {
        return 0;
    }
    let total = *remainder + delta;
    *remainder = total.fract();
    // Bound PTY messages per event so an extreme wheel delta cannot stall input.
    total
        .trunc()
        .clamp(-MAX_STEPS_PER_EVENT, MAX_STEPS_PER_EVENT) as i32
}

#[cfg(test)]
mod tests {
    use super::{CopyModePosition, PhysicalPosition, SelectionDrag};

    #[test]
    fn mouse_drag_ignores_click_jitter_and_can_return_to_its_anchor() {
        let origin = PhysicalPosition::new(50.0, 50.0);
        let mut drag = SelectionDrag {
            anchor: CopyModePosition::new(0, 0),
            origin,
            started: false,
        };
        assert!(!drag.update(origin));
        assert!(!drag.update(PhysicalPosition::new(51.0, 49.0)));
        assert!(drag.update(PhysicalPosition::new(53.0, 50.0)));
        assert!(drag.update(origin));
    }

    #[test]
    fn extreme_wheel_deltas_do_not_leave_unbounded_work_pending() {
        let mut remainder = 0.0;
        assert_eq!(super::scroll_steps(&mut remainder, f64::MAX), 256);
        assert_eq!(super::scroll_steps(&mut remainder, 0.5), 0);
        assert_eq!(super::scroll_steps(&mut remainder, 0.5), 1);
        assert_eq!(super::scroll_steps(&mut remainder, -f64::MAX), -256);
        assert_eq!(remainder, 0.0);
    }

    #[test]
    fn touchpad_retains_fractional_scroll_in_both_directions() {
        let mut remainder = 0.0;
        assert_eq!(super::scroll_steps(&mut remainder, 0.25), 0);
        assert_eq!(super::scroll_steps(&mut remainder, 0.5), 0);
        assert_eq!(super::scroll_steps(&mut remainder, 0.5), 1);
        assert_eq!(super::scroll_steps(&mut remainder, -1.5), -1);
        assert_eq!(remainder, -0.25);
        assert_eq!(super::scroll_steps(&mut remainder, f64::NAN), 0);
    }
}
