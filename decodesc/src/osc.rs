#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscMessage {
    SetIconName(String),
    SetWindowTitle(String),
    SetIconAndWindowTitle(String),
    SetCursorColor([u8; 4]),
    ResetCursorColor,
    Unknown {
        params: Vec<Vec<u8>>,
        bell_terminated: bool,
    },
}

#[must_use]
pub fn decode_osc(params: &[&[u8]], bell_terminated: bool) -> Option<OscMessage> {
    let code = params.first()?;
    let value = params
        .get(1)
        .map(|value| String::from_utf8_lossy(value).into_owned())
        .unwrap_or_default();

    Some(match *code {
        b"0" => OscMessage::SetIconAndWindowTitle(value),
        b"1" => OscMessage::SetIconName(value),
        b"2" => OscMessage::SetWindowTitle(value),
        b"12" => parse_color(params.get(1).copied())
            .map(OscMessage::SetCursorColor)
            .unwrap_or_else(|| OscMessage::Unknown {
                params: params.iter().map(|param| param.to_vec()).collect(),
                bell_terminated,
            }),
        b"112" => OscMessage::ResetCursorColor,
        _ => OscMessage::Unknown {
            params: params.iter().map(|param| param.to_vec()).collect(),
            bell_terminated,
        },
    })
}

fn parse_color(value: Option<&[u8]>) -> Option<[u8; 4]> {
    let value = std::str::from_utf8(value?).ok()?;

    parse_hash_color(value).or_else(|| parse_rgb_color(value))
}

fn parse_hash_color(value: &str) -> Option<[u8; 4]> {
    let value = value.strip_prefix('#')?;
    if value.len() != 6 {
        return None;
    }

    Some([
        u8::from_str_radix(&value[0..2], 16).ok()?,
        u8::from_str_radix(&value[2..4], 16).ok()?,
        u8::from_str_radix(&value[4..6], 16).ok()?,
        255,
    ])
}

fn parse_rgb_color(value: &str) -> Option<[u8; 4]> {
    let value = value.strip_prefix("rgb:")?;
    let mut parts = value.split('/');

    Some([
        parse_rgb_component(parts.next()?)?,
        parse_rgb_component(parts.next()?)?,
        parse_rgb_component(parts.next()?)?,
        255,
    ])
}

fn parse_rgb_component(value: &str) -> Option<u8> {
    if value.is_empty() || value.len() > 4 {
        return None;
    }

    let digit_count = value.len();
    let value = u16::from_str_radix(value, 16).ok()?;
    let max = (1_u32 << (digit_count * 4)) - 1;

    Some(((u32::from(value) * 255 + (max / 2)) / max) as u8)
}

#[cfg(test)]
mod tests {
    use super::{OscMessage, decode_osc};

    #[test]
    fn decodes_hash_cursor_color() {
        let message = decode_osc(&[b"12", b"#00ff80"], true);

        assert_eq!(
            message,
            Some(OscMessage::SetCursorColor([0, 255, 128, 255]))
        );
    }

    #[test]
    fn decodes_rgb_cursor_color() {
        let message = decode_osc(&[b"12", b"rgb:ffff/8000/0000"], true);

        assert_eq!(
            message,
            Some(OscMessage::SetCursorColor([255, 128, 0, 255]))
        );
    }

    #[test]
    fn decodes_cursor_color_reset() {
        let message = decode_osc(&[b"112"], true);

        assert_eq!(message, Some(OscMessage::ResetCursorColor));
    }
}
