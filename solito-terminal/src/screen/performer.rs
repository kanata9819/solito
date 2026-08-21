use super::Screen;
use decodesc::{decode_csi, decode_esc, decode_osc};
use vte::{Params, Perform};

impl Perform for Screen {
    fn print(&mut self, c: char) {
        self.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        self.apply_execute(byte);
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        if let Some(osc) = decode_osc(params, bell_terminated) {
            self.apply_osc(&osc);
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        self.apply_csi(decode_csi(params, intermediates, ignore, action));
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        self.apply_esc(decode_esc(intermediates, ignore, byte));
    }
}
