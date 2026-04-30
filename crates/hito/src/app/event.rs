use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, mpsc::Sender},
};

use tracing::{error, info};
use winit::{
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

use crate::{
    renderer::state::State,
    util::{
        self,
        keycode_parser::{CodeKind, KeyState, ParseError, ParseResult},
    },
};

pub fn event_handler(
    windows: &mut HashMap<WindowId, Arc<Window>>,
    window_id: WindowId,
    state: &mut State,
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
            handle_key(state, event_loop, &key_state, input_tx)?
        }
        WindowEvent::Resized(size) => {
            state.resize(size);
        }
        _ => {
            info!("unhandled event: {event:?}");
        }
    }
    Ok(())
}

/// This is where we'll handle keyboard events.
pub fn handle_key(
    state: &mut State,
    _event_loop: &ActiveEventLoop,
    key_state: &KeyState,
    input_tx: &Sender<Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    match util::keycode_parser::parse(key_state) {
        ParseResult::Ok(kind) => match kind {
            CodeKind::Char(char) => {
                input_tx.send(char.to_string().into_bytes())?;
                state.add_char_to_buffer(char);
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
