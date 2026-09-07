mod command;

use anyhow::Result;
use solito_config::app::AppConfig;
use solito_renderer::{Renderer, RendererConfig, TabBarSnapshot, TerminalSize, estimate_term_size};
use solito_terminal::ScreenSnapshot;
use std::sync::Arc;
use tracing::error;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    keyboard::ModifiersState,
    window::{Window, WindowAttributes, WindowId},
};

use crate::app::{copy::CopyMode, event::AppEvent, icon, input, tabs::AppTabs};

pub(super) type AppResult<T = ()> = Result<T>;

pub(crate) struct SolitoApplication {
    config: AppConfig,
    renderer_config: RendererConfig,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    tabs: AppTabs,
    copy_mode: CopyMode,
    modifiers: ModifiersState,
    event_proxy: EventLoopProxy<AppEvent>,
    // State updates set this flag; about_to_wait requests a redraw; RedrawRequested draws.
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
        let mut renderer = self.create_renderer(&window)?;
        // Start the shell with an estimated size, then adjust it using the actual font metrics.
        let estimated_size =
            estimate_term_size(window_size.width, window_size.height, &self.renderer_config);
        self.open_initial_tab(estimated_size);
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
    fn drain_terminal_output(&mut self) {
        if self.tabs.drain_outputs() {
            self.refresh_active_terminal();
        }
    }

    fn refresh_active_terminal(&mut self) {
        if let (Some(renderer), Some(snapshot)) = (&mut self.renderer, self.tabs.active_snapshot())
        {
            let copy_mode = self.copy_mode.renderer_snapshot(&snapshot);
            renderer.set_terminal_snapshot(snapshot);
            renderer.set_copy_mode(copy_mode);
            self.needs_redraw = true;
        }
    }

    fn show_active_terminal_at_bottom(&mut self) {
        if let (Some(renderer), Some(snapshot)) = (&mut self.renderer, self.tabs.active_snapshot())
        {
            let copy_mode = self.copy_mode.renderer_snapshot(&snapshot);
            renderer.set_terminal_snapshot_at_bottom(snapshot);
            renderer.set_copy_mode(copy_mode);
            self.needs_redraw = true;
        }
    }

    fn refresh_tab_bar(&mut self) {
        let snapshot = self.tab_bar_snapshot();
        if let Some(renderer) = &mut self.renderer {
            renderer.set_tab_bar(snapshot);
            self.needs_redraw = true;
        }
    }

    fn tab_bar_snapshot(&self) -> TabBarSnapshot {
        TabBarSnapshot::new(self.tabs.titles(), self.tabs.active_index())
    }

    fn refresh_copy_mode(&mut self, snapshot: &ScreenSnapshot) {
        if let Some(renderer) = &mut self.renderer {
            renderer.set_copy_mode(self.copy_mode.renderer_snapshot(snapshot));
            self.needs_redraw = true;
        }
    }

    fn leave_copy_mode(&mut self) {
        if !self.copy_mode.is_active() {
            return;
        }

        self.copy_mode.exit();
        if let Some(renderer) = &mut self.renderer {
            renderer.set_copy_mode(Default::default());
            self.needs_redraw = true;
        }
        self.update_window_title();
    }

    fn update_window_title(&self) {
        let title = if self.copy_mode.is_active() {
            "Solito - Copy Mode"
        } else {
            "Solito"
        };

        if let Some(window) = &self.window {
            window.set_title(title);
        }
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
        match event {
            WindowEvent::CloseRequested => {
                if self
                    .window
                    .as_ref()
                    .is_some_and(|window| window.id() == window_id)
                {
                    self.window = None;
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                self.needs_redraw = false;
                if let Some(renderer) = &mut self.renderer
                    && let Err(err) = renderer.draw_frame()
                {
                    error!("{err}");
                    event_loop.exit();
                }
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
                // Only keyboard input is translated into commands by input.rs before execution.
                let command = input::handle_key(
                    text,
                    &logical_key,
                    key_state,
                    self.modifiers,
                    self.copy_mode.is_active(),
                );
                if let Err(err) = self.handle_keyboard_command(command, event_loop) {
                    error!("application command failed: {err}");
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::Resized(window_size) => {
                let Some(renderer) = &self.renderer else {
                    return;
                };
                let terminal_size = renderer.terminal_size_for(window_size);
                if let Err(err) = self.tabs.resize_all(terminal_size) {
                    error!("application resize failed: {err}");
                    return;
                }
                if let (Some(renderer), Some(snapshot)) =
                    (&mut self.renderer, self.tabs.active_snapshot())
                {
                    let copy_mode = self.copy_mode.renderer_snapshot(&snapshot);
                    renderer.resize(window_size, snapshot);
                    renderer.set_copy_mode(copy_mode);
                    self.needs_redraw = true;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let Some(renderer) = &mut self.renderer else {
                    return;
                };

                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => renderer.scroll(x, y),
                    winit::event::MouseScrollDelta::PixelDelta(position) => renderer.scroll(
                        position.x as f32 / self.renderer_config.line_height,
                        position.y as f32 / self.renderer_config.line_height,
                    ),
                }
                self.needs_redraw = true;
            }
            _ => {
                tracing::debug!("unhandled event: {event:?}");
            }
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
