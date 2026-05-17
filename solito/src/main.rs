#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod session;

use std::error::Error;

use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

use crate::app::core::SolitoApplication;

fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();
    run_app()?;

    Ok(())
}

fn init_tracing() {
    let env_filter: String = format!("error,solito={}", config::TracingFilter::FILTER_ERROR);
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(env_filter))
        .init();
}

fn run_app() -> Result<(), Box<dyn Error>> {
    let event_loop: EventLoop<()> = EventLoop::new()?;
    let mut solito: SolitoApplication = SolitoApplication::new();
    event_loop.run_app(&mut solito)?;

    Ok(())
}
