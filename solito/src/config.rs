pub(crate) struct TracingFilter;

#[allow(unused)]
impl TracingFilter {
    pub(crate) const FILTER_DEBUG: &str = "debug";
    pub(crate) const FILTER_ERROR: &str = "error";
}

pub(crate) struct WindowAttr;
impl WindowAttr {
    pub(crate) const WINDOW_WIDTH: f32 = 1000.0;
    pub(crate) const WINDOW_HIGHT: f32 = 650.0;
}
