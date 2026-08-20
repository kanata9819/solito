//! Application lifecycle and top-level ownership.
//!
//! Follow behavior into the child modules:
//! - `window_event`: winit event -> application command
//! - `command`: command -> application state change
//! - `view_sync`: application state -> renderer snapshot

mod command;
mod view_sync;
mod window_event;

use solito_config::app::AppConfig;
use solito_renderer::{Renderer, RendererConfig, TerminalSize, estimate_terminal_size};
use std::{error::Error, sync::Arc};
use tracing::error;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    keyboard::ModifiersState,
    window::{Window, WindowAttributes, WindowId},
};

use crate::app::{copy::CopyMode, event::AppEvent, icon, tabs::AppTabs};

pub(super) type AppResult<T = ()> = Result<T, Box<dyn Error>>;

pub(crate) struct SolitoApplication {
    config: AppConfig,
    renderer_config: RendererConfig,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    tabs: AppTabs,
    copy_mode: CopyMode,
    modifiers: ModifiersState,
    event_proxy: EventLoopProxy<AppEvent>,
    needs_redraw: bool,
}

impl SolitoApplication {
    pub(crate) fn new(config: AppConfig, event_proxy: EventLoopProxy<AppEvent>) -> Self {
        let renderer_config = config.renderer_config();

        Self {
            config,
            renderer_config,
            window: None,
            renderer: None,
            tabs: AppTabs::new(),
            copy_mode: CopyMode::default(),
            modifiers: ModifiersState::default(),
            event_proxy,
            needs_redraw: false,
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> AppResult {
        // Keep the native window hidden while GPU setup runs. Drawing before
        // showing it avoids a blank startup frame.
        let window = Arc::new(
            event_loop.create_window(Self::with_platform_window_attributes(
                self.window_attributes(),
            ))?,
        );

        self.window = Some(window.clone());
        let window_size = window.inner_size();
        let estimated_size =
            estimate_terminal_size(window_size.width, window_size.height, &self.renderer_config);
        self.open_initial_tab(estimated_size);

        let mut renderer = self.create_renderer(&window)?;
        let actual_size = renderer.terminal_size();
        if actual_size != estimated_size {
            self.tabs.resize_all(actual_size)?;
        }

        self.tabs.drain_outputs();
        renderer.set_tab_bar(self.tab_bar_snapshot());
        if let Some(snapshot) = self.tabs.active_snapshot() {
            renderer.resize(window_size, snapshot);
        }

        renderer.draw_frame()?;
        window.set_visible(true);
        renderer.draw_frame()?;

        self.renderer = Some(renderer);
        self.drain_terminal_output();
        Ok(())
    }

    fn open_initial_tab(&mut self, size: TerminalSize) {
        self.tabs.open(
            size,
            self.config.shell.program.clone(),
            self.event_proxy.clone(),
        );
    }

    fn create_renderer(&self, window: &Arc<Window>) -> AppResult<Renderer> {
        pollster::block_on(Renderer::new(
            Arc::clone(window),
            self.renderer_config.clone(),
        ))
    }

    fn window_attributes(&self) -> WindowAttributes {
        WindowAttributes::default()
            .with_inner_size(LogicalSize::new(
                self.config.window.width,
                self.config.window.height,
            ))
            .with_transparent(self.renderer_config.window_backdrop.is_transparent())
            .with_window_icon(icon::app_icon())
            .with_title("Solito")
            .with_visible(false)
    }

    #[cfg(target_os = "windows")]
    fn with_platform_window_attributes(attributes: WindowAttributes) -> WindowAttributes {
        use winit::platform::windows::WindowAttributesExtWindows;

        attributes.with_taskbar_icon(icon::app_icon())
    }

    #[cfg(not(target_os = "windows"))]
    fn with_platform_window_attributes(attributes: WindowAttributes) -> WindowAttributes {
        attributes
    }
}

impl ApplicationHandler<AppEvent> for SolitoApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(err) = self.create_window(event_loop)
        {
            error!("create window failed: {err}");
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::TerminalOutputReady => self.drain_terminal_output(),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let command = self.handle_window_event(event_loop, window_id, event);

        if let Err(err) = self.handle_command(command, event_loop) {
            error!("application command failed: {err}");
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.needs_redraw
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
    }
}
