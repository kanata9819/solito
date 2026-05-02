mod csi;
mod esc;
mod osc;

pub use csi::CsiMessage;
pub use esc::EscMessage;
pub use osc::OscMessage;

use csi::decode_csi;
use esc::decode_esc;
use osc::decode_osc;

pub struct DecodedEvent {
    pub csi: Option<CsiMessage>,
    pub osc: Option<OscMessage>,
    pub esc: Option<EscMessage>,
}

pub enum VteEvent<'a> {
    Csi {
        params: &'a vte::Params,
        intermediates: &'a [u8],
        ignore: bool,
        action: char,
    },
    Osc {
        params: &'a [&'a [u8]],
        bell_terminated: bool,
    },
    Esc {
        intermediates: &'a [u8],
        ignore: bool,
        byte: u8,
    },
}

pub fn decode(event: VteEvent) -> Option<DecodedEvent> {
    match event {
        VteEvent::Csi {
            params,
            intermediates,
            ignore,
            action,
        } => match decode_csi(params, intermediates, ignore, action) {
            Some(csi) => Some(DecodedEvent {
                csi: Some(csi),
                osc: None,
                esc: None,
            }),
            None => None,
        },
        VteEvent::Osc {
            params,
            bell_terminated,
        } => match decode_osc(params, bell_terminated) {
            Some(osc) => Some(DecodedEvent {
                csi: None,
                osc: Some(osc),
                esc: None,
            }),
            None => None,
        },
        VteEvent::Esc {
            intermediates,
            ignore,
            byte,
        } => match decode_esc(intermediates, ignore, byte) {
            Some(esc) => Some(DecodedEvent {
                csi: None,
                osc: None,
                esc: Some(esc),
            }),
            None => None,
        },
    }
}
