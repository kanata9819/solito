use std::fmt::{self, Display};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscMessage {
    SaveCursor,
    RestoreCursor,
    Index,
    NextLine,
    ReverseIndex,
    Reset,
    Unknown {
        intermediates: Vec<u8>,
        ignore: bool,
        byte: u8,
    },
}

impl Display for EscMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EscMessage::SaveCursor => write!(f, "ESC: SaveCursor"),
            EscMessage::RestoreCursor => write!(f, "ESC: RestoreCursor"),
            EscMessage::Index => write!(f, "ESC: Index"),
            EscMessage::NextLine => write!(f, "ESC: NextLine"),
            EscMessage::ReverseIndex => write!(f, "ESC: ReverseIndex"),
            EscMessage::Reset => write!(f, "ESC: Reset"),
            EscMessage::Unknown {
                intermediates,
                ignore,
                byte,
            } => write!(
                f,
                "ESC: Unknown(intermediates={:?}, ignore={}, byte={})",
                intermediates, ignore, byte
            ),
        }
    }
}

pub fn decode_esc(intermediates: &[u8], ignore: bool, byte: u8) -> Option<EscMessage> {
    Some(match byte {
        b'7' => EscMessage::SaveCursor,
        b'8' => EscMessage::RestoreCursor,
        b'D' => EscMessage::Index,
        b'E' => EscMessage::NextLine,
        b'M' => EscMessage::ReverseIndex,
        b'c' => EscMessage::Reset,
        _ => EscMessage::Unknown {
            intermediates: intermediates.to_vec(),
            ignore,
            byte,
        },
    })
}
