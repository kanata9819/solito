use solito_renderer::{RendererConfig, TerminalViewRenderer, WindowRenderer};
use solito_terminal::TerminalState;
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, mpsc::Sender},
};
use tracing::error;
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey, SmolStr},
    window::{Window, WindowId},
};

use crate::session::runtime::SessionInput;

pub(super) fn event_handler<T: TerminalViewRenderer + WindowRenderer>(
    windows: &mut HashMap<WindowId, Arc<Window>>,
    window_id: WindowId,
    state: &mut T,
    terminal: &mut TerminalState,
    event_loop: &ActiveEventLoop,
    event: WindowEvent,
    input_tx: &Sender<SessionInput>,
) -> Result<(), Box<dyn Error>> {
    match event {
        WindowEvent::CloseRequested => {
            let _: Option<Arc<Window>> = windows.remove(&window_id);
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
                    logical_key,
                    state: key_state,
                    text,
                    ..
                },
            ..
        } => handle_key(text, logical_key, key_state, input_tx)?,
        WindowEvent::Resized(size) => {
            let (cols, rows): (usize, usize) = state.terminal_size_for(size);
            terminal.set_width(cols);
            terminal.set_height(rows);
            input_tx.send(SessionInput::resize(cols, rows))?;
            state.resize(size, terminal.snapshot());
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
                    pos.x as f32 / RendererConfig::LINE_HEIGHT,
                    pos.y as f32 / RendererConfig::LINE_HEIGHT,
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

fn handle_key(
    text: Option<SmolStr>,
    logical_key: Key<SmolStr>,
    key_state: ElementState,
    input_tx: &Sender<SessionInput>,
) -> Result<(), Box<dyn Error>> {
    const ENTER: &[u8; 1] = b"\r";
    const BACKSPACE: &[u8; 1] = b"\x7f";
    const TAB: &[u8; 1] = b"\t";
    const ESCAPE: &[u8; 1] = b"\x1b";
    const ARROWUP: &[u8; 3] = b"\x1b[A";
    const ARROWDOWN: &[u8; 3] = b"\x1b[B";
    const ARROWRIGHT: &[u8; 3] = b"\x1b[C";
    const ARROWLEFT: &[u8; 3] = b"\x1b[D";

    if key_state == ElementState::Pressed {
        if let Some(text) = text {
            input_tx.send(SessionInput::Write(text.as_bytes().to_vec()))?;
            return Ok(());
        }

        match &logical_key {
            Key::Named(NamedKey::Enter) => input_tx.send(SessionInput::write(ENTER.to_vec()))?,
            Key::Named(NamedKey::Backspace) => {
                input_tx.send(SessionInput::write(BACKSPACE.to_vec()))?
            }
            Key::Named(NamedKey::Tab) => input_tx.send(SessionInput::write(TAB.to_vec()))?,
            Key::Named(NamedKey::Escape) => input_tx.send(SessionInput::write(ESCAPE.to_vec()))?,
            Key::Named(NamedKey::ArrowUp) => {
                input_tx.send(SessionInput::write(ARROWUP.to_vec()))?
            }
            Key::Named(NamedKey::ArrowDown) => {
                input_tx.send(SessionInput::write(ARROWDOWN.to_vec()))?
            }
            Key::Named(NamedKey::ArrowRight) => {
                input_tx.send(SessionInput::write(ARROWRIGHT.to_vec()))?
            }
            Key::Named(NamedKey::ArrowLeft) => {
                input_tx.send(SessionInput::write(ARROWLEFT.to_vec()))?
            }
            _ => {}
        }
    }

    Ok(())
}
