use tracing::debug;
use vte::{Parser, Perform};

pub struct EscParser {}

impl EscParser {
    pub fn new() -> Self {
        Self {}
    }

    pub fn parse(&mut self, bytes: impl AsRef<[u8]>) {
        let mut parser: Parser<1024> = Parser::new();
        parser.advance(self, bytes.as_ref());
    }
}

impl Perform for EscParser {
    fn print(&mut self, _c: char) {
        debug!("parser print: {:?}", _c);
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
    fn execute(&mut self, _byte: u8) {
        debug!("parser excute: {:?}", _byte);
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
