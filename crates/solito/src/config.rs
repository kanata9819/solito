pub struct TracingFilter;

#[allow(unused)]
impl TracingFilter {
    pub const FILTER_DEBUG: &str = "debug";
    pub const FILTER_ERROR: &str = "error";
}

pub struct WindowAttr;
impl WindowAttr {
    pub const WINDOW_WIDTH: f32 = 900.0;
    pub const WINDOW_HIGHT: f32 = 650.0;
}

pub struct BufferAttr;
impl BufferAttr {
    pub const FONT_SIZE: f32 = 20.0;
    pub const LINE_HEIGHT: f32 = 30.0;
}
