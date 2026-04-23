use std::{
    collections::HashMap,
    error::Error,
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
};

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
    input_tx: Sender<Vec<u8>>,
    output_rx: Receiver<Vec<u8>>,
}

impl HitoApplication {
    pub fn new(input_tx: Sender<Vec<u8>>, output_rx: Receiver<Vec<u8>>) -> Self {
        Self {
            windows: HashMap::new(),
            state: None,
            input_tx,
            output_rx,
        }
    }

    fn drain_output(&mut self) -> Result<(), Box<dyn Error>> {
        while let Ok(output) = self.output_rx.try_recv() {
            if let Some(state) = &mut self.state {
                let mut dbg: Vec<char> = Vec::new();
                let output: String = String::from_utf8(output)?;
                for char in output.chars() {
                    state.add_char_to_buffer(char);

                    dbg.push(char);
                }

                for c in dbg {
                    println!("{:?}", c);
                }
            }
        }
        Ok(())
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

        if let Err(err) = self.drain_output() {
            error!("drain output error occured: {}", err);
        };

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

        if let Err(err) = AppEvent::event_handler(
            &mut self.windows,
            window_id,
            state,
            event_loop,
            event,
            &self.input_tx,
        ) {
            error!("event handle error: {}", err);
        };
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Err(err) = self.drain_output() {
            error!("drain output error occured: {}", err);
        }

        for window in self.windows.values() {
            window.request_redraw();
        }
    }
}
