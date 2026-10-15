use glyphon::{
    Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, TextArea, TextBounds, Weight, Wrap,
};

use crate::{
    pipeline::rect::RectSpec,
    util::color::{ThemeColor, rgba_to_f32, srgb_to_linear_rgba},
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TabBarSnapshot {
    titles: Vec<String>,
    active_index: usize,
}

impl TabBarSnapshot {
    pub fn new(titles: Vec<String>, active_index: usize) -> Self {
        let active_index = active_index.min(titles.len().saturating_sub(1));
        Self {
            titles,
            active_index,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TabPlacement {
    index: usize,
    x: f32,
    width: f32,
}

struct TabLabel {
    placement: TabPlacement,
    buffer: Buffer,
}

pub(crate) struct TabBar {
    snapshot: TabBarSnapshot,
    labels: Vec<TabLabel>,
    width: u32,
    hovered: Option<usize>,
    focused: bool,
    pub(crate) dirty: bool,
}

impl TabBar {
    pub(crate) const HEIGHT: f32 = 44.0;
    const INSET: f32 = 16.0;
    const GAP: f32 = 6.0;
    const TOP: f32 = 7.0;
    const BUTTON_HEIGHT: f32 = 30.0;
    const TEXT_INSET: f32 = 14.0;

    pub(crate) fn new(width: u32) -> Self {
        Self {
            snapshot: TabBarSnapshot::default(),
            labels: Vec::new(),
            width,
            hovered: None,
            focused: true,
            dirty: true,
        }
    }

    pub(crate) fn set_snapshot(&mut self, snapshot: TabBarSnapshot, fonts: &mut FontSystem) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        self.rebuild(fonts);
    }

    pub(crate) fn resize(&mut self, width: u32, fonts: &mut FontSystem) {
        if self.width == width {
            return;
        }
        self.width = width;
        self.rebuild(fonts);
    }

    fn placements(width: u32, count: usize, active: usize) -> Vec<TabPlacement> {
        if count == 0 {
            return Vec::new();
        }
        let available = (width as f32 - Self::INSET * 2.0).max(0.0);
        if available < 1.0 {
            return Vec::new();
        }
        let slots = (((available + Self::GAP) / (112.0 + Self::GAP)) as usize)
            .max(1)
            .min(count);
        let tab_width = ((available - Self::GAP * (slots - 1) as f32) / slots as f32).min(180.0);
        // Keyboard switching must always reveal the active tab, including in narrow windows.
        let start = active
            .saturating_add(1)
            .saturating_sub(slots)
            .min(count - slots);
        (start..start + slots)
            .enumerate()
            .map(|(slot, index)| TabPlacement {
                index,
                x: Self::INSET + slot as f32 * (tab_width + Self::GAP),
                width: tab_width,
            })
            .collect()
    }

    fn rebuild(&mut self, fonts: &mut FontSystem) {
        self.hovered = None;
        self.labels = Self::placements(
            self.width,
            self.snapshot.titles.len(),
            self.snapshot.active_index,
        )
        .into_iter()
        .map(|placement| {
            // UI labels have their own metrics; shell font settings never distort the tab strip.
            let mut buffer = Buffer::new(fonts, Metrics::new(13.0, Self::BUTTON_HEIGHT));
            buffer.set_wrap(Wrap::None);
            let text_width = (placement.width - Self::TEXT_INSET * 2.0).max(1.0);
            buffer.set_size(Some(text_width), Some(Self::BUTTON_HEIGHT));
            let attrs = Attrs::new()
                .family(Family::SansSerif)
                .weight(Weight::MEDIUM);
            let title: String = self.snapshot.titles[placement.index]
                .chars()
                .filter(|ch| !ch.is_control())
                .take(48)
                .collect();
            let mut name = title.clone();
            loop {
                let suffix = if name == title { "" } else { "…" };
                buffer.set_text(
                    &format!("{}   {name}{suffix}", placement.index + 1),
                    &attrs,
                    Shaping::Advanced,
                    None,
                );
                buffer.shape_until_scroll(fonts, false);
                if buffer
                    .layout_runs()
                    .next()
                    .is_none_or(|run| run.line_w <= text_width)
                    || name.is_empty()
                {
                    break;
                }
                name.pop();
            }
            TabLabel { placement, buffer }
        })
        .collect();
        self.dirty = true;
    }

    pub(crate) fn tab_at(&self, x: f64, y: f64) -> Option<usize> {
        if !x.is_finite()
            || !(f64::from(Self::TOP)..f64::from(Self::TOP + Self::BUTTON_HEIGHT)).contains(&y)
        {
            return None;
        }
        self.labels.iter().find_map(|label| {
            let p = label.placement;
            (x >= f64::from(p.x) && x < f64::from(p.x + p.width)).then_some(p.index)
        })
    }

    pub(crate) fn hover(&mut self, hovered: Option<usize>) -> bool {
        if self.hovered == hovered {
            return false;
        }
        self.hovered = hovered;
        self.dirty = true;
        true
    }

    pub(crate) fn focus(&mut self, focused: bool) -> bool {
        if self.focused == focused {
            return false;
        }
        self.focused = focused;
        self.hovered = None;
        self.dirty = true;
        true
    }

    pub(crate) fn text_areas(&self) -> impl Iterator<Item = TextArea<'_>> {
        self.labels.iter().map(|label| {
            let p = label.placement;
            let color = if !self.focused {
                ThemeColor::TEXT_DIM
            } else if p.index == self.snapshot.active_index || self.hovered == Some(p.index) {
                ThemeColor::TEXT
            } else {
                ThemeColor::TEXT_MUTED
            };
            TextArea {
                buffer: &label.buffer,
                left: p.x + Self::TEXT_INSET,
                top: Self::TOP,
                scale: 1.0,
                bounds: TextBounds {
                    left: (p.x + Self::TEXT_INSET) as i32,
                    top: 0,
                    right: (p.x + p.width - Self::TEXT_INSET).max(p.x + Self::TEXT_INSET) as i32,
                    bottom: Self::HEIGHT as i32,
                },
                default_color: Color::rgba(color[0], color[1], color[2], color[3]),
                custom_glyphs: &[],
            }
        })
    }

    pub(crate) fn rects(&self, srgb: bool) -> Vec<RectSpec> {
        let color = |rgba| {
            if srgb {
                srgb_to_linear_rgba(rgba)
            } else {
                rgba_to_f32(rgba)
            }
        };
        let mut rects = vec![
            RectSpec::new(
                0.0,
                0.0,
                self.width as f32,
                Self::HEIGHT,
                color(ThemeColor::CHROME),
            ),
            RectSpec::new(
                0.0,
                Self::HEIGHT - 1.0,
                self.width as f32,
                1.0,
                color(ThemeColor::BORDER),
            ),
        ];
        for label in &self.labels {
            let p = label.placement;
            let active = p.index == self.snapshot.active_index;
            if active || self.hovered == Some(p.index) {
                if active {
                    rects.push(RectSpec::rounded(
                        p.x,
                        Self::TOP,
                        p.width,
                        Self::BUTTON_HEIGHT,
                        color(ThemeColor::BORDER),
                        6.0,
                    ));
                }
                let inset = if active { 1.0 } else { 0.0 };
                rects.push(RectSpec::rounded(
                    p.x + inset,
                    Self::TOP + inset,
                    (p.width - inset * 2.0).max(0.0),
                    Self::BUTTON_HEIGHT - inset * 2.0,
                    color(if active {
                        ThemeColor::TAB_ACTIVE
                    } else {
                        ThemeColor::TAB_HOVER
                    }),
                    5.0,
                ));
            }
            if active {
                rects.push(RectSpec::rounded(
                    p.x + Self::TEXT_INSET,
                    Self::HEIGHT - 4.0,
                    22.0_f32.min((p.width - Self::TEXT_INSET * 2.0).max(0.0)),
                    2.0,
                    color(if self.focused {
                        ThemeColor::ACCENT
                    } else {
                        ThemeColor::TEXT_DIM
                    }),
                    1.0,
                ));
            }
        }
        rects
    }
}

#[cfg(test)]
mod tests {
    use super::{TabBar, TabBarSnapshot};

    #[test]
    fn narrow_windows_keep_active_tabs_visible_and_hit_targets_inside_bounds() {
        let mut fonts = glyphon::FontSystem::new();
        for width in [24, 60, 320, 1000] {
            for active in 0..12 {
                let placements = TabBar::placements(width, 12, active);
                if width < 33 {
                    assert!(placements.is_empty());
                    continue;
                }
                assert!(placements.iter().any(|p| p.index == active));
                let mut bar = TabBar::new(width);
                // Hit testing uses the exact geometry drawn, not text character counts.
                bar.set_snapshot(
                    TabBarSnapshot::new(vec!["日本語 nu".into(); 12], active),
                    &mut fonts,
                );
                for p in placements {
                    assert!(p.x >= 0.0 && p.x + p.width <= width as f32);
                    assert_eq!(
                        bar.tab_at(f64::from(p.x + p.width / 2.0), 20.0),
                        Some(p.index)
                    );
                    assert_eq!(bar.tab_at(f64::from(p.x), 45.0), None);
                }
                assert_eq!(bar.tab_at(f64::NAN, 20.0), None);
            }
        }
    }
}
