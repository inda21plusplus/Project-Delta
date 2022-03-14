use crate::renderer::{PhysicalSize, Renderer};
use raw_window_handle::HasRawWindowHandle;

pub struct Context {
    pub renderer: Renderer,
}

impl Context {
    pub fn new<W: HasRawWindowHandle>(w: &W, size: (u32, u32)) -> Self {
        Self {
            renderer: Renderer::new(
                w,
                PhysicalSize {
                    width: size.0,
                    height: size.1,
                },
            ),
        }
    }
}
