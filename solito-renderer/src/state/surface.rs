//! Window surface creation and platform-specific backdrop setup.

use std::sync::Arc;
use wgpu::{Surface, SurfaceConfiguration};
use winit::{dpi::PhysicalSize, window::Window};

use super::gpu::GpuContext;
use crate::{RendererConfig, WindowBackdrop};

pub(super) struct WindowSurface {
    pub(super) surface: Surface<'static>,
    pub(super) config: SurfaceConfiguration,
    pub(super) window: Arc<Window>,
    pub(super) clear_color: wgpu::Color,
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

        let surface_caps = surface.get_capabilities(&gpu.adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let alpha_mode = Self::choose_alpha_mode(&surface_caps);
        let clear_color = Self::clear_color(alpha_mode, renderer_config);

        let config = SurfaceConfiguration {
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

            // SAFETY: BLACK_BRUSH is a valid stock-object identifier and the
            // returned system-owned brush remains valid for the process lifetime.
            let brush = unsafe { GetStockObject(BLACK_BRUSH) };
            if brush.is_null() {
                tracing::warn!("failed to get stock black brush for initial background");
                return;
            }

            // wgpu can fail to acquire a surface texture while the window is hidden
            // or just becoming visible. Set the native erase background to black so
            // a one-frame OS fallback paint matches the terminal instead of flashing white.
            // SAFETY: hwnd comes from winit's live Win32 window and brush is the
            // valid stock brush checked above.
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

            let hwnd = handle.hwnd.get() as HWND;
            let dark_mode = 1;
            // SAFETY: hwnd is owned by the live winit window; the attribute
            // pointer and byte size refer to `dark_mode` for this call only.
            let dark_hr = unsafe {
                DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                    &dark_mode as *const _ as *const c_void,
                    size_of_val(&dark_mode) as u32,
                )
            };
            if dark_hr < 0 {
                tracing::debug!(hr = dark_hr, "failed to apply immersive dark mode");
            }

            let backdrop = DWMSBT_TRANSIENTWINDOW;
            // SAFETY: the attribute pointer and size refer to the local
            // `backdrop` value for the duration of the synchronous call.
            let hr = unsafe {
                DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_SYSTEMBACKDROP_TYPE as u32,
                    &backdrop as *const _ as *const c_void,
                    size_of_val(&backdrop) as u32,
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

            // SAFETY: the library name is a static NUL-terminated string.
            let user32 = unsafe { LoadLibraryA(c"user32.dll".as_ptr() as PCSTR) };
            if user32.is_null() {
                tracing::warn!("failed to load user32.dll for acrylic");
                return;
            }

            // SAFETY: both the module handle and symbol name are valid.
            let Some(function) = (unsafe {
                GetProcAddress(user32, c"SetWindowCompositionAttribute".as_ptr() as PCSTR)
            }) else {
                tracing::warn!("SetWindowCompositionAttribute is unavailable");
                return;
            };

            // SAFETY: Windows exports this symbol with the signature declared
            // by `SetWindowCompositionAttribute`.
            let set_window_composition_attribute =
                unsafe { std::mem::transmute::<_, SetWindowCompositionAttribute>(function) };

            let (r, g, b, mut a) = acrylic_tint;
            if a == 0 {
                a = 1;
            }

            let mut policy = AccentPolicy {
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
                size_of_data: size_of::<AccentPolicy>(),
            };

            // SAFETY: hwnd belongs to the live winit window and `data` points to
            // the live `policy` value with the matching C layout.
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
