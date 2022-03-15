use camera_controller::CameraController;
use game_engine::{renderer::Instance, Context};

use winit::{
    dpi::LogicalPosition,
    event::{Event, KeyboardInput, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
};

//use image;
use env_logger;
use vek::quaternion::repr_c::Quaternion;
use vek::vec::repr_c::Vec3;

mod camera_controller;
mod im;

const SPACE_BETWEEN: f32 = 3.0;
const NUM_INSTANCES_PER_ROW: u32 = 4;

fn update(start: std::time::Instant, dt: f32, objects: &mut Vec<Instance>) {
    let offset = start.elapsed().as_secs_f32().sin();
    for obj in objects {
        obj.position.y += offset / 50.0
    }
}

fn main() {
    env_logger::init();

    //let icon_vec: Vec<u8> = vec![0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0];
    let (img_width, img_height, img_vec) = im::get_logo("icon.ppm".to_string());
    let icon = Icon::from_rgba(img_vec, img_width, img_height).unwrap();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_window_icon(Some(icon))
        .build(&event_loop)
        .unwrap();

    window.set_cursor_visible(false);
    match window.set_cursor_grab(true) {
        Ok(_) => (),
        Err(e) => eprint!("{:?}", e),
    }

    let size = window.inner_size();

    let mut context = Context::new(&window, (size.width, size.height));
    let model = context.renderer.load_model("./res/Cube.obj").unwrap();
    let model_pawn = context.renderer.load_model("./res/ball.obj").unwrap();
    let start = std::time::Instant::now();

    let mut camera_controller = CameraController::new(
        0.2,
        0.01,
        Vec3::new(-15.0, 10.0, 0.0),
        Vec3::new(-35.0f32.to_radians(), 90.0f32.to_radians(), 0.0),
    );

    let mut instances = (0..NUM_INSTANCES_PER_ROW)
        .flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                let position = Vec3 { x, y: 0.0, z };

                let rotation = if position == Vec3::zero() {
                    Quaternion::rotation_3d(0.0, Vec3::unit_z())
                } else {
                    Quaternion::rotation_3d(std::f32::consts::FRAC_PI_4, position.normalized())
                };

                Instance {
                    position,
                    rotation,
                    scale: Vec3::new(1.0, 1.0, 1.0),
                }
            })
        })
        .collect::<Vec<_>>();

    let mut allow_camera_update = true;
    let mut last_frame = std::time::Instant::now();
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
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }),
                window_id,
            } if window_id == window.id() => context
                .renderer
                .resize(game_engine::renderer::PhysicalSize { width, height }),

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
                        winit::event::VirtualKeyCode::Q => {
                            allow_camera_update = false;
                            window.set_cursor_visible(true);
                            match window.set_cursor_grab(false) {
                                Ok(_) => (),
                                Err(e) => eprintln!("{:?}", e),
                            }
                            match window.set_cursor_position(LogicalPosition::new(
                                size.width / 2,
                                size.height / 2,
                            )) {
                                Ok(_) => (),
                                Err(e) => eprintln!("{:?}", e),
                            }
                        }
                        winit::event::VirtualKeyCode::E => {
                            match window.set_cursor_position(LogicalPosition::new(
                                size.width / 2,
                                size.height / 2,
                            )) {
                                Ok(_) => (),
                                Err(e) => eprintln!("{:?}", e),
                            }
                            allow_camera_update = true;
                            window.set_cursor_visible(false);
                            match window.set_cursor_grab(true) {
                                Ok(_) => (),
                                Err(e) => eprintln!("{:?}", e),
                            }
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }

            Event::MainEventsCleared => window.request_redraw(),
            Event::RedrawRequested(_) => {
                if allow_camera_update {
                    camera_controller.update_camera(&mut context.renderer.camera);
                }
                let dt = last_frame.elapsed().as_secs_f32();
                last_frame = std::time::Instant::now();
                let _frame_rate = 1.0 / dt; // TODO render on screen

                update(start, dt, &mut instances);

                context.renderer.update_camera();

                context
                    .renderer
                    .update_instances(&[(model, &instances[..8]), (model_pawn, &instances[8..])]);
                context
                    .renderer
                    .render([0.229, 0.507, 0.921, 1.0])
                    .expect("render error");
            }

            _ => (),
        }
    });
}
