/// Xterm mouse tracking and encoding, shared by both screen buffers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseMode {
    pub tracking: u16,
    pub sgr: bool,
}

impl MouseMode {
    pub(crate) fn set(&mut self, mode: u16, enabled: bool) {
        match mode {
            1000 | 1002 | 1003 => self.tracking = if enabled { mode } else { 0 },
            1006 => self.sgr = enabled,
            _ => {}
        }
    }

    /// `code` is the xterm button code including modifier and motion bits.
    /// Cell coordinates are zero-based; wire coordinates are one-based.
    pub fn encode(self, code: u8, released: bool, col: usize, row: usize) -> Option<Vec<u8>> {
        if self.tracking == 0
            || (code & 32 != 0
                && (self.tracking == 1000 || (self.tracking == 1002 && code & 3 == 3)))
        {
            return None;
        }
        let (x, y) = (col.checked_add(1)?, row.checked_add(1)?);
        if self.sgr {
            let suffix = if released { 'm' } else { 'M' };
            Some(format!("\x1b[<{code};{x};{y}{suffix}").into_bytes())
        } else if x <= 223 && y <= 223 {
            let code = if released { (code & 28) | 3 } else { code };
            Some(vec![
                0x1b,
                b'[',
                b'M',
                code.checked_add(32)?,
                x as u8 + 32,
                y as u8 + 32,
            ])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{TerminalSize, TerminalState};

    #[test]
    fn mouse_negotiation_and_reports() {
        let mut terminal = TerminalState::new(TerminalSize::new(300, 30));
        assert_eq!(terminal.mouse_mode().encode(0, false, 0, 0), None);
        terminal.apply_terminal_output(b"\x1b[?1002;1006h\x1b[?1049h");
        let mode = terminal.mouse_mode();
        for (code, released, expected) in [
            (0, false, "\x1b[<0;250;5M"),
            (0, true, "\x1b[<0;250;5m"),
            (32, false, "\x1b[<32;250;5M"),
            (64, false, "\x1b[<64;250;5M"),
            (65, false, "\x1b[<65;250;5M"),
            (20, false, "\x1b[<20;250;5M"),
        ] {
            assert_eq!(
                mode.encode(code, released, 249, 4).unwrap(),
                expected.as_bytes()
            );
        }
        assert_eq!(mode.encode(35, false, 0, 0), None);
        terminal.apply_terminal_output(b"\x1b[?1003h");
        assert!(terminal.mouse_mode().encode(35, false, 0, 0).is_some());
        terminal.apply_terminal_output(b"\x1b[?1000h\x1b[?1006l");
        let mode = terminal.mouse_mode();
        assert_eq!(mode.encode(32, false, 0, 0), None);
        assert_eq!(mode.encode(0, false, 0, 0).unwrap(), b"\x1b[M !!");
        assert_eq!(mode.encode(0, true, 0, 0).unwrap(), b"\x1b[M#!!");
        assert_eq!(mode.encode(0, false, 223, 0), None);
        terminal.apply_terminal_output(b"\x1b[?1000l\x1b[?1049l");
        assert_eq!(terminal.mouse_mode().tracking, 0);
        terminal.apply_terminal_output(b"\x1b[?1002;1006h\x1bc");
        assert_eq!(terminal.mouse_mode(), Default::default());
    }
}
