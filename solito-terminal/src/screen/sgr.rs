use super::buffer::CellStyle;

const RESET: u16 = 0;
const FAINT_ON: u16 = 2;
const FAINT_OFF: u16 = 22;
const FOREGROUND_DEFAULT: u16 = 39;
const FOREGROUND_EXTENDED: u16 = 38;
const BACKGROUND_EXTENDED: u16 = 48;
const BACKGROUND_DEFAULT: u16 = 49;
const UNDERLINE_COLOR_EXTENDED: u16 = 58;
const FOREGROUND_LOW_START: u16 = 30;
const FOREGROUND_LOW_END: u16 = 37;
const FOREGROUND_HIGH_START: u16 = 90;
const FOREGROUND_HIGH_END: u16 = 97;
const FOREGROUND_BRIGHT_OFFSET: u16 = 8;

/// Applies SGR parameters. Each entry is one `;`-separated parameter with its
/// `:`-separated subparameters, so `38;2;R;G;B` and `38:2::R:G:B` both work.
pub(super) fn apply(style: &mut CellStyle, params: &[Vec<u16>]) {
    let mut index = 0;

    while index < params.len() {
        let group = &params[index];
        let Some(&code) = group.first() else {
            index += 1;
            continue;
        };

        if group.len() > 1 {
            // Colon form keeps the whole extended color inside one parameter.
            match code {
                FOREGROUND_EXTENDED => apply_colon_color(&mut style.fg_rgba, group),
                BACKGROUND_EXTENDED => apply_colon_color(&mut style.bg_rgba, group),
                UNDERLINE_COLOR_EXTENDED => {}
                _ => apply_code(style, code),
            }
            index += 1;
            continue;
        }

        match code {
            FOREGROUND_EXTENDED | BACKGROUND_EXTENDED | UNDERLINE_COLOR_EXTENDED => {
                let rest: Vec<u16> = params[index..]
                    .iter()
                    .take(5)
                    .map(|param| param.first().copied().unwrap_or(0))
                    .collect();
                let consumed = match code {
                    FOREGROUND_EXTENDED => apply_extended_color(&mut style.fg_rgba, &rest),
                    BACKGROUND_EXTENDED => apply_extended_color(&mut style.bg_rgba, &rest),
                    _ => extended_color_param_count(&rest),
                };
                index += consumed;
            }
            _ => {
                apply_code(style, code);
                index += 1;
            }
        }
    }
}

fn apply_code(style: &mut CellStyle, code: u16) {
    match code {
        RESET => *style = CellStyle::default(),
        FAINT_ON => style.faint = true,
        FAINT_OFF => style.faint = false,
        FOREGROUND_LOW_START..=FOREGROUND_LOW_END => {
            style.fg_rgba = Some(ansi_16_color(usize::from(code - FOREGROUND_LOW_START)));
        }
        FOREGROUND_HIGH_START..=FOREGROUND_HIGH_END => {
            style.fg_rgba = Some(ansi_16_color(usize::from(
                code - FOREGROUND_HIGH_START + FOREGROUND_BRIGHT_OFFSET,
            )));
        }
        40..=47 => style.bg_rgba = Some(ansi_16_color(usize::from(code - 40))),
        100..=107 => style.bg_rgba = Some(ansi_16_color(usize::from(code - 100 + 8))),
        BACKGROUND_DEFAULT => style.bg_rgba = None,
        FOREGROUND_DEFAULT => style.fg_rgba = None,
        _ => {}
    }
}

/// Parses `38:5:N`, `38:2:R:G:B` and the ITU form `38:2:CS:R:G:B`, where the
/// color-space id may be empty.
fn apply_colon_color(color: &mut Option<[u8; 4]>, group: &[u16]) {
    let channel = |value: u16| value.min(255) as u8;
    match group.get(1).copied() {
        Some(5) => {
            if let Some(&index) = group.get(2) {
                *color = Some(ansi_256_color(usize::from(index)));
            }
        }
        Some(2) => {
            let rgb = if group.len() >= 6 {
                &group[3..6]
            } else if group.len() == 5 {
                &group[2..5]
            } else {
                return;
            };
            *color = Some([channel(rgb[0]), channel(rgb[1]), channel(rgb[2]), 255]);
        }
        _ => {}
    }
}

fn extended_color_param_count(params: &[u16]) -> usize {
    match params.get(1).copied() {
        Some(5) => 3,
        Some(2) => 5,
        _ => 1,
    }
}

fn apply_extended_color(color: &mut Option<[u8; 4]>, params: &[u16]) -> usize {
    match params.get(1).copied() {
        Some(5) => {
            if let Some(index) = params.get(2).copied() {
                *color = Some(ansi_256_color(usize::from(index)));
            }
            3
        }
        Some(2) => {
            let channel = |index| params.get(index).copied().unwrap_or(0).min(255) as u8;
            *color = Some([channel(2), channel(3), channel(4), 255]);
            5
        }
        _ => 1,
    }
}

fn ansi_16_color(index: usize) -> [u8; 4] {
    const COLORS: [[u8; 4]; 16] = [
        [12, 12, 12, 255],
        [197, 15, 31, 255],
        [19, 161, 14, 255],
        [193, 156, 0, 255],
        [0, 55, 218, 255],
        [136, 23, 152, 255],
        [58, 150, 221, 255],
        [204, 204, 204, 255],
        [118, 118, 118, 255],
        [231, 72, 86, 255],
        [22, 198, 12, 255],
        [249, 241, 165, 255],
        [59, 120, 255, 255],
        [180, 0, 158, 255],
        [97, 214, 214, 255],
        [242, 242, 242, 255],
    ];

    COLORS[index.min(15)]
}

fn ansi_256_color(index: usize) -> [u8; 4] {
    match index {
        0..=15 => ansi_16_color(index),
        16..=231 => {
            let cube_index = index - 16;
            let red = cube_index / 36;
            let green = (cube_index % 36) / 6;
            let blue = cube_index % 6;
            [
                cube_component(red),
                cube_component(green),
                cube_component(blue),
                255,
            ]
        }
        232..=255 => {
            let gray = (8 + (index - 232) * 10).min(255) as u8;
            [gray, gray, gray, 255]
        }
        _ => ansi_16_color(7),
    }
}

fn cube_component(level: usize) -> u8 {
    if level == 0 {
        0
    } else {
        (55 + level * 40).min(255) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::{CellStyle, apply};

    fn semicolons(params: &[u16]) -> Vec<Vec<u16>> {
        params.iter().map(|&param| vec![param]).collect()
    }

    #[test]
    fn applies_ansi_and_resets_foreground() {
        let mut style = CellStyle::default();

        apply(&mut style, &semicolons(&[31]));
        assert_eq!(style.fg_rgba, Some([197, 15, 31, 255]));

        apply(&mut style, &semicolons(&[39]));
        assert_eq!(style.fg_rgba, None);
    }

    #[test]
    fn applies_indexed_foreground() {
        let mut style = CellStyle::default();

        apply(&mut style, &semicolons(&[38, 5, 196]));

        assert_eq!(style.fg_rgba, Some([255, 0, 0, 255]));
    }

    #[test]
    fn applies_true_color_foreground() {
        let mut style = CellStyle::default();

        apply(&mut style, &semicolons(&[38, 2, 12, 34, 56]));

        assert_eq!(style.fg_rgba, Some([12, 34, 56, 255]));
    }

    #[test]
    fn applies_colon_true_color_with_empty_color_space() {
        let mut style = CellStyle::default();

        // `ESC[2;38:2::255:0:0m`: the empty color-space id must not reset.
        apply(&mut style, &[vec![2], vec![38, 2, 0, 255, 0, 0]]);

        assert_eq!(style.fg_rgba, Some([255, 0, 0, 255]));
        assert!(style.faint);
    }

    #[test]
    fn applies_colon_true_color_without_color_space() {
        let mut style = CellStyle::default();

        apply(&mut style, &[vec![48, 2, 12, 34, 56]]);

        assert_eq!(style.bg_rgba, Some([12, 34, 56, 255]));
    }

    #[test]
    fn applies_colon_indexed_color_and_continues() {
        let mut style = CellStyle::default();

        apply(&mut style, &[vec![38, 5, 196], vec![44]]);

        assert_eq!(style.fg_rgba, Some([255, 0, 0, 255]));
        assert_eq!(style.bg_rgba, Some([0, 55, 218, 255]));
    }

    #[test]
    fn colon_underline_style_does_not_change_colors() {
        let mut style = CellStyle::default();
        apply(&mut style, &semicolons(&[31]));

        apply(&mut style, &[vec![4, 3], vec![58, 2, 0, 1, 2, 3]]);

        assert_eq!(style.fg_rgba, Some([197, 15, 31, 255]));
    }
}
