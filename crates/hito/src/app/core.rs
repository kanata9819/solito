use std::collections::HashMap;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

pub struct HitoApplication {
    pub window: HashMap<WindowId, Window>,
}

impl HitoApplication {
    pub fn new() -> Self {
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
