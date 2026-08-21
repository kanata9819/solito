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

#[must_use]
pub fn decode_esc(intermediates: &[u8], ignore: bool, byte: u8) -> EscMessage {
    if !ignore && intermediates.is_empty() {
        return match byte {
            b'7' => EscMessage::SaveCursor,
            b'8' => EscMessage::RestoreCursor,
            b'D' => EscMessage::Index,
            b'E' => EscMessage::NextLine,
            b'M' => EscMessage::ReverseIndex,
            b'c' => EscMessage::Reset,
            _ => EscMessage::Unknown {
                intermediates: Vec::new(),
                ignore: false,
                byte,
            },
        };
    }

    EscMessage::Unknown {
        intermediates: intermediates.to_vec(),
        ignore,
        byte,
    }
}

#[cfg(test)]
mod tests {
    use super::{EscMessage, decode_esc};

    #[test]
    fn decodes_save_cursor() {
        assert_eq!(decode_esc(&[], false, b'7'), EscMessage::SaveCursor);
    }

    #[test]
    fn preserves_charset_selection_as_unknown() {
        assert_eq!(
            decode_esc(b"(", false, b'D'),
            EscMessage::Unknown {
                intermediates: vec![b'('],
                ignore: false,
                byte: b'D',
            }
        );
    }
}
