use std::fmt::{self, Display};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscMessage {
    SetIconName(String),
    SetWindowTitle(String),
    SetIconAndWindowTitle(String),
    Unknown {
        params: Vec<Vec<u8>>,
        bell_terminated: bool,
    },
}

impl Display for OscMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OscMessage::SetIconName(value) => write!(f, "OSC: SetIconName({:?})", value),
            OscMessage::SetWindowTitle(value) => write!(f, "OSC: SetWindowTitle({:?})", value),
            OscMessage::SetIconAndWindowTitle(value) => {
                write!(f, "OSC: SetIconAndWindowTitle({:?})", value)
            }
            OscMessage::Unknown {
                params,
                bell_terminated,
            } => write!(
                f,
                "OSC: Unknown(params={:?}, bell_terminated={})",
                params, bell_terminated
            ),
        }
    }
}

pub fn decode_osc(params: &[&[u8]], bell_terminated: bool) -> Option<OscMessage> {
    let code: &&[u8] = params.first()?;
    let value: String = params
        .get(1)
        .map(|value| String::from_utf8_lossy(value).into_owned())
        .unwrap_or_default();

    Some(match *code {
        b"0" => OscMessage::SetIconAndWindowTitle(value),
        b"1" => OscMessage::SetIconName(value),
        b"2" => OscMessage::SetWindowTitle(value),
        _ => OscMessage::Unknown {
            params: params.iter().map(|param| param.to_vec()).collect(),
            bell_terminated,
        },
    })
}
