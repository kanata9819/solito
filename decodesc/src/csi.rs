use vte::Params;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraseMode {
    ToEnd,
    ToStart,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabClearMode {
    Current,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsiMessage {
    CursorUp(u16),
    CursorDown(u16),
    CursorForward(u16),
    CursorBackward(u16),
    CursorNextLine(u16),
    CursorPreviousLine(u16),
    CursorHorizontalAbsolute(u16),
    CursorVerticalAbsolute(u16),
    CursorHorizontalRelative(u16),
    CursorVerticalRelative(u16),
    CursorPosition {
        row: u16,
        col: u16,
    },
    EraseDisplay(EraseMode),
    EraseLine(EraseMode),
    EraseCharacters(u16),
    DeleteCharacters(u16),
    InsertBlankCharacters(u16),
    InsertLines(u16),
    DeleteLines(u16),
    ScrollUp(u16),
    ScrollDown(u16),
    SetScrollRegion {
        top: u16,
        bottom: Option<u16>,
    },
    CursorForwardTabulation(u16),
    CursorBackwardTabulation(u16),
    TabClear(TabClearMode),
    RepeatPrecedingCharacter(u16),
    SaveCursor,
    RestoreCursor,
    SelectGraphicRendition(Vec<u16>),
    DeviceStatusReport(u16),
    SetMode {
        private: bool,
        modes: Vec<u16>,
    },
    ResetMode {
        private: bool,
        modes: Vec<u16>,
    },
    ShowCursor,
    HideCursor,
    Unknown {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        ignore: bool,
        action: char,
    },
}

#[must_use]
pub fn decode_csi(params: &Params, intermediates: &[u8], ignore: bool, action: char) -> CsiMessage {
    let amount = param(params, 0, 1);
    let standard = !ignore && intermediates.is_empty();
    let private = !ignore && intermediates == b"?";
    let modes = params_to_vec(params);

    match (standard, private, action) {
        (true, _, 'A') => CsiMessage::CursorUp(amount),
        (true, _, 'B') => CsiMessage::CursorDown(amount),
        (true, _, 'C') => CsiMessage::CursorForward(amount),
        (true, _, 'D') => CsiMessage::CursorBackward(amount),
        (true, _, 'E') => CsiMessage::CursorNextLine(amount),
        (true, _, 'F') => CsiMessage::CursorPreviousLine(amount),
        (true, _, 'G') => CsiMessage::CursorHorizontalAbsolute(amount),
        (true, _, 'a') => CsiMessage::CursorHorizontalRelative(amount),
        (true, _, 'd') => CsiMessage::CursorVerticalAbsolute(amount),
        (true, _, 'e') => CsiMessage::CursorVerticalRelative(amount),
        (true, _, 'H' | 'f') => CsiMessage::CursorPosition {
            row: param(params, 0, 1),
            col: param(params, 1, 1),
        },
        (true, _, 'J') => CsiMessage::EraseDisplay(EraseMode::from_csi_mode(param(params, 0, 0))),
        (true, _, 'K') => CsiMessage::EraseLine(EraseMode::from_csi_mode(param(params, 0, 0))),
        (true, _, 'X') => CsiMessage::EraseCharacters(amount),
        (true, _, 'P') => CsiMessage::DeleteCharacters(amount),
        (true, _, '@') => CsiMessage::InsertBlankCharacters(amount),
        (true, _, 'L') => CsiMessage::InsertLines(amount),
        (true, _, 'M') => CsiMessage::DeleteLines(amount),
        (true, _, 'S') => CsiMessage::ScrollUp(amount),
        (true, _, 'T') => CsiMessage::ScrollDown(amount),
        (true, _, 'r') => CsiMessage::SetScrollRegion {
            top: param(params, 0, 1),
            bottom: param_optional(params, 1),
        },
        (true, _, 'I') => CsiMessage::CursorForwardTabulation(amount),
        (true, _, 'Z') => CsiMessage::CursorBackwardTabulation(amount),
        (true, _, 'g') => CsiMessage::TabClear(TabClearMode::from_csi_mode(param(params, 0, 0))),
        (true, _, 'b') => CsiMessage::RepeatPrecedingCharacter(amount),
        (true, _, 'm') => CsiMessage::SelectGraphicRendition(graphics_rendition_params(params)),
        (true, _, 'n') => CsiMessage::DeviceStatusReport(param(params, 0, 0)),
        (true, _, 's') => CsiMessage::SaveCursor,
        (true, _, 'u') => CsiMessage::RestoreCursor,
        (_, true, 'h') if modes == [25] => CsiMessage::ShowCursor,
        (_, true, 'l') if modes == [25] => CsiMessage::HideCursor,
        (true, _, 'h') => CsiMessage::SetMode {
            private: false,
            modes,
        },
        (true, _, 'l') => CsiMessage::ResetMode {
            private: false,
            modes,
        },
        (_, true, 'h') => CsiMessage::SetMode {
            private: true,
            modes,
        },
        (_, true, 'l') => CsiMessage::ResetMode {
            private: true,
            modes,
        },
        _ => CsiMessage::Unknown {
            params: params_to_vec(params),
            intermediates: intermediates.to_vec(),
            ignore,
            action,
        },
    }
}

impl EraseMode {
    fn from_csi_mode(mode: u16) -> Self {
        match mode {
            1 => Self::ToStart,
            2 => Self::All,
            _ => Self::ToEnd,
        }
    }
}

impl TabClearMode {
    fn from_csi_mode(mode: u16) -> Self {
        if mode == 3 { Self::All } else { Self::Current }
    }
}

fn param(params: &Params, index: usize, default: u16) -> u16 {
    params
        .iter()
        .nth(index)
        .and_then(|param| param.iter().next().copied())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn param_optional(params: &Params, index: usize) -> Option<u16> {
    params
        .iter()
        .nth(index)
        .and_then(|param| param.iter().next().copied())
        .filter(|value| *value > 0)
}

fn graphics_rendition_params(params: &Params) -> Vec<u16> {
    let collected = params_to_vec(params);

    if collected.is_empty() {
        vec![0]
    } else {
        collected
    }
}

fn params_to_vec(params: &Params) -> Vec<u16> {
    params
        .iter()
        .flat_map(|param| param.iter().copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{CsiMessage, TabClearMode, decode_csi};
    use vte::{Params, Parser, Perform};

    #[derive(Default)]
    struct CsiCollector(Option<CsiMessage>);

    impl Perform for CsiCollector {
        fn csi_dispatch(
            &mut self,
            params: &Params,
            intermediates: &[u8],
            ignore: bool,
            action: char,
        ) {
            self.0 = Some(decode_csi(params, intermediates, ignore, action));
        }
    }

    fn decode(sequence: &[u8]) -> CsiMessage {
        let mut parser = Parser::new();
        let mut collector = CsiCollector::default();
        parser.advance(&mut collector, sequence);
        collector.0.expect("CSI sequence must dispatch")
    }

    #[test]
    fn empty_cursor_up_uses_default_amount() {
        let message = decode_csi(&Params::default(), &[], false, 'A');

        assert_eq!(message, CsiMessage::CursorUp(1));
    }

    #[test]
    fn empty_erase_characters_uses_default_amount() {
        let message = decode_csi(&Params::default(), &[], false, 'X');

        assert_eq!(message, CsiMessage::EraseCharacters(1));
    }

    #[test]
    fn private_erase_characters_remains_unknown() {
        let message = decode_csi(&Params::default(), b"?", false, 'X');

        assert_eq!(
            message,
            CsiMessage::Unknown {
                params: Vec::new(),
                intermediates: vec![b'?'],
                ignore: false,
                action: 'X',
            }
        );
    }

    #[test]
    fn decodes_editing_and_scrolling_commands() {
        assert_eq!(decode(b"\x1b[2@"), CsiMessage::InsertBlankCharacters(2));
        assert_eq!(decode(b"\x1b[3L"), CsiMessage::InsertLines(3));
        assert_eq!(decode(b"\x1b[4M"), CsiMessage::DeleteLines(4));
        assert_eq!(decode(b"\x1b[2S"), CsiMessage::ScrollUp(2));
        assert_eq!(decode(b"\x1b[3T"), CsiMessage::ScrollDown(3));
        assert_eq!(
            decode(b"\x1b[3;12r"),
            CsiMessage::SetScrollRegion {
                top: 3,
                bottom: Some(12),
            }
        );
    }

    #[test]
    fn decodes_cursor_and_tabulation_commands() {
        assert_eq!(decode(b"\x1b[2a"), CsiMessage::CursorHorizontalRelative(2));
        assert_eq!(decode(b"\x1b[3d"), CsiMessage::CursorVerticalAbsolute(3));
        assert_eq!(decode(b"\x1b[4e"), CsiMessage::CursorVerticalRelative(4));
        assert_eq!(decode(b"\x1b[2I"), CsiMessage::CursorForwardTabulation(2));
        assert_eq!(decode(b"\x1b[3Z"), CsiMessage::CursorBackwardTabulation(3));
        assert_eq!(decode(b"\x1b[3g"), CsiMessage::TabClear(TabClearMode::All));
        assert_eq!(decode(b"\x1b[5b"), CsiMessage::RepeatPrecedingCharacter(5));
    }

    #[test]
    fn decodes_standard_and_private_modes() {
        assert_eq!(
            decode(b"\x1b[4h"),
            CsiMessage::SetMode {
                private: false,
                modes: vec![4],
            }
        );
        assert_eq!(
            decode(b"\x1b[?1049l"),
            CsiMessage::ResetMode {
                private: true,
                modes: vec![1049],
            }
        );
        assert_eq!(decode(b"\x1b[?25h"), CsiMessage::ShowCursor);
    }
}
