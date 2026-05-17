use solito_renderer::{
    RendererConfig, State, TabBarSnapshot, TerminalViewRenderer, WindowRenderer,
};
use std::{collections::HashMap, error::Error, sync::Arc};

use crate::app::event as AppEvent;
use crate::app::event::AppCommand;
use crate::app::icon;
use crate::app::tabs::AppTabs;
use crate::config::WindowAttr;
use tracing::error;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    keyboard::ModifiersState,
    window::{Window, WindowAttributes, WindowId},
};

#[cfg(target_os = "windows")]
use winit::platform::windows::WindowAttributesExtWindows;

pub(crate) struct SolitoApplication {
    windows: HashMap<WindowId, Arc<Window>>,
    state: Option<State>,
    tabs: AppTabs,
    modifiers: ModifiersState,
}

impl SolitoApplication {
    pub(crate) fn new() -> Self {
        Self {
            windows: HashMap::new(),
            state: None,
            tabs: AppTabs::new(),
            modifiers: ModifiersState::default(),
        }
    }

    fn drain_output(&mut self) -> Result<(), Box<dyn Error>> {
        if self.tabs.drain_outputs() {
            self.set_active_snapshot();
        }

        Ok(())
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window_attributes: WindowAttributes = WindowAttributes::default()
            .with_inner_size(LogicalSize::new(
                WindowAttr::WINDOW_WIDTH,
                WindowAttr::WINDOW_HIGHT,
            ))
            .with_transparent(RendererConfig::WINDOW_BACKDROP.is_transparent())
            .with_window_icon(icon::window_icon())
            .with_title("Solito");
        #[cfg(target_os = "windows")]
        let window_attributes: WindowAttributes =
            window_attributes.with_taskbar_icon(icon::taskbar_icon());

        let window: Arc<Window> = Arc::new(event_loop.create_window(window_attributes)?);
        let window_id: WindowId = window.id();
        self.windows.insert(window_id, window.clone());
        let mut state: State = pollster::block_on(State::new(window))?;
        let (cols, rows): (usize, usize) = state.terminal_size();
        self.tabs.open(cols, rows);
        state.set_tab_bar(self.tab_bar_snapshot());

        if let Some(snapshot) = self.tabs.active_snapshot() {
            state.set_terminal_snapshot(snapshot);
        }

        state.render()?;
        self.state = Some(state);
        self.drain_output()?;

        Ok(())
    }

    fn handle_command(
        &mut self,
        command: AppCommand,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn Error>> {
        match command {
            AppCommand::None => {}
            AppCommand::NewTab => {
                if let Some(state) = &mut self.state {
                    let (cols, rows): (usize, usize) = state.terminal_size();
                    self.tabs.open(cols, rows);
                    self.set_tab_bar();
                    self.set_active_snapshot();
                }
            }
            AppCommand::CloseTab => {
                if self.tabs.close_active() {
                    if self.tabs.is_empty() {
                        event_loop.exit();
                    } else {
                        self.set_tab_bar();
                        self.set_active_snapshot();
                    }
                }
            }
            AppCommand::NextTab => {
                if self.tabs.activate_next() {
                    self.set_tab_bar();
                    self.set_active_snapshot();
                }
            }
            AppCommand::PreviousTab => {
                if self.tabs.activate_previous() {
                    self.set_tab_bar();
                    self.set_active_snapshot();
                }
            }
            AppCommand::Resize { size, cols, rows } => {
                self.tabs.resize_all(cols, rows)?;

                if let (Some(state), Some(snapshot)) =
                    (&mut self.state, self.tabs.active_snapshot())
                {
                    state.resize(size, snapshot);
                }
            }
        }

        Ok(())
    }

    fn set_active_snapshot(&mut self) {
        if let (Some(state), Some(snapshot)) = (&mut self.state, self.tabs.active_snapshot()) {
            state.set_terminal_snapshot(snapshot);
        }
    }

    fn set_tab_bar(&mut self) {
        let snapshot: TabBarSnapshot = self.tab_bar_snapshot();

        if let Some(state) = &mut self.state {
            state.set_tab_bar(snapshot);
        }
    }

    fn tab_bar_snapshot(&self) -> TabBarSnapshot {
        TabBarSnapshot::new(self.tabs.titles(), self.tabs.active_index())
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
        let command: AppCommand = {
            let state: &mut State = match &mut self.state {
                Some(canvas) => canvas,
                None => return,
            };
            let input_tx = match self.tabs.active_input_tx() {
                Some(input_tx) => input_tx,
                None => return,
            };

            match AppEvent::event_handler(
                &mut self.windows,
                window_id,
                state,
                event_loop,
                event,
                &mut self.modifiers,
                input_tx,
            ) {
                Ok(command) => command,
                Err(err) => {
                    error!("event handle error: {}", err);
                    return;
                }
            }
        };

        if let Err(err) = self.handle_command(command, event_loop) {
            error!("event command error: {}", err);
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
