use super::SolitoApplication;
use crate::session::runtime::SessionInput;
use solito_terminal::MouseMode;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, MouseScrollDelta},
};

#[derive(Default)]
pub(super) struct MouseInputState {
    pub(super) position: Option<PhysicalPosition<f64>>,
    cell: Option<(usize, usize)>,
    buttons: [bool; 3],
    scroll: [f64; 2],
    context: Option<(usize, MouseMode, bool)>,
}

impl SolitoApplication {
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

    pub(super) fn report_mouse_motion(&mut self, position: PhysicalPosition<f64>) {
        let mode = self.mouse_mode();
        self.mouse.position = Some(position);
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

    pub(super) fn report_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        let mode = self.mouse_mode();
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
    if !delta.is_finite() {
        return 0;
    }
    *remainder += delta;
    let steps = remainder.trunc() as i32;
    *remainder -= f64::from(steps);
    steps
}

#[cfg(test)]
mod tests {
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
