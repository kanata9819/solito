use std::{collections::HashMap, sync::Arc};

use tracing::error;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::{Window, WindowAttributes, WindowId},
};

use crate::renderer::state::State;

pub struct HitoApplication {
    pub window: HashMap<WindowId, Arc<Window>>,
    state: Option<State>,
}

impl HitoApplication {
    pub fn new() -> Self {
        Self {
            window: HashMap::new(),
            state: None,
        }
    }

    async fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes: WindowAttributes = WindowAttributes::default().with_title("Hito");

        let window: Arc<Window> = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("failed to create window"),
        );

        let window_id: WindowId = window.id();
        self.window.insert(window_id, window.clone());

        let state: State = State::new(window).await.unwrap();
    }

    fn redraw(&self, window_id: WindowId) {
        self.window[&window_id].request_redraw();
    }
}

impl ApplicationHandler for HitoApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.create_window(event_loop);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                let _ = self.window.remove(&window_id);
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.redraw(window_id);
            }
            _ => {}
        }
    }
}

impl ApplicationHandler<State> for HitoApplication {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // remove `todo!()`
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            // ...
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(e) => {
                        // Log the error and exit gracefully
                        error!("{e}");
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }
}
