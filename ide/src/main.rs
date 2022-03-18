use editor::Editor;

mod camera_controller;
mod editor;
mod window;

fn main() {
    env_logger::init();

    Editor::new().unwrap().run();
}
