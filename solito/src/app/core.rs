use solito_renderer::{
    RendererConfig, State, TabBarSnapshot, TerminalViewRenderer, WindowRenderer,
    estimate_terminal_size,
};
use std::{collections::HashMap, error::Error, sync::Arc};

use crate::app::copy::CopyMode;
use crate::app::event as AppEvent;
use crate::app::event::{AppCommand, CopyModeCommand};
use crate::app::icon;
use crate::app::tabs::AppTabs;
use crate::session::runtime::SessionInput;
use solito_config::app::AppConfig;
use tracing::error;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::ModifiersState,
    window::{Window, WindowAttributes, WindowId},
};

pub(crate) struct SolitoApplication {
    config: AppConfig,
    renderer_config: RendererConfig,
    windows: HashMap<WindowId, Arc<Window>>,
    state: Option<State>,
    tabs: AppTabs,
    copy_mode: CopyMode,
    modifiers: ModifiersState,
}

impl SolitoApplication {
    pub(crate) fn new(config: AppConfig) -> Self {
        let renderer_config: RendererConfig = config.renderer_config();

        Self {
            config,
            renderer_config,
            windows: HashMap::new(),
            state: None,
            tabs: AppTabs::new(),
            copy_mode: CopyMode::default(),
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
        // Keep the native window hidden while GPU setup runs; the renderer also
        // sets a black native fallback background for the first visible acquire.
        let window: Arc<Window> = Arc::new(event_loop.create_window(
            Self::with_platform_window_attributes(self.init_window_attribute()),
        )?);

        self.windows.insert(window.id(), window.clone());
        let initial_size = window.inner_size();
        let (cols, rows) = estimate_terminal_size(
            initial_size.width,
            initial_size.height,
            &self.renderer_config,
        );

        self.init_tab(cols, rows);

        let mut state = self.init_state(&window)?;
        let actual_size: (usize, usize) = state.terminal_size();
        if actual_size != (cols, rows) {
            self.tabs.resize_all(actual_size.0, actual_size.1)?;
        }

        self.tabs.drain_outputs();

        state.set_tab_bar(self.tab_bar_snapshot());

        if let Some(snapshot) = self.tabs.active_snapshot() {
            state.resize(initial_size, snapshot);
        }

        state.draw_frame()?;
        window.set_visible(true);
        state.draw_frame()?;
        self.state = Some(state);
        self.drain_output()?;

        Ok(())
    }

    fn init_tab(&mut self, cols: usize, rows: usize) {
        self.tabs
            .open(cols, rows, self.config.shell.program.clone());
    }

    fn init_state(&self, window: &Arc<Window>) -> Result<State, Box<dyn Error>> {
        Ok(pollster::block_on(State::new(
            Arc::clone(window),
            self.renderer_config.clone(),
        ))?)
    }

    fn init_window_attribute(&self) -> WindowAttributes {
        WindowAttributes::default()
            .with_inner_size(LogicalSize::new(
                self.config.window.width,
                self.config.window.height,
            ))
            .with_transparent(self.renderer_config.window_backdrop.is_transparent())
            .with_window_icon(icon::window_icon())
            .with_title("Solito")
            .with_visible(false)
    }

    #[cfg(target_os = "windows")]
    fn with_platform_window_attributes(window_attributes: WindowAttributes) -> WindowAttributes {
        use winit::platform::windows::WindowAttributesExtWindows;

        window_attributes.with_taskbar_icon(icon::taskbar_icon())
    }

    #[cfg(not(target_os = "windows"))]
    fn with_platform_window_attributes(window_attributes: WindowAttributes) -> WindowAttributes {
        window_attributes
    }

    fn handle_command(
        &mut self,
        command: AppCommand,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn Error>> {
        match command {
            AppCommand::None => {}
            AppCommand::EnterCopyMode => {
                if let Some(snapshot) = self.tabs.active_snapshot() {
                    self.copy_mode.enter(&snapshot);
                    self.sync_copy_mode(&snapshot);
                    self.set_window_title();
                }
            }
            AppCommand::CopyMode(command) => {
                self.handle_copy_mode_command(command)?;
            }
            AppCommand::NewTab => {
                self.exit_copy_mode();
                if let Some(state) = &mut self.state {
                    let (cols, rows): (usize, usize) = state.terminal_size();
                    self.tabs
                        .open(cols, rows, self.config.shell.program.clone());
                    self.set_tab_bar();
                    self.set_active_snapshot_at_bottom();
                }
            }
            AppCommand::CloseTab => {
                self.exit_copy_mode();
                if self.tabs.close_active() {
                    if self.tabs.is_empty() {
                        event_loop.exit();
                    } else {
                        self.set_tab_bar();
                        self.set_active_snapshot_at_bottom();
                    }
                }
            }
            AppCommand::NextTab => {
                self.exit_copy_mode();
                if self.tabs.activate_next() {
                    self.set_tab_bar();
                    self.set_active_snapshot_at_bottom();
                }
            }
            AppCommand::PreviousTab => {
                self.exit_copy_mode();
                if self.tabs.activate_previous() {
                    self.set_tab_bar();
                    self.set_active_snapshot_at_bottom();
                }
            }
            AppCommand::CopySelection => {
                if let Some(snapshot) = self.tabs.active_snapshot()
                    && let Some(text) = self.copy_mode.selected_text(&snapshot)
                {
                    Self::copy_to_clipboard(text)?;
                }
            }
            AppCommand::PasteFromClipboard => {
                self.paste_from_clipboard()?;
            }
            AppCommand::Resize { size, cols, rows } => {
                self.tabs.resize_all(cols, rows)?;

                if let (Some(state), Some(snapshot)) =
                    (&mut self.state, self.tabs.active_snapshot())
                {
                    let copy_mode_snapshot = self.copy_mode.renderer_snapshot(&snapshot);
                    state.resize(size, snapshot);
                    state.set_copy_mode(copy_mode_snapshot);
                }
            }
        }

        Ok(())
    }

    fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) -> Result<AppCommand, Box<dyn Error>> {
        match event {
            WindowEvent::CloseRequested => {
                self.windows.remove(&window_id);
                event_loop.exit();
                Ok(AppCommand::None)
            }
            WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state
                    && let Err(err) = state.draw_frame()
                {
                    error!("{err}");
                    event_loop.exit();
                }

                Ok(AppCommand::None)
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: key_state,
                        text,
                        ..
                    },
                ..
            } => {
                let Some(input_tx) = self.tabs.active_input_tx() else {
                    return Ok(AppCommand::None);
                };

                AppEvent::handle_key(
                    text,
                    logical_key,
                    key_state,
                    self.modifiers,
                    input_tx,
                    self.copy_mode.is_active(),
                )
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                Ok(AppCommand::None)
            }
            WindowEvent::Resized(size) => {
                let Some(state) = &mut self.state else {
                    return Ok(AppCommand::None);
                };
                let (cols, rows): (usize, usize) = state.terminal_size_for(size);
                Ok(AppCommand::Resize { size, cols, rows })
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => {
                let Some(state) = &mut self.state else {
                    return Ok(AppCommand::None);
                };

                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        tracing::debug!("MouseScrollDelta.LineDelta: x({:?}), y({:?})", x, y);
                        state.scroll(x, y);
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        tracing::debug!("MouseScrollDelta.PixelDelta: pos({:?})", pos);
                        state.scroll(
                            pos.x as f32 / self.renderer_config.line_height,
                            pos.y as f32 / self.renderer_config.line_height,
                        );
                    }
                }
                Ok(AppCommand::None)
            }
            #[allow(unused)]
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => Ok(AppCommand::None),
            #[allow(unused)]
            WindowEvent::CursorLeft { device_id } => Ok(AppCommand::None),
            #[allow(unused)]
            WindowEvent::CursorEntered { device_id } => Ok(AppCommand::None),
            _ => {
                tracing::debug!("unhandled event: {event:?}");
                Ok(AppCommand::None)
            }
        }
    }

    fn handle_copy_mode_command(&mut self, command: CopyModeCommand) -> Result<(), Box<dyn Error>> {
        let Some(snapshot) = self.tabs.active_snapshot() else {
            return Ok(());
        };

        match command {
            CopyModeCommand::Move(direction) => {
                self.copy_mode.move_cursor(&snapshot, direction);
                self.sync_copy_mode(&snapshot);
            }
            CopyModeCommand::ToggleCellSelection => {
                self.copy_mode.toggle_cell_selection();
                self.sync_copy_mode(&snapshot);
            }
            CopyModeCommand::ToggleLineSelection => {
                self.copy_mode.toggle_line_selection();
                self.sync_copy_mode(&snapshot);
            }
            CopyModeCommand::CopyAndExit => {
                let copy_result: Result<(), Box<dyn Error>> =
                    if let Some(text) = self.copy_mode.selected_text(&snapshot) {
                        Self::copy_to_clipboard(text)
                    } else {
                        Ok(())
                    };

                self.exit_copy_mode();
                copy_result?;
            }
            CopyModeCommand::Exit => {
                self.exit_copy_mode();
            }
        }

        Ok(())
    }

    fn set_active_snapshot(&mut self) {
        if let (Some(state), Some(snapshot)) = (&mut self.state, self.tabs.active_snapshot()) {
            let copy_mode_snapshot = self.copy_mode.renderer_snapshot(&snapshot);
            state.set_terminal_snapshot(snapshot);
            state.set_copy_mode(copy_mode_snapshot);
        }
    }

    fn set_active_snapshot_at_bottom(&mut self) {
        if let (Some(state), Some(snapshot)) = (&mut self.state, self.tabs.active_snapshot()) {
            let copy_mode_snapshot = self.copy_mode.renderer_snapshot(&snapshot);
            state.set_terminal_snapshot_at_bottom(snapshot);
            state.set_copy_mode(copy_mode_snapshot);
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

    fn sync_copy_mode(&mut self, snapshot: &solito_terminal::ScreenSnapshot) {
        if let Some(state) = &mut self.state {
            state.set_copy_mode(self.copy_mode.renderer_snapshot(snapshot));
        }
    }

    fn exit_copy_mode(&mut self) {
        if !self.copy_mode.is_active() {
            return;
        }

        self.copy_mode.exit();

        if let Some(state) = &mut self.state {
            state.set_copy_mode(Default::default());
        }

        self.set_window_title();
    }

    fn set_window_title(&self) {
        let title: &str = if self.copy_mode.is_active() {
            "Solito - Copy Mode"
        } else {
            "Solito"
        };

        for window in self.windows.values() {
            window.set_title(title);
        }
    }

    fn copy_to_clipboard(text: String) -> Result<(), Box<dyn Error>> {
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(text)?;

        Ok(())
    }

    fn paste_from_clipboard(&self) -> Result<(), Box<dyn Error>> {
        let Some(input_tx) = self.tabs.active_input_tx() else {
            return Ok(());
        };

        let Ok(mut clipboard) = arboard::Clipboard::new() else {
            return Ok(());
        };
        let Ok(text) = clipboard.get_text() else {
            return Ok(());
        };

        if !text.is_empty() {
            input_tx.send(SessionInput::write(text.into_bytes()))?;
        }

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
        let command: AppCommand = match self.handle_window_event(event_loop, window_id, event) {
            Ok(command) => command,
            Err(err) => {
                error!("event handle error: {}", err);
                return;
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
