use vte::Params;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraseMode {
    ToEnd,
    ToStart,
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
    CursorPosition {
        row: u16,
        col: u16,
    },
    EraseDisplay(EraseMode),
    EraseLine(EraseMode),
    EraseCharacters(u16),
    DeleteCharacters(u16),
    ScrollUp(u16),
    ScrollDown(u16),
    SaveCursor,
    RestoreCursor,
    SelectGraphicRendition(Vec<u16>),
    DeviceStatusReport(u16),
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

    match action {
        'A' => CsiMessage::CursorUp(amount),
        'B' => CsiMessage::CursorDown(amount),
        'C' => CsiMessage::CursorForward(amount),
        'D' => CsiMessage::CursorBackward(amount),
        'E' => CsiMessage::CursorNextLine(amount),
        'F' => CsiMessage::CursorPreviousLine(amount),
        'G' => CsiMessage::CursorHorizontalAbsolute(amount),
        'H' | 'f' => CsiMessage::CursorPosition {
            row: param(params, 0, 1),
            col: param(params, 1, 1),
        },
        'J' => CsiMessage::EraseDisplay(EraseMode::from_csi_mode(param(params, 0, 0))),
        'K' => CsiMessage::EraseLine(EraseMode::from_csi_mode(param(params, 0, 0))),
        'X' if !ignore && intermediates.is_empty() => CsiMessage::EraseCharacters(amount),
        'P' => CsiMessage::DeleteCharacters(amount),
        'S' => CsiMessage::ScrollUp(amount),
        'T' => CsiMessage::ScrollDown(amount),
        'm' => CsiMessage::SelectGraphicRendition(graphics_rendition_params(params)),
        'n' => CsiMessage::DeviceStatusReport(param(params, 0, 0)),
        's' => CsiMessage::SaveCursor,
        'u' => CsiMessage::RestoreCursor,
        'h' if param(params, 0, 0) == 25 => CsiMessage::ShowCursor,
        'l' if param(params, 0, 0) == 25 => CsiMessage::HideCursor,
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

fn param(params: &Params, index: usize, default: u16) -> u16 {
    params
        .iter()
        .nth(index)
        .and_then(|param| param.iter().next().copied())
        .filter(|value| *value > 0)
        .unwrap_or(default)
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
    use super::{CsiMessage, decode_csi};
    use vte::Params;

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
}
