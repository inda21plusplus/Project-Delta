use std::f32::consts::PI;

use game_engine::{Vec2, Vec3};
use winit::event::{DeviceEvent, KeyboardInput, VirtualKeyCode, WindowEvent};

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
                log::info!("Camera rotation {}", self.rotation);
            }
            _ => (), /*
                     DeviceEvent::Added => todo!(),
                     DeviceEvent::Removed => todo!(),
                     DeviceEvent::MouseMotion { delta } => todo!(),
                     DeviceEvent::MouseWheel { delta } => todo!(),
                     DeviceEvent::Motion { axis, value } => todo!(),
                     DeviceEvent::Button { button, state } => todo!(),
                     DeviceEvent::Key(_) => todo!(),
                     DeviceEvent::Text { codepoint } => todo!(),*/
        }
    }

    pub fn process_window_events(&mut self, event: &WindowEvent) {
        match event {
            &WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
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
                    _ => (),
                }
            }
            _ => (),
        }
    }

    pub fn update_camera(&mut self, dt: f32, camera: &mut game_engine::renderer::Camera) {
        const MAX_PITCH: f32 = 89f32 * (PI / 180.0f32);
        if self.rotation.x < -MAX_PITCH {
            self.rotation.x = -MAX_PITCH;
        }
        if self.rotation.x > MAX_PITCH {
            self.rotation.x = MAX_PITCH;
        }

        let frame_speed = dt * self.movement_speed;

        let forward = Vec3::new(
            self.rotation.y.sin() * self.rotation.x.cos(),
            self.rotation.x.sin(),
            self.rotation.y.cos() * self.rotation.x.cos(),
        );

        let forward_norm = forward.normalized();

        if self.is_forward_pressed {
            self.position += forward_norm * frame_speed;
        }
        if self.is_backward_pressed {
            self.position -= forward_norm * frame_speed;
        }

        let right = forward_norm.cross(camera.up).normalized();
        let up = camera.up;

        if self.is_right_pressed {
            self.position += right * frame_speed;
        }

        if self.is_left_pressed {
            self.position -= right * frame_speed;
        }

        if self.is_up_pressed {
            self.position += up * frame_speed;
        }

        if self.is_down_pressed {
            self.position -= up * frame_speed;
        }

        camera.eye = self.position;
        camera.target = self.position + forward;
    }
}
