use winit::window::Icon;

const ICON_WIDTH: u32 = 64;
const ICON_HEIGHT: u32 = 64;
const ICON_RGBA: &[u8] = include_bytes!("../../assets/solito-icon-64.rgba");

pub(super) fn window_icon() -> Option<Icon> {
    Icon::from_rgba(ICON_RGBA.to_vec(), ICON_WIDTH, ICON_HEIGHT).ok()
}

pub(super) fn taskbar_icon() -> Option<Icon> {
    Icon::from_rgba(ICON_RGBA.to_vec(), ICON_WIDTH, ICON_HEIGHT).ok()
}

#[cfg(test)]
mod tests {
    use super::{ICON_HEIGHT, ICON_RGBA, ICON_WIDTH};

    #[test]
    fn bundled_icon_rgba_size_matches_dimensions() {
        assert_eq!(
            ICON_RGBA.len(),
            ICON_WIDTH as usize * ICON_HEIGHT as usize * 4
        );
    }
}
