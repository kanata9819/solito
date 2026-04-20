use std::{collections::HashMap, error::Error, sync::Arc};

use tracing::error;
use winit::{
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::{
    renderer::state::State,
    util::{
        self,
        keycode_parser::{CodeKind, ParseError, ParseResult},
    },
};

pub fn event_handler(
    windows: &mut HashMap<WindowId, Arc<Window>>,
    window_id: WindowId,
    state: &mut State,
    event_loop: &ActiveEventLoop,
    event: WindowEvent,
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
        } => handle_key(state, &event_loop, code, key_state.is_pressed()),
        WindowEvent::Resized(size) => {
            state.resize(size);
        }
        _ => {}
    }
    Ok(())
}

/// This is where we'll handle keyboard events.
pub fn handle_key(
    state: &mut State,
    _event_loop: &ActiveEventLoop,
    code: KeyCode,
    is_pressed: bool,
) {
    match util::keycode_parser::parse(&code, is_pressed) {
        ParseResult::Ok(kind) => match kind {
            CodeKind::Char(char) => {
                state.add_char_to_buffer(char);
            }
            CodeKind::Function => {}
            CodeKind::Special => {}
        },
        ParseResult::Err(ParseError::InvalidCode) => {}
    }
}
