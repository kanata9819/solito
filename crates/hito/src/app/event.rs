use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, mpsc::Sender},
};
use tracing::error;
use winit::{
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

use crate::{
    config::BufferAttr,
    renderer::state::context::TerminalOutputSink,
    renderer::state::context::WindowRenderer,
    util::{
        self,
        keycode_parser::{CodeKind, KeyState, ParseError, ParseResult},
    },
};

pub(super) fn event_handler<T: TerminalOutputSink + WindowRenderer>(
    windows: &mut HashMap<WindowId, Arc<Window>>,
    window_id: WindowId,
    state: &mut T,
    event_loop: &ActiveEventLoop,
    event: WindowEvent,
    input_tx: &Sender<Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    match event {
        WindowEvent::CloseRequested => {
            let _ = windows.remove(&window_id);
            event_loop.exit();
        }
        WindowEvent::RedrawRequested => {
            windows[&window_id].set_blur(true);

            if let Err(e) = state.render() {
                error!("{e}");
                event_loop.exit();
            }

            state.redraw()?;
        }
        WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    state: key_state,
                    ..
                },
            ..
        } => {
            let key_state: KeyState = KeyState {
                key_code: code,
                is_pressed: key_state == winit::event::ElementState::Pressed,
                is_released: key_state == winit::event::ElementState::Released,
            };
            handle_key(&key_state, input_tx)?
        }
        WindowEvent::Resized(size) => {
            state.resize(size);
        }
        WindowEvent::MouseWheel {
            device_id: _,
            delta,
            phase: _,
        } => match delta {
            winit::event::MouseScrollDelta::LineDelta(x, y) => {
                tracing::debug!("MouseScrollDelta.LineDelta: x({:?}), y({:?})", x, y);
                state.scroll(x, y);
            }
            winit::event::MouseScrollDelta::PixelDelta(pos) => {
                tracing::debug!("MouseScrollDelta.PixelDelta: pos({:?})", pos);
                state.scroll(
                    pos.x as f32 / BufferAttr::LINE_HEIGHT,
                    pos.y as f32 / BufferAttr::LINE_HEIGHT,
                );
            }
        },
        #[allow(unused)]
        WindowEvent::CursorMoved {
            device_id,
            position,
        } => {}
        #[allow(unused)]
        WindowEvent::CursorLeft { device_id } => {}
        #[allow(unused)]
        WindowEvent::CursorEntered { device_id } => {}
        _ => {
            tracing::debug!("unhandled event: {event:?}");
        }
    }
    Ok(())
}

/// This is where we'll handle keyboard events.
fn handle_key(key_state: &KeyState, input_tx: &Sender<Vec<u8>>) -> Result<(), Box<dyn Error>> {
    match util::keycode_parser::parse(key_state) {
        ParseResult::Ok(kind) => match kind {
            CodeKind::Char(char) => {
                input_tx.send(char.to_string().into_bytes())?;
                // The PTY echoes printable input back to us, so drawing here would duplicate
                // or briefly conflict with the terminal output stream.
                Ok(())
            }
            CodeKind::Function => Ok(()),
            CodeKind::Special => Ok(()),
        },
        ParseResult::Err(ParseError::InvalidCode(code)) => {
            Err((format!("Invalid Code: {:?}", code).to_string()).into())
        }
        ParseResult::Err(ParseError::UnHandled(code)) => {
            Err((format!("UnHandled Code: {:?}", code).to_string()).into())
        }
    }
}
