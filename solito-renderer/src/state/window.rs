use glyphon::{Color, Resolution, TextArea, TextBounds};
use solito_terminal::ScreenSnapshot;
use std::{error::Error, sync::Arc};
use wgpu::{
    CommandEncoder, CommandEncoderDescriptor, Surface, SurfaceConfiguration, SurfaceTexture,
    TextureView, TextureViewDescriptor,
};
use winit::{dpi::PhysicalSize, window::Window};

use super::{context::State, gpu::GpuContext};
use crate::{
    RendererConfig, WindowBackdrop, pass, pipeline::rect, terminal_view::TerminalView,
    util::color::ThemeColor,
};

pub(super) struct WindowSurface {
    pub(super) surface: Surface<'static>,
    pub(super) config: SurfaceConfiguration,
    pub(super) is_configured: bool,
    pub(super) window: Arc<Window>,
    clear_color: wgpu::Color,
}

impl WindowSurface {
    pub(super) fn new(
        window: Arc<Window>,
        surface: Surface<'static>,
        gpu: &GpuContext,
        size: PhysicalSize<u32>,
        renderer_config: &RendererConfig,
    ) -> Self {
        Self::apply_initial_window_background(window.as_ref());
        Self::apply_window_effects(window.as_ref(), renderer_config);

        let surface_caps: wgpu::SurfaceCapabilities = surface.get_capabilities(&gpu.adapter);

        let surface_format: wgpu::TextureFormat = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let alpha_mode: wgpu::CompositeAlphaMode = Self::choose_alpha_mode(&surface_caps);
        let clear_color: wgpu::Color = Self::clear_color(alpha_mode, renderer_config);

        let config: SurfaceConfiguration = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        Self {
            surface,
            config,
            is_configured: false,
            window,
            clear_color,
        }
    }

    fn choose_alpha_mode(surface_caps: &wgpu::SurfaceCapabilities) -> wgpu::CompositeAlphaMode {
        if surface_caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PostMultiplied)
        {
            wgpu::CompositeAlphaMode::PostMultiplied
        } else if surface_caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
        {
            wgpu::CompositeAlphaMode::PreMultiplied
        } else if surface_caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::Inherit)
        {
            wgpu::CompositeAlphaMode::Inherit
        } else {
            wgpu::CompositeAlphaMode::Auto
        }
    }

    fn clear_color(
        _alpha_mode: wgpu::CompositeAlphaMode,
        renderer_config: &RendererConfig,
    ) -> wgpu::Color {
        if renderer_config.window_backdrop.is_transparent() {
            wgpu::Color::TRANSPARENT
        } else {
            wgpu::Color::BLACK
        }
    }

    fn apply_window_effects(window: &Window, renderer_config: &RendererConfig) {
        match renderer_config.window_backdrop {
            WindowBackdrop::None | WindowBackdrop::Transparent => {}
            WindowBackdrop::Acrylic => {
                Self::apply_platform_acrylic(window, renderer_config);
            }
        };
    }

    fn apply_initial_window_background(window: &Window) {
        if cfg!(target_os = "windows") {
            use windows_sys::Win32::{
                Foundation::HWND,
                Graphics::Gdi::{BLACK_BRUSH, GetStockObject},
                UI::WindowsAndMessaging::{GCLP_HBRBACKGROUND, SetClassLongPtrW},
            };
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

            let Ok(handle) = window.window_handle() else {
                tracing::warn!("failed to get window handle for initial background");
                return;
            };
            let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                return;
            };

            let brush = unsafe { GetStockObject(BLACK_BRUSH) };
            if brush.is_null() {
                tracing::warn!("failed to get stock black brush for initial background");
                return;
            }

            // wgpu can fail to acquire a surface texture while the window is hidden
            // or just becoming visible. Set the native erase background to black so
            // a one-frame OS fallback paint matches the terminal instead of flashing white.
            unsafe {
                SetClassLongPtrW(
                    handle.hwnd.get() as HWND,
                    GCLP_HBRBACKGROUND,
                    brush as isize,
                );
            }
        } else {
            let _ = window;
        }
    }

    fn apply_platform_acrylic(window: &Window, renderer_config: &RendererConfig) {
        if cfg!(target_os = "windows") {
            if !Self::apply_windows_system_acrylic(window) {
                Self::apply_windows_accent_acrylic(window, renderer_config.window_acrylic_tint);
            }
        } else {
            let _ = (window, renderer_config);
        }
    }

    fn apply_windows_system_acrylic(window: &Window) -> bool {
        if cfg!(target_os = "windows") {
            use std::ffi::c_void;
            use windows_sys::Win32::{
                Foundation::HWND,
                Graphics::Dwm::{
                    DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE,
                    DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
                },
            };
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

            let Ok(handle) = window.window_handle() else {
                tracing::warn!("failed to get window handle for system acrylic");
                return false;
            };
            let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                return false;
            };

            let hwnd: HWND = handle.hwnd.get() as HWND;
            let dark_mode: i32 = 1;
            let dark_hr = unsafe {
                DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                    &dark_mode as *const _ as *const c_void,
                    std::mem::size_of_val(&dark_mode) as u32,
                )
            };
            if dark_hr < 0 {
                tracing::debug!(hr = dark_hr, "failed to apply immersive dark mode");
            }

            let backdrop = DWMSBT_TRANSIENTWINDOW;
            let hr = unsafe {
                DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_SYSTEMBACKDROP_TYPE as u32,
                    &backdrop as *const _ as *const c_void,
                    std::mem::size_of_val(&backdrop) as u32,
                )
            };

            if hr < 0 {
                tracing::warn!(hr, "failed to apply DWM system acrylic backdrop");
                return false;
            }

            tracing::info!("applied DWM system acrylic backdrop");
            return true;
        }

        let _ = window;
        false
    }

    fn apply_windows_accent_acrylic(window: &Window, acrylic_tint: (u8, u8, u8, u8)) {
        if cfg!(target_os = "windows") {
            use std::ffi::c_void;
            use windows_sys::Win32::{
                Foundation::HWND,
                System::LibraryLoader::{GetProcAddress, LoadLibraryA},
            };
            use windows_sys::core::{BOOL, PCSTR};
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

            #[repr(C)]
            struct AccentPolicy {
                accent_state: u32,
                accent_flags: u32,
                gradient_color: u32,
                animation_id: u32,
            }

            #[repr(C)]
            struct WindowCompositionAttribData {
                attrib: u32,
                data: *mut c_void,
                size_of_data: usize,
            }

            type SetWindowCompositionAttribute =
                unsafe extern "system" fn(HWND, *mut WindowCompositionAttribData) -> BOOL;

            const WCA_ACCENT_POLICY: u32 = 0x13;
            const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;

            let Ok(handle) = window.window_handle() else {
                tracing::warn!("failed to get window handle for acrylic");
                return;
            };
            let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                return;
            };

            let user32 = unsafe { LoadLibraryA(c"user32.dll".as_ptr() as PCSTR) };
            if user32.is_null() {
                tracing::warn!("failed to load user32.dll for acrylic");
                return;
            }

            let Some(function) = (unsafe {
                GetProcAddress(user32, c"SetWindowCompositionAttribute".as_ptr() as PCSTR)
            }) else {
                tracing::warn!("SetWindowCompositionAttribute is unavailable");
                return;
            };

            let set_window_composition_attribute: SetWindowCompositionAttribute =
                unsafe { std::mem::transmute(function) };

            let (r, g, b, mut a) = acrylic_tint;
            if a == 0 {
                a = 1;
            }

            let mut policy: AccentPolicy = AccentPolicy {
                accent_state: ACCENT_ENABLE_ACRYLICBLURBEHIND,
                accent_flags: 0,
                gradient_color: r as u32
                    | ((g as u32) << 8)
                    | ((b as u32) << 16)
                    | ((a as u32) << 24),
                animation_id: 0,
            };
            let mut data = WindowCompositionAttribData {
                attrib: WCA_ACCENT_POLICY,
                data: &mut policy as *mut _ as *mut c_void,
                size_of_data: std::mem::size_of::<AccentPolicy>(),
            };

            if unsafe { set_window_composition_attribute(handle.hwnd.get() as HWND, &mut data) }
                == 0
            {
                tracing::warn!("failed to apply accent acrylic window backdrop");
            }
        } else {
            let _ = (window, acrylic_tint);
        }
    }
}

impl State {
    fn prepare_render(&mut self) -> Result<(), Box<dyn Error>> {
        let [default_r, default_g, default_b, _] = ThemeColor::WHITE;

        self.terminal_view.glyphs.text_renderer.prepare(
            &self.gpu.device,
            &self.gpu.queue,
            &mut self.terminal_view.glyphs.font_system,
            &mut self.terminal_view.glyphs.atlas,
            &self.terminal_view.glyphs.viewport,
            [TextArea {
                buffer: &self.terminal_view.glyphs.text_buffer,
                left: TerminalView::PADDING_X,
                top: TerminalView::PADDING_Y,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    ..Default::default()
                },
                default_color: Color::rgb(default_r, default_g, default_b),
                custom_glyphs: &[],
            }],
            &mut self.terminal_view.glyphs.swash_cache,
        )?;

        Ok(())
    }

    fn render_pass(
        &mut self,
        encoder: &mut CommandEncoder,
        view: TextureView,
    ) -> Result<(), Box<dyn Error>> {
        self.update_rect_screen_uniform();

        let mut rects: Vec<rect::RectSpec> = self
            .terminal_view
            .tab_bar_rects(self.window_surface.config.width);
        rects.extend(self.terminal_view.copy_mode_rects());

        // Copy mode draws its own cursor over the scrollback; hide the shell cursor
        // so the user does not see two active cursor positions at once.
        if !self.terminal_view.copy_mode_active() {
            let (caret_x, caret_y, caret_w, caret_h) = self.terminal_view.caret_rect();

            if caret_w > 0.0 && caret_h > 0.0 {
                rects.push(rect::RectSpec::new(
                    caret_x,
                    caret_y,
                    caret_w,
                    caret_h,
                    self.terminal_view.caret_color(),
                ));
            }
        }

        let rect_instance_buffer: Option<wgpu::Buffer> =
            rect::RectPipeline::create_instance_buffer(&self.gpu.device, &rects);
        let rect_bind_group: wgpu::BindGroup = self
            .render_resources
            .rect_pipeline
            .rect_bind_group(&self.gpu.device, &self.render_resources.uniform_buffer);

        let mut pass: wgpu::RenderPass<'_> =
            pass::begin_render_pass(encoder, &view, self.window_surface.clear_color);

        if let Some(rect_instance_buffer) = rect_instance_buffer.as_ref() {
            self.render_resources.rect_pipeline.draw_rects(
                &mut pass,
                rect_bind_group,
                rect_instance_buffer,
                rects.len(),
            );
        }

        self.terminal_view.glyphs.text_renderer.render(
            &self.terminal_view.glyphs.atlas,
            &self.terminal_view.glyphs.viewport,
            &mut pass,
        )?;

        Ok(())
    }

    fn initialize_frame(&mut self) -> Result<Option<SurfaceTexture>, Box<dyn Error>> {
        for _ in 0..2 {
            match self.window_surface.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => return Ok(Some(frame)),
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    self.window_surface
                        .surface
                        .configure(&self.gpu.device, &self.window_surface.config);
                    return Ok(Some(frame));
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    self.window_surface.window.request_redraw();
                    return Ok(None);
                }
                wgpu::CurrentSurfaceTexture::Outdated => {
                    self.window_surface
                        .surface
                        .configure(&self.gpu.device, &self.window_surface.config);
                }
                wgpu::CurrentSurfaceTexture::Lost => {
                    self.window_surface.surface = self
                        .gpu
                        .instance
                        .create_surface(self.window_surface.window.clone())?;
                    self.window_surface
                        .surface
                        .configure(&self.gpu.device, &self.window_surface.config);
                }
                wgpu::CurrentSurfaceTexture::Validation => panic!("validation error"),
            }
        }

        self.window_surface.window.request_redraw();
        Ok(None)
    }

    fn create_encoder(&self, label: Option<&str>) -> CommandEncoder {
        self.gpu
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label })
    }
}

impl State {
    pub fn resize(&mut self, size: PhysicalSize<u32>, snapshot: ScreenSnapshot) {
        if size.width > 0 && size.height > 0 {
            self.window_surface.config.width = size.width;
            self.window_surface.config.height = size.height;
            self.window_surface
                .surface
                .configure(&self.gpu.device, &self.window_surface.config);
            self.window_surface.is_configured = true;

            self.terminal_view.resize(size.width, size.height, snapshot);
            self.update_rect_screen_uniform();
        }
    }

    pub fn draw_frame(&mut self) -> Result<(), Box<dyn Error>> {
        self.terminal_view.glyphs.viewport.update(
            &self.gpu.queue,
            Resolution {
                width: self.window_surface.config.width,
                height: self.window_surface.config.height,
            },
        );

        self.prepare_render()?;

        let frame: Option<SurfaceTexture> = match self.initialize_frame() {
            Ok(Some(frame)) => Some(frame),
            Ok(None) => None,
            Err(err) => {
                tracing::error!("initialize frame failed: {}", err);
                None
            }
        };

        if let Some(frame) = frame {
            let view: TextureView = frame.texture.create_view(&TextureViewDescriptor::default());
            let mut encoder: CommandEncoder = self.create_encoder(None);
            self.render_pass(&mut encoder, view)?;
            self.gpu.queue.submit(Some(encoder.finish()));
            self.gpu.queue.present(frame);
        }

        self.terminal_view.glyphs.atlas.trim();

        Ok(())
    }

    pub fn scroll(&mut self, x: f32, y: f32) {
        self.terminal_view.scroll(x, y);
    }
}
