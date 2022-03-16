use winit::{
    self,
    dpi::{LogicalPosition, PhysicalSize},
    error::ExternalError,
    event_loop::EventLoop,
    window::{Icon, WindowBuilder},
};

pub enum WindowMode {
    CameraMode,
    CursorMode,
}

pub struct Window {
    pub winit_window: winit::window::Window,
    cursor_is_visible: bool,
    cursor_grab: bool,
    window_mode: WindowMode,
    pub size: PhysicalSize<u32>,
}

impl Window {
    pub fn new(event_loop: &EventLoop<()>, icon: Option<Icon>) -> Self {
        let window = WindowBuilder::new()
            .with_window_icon(icon)
            .build(event_loop)
            .expect("Could not build window");

        window.set_cursor_visible(false);
        match window.set_cursor_grab(true) {
            Ok(_) => (),
            Err(e) => eprint!("{:?}", e),
        };
        let size = window.inner_size();
        Self {
            winit_window: window,
            cursor_is_visible: false,
            cursor_grab: true,
            window_mode: WindowMode::CameraMode,
            size,
        }
    }

    pub fn set_window_mode(&mut self, mode: WindowMode) -> Result<(), ExternalError> {
        match mode {
            WindowMode::CameraMode => {
                self.set_cursor_visible(false);
                self.window_mode = WindowMode::CameraMode;
                self.set_cursor_grab(true)
            }
            WindowMode::CursorMode => {
                self.set_cursor_visible(true);
                self.window_mode = WindowMode::CursorMode;
                self.set_cursor_grab(false)
            }
        }
    }
    fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_is_visible = visible;
        self.winit_window.set_cursor_visible(visible);
    }

    fn set_cursor_grab(&mut self, grab: bool) -> Result<(), ExternalError> {
        self.cursor_grab = grab;
        self.winit_window.set_cursor_grab(grab)
    }

    pub fn set_cursor_position(&mut self, x: u32, y: u32) -> Result<(), ExternalError> {
        let pos = LogicalPosition::new(x, y);
        self.winit_window.set_cursor_position(pos)
    }

    pub fn center_cusor(&mut self) -> Result<(), ExternalError> {
        self.set_cursor_position(self.size.width / 2, self.size.height / 2)
    }

    pub fn update_size(&mut self) {
        self.size = self.winit_window.inner_size()
    }
}
