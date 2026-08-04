#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod session;

use std::error::Error;

use solito_config::app::AppConfig;
use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

use crate::app::application::SolitoApplication;

fn main() -> Result<(), Box<dyn Error>> {
    let config = AppConfig::load_or_create()?;
    init_tracing(&config);
    run_app(config)?;

    Ok(())
}

fn init_tracing(config: &AppConfig) {
    let env_filter = format!("error,solito={}", config.tracing.filter);
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(env_filter))
        .init();
}

fn run_app(config: AppConfig) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut solito = SolitoApplication::new(config);
    event_loop.run_app(&mut solito)?;

    Ok(())
}
