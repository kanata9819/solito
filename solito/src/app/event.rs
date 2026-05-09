use solito_renderer::{RendererConfig, TerminalViewRenderer, WindowRenderer};
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, mpsc::Sender},
};
use tracing::error;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, ModifiersState, NamedKey, SmolStr},
    window::{Window, WindowId},
};

use crate::session::runtime::SessionInput;

pub(super) enum AppCommand {
    None,
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
    Resize {
        size: PhysicalSize<u32>,
        cols: usize,
        rows: usize,
    },
}

pub(super) fn event_handler<T: TerminalViewRenderer + WindowRenderer>(
    windows: &mut HashMap<WindowId, Arc<Window>>,
    window_id: WindowId,
    state: &mut T,
    event_loop: &ActiveEventLoop,
    event: WindowEvent,
    modifiers_state: &mut ModifiersState,
    input_tx: &Sender<SessionInput>,
) -> Result<AppCommand, Box<dyn Error>> {
    match event {
        WindowEvent::CloseRequested => {
            let _: Option<Arc<Window>> = windows.remove(&window_id);
            event_loop.exit();
            Ok(AppCommand::None)
        }
        WindowEvent::RedrawRequested => {
            windows[&window_id].set_blur(true);

            if let Err(e) = state.render() {
                error!("{e}");
                event_loop.exit();
            }

            state.redraw()?;
            Ok(AppCommand::None)
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
        } => handle_key(text, logical_key, key_state, *modifiers_state, input_tx),
        WindowEvent::ModifiersChanged(modifiers) => {
            *modifiers_state = modifiers.state();
            Ok(AppCommand::None)
        }
        WindowEvent::Resized(size) => {
            let (cols, rows): (usize, usize) = state.terminal_size_for(size);
            Ok(AppCommand::Resize { size, cols, rows })
        }
        WindowEvent::MouseWheel {
            device_id: _,
            delta,
            phase: _,
        } => {
            match delta {
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
            }
            Ok(AppCommand::None)
        }
        #[allow(unused)]
        WindowEvent::CursorMoved {
            device_id,
            position,
        } => Ok(AppCommand::None),
        #[allow(unused)]
        WindowEvent::CursorLeft { device_id } => Ok(AppCommand::None),
        #[allow(unused)]
        WindowEvent::CursorEntered { device_id } => Ok(AppCommand::None),
        _ => {
            tracing::debug!("unhandled event: {event:?}");
            Ok(AppCommand::None)
        }
    }
}

fn handle_key(
    text: Option<SmolStr>,
    logical_key: Key<SmolStr>,
    key_state: ElementState,
    modifiers: ModifiersState,
    input_tx: &Sender<SessionInput>,
) -> Result<AppCommand, Box<dyn Error>> {
    const ENTER: &[u8; 1] = b"\r";
    const BACKSPACE: &[u8; 1] = b"\x7f";
    const TAB: &[u8; 1] = b"\t";
    const ESCAPE: &[u8; 1] = b"\x1b";
    const ARROWUP: &[u8; 3] = b"\x1b[A";
    const ARROWDOWN: &[u8; 3] = b"\x1b[B";
    const ARROWRIGHT: &[u8; 3] = b"\x1b[C";
    const ARROWLEFT: &[u8; 3] = b"\x1b[D";

    if key_state == ElementState::Pressed {
        if let Some(command) = tab_shortcut_command(&logical_key, modifiers) {
            return Ok(command);
        }

        match &logical_key {
            Key::Named(NamedKey::F1) => return Ok(AppCommand::NewTab),
            Key::Named(NamedKey::F2) => return Ok(AppCommand::NextTab),
            Key::Named(NamedKey::F3) => return Ok(AppCommand::PreviousTab),
            _ => {}
        }

        if let Some(text) = text {
            input_tx.send(SessionInput::Write(text.as_bytes().to_vec()))?;
            return Ok(AppCommand::None);
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

    Ok(AppCommand::None)
}

fn tab_shortcut_command(
    logical_key: &Key<SmolStr>,
    modifiers: ModifiersState,
) -> Option<AppCommand> {
    if modifiers.control_key() || modifiers.shift_key() {
        if let Key::Character(character) = &logical_key {
            if character.eq_ignore_ascii_case("t") {
                return Some(AppCommand::NewTab);
            } else if character.eq_ignore_ascii_case("w") {
                return Some(AppCommand::CloseTab);
            } else {
                return None;
            }
        }
    }

    if modifiers.control_key() {
        if let Key::Named(NamedKey::Tab) = &logical_key {
            return Some(AppCommand::NextTab);
        } else {
            return None;
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{AppCommand, tab_shortcut_command};
    use winit::keyboard::{Key, ModifiersState, SmolStr};

    #[test]
    fn ctrl_shift_t_opens_new_tab() {
        let command = tab_shortcut_command(
            &Key::Character(SmolStr::new("T")),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );

        assert!(matches!(command, Some(AppCommand::NewTab)));
    }

    #[test]
    fn ctrl_shift_w_closes_active_tab() {
        let command = tab_shortcut_command(
            &Key::Character(SmolStr::new("W")),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );

        assert!(matches!(command, Some(AppCommand::CloseTab)));
    }
}
