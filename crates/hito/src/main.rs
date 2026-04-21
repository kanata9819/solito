mod app;
mod renderer;
mod session;
mod util;

use std::{
    error::Error,
    sync::mpsc::{Receiver, Sender, channel},
};

use session::runtime::SessionRuntime;
use tracing::{error, level_filters::LevelFilter};
use winit::event_loop::EventLoop;

use crate::app::core::HitoApplication;

fn main() -> Result<(), Box<dyn Error>> {
    let (input_tx, input_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel::<Vec<u8>>();
    let (output_tx, output_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel::<Vec<u8>>();

    init_tracing();
    run_session(input_rx, output_tx);
    run_app(input_tx, output_rx)?;

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::ERROR)
        .init();
}

fn run_session(input_rx: Receiver<Vec<u8>>, output_tx: Sender<Vec<u8>>) {
    std::thread::spawn(move || {
        let runtime: SessionRuntime = SessionRuntime::new(input_rx, output_tx);
        if let Err(err) = runtime.run_session() {
            error!("run session failed: {}", err);
        };
    });
}

fn run_app(input_tx: Sender<Vec<u8>>, output_rx: Receiver<Vec<u8>>) -> Result<(), Box<dyn Error>> {
    let event_loop: EventLoop<()> = EventLoop::new()?;
    let mut hito: HitoApplication = HitoApplication::new(input_tx, output_rx);
    event_loop.run_app(&mut hito)?;

    Ok(())
}
