mod app;
mod session;

use session::session::SessionRuntime;
use std::error::Error;
use tracing::level_filters::LevelFilter;
use winit::event_loop::EventLoop;

use crate::app::core::HitoApplication;

fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();
    run_session();
    run_app()?;

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .init();
}

fn run_session() {
    std::thread::spawn(move || {
        let runtime: SessionRuntime = SessionRuntime::new();
        runtime.run_session();
    });
}

fn run_app() -> Result<(), Box<dyn Error>> {
    let event_loop: EventLoop<()> = EventLoop::new()?;
    let mut hito = HitoApplication::new();
    event_loop.run_app(&mut hito)?;

    Ok(())
}
