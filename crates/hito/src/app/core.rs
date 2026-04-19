use std::{collections::HashMap, error::Error, sync::Arc};

use crate::app::event as AppEvent;
use crate::renderer::state::State;
use tracing::error;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

pub struct HitoApplication {
    pub windows: HashMap<WindowId, Arc<Window>>,
    state: Option<State>,
}

impl HitoApplication {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            state: None,
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window_attributes: WindowAttributes = WindowAttributes::default()
            .with_inner_size(LogicalSize::new(1200.0, 750.0))
            .with_title("Hito");

        let window: Arc<Window> = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("failed to create window"),
        );

        let window_id: WindowId = window.id();
        self.windows.insert(window_id, window.clone());

        let mut state: State = pollster::block_on(State::new(window))?;
        state.render()?;
        self.state = Some(state);

        Ok(())
    }
}

impl ApplicationHandler for HitoApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(err) = self.create_window(event_loop) {
            error!("create window error occured: {}", err)
        };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let state: &mut State = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        if let Err(err) =
            AppEvent::event_handler(&mut self.windows, window_id, state, event_loop, event)
        {
            error!("event handle error: {}", err);
        };
    }
}
