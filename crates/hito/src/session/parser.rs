use std::sync::mpsc::Sender;

use tracing::{debug, error};
use vte::{Parser, Perform};

pub struct EscParser {
    pub parser: Parser<1024>,
    pub performer: EscPerformer,
}

impl EscParser {
    pub fn new(output_tx: Sender<Vec<u8>>) -> Self {
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
    output_tx: Sender<Vec<u8>>,
}

impl EscPerformer {
    pub fn new(output_tx: Sender<Vec<u8>>) -> Self {
        Self { output_tx }
    }
}

impl Perform for EscPerformer {
    fn print(&mut self, c: char) {
        if cfg!(debug_assertions) {
            debug!("parser print: {:?}", c);
        }

        if let Err(err) = self.output_tx.send(c.to_string().into_bytes()) {
            error!("error occured at output_tx.send() {}", err);
        };
    }

    fn execute(&mut self, byte: u8) {
        if cfg!(debug_assertions) {
            debug!("parser excute: {}", String::from_utf8_lossy(&[byte]));
        }

        // if we convert "byte" to string and then send to the terminal,
        // "\n" will be converted to "10", which is not what we want.
        if let Err(err) = self.output_tx.send(vec![byte]) {
            error!("error occured at output_tx.send() {}", err);
        };
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
        debug!("parser hook: {:?}", _params);
    }

    fn put(&mut self, _byte: u8) {
        debug!("parser csi_dispatch: {:?}", _byte);
    }

    fn unhook(&mut self) {
        debug!("parser unhook");
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        debug!("parser osc_dispatch: {:?}", _params);
    }

    fn csi_dispatch(
        &mut self,
        _params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
        dbg!(_params, _action);
        debug!("parser csi_dispatch: {:?}", _params);
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        debug!("parser csi_dispatch: {:?}", _byte);
    }

    fn terminated(&self) -> bool {
        debug!("parser terminated");
        true
    }
}
