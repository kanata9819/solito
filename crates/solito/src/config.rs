pub(crate) struct TracingFilter;

#[allow(unused)]
impl TracingFilter {
    pub(crate) const FILTER_DEBUG: &str = "debug";
    pub(crate) const FILTER_ERROR: &str = "error";
}

pub(crate) struct WindowAttr;
impl WindowAttr {
    pub(crate) const WINDOW_WIDTH: f32 = 900.0;
    pub(crate) const WINDOW_HIGHT: f32 = 650.0;
}

pub(crate) struct BufferAttr;
impl BufferAttr {
    pub(crate) const FONT_SIZE: f32 = 20.0;
    pub(crate) const LINE_HEIGHT: f32 = 30.0;
}

pub struct TerminalAttr;
impl TerminalAttr {
    pub const TERMINAL_COL: usize = 120;
    pub const TERMINAL_ROW: usize = 30;
}
