pub struct RendererConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    pub const FONT_SIZE: f32 = 20.0;
    pub const LINE_HEIGHT: f32 = 30.0;
    pub const WINDOW_BACKDROP: WindowBackdrop = WindowBackdrop::Acrylic;
    pub const WINDOW_ACRYLIC_TINT: (u8, u8, u8, u8) = (18, 18, 18, 190);
}
