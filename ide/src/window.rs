use raw_window_handle::HasRawWindowHandle;
use thiserror::Error;
use winit::{
    self,
    dpi::LogicalPosition,
    error::{ExternalError, OsError},
    event_loop::EventLoop,
    window::{Icon, Window as WinitWindow, WindowBuilder},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMode {
    CameraMode,
    CursorMode,
}

pub struct Window {
    winit_window: WinitWindow,
    window_mode: WindowMode,
    inner_size: (u32, u32),
}

#[derive(Error, Debug)]
pub enum WindowError {
    #[error("error from the operating system")]
    WinitErrorOs(#[from] OsError),
    #[error("external error")]
    WinitErrorExternal(#[from] ExternalError),
}

impl Window {
    pub fn new(event_loop: &EventLoop<()>, icon: Option<Icon>) -> Result<Self, WindowError> {
        let window = WindowBuilder::new()
            .with_window_icon(icon)
            .build(event_loop)?;

        let inner_size = window.inner_size().into();
        Ok(Self {
            winit_window: window,
            window_mode: WindowMode::CursorMode,
            inner_size,
        })
    }

    pub fn raw_window_handle(&self) -> &impl HasRawWindowHandle {
        &self.winit_window
    }

    pub fn window_mode(&self) -> WindowMode {
        self.window_mode
    }

    pub fn inner_size(&self) -> (u32, u32) {
        self.inner_size
    }

    pub fn set_window_mode(&mut self, mode: WindowMode) -> Result<(), WindowError> {
        if mode == self.window_mode {
            return Ok(());
        }
        self.window_mode = mode;
        match mode {
            WindowMode::CameraMode => {
                self.set_cursor_grab(true)?;
                self.set_cursor_visible(false);
            }
            WindowMode::CursorMode => {
                self.set_cursor_grab(false)?;
                self.set_cursor_visible(true);
                self.center_cursor()?;
            }
        }
        Ok(())
    }

    fn set_cursor_visible(&mut self, visible: bool) {
        self.winit_window.set_cursor_visible(visible);
    }

    fn set_cursor_grab(&mut self, grab: bool) -> Result<(), WindowError> {
        self.winit_window
            .set_cursor_grab(grab)
            .map_err(WindowError::from)
    }

    pub fn center_cursor(&mut self) -> Result<(), WindowError> {
        let size = self.winit_window.inner_size();
        self.set_cursor_position(size.width / 2, size.height / 2)
    }

    pub fn set_cursor_position(&mut self, x: u32, y: u32) -> Result<(), WindowError> {
        let pos = LogicalPosition::new(x, y);
        self.winit_window
            .set_cursor_position(pos)
            .map_err(WindowError::from)
    }

    pub fn update_size(&mut self) {
        self.inner_size = self.winit_window.inner_size().into();
    }

    /// Get a reference to the window's winit window.
    pub fn winit_window(&self) -> &WinitWindow {
        &self.winit_window
    }
}
