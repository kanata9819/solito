use std::sync::LazyLock;

pub struct ThemeColor;
impl ThemeColor {
    pub const BACKGROUND: [u8; 4] = [20, 24, 27, 255];
    pub const CHROME: [u8; 4] = [18, 22, 25, 255];
    pub const BORDER: [u8; 4] = [43, 51, 55, 255];
    pub const TAB_ACTIVE: [u8; 4] = [31, 39, 43, 255];
    pub const TAB_HOVER: [u8; 4] = [27, 33, 37, 255];
    pub const TEXT: [u8; 4] = [218, 225, 228, 255];
    pub const TEXT_MUTED: [u8; 4] = [145, 157, 164, 255];
    pub const TEXT_DIM: [u8; 4] = [111, 123, 130, 255];
    pub const ACCENT: [u8; 4] = [124, 174, 166, 255];
    // u8 color
    pub const BLACK: [u8; 4] = [0, 0, 0, 255];
    pub const WHITE: [u8; 4] = [255, 255, 255, 255];

    // f32 color
    pub const WHITE_ALPHA: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
    pub const SELECTION_ALPHA: [f32; 4] = [0.20, 0.42, 0.38, 0.28];
    pub const COPY_CURSOR_ALPHA: [f32; 4] = [0.73, 0.58, 0.28, 0.55];
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
pub fn srgb_to_linear_rgba([r, g, b, a]: [u8; 4]) -> [f32; 4] {
    static LINEAR_CHANNELS: LazyLock<[f32; 256]> = LazyLock::new(|| {
        std::array::from_fn(|value| {
            let channel = value as f32 / 255.0;
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        })
    });
    let channels = &*LINEAR_CHANNELS;
    [
        channels[usize::from(r)],
        channels[usize::from(g)],
        channels[usize::from(b)],
        f32::from(a) / 255.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::srgb_to_linear_rgba;

    #[test]
    fn every_channel_matches_the_srgb_transfer_function() {
        for value in 0..=u8::MAX {
            let normalized = f32::from(value) / 255.0;
            let expected = if value <= 10 {
                normalized / 12.92
            } else {
                ((normalized + 0.055) / 1.055).powf(2.4)
            };
            assert_eq!(
                srgb_to_linear_rgba([value; 4]),
                [expected, expected, expected, normalized],
            );
        }
    }

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
