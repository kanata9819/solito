use super::esc_sequence::Csi;
use crate::terminal::screen::cursor::CursorPosition;
use vte::Params;

#[derive(Clone, Copy)]
pub enum EraseMode {
    ToEnd,
    ToStart,
    All,
}

pub enum CsiCommand {
    MoveCursorUp(usize),
    MoveCursorDown(usize),
    MoveCursorForward(usize),
    MoveCursorBackward(usize),
    MoveCursorToColumn(usize),
    MoveCursorTo(CursorPosition),
    EraseDisplay(EraseMode),
    EraseLine(EraseMode),
    DeleteCharacters(usize),
    SaveCursorPosition,
    RestoreCursorPosition,
    SetGraphicsRendition(Vec<usize>),
    Unsupported,
}

impl CsiCommand {
    pub fn parse(params: &Params, action: char) -> Self {
        match action {
            Csi::CURSOR_UP => Self::MoveCursorUp(param_or_default(params, 0, 1)),
            Csi::CURSOR_DOWN => Self::MoveCursorDown(param_or_default(params, 0, 1)),
            Csi::CURSOR_FORWARD => Self::MoveCursorForward(param_or_default(params, 0, 1)),
            Csi::CURSOR_BACKWARD => Self::MoveCursorBackward(param_or_default(params, 0, 1)),
            Csi::CURSOR_HORIZONTAL_ABSOLUTE => {
                Self::MoveCursorToColumn(param_or_default(params, 0, 1).saturating_sub(1))
            }
            Csi::CURSOR_POSITION | Csi::HORIZONTAL_VERTICAL_POSITION => {
                Self::MoveCursorTo(CursorPosition {
                    row: param_or_default(params, 0, 1).saturating_sub(1),
                    col: param_or_default(params, 1, 1).saturating_sub(1),
                })
            }
            Csi::ERASE_IN_DISPLAY => {
                Self::EraseDisplay(EraseMode::from_csi_mode(param_or_default(params, 0, 0)))
            }
            Csi::ERASE_IN_LINE => {
                Self::EraseLine(EraseMode::from_csi_mode(param_or_default(params, 0, 0)))
            }
            Csi::DELETE_CHARACTER => Self::DeleteCharacters(param_or_default(params, 0, 1)),
            Csi::SAVE_CURSOR_POSITION => Self::SaveCursorPosition,
            Csi::RESTORE_CURSOR_POSITION => Self::RestoreCursorPosition,
            Csi::SELECT_GRAPHIC_RENDITION => {
                Self::SetGraphicsRendition(graphics_rendition_params(params))
            }
            _ => {
                let _: char = action;
                Self::Unsupported
            }
        }
    }
}

fn graphics_rendition_params(params: &Params) -> Vec<usize> {
    let collected: Vec<usize> = params
        .iter()
        .flat_map(|param| param.iter().copied())
        .map(usize::from)
        .collect();

    if collected.is_empty() {
        vec![0]
    } else {
        collected
    }
}

impl EraseMode {
    fn from_csi_mode(mode: usize) -> Self {
        match mode {
            1 => Self::ToStart,
            2 => Self::All,
            _ => Self::ToEnd,
        }
    }
}

pub fn param_or_default(params: &Params, idx: usize, default: usize) -> usize {
    params
        .iter()
        // Pick the idx-th parameter group (if present).
        .nth(idx)
        // Each group may have sub-parameters; this command parser uses only the first one.
        .and_then(|param| param.iter().next().copied())
        // Normalize into usize for internal command values.
        .map(usize::from)
        // In CSI, `0` is commonly treated the same as omitted for these commands.
        .filter(|value| *value > 0)
        // Fall back to command-specific default.
        .unwrap_or(default)
}
