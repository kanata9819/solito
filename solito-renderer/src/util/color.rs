pub struct ThemeColor;
impl ThemeColor {
    // u8 color
    pub const WHITE: [u8; 4] = [255, 255, 255, 255];
    pub const BLUE_GRAY_400: [u8; 4] = [140, 148, 160, 255];
    pub const BLUE_GRAY_700: [u8; 4] = [80, 88, 100, 255];

    // f32 color
    pub const NAVY_900_ALPHA: [f32; 4] = [0.055, 0.075, 0.118, 0.72];
    pub const NAVY_800_ALPHA: [f32; 4] = [0.118, 0.161, 0.220, 0.96];
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
