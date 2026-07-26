//! Convert winit window events into application commands or renderer actions.

use super::{AppResult, SolitoApplication};
use crate::app::input::{self, AppCommand};
use solito_renderer::TerminalSize;
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
    ) -> AppResult<AppCommand> {
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
                Ok(AppCommand::Noop)
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer
                    && let Err(err) = renderer.draw_frame()
                {
                    error!("{err}");
                    event_loop.exit();
                }
                Ok(AppCommand::Noop)
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
            } => {
                let Some(input_tx) = self.tabs.active_input_tx() else {
                    return Ok(AppCommand::Noop);
                };

                input::handle_key(
                    text,
                    logical_key,
                    key_state,
                    self.modifiers,
                    input_tx,
                    self.copy_mode.is_active(),
                )
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                Ok(AppCommand::Noop)
            }
            WindowEvent::Resized(window_size) => {
                let Some(renderer) = &self.renderer else {
                    return Ok(AppCommand::Noop);
                };
                let terminal_size: TerminalSize = renderer.terminal_size_for(window_size);
                Ok(AppCommand::Resize {
                    window_size,
                    terminal_size,
                })
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let Some(renderer) = &mut self.renderer else {
                    return Ok(AppCommand::Noop);
                };

                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => renderer.scroll(x, y),
                    winit::event::MouseScrollDelta::PixelDelta(position) => renderer.scroll(
                        position.x as f32 / self.renderer_config.line_height,
                        position.y as f32 / self.renderer_config.line_height,
                    ),
                }
                Ok(AppCommand::Noop)
            }
            _ => {
                tracing::debug!("unhandled event: {event:?}");
                Ok(AppCommand::Noop)
            }
        }
    }
}
