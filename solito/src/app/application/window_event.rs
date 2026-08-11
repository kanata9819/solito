//! Convert winit window events into application commands or renderer actions.

use super::SolitoApplication;
use crate::app::input::{self, AppCommand};
use tracing::error;
use winit::{
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

impl SolitoApplication {
    pub(super) fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) -> AppCommand {
        match event {
            WindowEvent::CloseRequested => {
                if self
                    .window
                    .as_ref()
                    .is_some_and(|window| window.id() == window_id)
                {
                    self.window = None;
                    event_loop.exit();
                }
                AppCommand::Noop
            }
            WindowEvent::RedrawRequested => {
                self.needs_redraw = false;
                if let Some(renderer) = &mut self.renderer
                    && let Err(err) = renderer.draw_frame()
                {
                    error!("{err}");
                    event_loop.exit();
                }
                AppCommand::Noop
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: key_state,
                        text,
                        ..
                    },
                ..
            } => input::handle_key(
                text,
                &logical_key,
                key_state,
                self.modifiers,
                self.copy_mode.is_active(),
            ),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                AppCommand::Noop
            }
            WindowEvent::Resized(window_size) => {
                let Some(renderer) = &self.renderer else {
                    return AppCommand::Noop;
                };
                let terminal_size = renderer.terminal_size_for(window_size);
                AppCommand::Resize {
                    window_size,
                    terminal_size,
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let Some(renderer) = &mut self.renderer else {
                    return AppCommand::Noop;
                };

                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => renderer.scroll(x, y),
                    winit::event::MouseScrollDelta::PixelDelta(position) => renderer.scroll(
                        position.x as f32 / self.renderer_config.line_height,
                        position.y as f32 / self.renderer_config.line_height,
                    ),
                }
                self.needs_redraw = true;
                AppCommand::Noop
            }
            _ => {
                tracing::debug!("unhandled event: {event:?}");
                AppCommand::Noop
            }
        }
    }
}
