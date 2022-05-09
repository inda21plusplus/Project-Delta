use std::ops::ControlFlow;

use editor::Editor;

mod camera_controller;
mod editor;
mod physics_context;
mod window;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let (event_loop, mut editor) = Editor::new()?;
    event_loop.run(
        move |event, _, control_flow| match editor.handle_event(event) {
            ControlFlow::Continue(_) => {}
            ControlFlow::Break(_) => *control_flow = winit::event_loop::ControlFlow::Exit,
        },
    );
}
