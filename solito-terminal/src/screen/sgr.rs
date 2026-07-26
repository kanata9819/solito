//! Apply ANSI Select Graphic Rendition (SGR) parameters to a cell style.

use super::buffer::CellStyle;

const RESET: u16 = 0;
const FAINT_ON: u16 = 2;
const FAINT_OFF: u16 = 22;
const FOREGROUND_DEFAULT: u16 = 39;
const FOREGROUND_EXTENDED: u16 = 38;
const BACKGROUND_EXTENDED: u16 = 48;
const UNDERLINE_COLOR_EXTENDED: u16 = 58;
const FOREGROUND_LOW_START: u16 = 30;
const FOREGROUND_LOW_END: u16 = 37;
const FOREGROUND_HIGH_START: u16 = 90;
const FOREGROUND_HIGH_END: u16 = 97;
const FOREGROUND_BRIGHT_OFFSET: u16 = 8;

pub(super) fn apply(style: &mut CellStyle, params: &[u16]) {
    let mut index = 0;

    while index < params.len() {
        let code = params[index];
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
            FOREGROUND_DEFAULT => style.fg_rgba = None,
            FOREGROUND_EXTENDED => {
                index += apply_extended_foreground(style, &params[index..]);
                continue;
            }
            BACKGROUND_EXTENDED | UNDERLINE_COLOR_EXTENDED => {
                index += extended_color_param_count(&params[index..]);
                continue;
            }
            _ => {}
        }

        index += 1;
    }
}

fn extended_color_param_count(params: &[u16]) -> usize {
    match params.get(1).copied() {
        Some(5) => 3,
        Some(2) => 5,
        _ => 1,
    }
}

fn apply_extended_foreground(style: &mut CellStyle, params: &[u16]) -> usize {
    match params.get(1).copied() {
        Some(5) => {
            if let Some(index) = params.get(2).copied() {
                style.fg_rgba = Some(ansi_256_color(usize::from(index)));
            }
            3
        }
        Some(2) => {
            let channel = |index| params.get(index).copied().unwrap_or(0).min(255) as u8;
            style.fg_rgba = Some([channel(2), channel(3), channel(4), 255]);
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

    #[test]
    fn applies_ansi_and_resets_foreground() {
        let mut style = CellStyle::default();

        apply(&mut style, &[31]);
        assert_eq!(style.fg_rgba, Some([197, 15, 31, 255]));

        apply(&mut style, &[39]);
        assert_eq!(style.fg_rgba, None);
    }

    #[test]
    fn applies_indexed_foreground() {
        let mut style = CellStyle::default();

        apply(&mut style, &[38, 5, 196]);

        assert_eq!(style.fg_rgba, Some([255, 0, 0, 255]));
    }

    #[test]
    fn applies_true_color_foreground() {
        let mut style = CellStyle::default();

        apply(&mut style, &[38, 2, 12, 34, 56]);

        assert_eq!(style.fg_rgba, Some([12, 34, 56, 255]));
    }
}
