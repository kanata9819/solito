mod pty;

use pty::runtime::Runtime;
use std::{collections::HashMap, error::Error};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

fn main() -> Result<(), Box<dyn Error>> {
    run_session();

    if let Err(err) = run_app() {
        eprintln!("run app error occured: {}", err);
    };

    Ok(())
}

fn run_app() -> Result<(), Box<dyn Error>> {
    let event_loop: EventLoop<()> = EventLoop::new()?;
    let mut hito = HitoApplication::new();
    event_loop.run_app(&mut hito)?;

    Ok(())
}

fn run_session() {
    std::thread::spawn(move || {
        let runtime: Runtime = Runtime::new();
        runtime.run_session();
    });
}

struct HitoApplication {
    pub window: HashMap<WindowId, Window>,
}

impl HitoApplication {
    fn new() -> Self {
        Self {
            window: HashMap::new(),
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes: WindowAttributes = WindowAttributes::default().with_title("Hito");

        let window: Window = event_loop
            .create_window(window_attributes)
            .expect("failed to create window");

        let window_id: WindowId = window.id();
        self.window.insert(window_id, window);
    }
}

impl ApplicationHandler for HitoApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.create_window(event_loop);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                let _ = self.window.remove(&window_id);
            }
            _ => {}
        }
    }
}
