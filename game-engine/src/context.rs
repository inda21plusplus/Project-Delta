use crate::renderer::Renderer;
use raw_window_handle::HasRawWindowHandle;

pub struct Context {
    renderer: Renderer,
}

impl Context {
    pub fn new<W: HasRawWindowHandle>(w: &W, size: (u32, u32)) -> Self {
        Self {
            renderer: Renderer::new(w, size),
        }
    }
}
