use common::{Vec2, Vec3};
use game_engine::rendering::Camera;
use winit::event::{DeviceEvent, KeyboardInput, MouseScrollDelta, VirtualKeyCode, WindowEvent};

pub struct CameraController {
    pub movement_speed: f32,
    /// Degrees per pixel
    pub mouse_sensitivity: f32,
    pub position: Vec3,
    /// Pitch, Yaw
    pub rotation: Vec2,
    pub is_forward_pressed: bool,
    pub is_backward_pressed: bool,
    pub is_left_pressed: bool,
    pub is_right_pressed: bool,
    pub is_up_pressed: bool,
    pub is_down_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32, mouse_sensitivity: f32, position: Vec3, rotation: Vec2) -> Self {
        Self {
            movement_speed: speed,
            position,
            rotation,
            mouse_sensitivity,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_up_pressed: false,
            is_down_pressed: false,
        }
    }

    pub fn process_device_events(&mut self, event: &DeviceEvent) {
        match *event {
            DeviceEvent::MouseMotion { delta: (dx, dy) } => {
                self.rotation.x -= (self.mouse_sensitivity * dy as f32).to_radians();
                self.rotation.y -= (self.mouse_sensitivity * dx as f32).to_radians();
            }
            DeviceEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_, dy),
            } => self.movement_speed *= (-dy as f32 / 1000.).exp(),
            DeviceEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(d),
            } => self.movement_speed *= (-d.y as f32 / 1000.).exp(),
            _ => {}
        }
    }

    pub fn process_window_events(&mut self, event: &WindowEvent) {
        if let &WindowEvent::KeyboardInput {
            input:
                KeyboardInput {
                    state,
                    virtual_keycode: Some(keycode),
                    ..
                },
            ..
        } = event
        {
            let is_pressed = state == winit::event::ElementState::Pressed;
            match keycode {
                VirtualKeyCode::W | VirtualKeyCode::Up => {
                    self.is_forward_pressed = is_pressed;
                }
                VirtualKeyCode::A | VirtualKeyCode::Left => {
                    self.is_left_pressed = is_pressed;
                }
                VirtualKeyCode::S | VirtualKeyCode::Down => {
                    self.is_backward_pressed = is_pressed;
                }
                VirtualKeyCode::D | VirtualKeyCode::Right => {
                    self.is_right_pressed = is_pressed;
                }
                VirtualKeyCode::Space | VirtualKeyCode::PageUp => {
                    self.is_up_pressed = is_pressed;
                }
                VirtualKeyCode::LControl | VirtualKeyCode::PageDown => {
                    self.is_down_pressed = is_pressed;
                }
                _ => {}
            }
        }
    }

    pub fn update_camera(&mut self, dt: f32, camera: &mut Camera) {
        let max_pitch: f32 = 89f32.to_radians();
        self.rotation.x = self.rotation.x.clamp(-max_pitch, max_pitch);

        let delta_pos = dt * self.movement_speed;

        let forward = Vec3::new(
            self.rotation.y.sin() * self.rotation.x.cos(),
            self.rotation.x.sin(),
            self.rotation.y.cos() * self.rotation.x.cos(),
        )
        .normalized();

        if self.is_forward_pressed {
            self.position += forward * delta_pos;
        }
        if self.is_backward_pressed {
            self.position -= forward * delta_pos;
        }

        let right = forward.cross(camera.up).normalized();

        if self.is_right_pressed {
            self.position += right * delta_pos;
        }

        if self.is_left_pressed {
            self.position -= right * delta_pos;
        }

        if self.is_up_pressed {
            self.position += camera.up * delta_pos;
        }

        if self.is_down_pressed {
            self.position -= camera.up * delta_pos;
        }

        camera.eye = self.position;
        camera.target = self.position + forward;

        //println!("{:?} {:?}", camera.eye, self.rotation);
    }
}
