#[derive(Default)]
pub struct Params {
    subparams: [u8; 32],
    params: [u16; 32],
    current_subparams: u8,
    len: usize,
}

pub enum VteEvent<'a> {
    Csi {
        params: &'a Params,
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

pub fn decode(event: VteEvent) {
    match event {
        VteEvent::Csi {
            params,
            intermediates,
            ignore,
            action,
        } => {
            decode_csi(params, intermediates, ignore, action);
        }
        VteEvent::Osc {
            params,
            bell_terminated,
        } => {
            decode_osc(params, bell_terminated);
        }
        VteEvent::Esc {
            intermediates,
            ignore,
            byte,
        } => {
            decode_esc(intermediates, ignore, byte);
        }
    }
}

fn decode_csi(params: &Params, intermediates: &[u8], ignore: bool, action: char) {}
fn decode_osc(params: &[&[u8]], bell_terminated: bool) {}
fn decode_esc(intermediates: &[u8], ignore: bool, byte: u8) {}
