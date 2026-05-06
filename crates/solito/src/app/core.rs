use std::{
    collections::HashMap,
    error::Error,
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
};

use crate::app::event as AppEvent;
use crate::config::WindowAttr;
use crate::renderer::state::context::{State, TerminalViewRenderer, WindowRenderer};
use crate::terminal::TerminalState;
use tracing::error;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

pub(crate) struct SolitoApplication {
    windows: HashMap<WindowId, Arc<Window>>,
    state: Option<State>,
    terminal: Option<TerminalState>,
    input_tx: Sender<Vec<u8>>,
    output_rx: Receiver<Vec<u8>>,
}

impl SolitoApplication {
    pub(crate) fn new(input_tx: Sender<Vec<u8>>, output_rx: Receiver<Vec<u8>>) -> Self {
        Self {
            windows: HashMap::new(),
            state: None,
            terminal: None,
            input_tx,
            output_rx,
        }
    }

    fn drain_output(&mut self) -> Result<(), Box<dyn Error>> {
        while let Ok(output) = self.output_rx.try_recv()
            && let (Some(state), Some(terminal)) = (&mut self.state, &mut self.terminal)
        {
            terminal.apply_terminal_output(&output);
            state.set_terminal_snapshot(terminal.snapshot());
        }
        Ok(())
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window_attributes: WindowAttributes = WindowAttributes::default()
            .with_inner_size(LogicalSize::new(
                WindowAttr::WINDOW_WIDTH,
                WindowAttr::WINDOW_HIGHT,
            ))
            .with_title("Solito");

        let window: Arc<Window> = Arc::new(event_loop.create_window(window_attributes)?);
        let window_id: WindowId = window.id();
        self.windows.insert(window_id, window.clone());
        let mut state: State = pollster::block_on(State::new(window))?;
        let (cols, rows) = state.terminal_size();
        let terminal = TerminalState::new(cols, rows);
        state.set_terminal_snapshot(terminal.snapshot());

        state.render()?;
        self.terminal = Some(terminal);
        self.state = Some(state);
        self.drain_output()?;

        Ok(())
    }
}

impl ApplicationHandler for SolitoApplication {
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
        let terminal: &mut TerminalState = match &mut self.terminal {
            Some(terminal) => terminal,
            None => return,
        };

        if let Err(err) = AppEvent::event_handler(
            &mut self.windows,
            window_id,
            state,
            terminal,
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
