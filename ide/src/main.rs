use game_engine::{renderer::Instance, Context};

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use env_logger;
use vek::quaternion::repr_c::Quaternion;
use vek::vec::repr_c::Vec3;

const SPACE_BETWEEN: f32 = 3.0;
const NUM_INSTANCES_PER_ROW: u32 = 4;

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let size = window.inner_size();

    let mut context = Context::new(&window, (size.width, size.height));
    let model = context.renderer.load_model("./res/Cube.obj").unwrap();

    let instances = (0..NUM_INSTANCES_PER_ROW)
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

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::RedrawRequested(_) => {
                context.renderer.update(&[(model, &instances[..])]);
                context.renderer.render([0.0; 4]).expect("lol");
            }
            Event::MainEventsCleared => window.request_redraw(),
            _ => (),
        }
    });
}
