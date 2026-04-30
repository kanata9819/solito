use std::{
    collections::HashMap,
    error::Error,
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
};

use crate::renderer::state::State;
use crate::{app::event as AppEvent, session::parser::TerminalEvent};
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
    output_rx: Receiver<TerminalEvent>,
}

impl HitoApplication {
    pub fn new(input_tx: Sender<Vec<u8>>, output_rx: Receiver<TerminalEvent>) -> Self {
        Self {
            windows: HashMap::new(),
            state: None,
            input_tx,
            output_rx,
        }
    }

    fn drain_output(&mut self) -> Result<(), Box<dyn Error>> {
        while let Ok(output) = self.output_rx.try_recv()
            && let Some(state) = &mut self.state
        {
            match output {
                TerminalEvent::Print(char) => {
                    state.add_char_to_buffer(char);
                }
                TerminalEvent::CarriageReturn => {
                    state.carriage_return();
                }
                TerminalEvent::LineFeed => {
                    state.line_feed();
                }
                TerminalEvent::ClearLine => {
                    state.clear_line();
                }
                TerminalEvent::MoveCursor(row, col) => {
                    state.move_cursor_to(row, col);
                }
            }
        }
        Ok(())
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        const WINDOW_WIDTH: f32 = 800.0;
        const WINDOW_HIGHT: f32 = 500.0;

        let window_attributes: WindowAttributes = WindowAttributes::default()
            .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HIGHT))
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
        self.drain_output()?;

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
