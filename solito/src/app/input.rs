//! Translate keyboard input into side-effect-free application commands.

use solito_terminal::TerminalSize;
use winit::{
    dpi::PhysicalSize,
    event::ElementState,
    keyboard::{Key, ModifiersState, NamedKey, SmolStr},
};

use crate::app::copy::CopyModeMove;

#[derive(Debug, PartialEq)]
pub(super) enum AppCommand {
    Noop,
    SendTerminalInput(Vec<u8>),
    EnterCopyMode,
    CopyMode(CopyModeCommand),
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
    CopySelection,
    PasteFromClipboard,
    Resize {
        window_size: PhysicalSize<u32>,
        terminal_size: TerminalSize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum CopyModeCommand {
    Move(CopyModeMove),
    ToggleCellSelection,
    ToggleLineSelection,
    CopyAndExit,
    Exit,
}

pub(super) fn handle_key(
    text: Option<SmolStr>,
    logical_key: &Key<SmolStr>,
    key_state: ElementState,
    modifiers: ModifiersState,
    copy_mode_active: bool,
) -> AppCommand {
    const ENTER: &[u8; 1] = b"\r";
    const BACKSPACE: &[u8; 1] = b"\x7f";
    const TAB: &[u8; 1] = b"\t";
    const ESCAPE: &[u8; 1] = b"\x1b";
    const ARROWUP: &[u8; 3] = b"\x1b[A";
    const ARROWDOWN: &[u8; 3] = b"\x1b[B";
    const ARROWRIGHT: &[u8; 3] = b"\x1b[C";
    const ARROWLEFT: &[u8; 3] = b"\x1b[D";

    if key_state != ElementState::Pressed {
        return AppCommand::Noop;
    }

    if copy_mode_active {
        return copy_mode_command(logical_key, modifiers)
            .map_or(AppCommand::Noop, AppCommand::CopyMode);
    }

    if let Some(command) = shortcut_command(logical_key, modifiers) {
        return command;
    }

    let bytes = match logical_key {
        Key::Named(NamedKey::Enter) => Some(ENTER.to_vec()),
        Key::Named(NamedKey::Backspace) => Some(BACKSPACE.to_vec()),
        Key::Named(NamedKey::Tab) => Some(TAB.to_vec()),
        Key::Named(NamedKey::Escape) => Some(ESCAPE.to_vec()),
        Key::Named(NamedKey::ArrowUp) => Some(ARROWUP.to_vec()),
        Key::Named(NamedKey::ArrowDown) => Some(ARROWDOWN.to_vec()),
        Key::Named(NamedKey::ArrowRight) => Some(ARROWRIGHT.to_vec()),
        Key::Named(NamedKey::ArrowLeft) => Some(ARROWLEFT.to_vec()),
        Key::Character(character) if modifiers.control_key() => {
            control_character(character).map(|byte| vec![byte])
        }
        _ => text.map(|text| text.as_bytes().to_vec()),
    };

    bytes.map_or(AppCommand::Noop, AppCommand::SendTerminalInput)
}

fn control_character(character: &str) -> Option<u8> {
    let [byte] = character.as_bytes() else {
        return None;
    };

    byte.is_ascii_alphabetic()
        .then(|| byte.to_ascii_uppercase() - b'@')
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
    use super::{AppCommand, CopyModeCommand, copy_mode_command, handle_key, shortcut_command};
    use crate::app::copy::CopyModeMove;
    use winit::{
        event::ElementState,
        keyboard::{Key, ModifiersState, NamedKey, SmolStr},
    };

    #[test]
    fn enter_becomes_terminal_input() {
        let command = handle_key(
            None,
            &Key::Named(NamedKey::Enter),
            ElementState::Pressed,
            ModifiersState::empty(),
            false,
        );

        assert_eq!(command, AppCommand::SendTerminalInput(b"\r".to_vec()));
    }

    #[test]
    fn text_becomes_utf8_terminal_input() {
        let command = handle_key(
            Some(SmolStr::new("あ")),
            &Key::Character(SmolStr::new("あ")),
            ElementState::Pressed,
            ModifiersState::empty(),
            false,
        );

        assert_eq!(
            command,
            AppCommand::SendTerminalInput("あ".as_bytes().to_vec())
        );
    }

    #[test]
    fn ctrl_c_becomes_interrupt_byte() {
        let command = handle_key(
            None,
            &Key::Character(SmolStr::new("c")),
            ElementState::Pressed,
            ModifiersState::CONTROL,
            false,
        );

        assert_eq!(command, AppCommand::SendTerminalInput(b"\x03".to_vec()));
    }

    #[test]
    fn ctrl_e_becomes_end_of_line_byte() {
        let command = handle_key(
            None,
            &Key::Character(SmolStr::new("e")),
            ElementState::Pressed,
            ModifiersState::CONTROL,
            false,
        );

        assert_eq!(command, AppCommand::SendTerminalInput(b"\x05".to_vec()));
    }

    #[test]
    fn released_key_is_ignored() {
        let command = handle_key(
            Some(SmolStr::new("a")),
            &Key::Character(SmolStr::new("a")),
            ElementState::Released,
            ModifiersState::empty(),
            false,
        );

        assert_eq!(command, AppCommand::Noop);
    }

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
        let command = shortcut_command(
            &Key::Named(winit::keyboard::NamedKey::Tab),
            ModifiersState::CONTROL,
        );

        assert!(matches!(command, Some(AppCommand::NextTab)));
    }

    #[test]
    fn ctrl_tab_switch_previous_tab() {
        let command = shortcut_command(
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
