#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyModePosition {
    pub row: usize,
    pub col: usize,
}

impl CopyModePosition {
    pub const fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyModeSelectionKind {
    Cell,
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopyModeSelection {
    pub anchor: CopyModePosition,
    pub cursor: CopyModePosition,
    pub kind: CopyModeSelectionKind,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyModeSnapshot {
    pub active: bool,
    pub cursor: CopyModePosition,
    pub selection: Option<CopyModeSelection>,
}
