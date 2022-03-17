use std::{f32, time::Instant};

use camera_controller::CameraController;
use common::{Quaternion, Transform, Vec2, Vec3};
use game_engine::{rendering::Renderer, Context};

use winit::{
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Icon,
};

use window::{Window, WindowMode};

mod camera_controller;
mod window;

const SPACE_BETWEEN: f32 = 3.0;
const INSTANCES_PER_ROW: u32 = 4;

fn create_instances() -> Vec<Transform> {
    (0..INSTANCES_PER_ROW)
        .flat_map(|z| {
            (0..INSTANCES_PER_ROW).map(move |x| {
                let x = SPACE_BETWEEN * (x as f32 - INSTANCES_PER_ROW as f32);
                let z = SPACE_BETWEEN * (z as f32 - INSTANCES_PER_ROW as f32);

                let position = Vec3 { x, y: 0.0, z };

                let rotation = if position == Vec3::zero() {
                    Quaternion::identity()
                } else {
                    Quaternion::rotation_3d(f32::consts::FRAC_PI_4, position)
                };

                Transform {
                    position,
                    rotation,
                    scale: Vec3::broadcast(1.0),
                }
            })
        })
        .collect::<Vec<_>>()
}

fn main() {
    env_logger::init();

    let icon = image::open("res/icon.png").unwrap().into_rgba8();
    let (icon_width, icon_height) = icon.dimensions();
    let icon = Icon::from_rgba(icon.into_raw(), icon_width, icon_height).unwrap();

    let event_loop = EventLoop::new();

    let mut window = Window::new(&event_loop, Some(icon)).unwrap();

    let mut context = Context {
        renderer: Renderer::new(
            window.raw_window_handle(),
            window.inner_size(),
            [0.229, 0.507, 0.921],
        )
        .unwrap(),
    };

    let model_cube = context.renderer.load_model("res/cube.obj").unwrap();
    let model_ball = context.renderer.load_model("res/ball.obj").unwrap();

    let mut camera_controller = CameraController::new(
        10.0,
        0.1,
        Vec3::new(-15.0, 3.0, 0.0),
        Vec2::new(-0.3, 135f32.to_radians()),
    );

    let mut instances = create_instances();

    let start_time = Instant::now();
    let mut last_frame = start_time;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::DeviceEvent { event, .. } if window.window_mode() == WindowMode::CameraMode => {
                camera_controller.process_device_events(&event);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.winit_window().id() => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }),
                window_id,
            } if window_id == window.winit_window().id() => {
                context.renderer.resize((width, height));
                window.update_size();
            }
            Event::WindowEvent { event, .. } => {
                camera_controller.process_window_events(&event);
                if let WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(keycode),
                            ..
                        },
                    ..
                } = event
                {
                    match keycode {
                        VirtualKeyCode::Q => {
                            window
                                .set_window_mode(WindowMode::CursorMode)
                                .unwrap_or_else(|_| log::error!("Could not unlock cursor"));
                        }
                        VirtualKeyCode::E => {
                            window
                                .set_window_mode(WindowMode::CameraMode)
                                .unwrap_or_else(|_| log::error!("Could not lock cursor"));
                        }
                        _ => {}
                    }
                }
            }
            Event::MainEventsCleared => window.winit_window().request_redraw(),
            Event::RedrawRequested(_) => {
                let now = Instant::now();
                let dt = now.duration_since(last_frame).as_secs_f32();
                last_frame = now;

                let total_elapsed = start_time.elapsed().as_secs_f32();
                for obj in &mut instances {
                    obj.position.y =
                        (total_elapsed - obj.position.x * 0.2 + obj.position.z * 0.3).sin() / 2.;
                    obj.rotation.rotate_y(obj.position.x * 0.03 * dt);
                }

                context.renderer.update_instances(&[
                    (model_cube, &instances[..8]),
                    (model_ball, &instances[8..]),
                ]);

                camera_controller.update_camera(dt, &mut context.renderer.camera);
                context.renderer.update_camera();

                context
                    .renderer
                    .render()
                    .unwrap_or_else(|err| log::error!("Failed to render: {}", err))
            }
            _ => {}
        }
    });
}
