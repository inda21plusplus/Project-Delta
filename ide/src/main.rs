use std::{f32, time::Instant};

use camera_controller::CameraController;
use game_engine::{renderer::Transform, Context};

use winit::{
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Icon,
};

use vek::quaternion::repr_c::Quaternion;
use vek::vec::repr_c::Vec3;
use window::{Window, WindowMode};

mod camera_controller;
mod im;
mod window;

const SPACE_BETWEEN: f32 = 3.0;
const NUM_INSTANCES_PER_ROW: u32 = 4;

fn update(start: Instant, dt: f32, objects: &mut Vec<Transform>) {
    let offset = start.elapsed().as_secs_f32().sin();
    for obj in objects {
        obj.position.y += offset * 2.0 * dt
    }
}

fn create_instances() -> Vec<Transform> {
    let mut instances = (0..NUM_INSTANCES_PER_ROW)
        .flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                let position = Vec3 { x, y: 0.0, z };

                let rotation = if position == Vec3::zero() {
                    Quaternion::rotation_3d(0.0, Vec3::unit_z())
                } else {
                    Quaternion::rotation_3d(f32::consts::FRAC_PI_4, position.normalized())
                };

                Transform {
                    position,
                    rotation,
                    scale: Vec3::new(1.0, 1.0, 1.0),
                }
            })
        })
        .collect::<Vec<_>>();
    instances
}

fn main() {
    env_logger::init();

    let (img_width, img_height, img_vec) = im::get_logo("icon.png").unwrap();
    let icon = Icon::from_rgba(img_vec, img_width, img_height).unwrap();

    let event_loop = EventLoop::new();
    let mut window = Window::new(&event_loop, Some(icon));

    let mut context = Context::new(
        &window.winit_window,
        (window.size.width, window.size.height),
    )
    .expect("failed to build context");
    let model_cube = context.renderer.load_model("./res/cube.obj").unwrap();
    let model_ball = context.renderer.load_model("./res/ball.obj").unwrap();
    let start_time = Instant::now();

    let mut camera_controller = CameraController::new(
        10.0,
        0.01,
        Vec3::new(-15.0, 10.0, 0.0),
        Vec3::new(-35.0f32.to_radians(), 90.0f32.to_radians(), 0.0),
    );

    let mut instances = create_instances();

    let mut allow_camera_update = true;
    let mut last_frame = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::DeviceEvent {
                device_id: _,
                event,
            } => {
                if allow_camera_update {
                    camera_controller.process_device_events(&event);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.winit_window.id() => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }),
                window_id,
            } if window_id == window.winit_window.id() => {
                context
                    .renderer
                    .resize(game_engine::renderer::PhysicalSize { width, height });
                window.update_size();
            }
            Event::WindowEvent {
                window_id: _,
                event,
            } => {
                camera_controller.process_window_events(&event);
                match event {
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(keycode),
                                ..
                            },
                        ..
                    } => match keycode {
                        VirtualKeyCode::Q => {
                            window
                                .set_window_mode(WindowMode::CursorMode)
                                .unwrap_or_else(|_| log::error!("Failed to unlock cursor"));
                            window
                                .center_cusor()
                                .unwrap_or_else(|_| log::error!("Failed to center cursor"));
                            allow_camera_update = false;
                        }
                        VirtualKeyCode::E => {
                            window
                                .set_window_mode(WindowMode::CameraMode)
                                .unwrap_or_else(|_| log::error!("Failed to lock cursor"));
                            window
                                .center_cusor()
                                .unwrap_or_else(|_| log::error!("Failed to center cursor"));
                            allow_camera_update = true;
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }
            Event::MainEventsCleared => window.winit_window.request_redraw(),
            Event::RedrawRequested(_) => {
                let now = Instant::now();
                let dt = now.duration_since(last_frame).as_secs_f32();
                last_frame = now;

                if allow_camera_update {
                    camera_controller.update_camera(dt, &mut context.renderer.camera);
                }

                update(start_time, dt, &mut instances);

                context.renderer.update_camera();

                context.renderer.update_instances(&[
                    (model_cube, &instances[..8]),
                    (model_ball, &instances[8..]),
                ]);
                context
                    .renderer
                    .render([0.229, 0.507, 0.921, 1.0])
                    .unwrap_or_else(|err| log::error!("Failed to render: {}", err))
            }

            _ => (),
        }
    });
}
