mod app;
mod renderer;
mod session;

use session::runtime::SessionRuntime;
use std::error::Error;
use tracing::{error, level_filters::LevelFilter};
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
        .with_max_level(LevelFilter::ERROR)
        .init();
}

fn run_session() {
    std::thread::spawn(move || {
        let runtime: SessionRuntime = SessionRuntime::new();
        if let Err(err) = runtime.run_session() {
            error!("run session failed: {}", err);
        };
    });
}

fn run_app() -> Result<(), Box<dyn Error>> {
    let event_loop: EventLoop<()> = EventLoop::new()?;
    let mut hito: HitoApplication = HitoApplication::new();
    event_loop.run_app(&mut hito)?;

    Ok(())
}
