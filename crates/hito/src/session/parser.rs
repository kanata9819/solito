use std::sync::mpsc::Sender;

use tracing::{debug, error};
use vte::{Parser, Perform};

pub enum TerminalEvent {
    Print(char),
    CarriageReturn,
    LineFeed,
    ClearLine,
    MoveCursor(u16, u16),
}

pub struct EscParser {
    pub parser: Parser<1024>,
    pub performer: EscPerformer,
}

impl EscParser {
    pub fn new(output_tx: Sender<TerminalEvent>) -> Self {
        Self {
            parser: Parser::new(),
            performer: EscPerformer::new(output_tx),
        }
    }

    pub fn parse(&mut self, bytes: impl AsRef<[u8]>) {
        self.parser.advance(&mut self.performer, bytes.as_ref());
    }
}

pub struct EscPerformer {
    output_tx: Sender<TerminalEvent>,
}

impl EscPerformer {
    pub fn new(output_tx: Sender<TerminalEvent>) -> Self {
        Self { output_tx }
    }

    fn send(&self, event: TerminalEvent) {
        // if we convert "byte" to string and then send to the terminal,
        // "\n" will be converted to "10", which is not what we want.
        if let Err(err) = self.output_tx.send(event) {
            error!("error occured at output_tx.send() {}", err);
        };
    }
}

impl Perform for EscPerformer {
    fn print(&mut self, c: char) {
        if let Err(err) = self.output_tx.send(TerminalEvent::Print(c)) {
            error!("error occured at output_tx.send() {}", err);
        };
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\r' => {
                self.send(TerminalEvent::CarriageReturn);
            }
            b'\n' => {
                self.send(TerminalEvent::LineFeed);
            }
            _ => {
                debug!("uncaught excute: {:?}", byte);
            }
        }
    }

    fn hook(&mut self, params: &vte::Params, _intermediates: &[u8], _ignore: bool, action: char) {
        debug!("hook: params({:?}), action({:?})", params, action);
    }

    fn put(&mut self, byte: u8) {
        debug!("csi_dispatch: byte({:?})", byte);
    }

    fn unhook(&mut self) {
        debug!("unhook");
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        debug!("osc_dispatch: params({:?})", params);
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        match action {
            'K' => {
                let mode: u16 = params.iter().next().map(|param| param[0]).unwrap_or(0);
                match mode {
                    0 => {
                        self.send(TerminalEvent::ClearLine);
                    }
                    _ => {}
                }
            }
            'H' | 'f' => {
                let mut iter: vte::ParamsIter<'_> = params.iter();
                let row: u16 = iter.next().map(|p| p[0]).unwrap_or(0);
                let col: u16 = iter.next().map(|p| p[0]).unwrap_or(0);

                self.send(TerminalEvent::MoveCursor(row, col));
            }
            _ => {
                debug!("csi_dispatch: params({:?}), action({:?})", params, action)
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        debug!("esc_dispatch: byte({:?})", byte);
    }

    fn terminated(&self) -> bool {
        debug!("terminated");
        true
    }
}
