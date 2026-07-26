/// Terminal dimensions measured in character cells.
///
/// Keeping this separate from pixel-based window sizes prevents accidental
/// width/height and pixel/cell mix-ups in resize code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: usize,
    pub rows: usize,
}

impl TerminalSize {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols: cols.max(1),
            rows: rows.max(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalSize;

    #[test]
    fn dimensions_are_never_zero() {
        assert_eq!(TerminalSize::new(0, 0), TerminalSize { cols: 1, rows: 1 });
    }
}
