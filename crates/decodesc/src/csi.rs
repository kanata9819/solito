use std::fmt::{self, Display};
use vte::Params;

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
    EraseDisplay(u16),
    EraseLine(u16),
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

impl Display for CsiMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CsiMessage::CursorUp(n) => write!(f, "CSI: CursorUp({})", n),
            CsiMessage::CursorDown(n) => write!(f, "CSI: CursorDown({})", n),
            CsiMessage::CursorForward(n) => write!(f, "CSI: CursorForward({})", n),
            CsiMessage::CursorBackward(n) => write!(f, "CSI: CursorBackward({})", n),
            CsiMessage::CursorNextLine(n) => write!(f, "CSI: CursorNextLine({})", n),
            CsiMessage::CursorPreviousLine(n) => write!(f, "CSI: CursorPreviousLine({})", n),
            CsiMessage::CursorHorizontalAbsolute(n) => {
                write!(f, "CSI: CursorHorizontalAbsolute({})", n)
            }
            CsiMessage::CursorPosition { row, col } => {
                write!(f, "CSI: CursorPosition(row={}, col={})", row, col)
            }
            CsiMessage::EraseDisplay(mode) => write!(f, "CSI: EraseDisplay({})", mode),
            CsiMessage::EraseLine(mode) => write!(f, "CSI: EraseLine({})", mode),
            CsiMessage::ScrollUp(n) => write!(f, "CSI: ScrollUp({})", n),
            CsiMessage::ScrollDown(n) => write!(f, "CSI: ScrollDown({})", n),
            CsiMessage::SaveCursor => write!(f, "CSI: SaveCursor"),
            CsiMessage::RestoreCursor => write!(f, "CSI: RestoreCursor"),
            CsiMessage::SelectGraphicRendition(params) => {
                write!(f, "CSI: SelectGraphicRendition({:?})", params)
            }
            CsiMessage::DeviceStatusReport(mode) => {
                write!(f, "CSI: DeviceStatusReport({})", mode)
            }
            CsiMessage::ShowCursor => write!(f, "CSI: ShowCursor"),
            CsiMessage::HideCursor => write!(f, "CSI: HideCursor"),
            CsiMessage::Unknown {
                params,
                intermediates,
                ignore,
                action,
            } => write!(
                f,
                "CSI: Unknown(params={:?}, intermediates={:?}, ignore={}, action={})",
                params, intermediates, ignore, action
            ),
        }
    }
}

pub fn decode_csi(
    params: &Params,
    intermediates: &[u8],
    ignore: bool,
    action: char,
) -> Option<CsiMessage> {
    let amount: u16 = param(params, 0, 1);

    Some(match action {
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
        'J' => CsiMessage::EraseDisplay(param(params, 0, 0)),
        'K' => CsiMessage::EraseLine(param(params, 0, 0)),
        'S' => CsiMessage::ScrollUp(amount),
        'T' => CsiMessage::ScrollDown(amount),
        'm' => CsiMessage::SelectGraphicRendition(params_to_vec(params)),
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
    })
}

fn param(params: &Params, index: usize, default: u16) -> u16 {
    params.iter().nth(index).map(|param| param[0]).unwrap_or(default)
}

fn params_to_vec(params: &Params) -> Vec<u16> {
    params.iter().flat_map(|param| param.iter().copied()).collect()
}
