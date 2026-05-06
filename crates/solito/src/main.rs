#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod session;
mod util;

use std::{
    error::Error,
    sync::mpsc::{Receiver, Sender, channel},
};

use session::runtime::SessionInput;
use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

use crate::app::core::SolitoApplication;

fn main() -> Result<(), Box<dyn Error>> {
    let (input_tx, input_rx): (Sender<SessionInput>, Receiver<SessionInput>) =
        channel::<SessionInput>();
    let (output_tx, output_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel::<Vec<u8>>();

    init_tracing();
    run_app(input_tx, input_rx, output_tx, output_rx)?;

    Ok(())
}

fn init_tracing() {
    let env_filter: String = format!("error,solito={}", config::TracingFilter::FILTER_DEBUG);
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(env_filter))
        .init();
}

fn run_app(
    input_tx: Sender<SessionInput>,
    input_rx: Receiver<SessionInput>,
    output_tx: Sender<Vec<u8>>,
    output_rx: Receiver<Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    let event_loop: EventLoop<()> = EventLoop::new()?;
    let mut solito: SolitoApplication =
        SolitoApplication::new(input_tx, input_rx, output_tx, output_rx);
    event_loop.run_app(&mut solito)?;

    Ok(())
}
