mod command;
mod mouse;

use anyhow::Result;
use solito_config::app::AppConfig;
use solito_renderer::{Renderer, RendererConfig, TabBarSnapshot, TerminalSize};
use solito_terminal::ScreenSnapshot;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::error;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy},
    keyboard::ModifiersState,
    window::{Window, WindowAttributes, WindowId},
};

use crate::app::{copy::CopyMode, event::AppEvent, icon, input, tabs::AppTabs};

pub(super) type AppResult<T = ()> = Result<T>;
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(500);

pub(crate) struct SolitoApplication {
    config: AppConfig,
    renderer_config: RendererConfig,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    tabs: AppTabs,
    copy_mode: CopyMode,
    modifiers: ModifiersState,
    ime: input::ImeInput,
    mouse: mouse::MouseInputState,
    event_proxy: EventLoopProxy<AppEvent>,
    // State updates set this flag; about_to_wait requests a redraw; RedrawRequested draws.
    needs_redraw: bool,
    terminal_dirty: bool,
    cursor_blink_visible: bool,
    next_cursor_blink: Instant,
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
            ime: Default::default(),
            mouse: Default::default(),
            event_proxy,
            needs_redraw: false,
            terminal_dirty: false,
            cursor_blink_visible: true,
            next_cursor_blink: Instant::now() + CURSOR_BLINK_INTERVAL,
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
        window.set_ime_allowed(true);
        let window_size = window.inner_size();
        let mut renderer = self.create_renderer(&window)?;
        // The renderer already measured the font; reuse its exact grid size.
        self.open_initial_tab(renderer.terminal_size());

        self.tabs.drain_outputs();
        renderer.set_tab_bar(self.tab_bar_snapshot());
        if let Some(snapshot) = self.tabs.active_snapshot() {
            renderer.resize(window_size, snapshot);
        }

        renderer.draw_frame()?;
        window.set_visible(true);
        self.next_cursor_blink = Instant::now() + CURSOR_BLINK_INTERVAL;

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
            .with_theme(Some(winit::window::Theme::Dark))
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
            self.terminal_dirty = true;
            self.needs_redraw = true;
            self.cursor_blink_visible = true;
            self.next_cursor_blink = Instant::now() + CURSOR_BLINK_INTERVAL;
        }
    }

    fn close_window(&mut self, event_loop: &ActiveEventLoop) {
        // The renderer also owns the native window; hide it before exiting.
        if let Some(window) = self.window.take() {
            window.set_visible(false);
        }
        event_loop.exit();
    }

    fn close_exited_tabs(&mut self, event_loop: &ActiveEventLoop) {
        let closed = self.tabs.close_exited();
        if !closed.any_closed {
            return;
        }
        if self.tabs.is_empty() {
            self.close_window(event_loop);
            return;
        }
        if closed.active_closed {
            self.leave_copy_mode();
            self.clear_ime();
            self.show_active_terminal_at_bottom();
            self.update_ime_cursor_area();
        }
        self.refresh_tab_bar();
    }

    fn refresh_active_terminal(&mut self) {
        self.terminal_dirty = false;
        if let (Some(renderer), Some(mut snapshot)) =
            (&mut self.renderer, self.tabs.active_snapshot())
        {
            snapshot.cursor_visible &= self.cursor_blink_visible;
            let copy_mode = self.copy_mode.renderer_snapshot(&snapshot);
            renderer.set_terminal_snapshot(snapshot);
            renderer.set_copy_mode(copy_mode);
            self.needs_redraw = true;
        }
    }

    fn resize_window(&mut self, window_size: PhysicalSize<u32>) -> AppResult {
        // Minimized windows have no drawable area. Resizing the PTY to 1x1
        // would clamp its cursor and scroll region before the window returns.
        if window_size.width == 0 || window_size.height == 0 {
            return Ok(());
        }

        let Some(renderer) = &self.renderer else {
            return Ok(());
        };
        let terminal_size = renderer.terminal_size_for(window_size);
        if terminal_size != renderer.terminal_size() {
            self.tabs.resize_all(terminal_size)?;
        }
        if let (Some(renderer), Some(mut snapshot)) =
            (&mut self.renderer, self.tabs.active_snapshot())
        {
            snapshot.cursor_visible &= self.cursor_blink_visible;
            let copy_mode = self.copy_mode.renderer_snapshot(&snapshot);
            renderer.resize(window_size, snapshot);
            renderer.set_copy_mode(copy_mode);
            self.terminal_dirty = false;
            self.needs_redraw = true;
        }
        Ok(())
    }

    fn restore_window(&mut self) -> AppResult {
        let Some(window_size) = self.window.as_ref().map(|window| window.inner_size()) else {
            return Ok(());
        };
        self.drain_terminal_output();
        self.cursor_blink_visible = true;
        self.next_cursor_blink = Instant::now() + CURSOR_BLINK_INTERVAL;
        // Reconfigure at the current physical size and rebuild glyph positions,
        // even if no terminal output or resize event accompanied activation.
        self.resize_window(window_size)?;
        self.update_ime_cursor_area();
        Ok(())
    }

    fn show_active_terminal_at_bottom(&mut self) {
        self.terminal_dirty = false;
        let position = self.mouse.position;
        self.mouse = Default::default();
        self.mouse.position = position;
        if let (Some(renderer), Some(mut snapshot)) =
            (&mut self.renderer, self.tabs.active_snapshot())
        {
            snapshot.cursor_visible &= self.cursor_blink_visible;
            let copy_mode = self.copy_mode.renderer_snapshot(&snapshot);
            renderer.set_terminal_snapshot_at_bottom(snapshot);
            renderer.set_copy_mode(copy_mode);
            self.needs_redraw = true;
        }
    }

    fn scroll_terminal_to_bottom(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            self.needs_redraw |= renderer.scroll_to_bottom();
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
        if let Some(window) = &self.window {
            window.set_ime_allowed(true);
        }
        if let Some(renderer) = &mut self.renderer {
            renderer.set_copy_mode(Default::default());
            self.needs_redraw = true;
        }
        self.update_window_title();
    }

    fn clear_ime(&mut self) {
        self.ime = Default::default();
        self.refresh_ime_preedit();
    }

    fn refresh_ime_preedit(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            renderer.set_ime_preedit(self.ime.preedit.clone(), self.ime.cursor_range);
            self.needs_redraw = true;
        }
    }

    fn update_ime_cursor_area(&self) {
        if self.ime.enabled
            && let (Some(window), Some(renderer)) = (&self.window, &self.renderer)
        {
            let (position, size) = renderer.ime_cursor_area();
            window.set_ime_cursor_area(position, size);
        }
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

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::TerminalOutputReady => self.drain_terminal_output(),
            AppEvent::TerminalSessionExited => self.close_exited_tabs(event_loop),
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
                if let Some(window) = &self.window
                    && window.id() == window_id
                {
                    self.close_window(event_loop);
                }
            }
            WindowEvent::RedrawRequested => {
                // Collapse all PTY notifications since the last frame into one snapshot.
                if self.terminal_dirty {
                    self.refresh_active_terminal();
                }
                self.needs_redraw = false;
                if let Some(renderer) = &mut self.renderer
                    && let Err(err) = renderer.draw_frame()
                {
                    error!("{err}");
                    event_loop.exit();
                }
                self.update_ime_cursor_area();
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
                if self.config.window.hide_cursor
                    && key_state == ElementState::Pressed
                    && let Some(window) = &self.window
                {
                    window.set_cursor_visible(false);
                }

                // Only keyboard input is translated into commands by input.rs before execution.
                let command = input::handle_key(
                    text,
                    &logical_key,
                    key_state,
                    self.modifiers,
                    self.copy_mode.is_active(),
                    self.ime.is_composing(),
                );
                if let Err(err) = self.handle_keyboard_command(command, event_loop) {
                    error!("application command failed: {err}");
                }
            }
            WindowEvent::Ime(event) => {
                let command = self.ime.handle_event(event, self.copy_mode.is_active());
                if self.ime.is_composing() {
                    self.scroll_terminal_to_bottom();
                }
                if self.terminal_dirty {
                    self.refresh_active_terminal();
                }
                self.refresh_ime_preedit();
                if let Err(err) = self.handle_keyboard_command(command, event_loop) {
                    error!("IME input failed: {err}");
                }
                self.update_ime_cursor_area();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::Resized(window_size) => {
                if let Err(err) = self.resize_window(window_size) {
                    error!("application resize failed: {err}");
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.report_mouse_wheel(delta) {
                    return;
                }
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
            WindowEvent::CursorMoved { position, .. } => {
                self.update_tab_hover(Some(position));
                if self.config.window.hide_cursor
                    && let Some(window) = &self.window
                {
                    window.set_cursor_visible(true);
                }
                self.report_mouse_motion(position)
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if self.activate_clicked_tab(state, button) {
                    return;
                }
                self.report_mouse_button(state, button)
            }
            WindowEvent::CursorLeft { .. } => {
                self.mouse.position = None;
                self.update_tab_hover(None);
            }
            WindowEvent::Focused(true) => {
                if let Some(renderer) = &mut self.renderer {
                    self.needs_redraw |= renderer.set_focused(true);
                }
                if let Err(err) = self.restore_window() {
                    error!("application restore failed: {err}");
                }
            }
            WindowEvent::Occluded(false) => {
                if let Err(err) = self.restore_window() {
                    error!("application restore failed: {err}");
                }
            }
            WindowEvent::Focused(false) => {
                if let Some(renderer) = &mut self.renderer {
                    self.needs_redraw |= renderer.set_focused(false);
                }
                self.clear_ime();
                self.modifiers = ModifiersState::default();
                if self.config.window.hide_cursor
                    && let Some(window) = &self.window
                {
                    window.set_cursor_visible(true);
                }
                self.mouse = Default::default();
            }
            _ => {
                tracing::debug!("unhandled event: {event:?}");
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_cursor_blink {
            self.cursor_blink_visible = !self.cursor_blink_visible;
            self.next_cursor_blink = now + CURSOR_BLINK_INTERVAL;
            self.terminal_dirty = true;
            self.needs_redraw = true;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_cursor_blink));
        if self.needs_redraw
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use solito_terminal::TerminalState;
    use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows};

    #[test]
    #[ignore = "requires a Windows desktop and GPU adapter"]
    fn focus_restore_preserves_terminal_geometry_and_requests_redraw() {
        struct Regression {
            proxy: EventLoopProxy<AppEvent>,
            verified: bool,
        }

        impl ApplicationHandler<AppEvent> for Regression {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                let window = Arc::new(
                    event_loop
                        .create_window(
                            Window::default_attributes()
                                .with_inner_size(PhysicalSize::new(800, 500))
                                .with_visible(false),
                        )
                        .unwrap(),
                );
                let mut config = AppConfig::default();
                config.window.hide_cursor = false;
                let mut app = SolitoApplication::new(config, self.proxy.clone());
                app.renderer_config.window_backdrop = solito_config::renderer::WindowBackdrop::None;
                let renderer = app.create_renderer(&window).unwrap();
                let size = renderer.terminal_size();
                let mut terminal = TerminalState::new(size);
                terminal.apply_terminal_output(b"first\r\nsecond\x1b[5;16H");
                let before = terminal.snapshot();
                let (tabs, input_rx) = AppTabs::test_fixture(terminal, self.proxy.clone());
                app.tabs = tabs;
                app.window = Some(window.clone());
                app.renderer = Some(renderer);
                app.resize_window(window.inner_size()).unwrap();
                app.window_event(event_loop, window.id(), WindowEvent::Focused(false));
                app.needs_redraw = false;

                for zero_size in [(0, 0), (0, 500), (800, 0)] {
                    app.window_event(
                        event_loop,
                        window.id(),
                        WindowEvent::Resized(PhysicalSize::new(zero_size.0, zero_size.1)),
                    );
                    let snapshot = app.tabs.active_snapshot().unwrap();
                    assert_eq!(snapshot.cursor_col, before.cursor_col);
                    assert_eq!(snapshot.cursor_row, before.cursor_row);
                    assert!(snapshot.lines.iter().eq(before.lines.iter()));
                    assert_eq!(app.renderer.as_ref().unwrap().terminal_size(), size);
                    assert!(input_rx.try_recv().is_err(), "must not resize the PTY");
                }

                for event in [WindowEvent::Focused(true), WindowEvent::Occluded(false)] {
                    app.needs_redraw = false;
                    app.cursor_blink_visible = false;
                    app.window_event(event_loop, window.id(), event);
                    assert!(app.needs_redraw, "restoring must schedule a fresh frame");
                    assert!(app.cursor_blink_visible);
                    assert_eq!(app.renderer.as_ref().unwrap().terminal_size(), size);
                    let snapshot = app.tabs.active_snapshot().unwrap();
                    assert_eq!(snapshot.cursor_col, before.cursor_col);
                    assert_eq!(snapshot.cursor_row, before.cursor_row);
                    assert!(snapshot.lines.iter().eq(before.lines.iter()));
                    assert!(
                        input_rx.try_recv().is_err(),
                        "same grid needs no PTY resize"
                    );
                }

                // Simulate a stale surface size without a matching Resized event.
                app.renderer
                    .as_mut()
                    .unwrap()
                    .resize(PhysicalSize::new(400, 300), before.clone());
                assert_ne!(app.renderer.as_ref().unwrap().terminal_size(), size);
                app.window_event(event_loop, window.id(), WindowEvent::Focused(true));
                assert_eq!(app.renderer.as_ref().unwrap().terminal_size(), size);
                assert!(matches!(
                    input_rx.try_recv(),
                    Ok(crate::session::runtime::SessionInput::Resize(restored)) if restored == size
                ));

                self.verified = true;
                event_loop.exit();
            }

            fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
        }

        let event_loop = EventLoop::<AppEvent>::with_user_event()
            .with_any_thread(true)
            .build()
            .unwrap();
        let mut regression = Regression {
            proxy: event_loop.create_proxy(),
            verified: false,
        };
        event_loop.run_app(&mut regression).unwrap();
        assert!(regression.verified);
    }
}
