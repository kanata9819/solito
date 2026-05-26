use crate::util::color::ThemeColor;

pub struct TabView {}

impl TabView {
    pub const TAB_ACTIVE_TEXT_COLOR: [u8; 4] = ThemeColor::WHITE;
    pub const TAB_INACTIVE_TEXT_COLOR: [u8; 4] = ThemeColor::BLUE_GRAY_400;
    pub const TAB_SEPARATOR_COLOR: [u8; 4] = ThemeColor::BLUE_GRAY_700;

    pub const TAB_ACTIVE_BACKGROUND: [f32; 4] = ThemeColor::NAVY_800_ALPHA;
    pub const TAB_INACTIVE_BACKGROUND: [f32; 4] = ThemeColor::NAVY_900_ALPHA;
    pub const TAB_ACTIVE_UNDERLINE: [f32; 4] = ThemeColor::CYAN_400;
    pub const TAB_ACTIVE_TOP_GLOW: [f32; 4] = ThemeColor::CYAN_GLOW;

    pub const TAB_TEXT_PADDING: usize = 2;
    pub const TAB_GAP_CHARS: usize = 1;
    pub const TAB_SLANT: f32 = 10.0;
    pub const TAB_TOP_GLOW_HEIGHT: f32 = 1.0;
    pub const TAB_UNDERLINE_HEIGHT: f32 = 3.0;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TabBarSnapshot {
    titles: Vec<String>,
    active_index: usize,
}

impl TabBarSnapshot {
    pub fn new(titles: Vec<String>, active_index: usize) -> Self {
        let active_index: usize = active_index.min(titles.len().saturating_sub(1));

        Self {
            titles,
            active_index,
        }
    }

    pub(crate) fn titles(&self) -> &[String] {
        &self.titles
    }

    pub(crate) fn active_index(&self) -> usize {
        self.active_index
    }
}

#[cfg(test)]
mod tests {
    use super::TabBarSnapshot;

    #[test]
    fn clamps_active_index_to_existing_tabs() {
        let snapshot: TabBarSnapshot = TabBarSnapshot::new(vec!["Tab 1".to_string()], 99);

        assert_eq!(snapshot.active_index(), 0);
    }
}
