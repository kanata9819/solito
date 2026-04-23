use std::sync::mpsc::Sender;

use tracing::{debug, error};
use vte::{Parser, Perform};

pub struct EscParser {
    output_tx: Sender<Vec<u8>>,
}

impl EscParser {
    pub fn new(output_tx: Sender<Vec<u8>>) -> Self {
        Self { output_tx }
    }

    pub fn parse(&mut self, bytes: impl AsRef<[u8]>) {
        let mut parser: Parser<1024> = Parser::new();
        parser.advance(self, bytes.as_ref());
    }
}

impl Perform for EscParser {
    fn print(&mut self, c: char) {
        debug!("parser print: {:?}", c);
        if let Err(err) = self.output_tx.send(c.to_string().as_bytes().to_vec()) {
            error!("error occured at output_tx.send() {}", err);
        };
    }
    fn csi_dispatch(
        &mut self,
        _params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
        debug!("parser csi_dispatch: {:?}", _params);
    }
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        debug!("parser csi_dispatch: {:?}", _byte);
    }
    fn execute(&mut self, byte: u8) {
        debug!("parser excute: {:?}", byte);
        if let Err(err) = self.output_tx.send(byte.to_string().as_bytes().to_vec()) {
            error!("error occured at output_tx.send() {}", err);
        };
    }
    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
        debug!("parser hook: {:?}", _params);
    }
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        debug!("parser osc_dispatch: {:?}", _params);
    }
    fn put(&mut self, _byte: u8) {
        debug!("parser csi_dispatch: {:?}", _byte);
    }
    fn terminated(&self) -> bool {
        debug!("parser terminated");
        true
    }
    fn unhook(&mut self) {
        debug!("parser unhook");
    }
}
