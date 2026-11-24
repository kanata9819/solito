use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub struct RendererConfig {
    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub window_backdrop: WindowBackdrop,
    pub window_acrylic_tint: (u8, u8, u8, u8),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WindowBackdrop {
    None,
    Transparent,
    Acrylic,
}

impl WindowBackdrop {
    pub const fn is_transparent(self) -> bool {
        !matches!(self, Self::None)
    }
}

impl RendererConfig {
    #[cfg(target_os = "windows")]
    pub const DEFAULT_FONT_FAMILY: &str = "Cascadia Mono";
    /// Generic name resolved to the system's default monospace font.
    #[cfg(not(target_os = "windows"))]
    pub const DEFAULT_FONT_FAMILY: &str = "monospace";
    pub const DEFAULT_FONT_SIZE: f32 = 20.0;
    pub const DEFAULT_LINE_HEIGHT: f32 = 30.0;
    pub const DEFAULT_WINDOW_BACKDROP: WindowBackdrop = WindowBackdrop::Acrylic;
    pub const DEFAULT_WINDOW_ACRYLIC_TINT: (u8, u8, u8, u8) = (18, 18, 18, 190);

    pub fn sanitized(mut self) -> Self {
        if self.font_family.trim().is_empty() {
            self.font_family = Self::DEFAULT_FONT_FAMILY.to_string();
        }
        self.font_size = sanitize_pixel_size(self.font_size, Self::DEFAULT_FONT_SIZE);
        self.line_height = sanitize_pixel_size(self.line_height, Self::DEFAULT_LINE_HEIGHT);

        self
    }
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            font_family: Self::DEFAULT_FONT_FAMILY.to_string(),
            font_size: Self::DEFAULT_FONT_SIZE,
            line_height: Self::DEFAULT_LINE_HEIGHT,
            window_backdrop: Self::DEFAULT_WINDOW_BACKDROP,
            window_acrylic_tint: Self::DEFAULT_WINDOW_ACRYLIC_TINT,
        }
    }
}

pub(crate) fn sanitize_pixel_size(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value >= 1.0 {
        value
    } else {
        fallback
    }
}
