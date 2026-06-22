use solito_renderer::{TerminalViewRenderer, WindowRenderer};
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

use crate::app::copy::CopyModeMove;
use crate::session::runtime::SessionInput;

pub(super) enum AppCommand {
    None,
    EnterCopyMode,
    CopyMode(CopyModeCommand),
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
    CopySelection,
    PasteFromClipboard,
    Resize {
        size: PhysicalSize<u32>,
        cols: usize,
        rows: usize,
    },
}

pub(super) enum CopyModeCommand {
    Move(CopyModeMove),
    ToggleCellSelection,
    ToggleLineSelection,
    CopyAndExit,
    Exit,
}

pub(super) fn event_handler<T: TerminalViewRenderer + WindowRenderer>(
    windows: &mut HashMap<WindowId, Arc<Window>>,
    window_id: WindowId,
    state: &mut T,
    event_loop: &ActiveEventLoop,
    event: WindowEvent,
    modifiers_state: &mut ModifiersState,
    input_tx: &Sender<SessionInput>,
    line_height: f32,
    copy_mode_active: bool,
) -> Result<AppCommand, Box<dyn Error>> {
    match event {
        WindowEvent::CloseRequested => {
            windows.remove(&window_id);
            event_loop.exit();
            Ok(AppCommand::None)
        }
        WindowEvent::RedrawRequested => {
            if let Err(e) = state.draw_frame() {
                error!("{e}");
                event_loop.exit();
            }

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
        } => handle_key(
            text,
            logical_key,
            key_state,
            *modifiers_state,
            input_tx,
            copy_mode_active,
        ),
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
                    state.scroll(pos.x as f32 / line_height, pos.y as f32 / line_height);
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
    copy_mode_active: bool,
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
        if copy_mode_active {
            return Ok(copy_mode_command(&logical_key, modifiers)
                .map(AppCommand::CopyMode)
                .unwrap_or(AppCommand::None));
        }

        if let Some(command) = shortcut_command(&logical_key, modifiers) {
            return Ok(command);
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
            _ => {
                handle_ctrl_c(&logical_key, modifiers, input_tx)?;

                if let Some(text) = text {
                    input_tx.send(SessionInput::Write(text.as_bytes().to_vec()))?;
                    return Ok(AppCommand::None);
                }
            }
        }
    }

    Ok(AppCommand::None)
}

fn handle_ctrl_c(
    logical_key: &Key<SmolStr>,
    modifiers: ModifiersState,
    input_tx: &Sender<SessionInput>,
) -> Result<AppCommand, Box<dyn Error>> {
    const EXT: &[u8; 1] = b"\x03";

    // Ctrl + c
    if let Key::Character(char) = &logical_key
        && modifiers.control_key()
        && char.eq_ignore_ascii_case("c")
    {
        input_tx.send(SessionInput::Write(EXT.to_vec()))?;
        return Ok(AppCommand::None);
    }

    Ok(AppCommand::None)
}

fn shortcut_command(logical_key: &Key<SmolStr>, modifiers: ModifiersState) -> Option<AppCommand> {
    if modifiers.control_key() && modifiers.shift_key() {
        match logical_key {
            Key::Character(character) if character.eq_ignore_ascii_case("q") => {
                Some(AppCommand::EnterCopyMode)
            }
            Key::Character(character) if character.eq_ignore_ascii_case("t") => {
                Some(AppCommand::NewTab)
            }
            Key::Character(character) if character.eq_ignore_ascii_case("w") => {
                Some(AppCommand::CloseTab)
            }
            Key::Character(character) if character.eq_ignore_ascii_case("c") => {
                Some(AppCommand::CopySelection)
            }
            Key::Character(character) if character.eq_ignore_ascii_case("v") => {
                Some(AppCommand::PasteFromClipboard)
            }
            Key::Named(NamedKey::Tab) => Some(AppCommand::PreviousTab),
            _ => None,
        }
    } else if modifiers.control_key() {
        match logical_key {
            Key::Character(character) if character.eq_ignore_ascii_case("v") => {
                Some(AppCommand::PasteFromClipboard)
            }
            Key::Named(NamedKey::Tab) => Some(AppCommand::NextTab),
            _ => None,
        }
    } else {
        None
    }
}

fn copy_mode_command(
    logical_key: &Key<SmolStr>,
    modifiers: ModifiersState,
) -> Option<CopyModeCommand> {
    match logical_key {
        Key::Named(NamedKey::Escape) => Some(CopyModeCommand::Exit),
        Key::Named(NamedKey::Home) => Some(CopyModeCommand::Move(CopyModeMove::StartOfLine)),
        Key::Named(NamedKey::End) => Some(CopyModeCommand::Move(CopyModeMove::EndOfLine)),
        Key::Named(NamedKey::PageUp) => Some(CopyModeCommand::Move(CopyModeMove::PageUp)),
        Key::Named(NamedKey::PageDown) => Some(CopyModeCommand::Move(CopyModeMove::PageDown)),
        Key::Named(NamedKey::ArrowLeft) => Some(CopyModeCommand::Move(CopyModeMove::Left)),
        Key::Named(NamedKey::ArrowDown) => Some(CopyModeCommand::Move(CopyModeMove::Down)),
        Key::Named(NamedKey::ArrowUp) => Some(CopyModeCommand::Move(CopyModeMove::Up)),
        Key::Named(NamedKey::ArrowRight) => Some(CopyModeCommand::Move(CopyModeMove::Right)),
        Key::Character(character)
            if modifiers.control_key() && character.eq_ignore_ascii_case("c") =>
        {
            Some(CopyModeCommand::CopyAndExit)
        }
        Key::Character(character) if character == "h" => {
            Some(CopyModeCommand::Move(CopyModeMove::Left))
        }
        Key::Character(character) if character == "j" => {
            Some(CopyModeCommand::Move(CopyModeMove::Down))
        }
        Key::Character(character) if character == "k" => {
            Some(CopyModeCommand::Move(CopyModeMove::Up))
        }
        Key::Character(character) if character == "l" => {
            Some(CopyModeCommand::Move(CopyModeMove::Right))
        }
        Key::Character(character) if character == "w" => {
            Some(CopyModeCommand::Move(CopyModeMove::NextWord))
        }
        Key::Character(character) if character == "b" => {
            Some(CopyModeCommand::Move(CopyModeMove::PreviousWord))
        }
        Key::Character(character) if character == "e" => {
            Some(CopyModeCommand::Move(CopyModeMove::WordEnd))
        }
        Key::Character(character) if character == "0" => {
            Some(CopyModeCommand::Move(CopyModeMove::StartOfLine))
        }
        Key::Character(character)
            if character == "$" || modifiers.shift_key() && character == "4" =>
        {
            Some(CopyModeCommand::Move(CopyModeMove::EndOfLine))
        }
        Key::Character(character) if character == "g" => {
            Some(CopyModeCommand::Move(CopyModeMove::FirstLine))
        }
        Key::Character(character)
            if modifiers.shift_key() && character.eq_ignore_ascii_case("g") =>
        {
            Some(CopyModeCommand::Move(CopyModeMove::LastLine))
        }
        Key::Character(character) if character.eq_ignore_ascii_case("q") => {
            Some(CopyModeCommand::Exit)
        }
        Key::Character(character) if character == "y" => Some(CopyModeCommand::CopyAndExit),
        Key::Character(character)
            if modifiers.shift_key() && character.eq_ignore_ascii_case("v") =>
        {
            Some(CopyModeCommand::ToggleLineSelection)
        }
        Key::Character(character) if character == "v" => Some(CopyModeCommand::ToggleCellSelection),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{AppCommand, CopyModeCommand, copy_mode_command, shortcut_command};
    use crate::app::copy::CopyModeMove;
    use winit::keyboard::{Key, ModifiersState, SmolStr};

    #[test]
    fn ctrl_shift_t_opens_new_tab() {
        let command = shortcut_command(
            &Key::Character(SmolStr::new("T")),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );

        assert!(matches!(command, Some(AppCommand::NewTab)));
    }

    #[test]
    fn ctrl_shift_w_closes_active_tab() {
        let command = shortcut_command(
            &Key::Character(SmolStr::new("W")),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );

        assert!(matches!(command, Some(AppCommand::CloseTab)));
    }

    #[test]
    fn ctrl_tab_switch_next_tab() {
        let command: Option<AppCommand> = shortcut_command(
            &Key::Named(winit::keyboard::NamedKey::Tab),
            ModifiersState::CONTROL,
        );

        assert!(matches!(command, Some(AppCommand::NextTab)));
    }

    #[test]
    fn ctrl_tab_switch_previous_tab() {
        let command: Option<AppCommand> = shortcut_command(
            &Key::Named(winit::keyboard::NamedKey::Tab),
            ModifiersState::SHIFT | ModifiersState::CONTROL,
        );

        assert!(matches!(command, Some(AppCommand::PreviousTab)));
    }

    #[test]
    fn ctrl_shift_q_enters_copy_mode() {
        let command = shortcut_command(
            &Key::Character(SmolStr::new("Q")),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );

        assert!(matches!(command, Some(AppCommand::EnterCopyMode)));
    }

    #[test]
    fn ctrl_v_pastes_from_clipboard() {
        let command = shortcut_command(&Key::Character(SmolStr::new("v")), ModifiersState::CONTROL);

        assert!(matches!(command, Some(AppCommand::PasteFromClipboard)));
    }

    #[test]
    fn ctrl_shift_v_pastes_from_clipboard() {
        let command = shortcut_command(
            &Key::Character(SmolStr::new("V")),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );

        assert!(matches!(command, Some(AppCommand::PasteFromClipboard)));
    }

    #[test]
    fn ctrl_shift_c_copies_selection() {
        let command = shortcut_command(
            &Key::Character(SmolStr::new("C")),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );

        assert!(matches!(command, Some(AppCommand::CopySelection)));
    }

    #[test]
    fn copy_mode_y_copies_and_exits() {
        let command =
            copy_mode_command(&Key::Character(SmolStr::new("y")), ModifiersState::empty());

        assert!(matches!(command, Some(CopyModeCommand::CopyAndExit)));
    }

    #[test]
    fn copy_mode_ctrl_c_copies_and_exits() {
        let command =
            copy_mode_command(&Key::Character(SmolStr::new("c")), ModifiersState::CONTROL);

        assert!(matches!(command, Some(CopyModeCommand::CopyAndExit)));
    }

    #[test]
    fn copy_mode_v_toggles_cell_selection() {
        let command =
            copy_mode_command(&Key::Character(SmolStr::new("v")), ModifiersState::empty());

        assert!(matches!(
            command,
            Some(CopyModeCommand::ToggleCellSelection)
        ));
    }

    #[test]
    fn copy_mode_shift_v_toggles_line_selection() {
        let command = copy_mode_command(&Key::Character(SmolStr::new("V")), ModifiersState::SHIFT);

        assert!(matches!(
            command,
            Some(CopyModeCommand::ToggleLineSelection)
        ));
    }

    #[test]
    fn copy_mode_h_moves_left() {
        let command =
            copy_mode_command(&Key::Character(SmolStr::new("h")), ModifiersState::empty());

        assert!(matches!(
            command,
            Some(CopyModeCommand::Move(CopyModeMove::Left))
        ));
    }

    #[test]
    fn copy_mode_word_keys_move_by_words() {
        assert!(matches!(
            copy_mode_command(&Key::Character(SmolStr::new("w")), ModifiersState::empty()),
            Some(CopyModeCommand::Move(CopyModeMove::NextWord))
        ));
        assert!(matches!(
            copy_mode_command(&Key::Character(SmolStr::new("b")), ModifiersState::empty()),
            Some(CopyModeCommand::Move(CopyModeMove::PreviousWord))
        ));
        assert!(matches!(
            copy_mode_command(&Key::Character(SmolStr::new("e")), ModifiersState::empty()),
            Some(CopyModeCommand::Move(CopyModeMove::WordEnd))
        ));
    }

    #[test]
    fn copy_mode_line_edge_keys_move_to_line_edges() {
        assert!(matches!(
            copy_mode_command(&Key::Character(SmolStr::new("0")), ModifiersState::empty()),
            Some(CopyModeCommand::Move(CopyModeMove::StartOfLine))
        ));
        assert!(matches!(
            copy_mode_command(&Key::Character(SmolStr::new("$")), ModifiersState::SHIFT),
            Some(CopyModeCommand::Move(CopyModeMove::EndOfLine))
        ));
    }
}
