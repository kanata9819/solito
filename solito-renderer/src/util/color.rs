pub struct ThemeColor;
impl ThemeColor {
    // Midnight: deep blue-black surfaces with a cyan-to-violet neon accent.
    pub const BACKGROUND: [u8; 4] = [11, 14, 21, 255];
    pub const CHROME: [u8; 4] = [7, 9, 14, 255];
    pub const BORDER: [u8; 4] = [30, 36, 52, 255];
    pub const TAB_ACTIVE: [u8; 4] = [20, 26, 40, 255];
    pub const TAB_HOVER: [u8; 4] = [15, 19, 30, 255];
    pub const TEXT: [u8; 4] = [222, 229, 242, 255];
    pub const TEXT_MUTED: [u8; 4] = [136, 148, 172, 255];
    pub const TEXT_DIM: [u8; 4] = [84, 94, 118, 255];
    pub const ACCENT: [u8; 4] = [64, 216, 255, 255];
    pub const ACCENT_END: [u8; 4] = [178, 120, 255, 255];
    // u8 color
    pub const BLACK: [u8; 4] = [0, 0, 0, 255];
    pub const WHITE: [u8; 4] = [255, 255, 255, 255];

    // f32 color
    pub const WHITE_ALPHA: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
    pub const SELECTION_ALPHA: [f32; 4] = [0.25, 0.85, 1.0, 0.24];
    pub const COPY_CURSOR_ALPHA: [f32; 4] = [0.70, 0.47, 1.0, 0.55];
}

/// Choose the encoding the rect pipeline expects for this target.
pub fn target_rgba(color: [u8; 4], srgb_target: bool) -> [f32; 4] {
    if srgb_target {
        srgb_to_linear_rgba(color)
    } else {
        rgba_to_f32(color)
    }
}

/// Same color at a different coverage.
pub fn with_alpha([r, g, b, _]: [f32; 4], alpha: f32) -> [f32; 4] {
    [r, g, b, alpha]
}

pub fn rgba_to_f32([r, g, b, a]: [u8; 4]) -> [f32; 4] {
    [
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
        f32::from(a) / 255.0,
    ]
}

/// Convert terminal sRGB colors to linear RGB for an sRGB render target.
/// Alpha represents coverage and must not undergo gamma conversion.
pub fn srgb_to_linear_rgba(color: [u8; 4]) -> [f32; 4] {
    let [r, g, b, a] = rgba_to_f32(color);
    let linear = |channel: f32| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    [linear(r), linear(g), linear(b), a]
}

#[cfg(test)]
mod tests {
    use super::srgb_to_linear_rgba;

    #[test]
    fn dark_background_round_trips_through_srgb_target() {
        let [r, g, b, a] = srgb_to_linear_rgba([30, 30, 30, 128]);
        assert!((r - 0.012983).abs() < 0.000001);
        assert_eq!(r, g);
        assert_eq!(g, b);
        let encoded = 1.055 * r.powf(1.0 / 2.4) - 0.055;
        assert!((encoded * 255.0 - 30.0).abs() < 0.001);
        assert_eq!(a, 128.0 / 255.0);
    }
}
