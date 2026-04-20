mod app;
mod renderer;
mod session;
mod util;

use session::runtime::SessionRuntime;
use std::error::Error;
use std::sync::mpsc::{Receiver, Sender, channel};
use tracing::{error, level_filters::LevelFilter};
use winit::event_loop::EventLoop;

use crate::app::core::HitoApplication;

fn main() -> Result<(), Box<dyn Error>> {
    let (input_tx, input_rx): (Sender<String>, Receiver<String>) = channel::<String>();
    let (output_tx, output_rx): (Sender<String>, Receiver<String>) = channel::<String>();

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

fn run_session(input_rx: Receiver<String>, output_tx: Sender<String>) {
    std::thread::spawn(move || {
        let runtime: SessionRuntime = SessionRuntime::new(input_rx, output_tx);
        if let Err(err) = runtime.run_session() {
            error!("run session failed: {}", err);
        };
    });
}

fn run_app(input_tx: Sender<String>, output_rx: Receiver<String>) -> Result<(), Box<dyn Error>> {
    let event_loop: EventLoop<()> = EventLoop::new()?;
    let mut hito: HitoApplication = HitoApplication::new(input_tx, output_rx);
    event_loop.run_app(&mut hito)?;

    Ok(())
}
