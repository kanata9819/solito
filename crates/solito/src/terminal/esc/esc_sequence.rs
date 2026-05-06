pub struct Csi;
impl Csi {
    pub const CURSOR_UP: char = 'A';
    pub const CURSOR_DOWN: char = 'B';
    pub const CURSOR_FORWARD: char = 'C';
    pub const CURSOR_BACKWARD: char = 'D';
    pub const CURSOR_HORIZONTAL_ABSOLUTE: char = 'G';
    pub const CURSOR_POSITION: char = 'H';
    pub const HORIZONTAL_VERTICAL_POSITION: char = 'f';
    pub const ERASE_IN_DISPLAY: char = 'J';
    pub const ERASE_IN_LINE: char = 'K';
    pub const SELECT_GRAPHIC_RENDITION: char = 'm';
    pub const DELETE_CHARACTER: char = 'P';
    pub const SAVE_CURSOR_POSITION: char = 's';
    pub const RESTORE_CURSOR_POSITION: char = 'u';
}
