pub struct ThemeColor;
impl ThemeColor {
    // u8 color
    pub const BLACK: [u8; 4] = [0, 0, 0, 255];
    pub const WHITE: [u8; 4] = [255, 255, 255, 255];
    pub const BLUE_GRAY_400: [u8; 4] = [140, 148, 160, 255];
    pub const BLUE_GRAY_700: [u8; 4] = [80, 88, 100, 255];

    // f32 color
    pub const WHITE_ALPHA: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
    pub const NAVY_900_ALPHA: [f32; 4] = [0.055, 0.075, 0.118, 0.72];
    pub const NAVY_800_ALPHA: [f32; 4] = [0.118, 0.161, 0.220, 0.96];
    pub const BLUE_500_ALPHA: [f32; 4] = [0.2, 0.45, 0.95, 0.36];
    pub const YELLOW_400_ALPHA: [f32; 4] = [0.95, 0.84, 0.25, 0.55];
    pub const CYAN_400: [f32; 4] = [0.125, 0.827, 0.933, 1.0];
    pub const CYAN_GLOW: [f32; 4] = [0.408, 0.878, 1.0, 0.34];
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
