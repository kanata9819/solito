use std::sync::mpsc::Sender;

use decodesc::{DecodedEvent, VteEvent};
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

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        let decoded: Option<DecodedEvent> = decodesc::decode(VteEvent::Osc {
            params,
            bell_terminated,
        });

        if let Some(decoded) = decoded
            && let Some(osc) = decoded.esc
        {
            match osc {
                _ => {
                    debug!("osc: {:?}", osc);
                }
            }
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        let decoded: Option<DecodedEvent> = decodesc::decode(VteEvent::Csi {
            params,
            intermediates,
            ignore,
            action,
        });

        if let Some(decoded) = decoded {
            if let Some(csi) = decoded.csi {
                match csi {
                    decodesc::CsiMessage::EraseLine(mode) => {
                        if mode == 0 {
                            self.send(TerminalEvent::ClearLine);
                        }
                    }
                    decodesc::CsiMessage::CursorPosition { row, col } => {
                        self.send(TerminalEvent::MoveCursor(row, col));
                    }
                    _ => {
                        debug!("csi: {:?}", csi);
                    }
                }
            }
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        let decoded: Option<DecodedEvent> = decodesc::decode(VteEvent::Esc {
            intermediates,
            ignore,
            byte,
        });

        if let Some(decoded) = decoded {
            if let Some(esc) = decoded.esc {
                match esc {
                    _ => {
                        debug!("esc: {:?}", esc);
                    }
                }
            }
        }
    }

    fn terminated(&self) -> bool {
        debug!("terminated");
        true
    }
}
