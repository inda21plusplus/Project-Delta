use camera_controller::CameraController;
use game_engine::physics;
use game_engine::{
    physics::{
        collision::update, r#box::BoxColider, sphere::SphereColider, Collider, PhysicsMaterial,
        PhysicsObject, Quaternion, RidgidBody, Vec3,
    },
    renderer::Transform,
    Context,
};
use rand::Rng;

use vek::Clamp;
use winit::{
    dpi::LogicalPosition,
    event::{Event, KeyboardInput, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
};

//use image;
use env_logger;

mod camera_controller;
mod im;

const SPACE_BETWEEN: f32 = 2.0;
const NUM_INSTANCES_PER_ROW: u32 = 4;

fn main() {
    env_logger::init();

    let (img_width, img_height, img_vec) = im::get_logo("icon.ppm".to_string()).unwrap();
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
    let cube_model = context.renderer.load_model("./res/Cube.obj").unwrap();
    let ball_model = context.renderer.load_model("./res/ball.obj").unwrap();
    let _start = std::time::Instant::now();

    let mut camera_controller = CameraController::new(
        10.0,
        0.01,
        Vec3 { x: -1.3849019, y: 2.8745177, z: 8.952639 }, Vec3 { x: -0.32086524, y: 2.8307953, z: 0.0 }
        //Vec3::new(-2.617557, 0.3896206, -2.1071591),
        //Vec3::new(-0.040865257, 2.8307953, 0.0),
    );

    let mut instances = vec![Transform {
        position: Vec3::new(0.0, 0.0, 0.0),
        rotation: Quaternion::rotation_x(0.0),
        scale: Vec3::new(100.0, 1.0, 100.0),
    }];
    let cubes = 0;
    let spheres = 15;
    let mut rng = rand::thread_rng();

    for _ in 0..(cubes + spheres) {
        let scale = rng.gen_range(1.0..1.5);
        instances.push(Transform {
            position: Vec3::new(
                rng.gen_range(-10.0..10.0),
                rng.gen_range(14.0..30.0),
                rng.gen_range(-10.0..10.0),
            ),
            rotation: Quaternion::identity(),
            scale: Vec3::new(scale, scale, scale),
        })
    }

    let physics_material = PhysicsMaterial {
        friction: 1.0,
        restfullness: 0.0,
    };

    let gravity = Vec3::new(0.0, -9.82, 0.0);

    let mut obj1 = PhysicsObject::new(
        RidgidBody::new(
            Vec3::new(5.0, 0.00, 0.000),
            Vec3::zero(),
            Vec3::new(0.0, 0.0, 0.0), // -1.6
            10.0,
        ),
        Collider::BoxColider(BoxColider::new(Vec3::new(1.0, 1.0, 1.0), physics_material)),
    );
    obj1.rb.is_static = true;

    let mut physics_objects: Vec<PhysicsObject> = vec![obj1]; //obj3, obj4 vec![obj1.clone(); 16];
    let vel = 1.0;
    let angle = 0.0001;

    for _ in 0..cubes {
        physics_objects.push(PhysicsObject::new(
            RidgidBody::new(
                Vec3::new(
                    rng.gen_range(-vel..vel),
                    rng.gen_range(-vel..vel),
                    rng.gen_range(-vel..vel),
                ),
                gravity,
                Vec3::new(
                    rng.gen_range(-angle..angle),
                    rng.gen_range(-angle..angle),
                    rng.gen_range(-angle..angle),
                ),
                10.0,
            ),
            Collider::BoxColider(BoxColider::new(Vec3::new(1.0, 1.0, 1.0), physics_material)),
        ));
    }

    for _ in 0..spheres {
        physics_objects.push(PhysicsObject::new(
            RidgidBody::new(
                Vec3::new(
                    rng.gen_range(-vel..vel),
                    rng.gen_range(-vel..vel),
                    rng.gen_range(-vel..vel),
                ),
                gravity,
                Vec3::new(
                    rng.gen_range(-angle..angle),
                    rng.gen_range(-angle..angle),
                    rng.gen_range(-angle..angle),
                ),
                10.0,
            ),
            Collider::SphereColider(SphereColider::new(1.0, physics_material)),
        ));
    }

    let mut allow_camera_update = true;
    let mut last_frame = std::time::Instant::now();
    let mut pause_physics = false;

    let can_pause_phx = false;

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
                let dt = last_frame.elapsed().as_secs_f32();
                let _frame_rate = 1.0 / dt; // TODO render on screen
                last_frame = std::time::Instant::now();

                let clamp = |v: &mut Vec3, range: Vec3| *v = v.clamped(-range, range);

                if allow_camera_update {
                    camera_controller.update_camera(dt, &mut context.renderer.camera);
                }
                if !pause_physics || !can_pause_phx {
                    for obj in &mut physics_objects {
                        //clamp(&mut obj.rb.angular_momentum, Vec3::from(10.0));
                        //clamp(&mut obj.rb.linear_momentum, Vec3::from(100.0));
                    }
                    update(&mut pause_physics, dt, &mut instances, &mut physics_objects);
                }

                context.renderer.update_camera();

                /*
                println!(
                    "camera: {:?} {:?}",
                    camera_controller.position, camera_controller.rotation
                );*/

                context
                    .renderer
                    //.update_instances(&[(cube_model, &instances[..])]); // , (ball_model, &instances[..1])
                    //.update_instances(&[(ball_model, &instances[..])]); // , (ball_model, &instances[..1])
                    .update_instances(&[
                        (cube_model, &instances[..(cubes + 1)]),
                        (ball_model, &instances[(cubes + 1)..]),
                    ]);
                // , (ball_model, &instances[..1])
                context
                    .renderer
                    .render([0.229, 0.507, 0.921, 1.0])
                    .expect("render error");
            }

            _ => (),
        }
    });
}
