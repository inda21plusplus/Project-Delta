use camera_controller::CameraController;
use game_engine::{renderer::Transform, Context};

use winit::{
    dpi::LogicalPosition,
    event::{Event, KeyboardInput, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
};

//use image;
use env_logger;
use vek::Ray;

mod camera_controller;
mod im;

const SPACE_BETWEEN: f32 = 2.0;
const NUM_INSTANCES_PER_ROW: u32 = 4;

type Vec3 = vek::vec::repr_c::Vec3<f32>;
type Quaternion = vek::quaternion::repr_c::Quaternion<f32>;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct PhysicsMaterial {
    //static_friction: f32,
    pub friction: f32,
    pub restfullness: f32, // bounciness
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SphereColider {
    pub local_position: Vec3,
    pub radius: f32,
    pub material: PhysicsMaterial,
}

impl SphereColider {
    pub fn new(radius: f32, material: PhysicsMaterial) -> Self {
        Self {
            radius,
            material,
            local_position: Vec3::zero(),
        }
    }
}

fn get_world_position(world_pos: Vec3, rotation: Quaternion, local_position: Vec3) -> Vec3 {
    world_pos + rotation * local_position
}

fn get_position(transform: &Transform, collider: &Collider) -> Vec3 {
    get_world_position(
        transform.position,
        transform.rotation,
        match collider {
            Collider::SphereColider(c) => c.local_position,
            Collider::BoxColider(c) => c.local_position,
        },
    )
}

/// Returns true if 2 objects are colliding
pub fn is_colliding(c1: &Collider, t1: &mut Transform, c2: &Collider, t2: &mut Transform) -> bool {
    let w1 = get_position(t1, c1);
    let w2 = get_position(t2, c2);

    match c1 {
        Collider::SphereColider(b1) => match c2 {
            Collider::SphereColider(b2) => {
                let r1 = b1.get_radius(t1.scale);
                let r2 = b2.get_radius(t2.scale);

                debug_assert!(r1 > 0.0);
                debug_assert!(r2 > 0.0);

                let total_radius = r1 + r2;

                w1.distance_squared(w2) <= total_radius * total_radius
            }
            Collider::BoxColider(_) => todo!(),
        },
        Collider::BoxColider(_) => todo!(),
    }
}

pub fn solve_colliding(
    c1: &Collider,
    rb1: &mut RidgidBody,
    t1: &mut Transform,
    c2: &Collider,
    rb2: &mut RidgidBody,
    t2: &mut Transform,
) {
    let w1 = get_position(t1, c1);
    let w2 = get_position(t2, c2);

    match c1 {
        Collider::SphereColider(b1) => match c2 {
            Collider::SphereColider(b2) => {
                collide_sphere_vs_sphere(b1, rb1, t1, w1, b2, rb2, t2, w2)
            }
            Collider::BoxColider(_) => todo!(),
        },
        Collider::BoxColider(_) => todo!(),
    }
}

pub fn collide_sphere_vs_sphere(
    c1: &SphereColider,
    rb1: &mut RidgidBody,
    t1: &mut Transform,
    mut w1: Vec3, // world position
    c2: &SphereColider,
    rb2: &mut RidgidBody,
    t2: &mut Transform,
    mut w2: Vec3, // world position
) {
    let re1 = c1.material.restfullness;
    let re2 = c2.material.restfullness;

    let r1 = c1.get_radius(t1.scale);
    let r2 = c2.get_radius(t2.scale);

    let m1 = rb1.mass;
    let m2 = rb2.mass;

    let mut v1 = rb1.velocity;
    let mut v2 = rb2.velocity;

    // pop
    if !rb1.is_static && !rb2.is_static {
        let diff = w2 - w1;
        let distance_pop = diff.magnitude() - r1 - r2;
        let normal = diff.normalized();

        const POP_SIZE: f32 = 1.10;
        let pop = normal * distance_pop * POP_SIZE;
        if rb1.is_static {
            t2.position -= pop;
        } else if rb2.is_static {
            t1.position += pop;
        } else {
            t2.position -= pop * 0.5;
            t1.position += pop * 0.5;
        }
    }

    //https://www.plasmaphysics.org.uk/collision3d.htm
    //https://www.plasmaphysics.org.uk/programs/coll3d_cpp.htm

    let r12 = r1 + r2;
    let m21 = m2 / m1;
    let b21 = w2 - w1;
    let v21 = v2 - v1;

    let v_cm = (m1 * v1 + m2 * v2) / (m1 + m2);

    //     **** calculate relative distance and relative speed ***
    let d = b21.magnitude();
    let v = v21.magnitude();
    //     **** return if relative speed = 0 ****
    if v.abs() < 0.00001f32 {
        return;
    }
    //     **** shift coordinate system so that ball 1 is at the origin ***
    w2 = b21;

    //     **** boost coordinate system so that ball 2 is resting ***
    v1 = -v21;

    //     **** find the polar coordinates of the location of ball 2 ***
    let theta2 = (w2.z / d).acos();
    let phi2 = if w2.x == 0.0 && w2.y == 0.0 {
        0.0
    } else {
        w2.y.atan2(w2.x)
    };

    let st = theta2.sin();
    let ct = theta2.cos();
    let sp = phi2.sin();
    let cp = phi2.cos();

    //     **** express the velocity vector of ball 1 in a rotated coordinate
    //          system where ball 2 lies on the z-axis ******
    let mut vx1r = ct * cp * v1.x + ct * sp * v1.y - st * v1.z;
    let mut vy1r = cp * v1.y - sp * v1.x;
    let mut vz1r = st * cp * v1.x + st * sp * v1.y + ct * v1.z;
    let mut fvz1r = vz1r / v;
    if fvz1r > 1.0 {
        fvz1r = 1.0;
    }
    // fix for possible rounding errors
    else if fvz1r < -1.0 {
        fvz1r = -1.0;
    }
    let thetav = fvz1r.acos();
    let phiv = if vx1r == 0.0 && vy1r == 0.0 {
        0.0
    } else {
        vy1r.atan2(vx1r)
    };

    //     **** calculate the normalized impact parameter ***
    let dr = d * (thetav.sin()) / r12;

    //     **** calculate impact angles if balls do collide ***
    let alpha = (-dr).asin();
    let beta = phiv;
    let sbeta = beta.sin();
    let cbeta = beta.cos();

    //     **** calculate time to collision ***
    let t = (d * thetav.cos() - r12 * (1.0 - dr * dr).sqrt()) / v;

    //     **** update positions and reverse the coordinate shift ***
    w2 += v2 * t + w1;
    w1 += (v1 + v2) * t;

    //  ***  update velocities ***

    let a = (thetav + alpha).tan();

    let dvz2 = 2.0 * (vz1r + a * (cbeta * vx1r + sbeta * vy1r)) / ((1.0 + a * a) * (1.0 + m21));

    let vz2r = dvz2;
    let vx2r = a * cbeta * dvz2;
    let vy2r = a * sbeta * dvz2;
    vz1r = vz1r - m21 * vz2r;
    vx1r = vx1r - m21 * vx2r;
    vy1r = vy1r - m21 * vy2r;

    //     **** rotate the velocity vectors back and add the initial velocity
    //           vector of ball 2 to retrieve the original coordinate system ****

    v1.x = ct * cp * vx1r - sp * vy1r + st * cp * vz1r + v2.x;
    v1.y = ct * sp * vx1r + cp * vy1r + st * sp * vz1r + v2.y;
    v1.z = ct * vz1r - st * vx1r + v2.z;
    v2.x = ct * cp * vx2r - sp * vy2r + st * cp * vz2r + v2.x;
    v2.y = ct * sp * vx2r + cp * vy2r + st * sp * vz2r + v2.y;
    v2.z = ct * vz2r - st * vx2r + v2.z;

    //     ***  velocity correction for inelastic collisions ***
    rb1.velocity = (v1 - v_cm) * re1 + v_cm;
    rb2.velocity = (v2 - v_cm) * re2 + v_cm;
}

impl SphereColider {
    fn get_radius(&self, scale: Vec3) -> f32 {
        // debug_assert!(radius >= 0.0);
        self.radius * scale.x.abs() // TODO FIX, this is left to the user to discover
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Collider {
    SphereColider(SphereColider),
    BoxColider(BoxColider),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct BoxColider {
    pub local_position: Vec3,
    pub local_rotation: Quaternion,
    pub scale: Vec3,
    pub material: PhysicsMaterial,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RayCastHit {
    pub hit: Vec3,    // world position
    pub normal: Vec3, // normalized
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RidgidBody {
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub acceleration: Vec3, // gravity
    pub center_of_mass_offset: Vec3,
    pub is_active_time: f32,
    pub mass: f32,
    pub is_using_global_gravity: bool,
    //is_trigger : bool,
    pub is_static: bool,
    pub is_active: bool, // after object is not moving for 2s then it becomes disabled
}

impl RidgidBody {
    fn new(velocity: Vec3, acceleration: Vec3, mass: f32) -> Self {
        Self {
            velocity,
            acceleration,
            mass,
            angular_velocity: Vec3::zero(),
            is_active: true,
            is_using_global_gravity: false,
            is_active_time: 0.0f32,
            center_of_mass_offset: Vec3::zero(),
            is_static: false,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PhysicsObject {
    pub rb: RidgidBody,
    pub colliders: Vec<Collider>,
}

impl PhysicsObject {
    pub fn new(rb: RidgidBody, colider: Collider) -> Self {
        Self {
            rb,
            colliders: vec![colider],
        }
    }
}

fn collide(
    rb1: &mut RidgidBody,
    t1: &mut Transform,
    cc1: &Vec<Collider>,
    t2: &mut Transform,
    rb2: &mut RidgidBody,
    cc2: &Vec<Collider>,
) {
    for c1 in cc1 {
        for c2 in cc2 {
            if is_colliding(c1, t1, c2, t2) {
                println!("Colliding! {:?} {:?}", c1, c2);
                solve_colliding(c1, rb1, t1, c2, rb2, t2)
            }
        }
    }
}

impl RidgidBody {
    fn add_impulse(&mut self, force: Vec3) {
        self.velocity += force / self.mass;
    }

    fn step(&mut self, dt: f32, transform: &mut Transform) {
        transform.position += self.velocity * dt;
        self.velocity += self.acceleration * dt;
    }
}

fn update(
    start: std::time::Instant,
    dt: f32,
    transforms: &mut Vec<Transform>,
    phx_objects: &mut Vec<PhysicsObject>,
) {
    let phx_length = phx_objects.len();
    for i in 0..phx_length {
        let (phx_first, phx_last) = phx_objects.split_at_mut(i + 1);
        let (trans_first, trans_last) = transforms.split_at_mut(i + 1);

        phx_first[i].rb.step(dt, &mut trans_first[i]);
        for (transform, phx_obj) in trans_last.iter_mut().zip(phx_last.iter_mut()) {
            collide(
                &mut phx_first[i].rb,
                &mut trans_first[i],
                &phx_first[i].colliders,
                transform,
                &mut phx_obj.rb,
                &phx_obj.colliders,
            );
        }
    }
}

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
    let model = context.renderer.load_model("./res/Cube.obj").unwrap();
    let model_pawn = context.renderer.load_model("./res/ball.obj").unwrap();
    let start = std::time::Instant::now();

    let mut camera_controller = CameraController::new(
        10.0,
        0.01,
        Vec3::new(-2.617557, 0.3896206, -2.1071591),
        Vec3::new(-0.040865257, 2.8307953, 0.0),
    );

    let mut instances = (0..NUM_INSTANCES_PER_ROW)
        .flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let r: f32 = rand::random();
                let position = Vec3 { x, y: 0.0, z };

                let rotation = if position == Vec3::zero() {
                    Quaternion::rotation_3d(0.0, Vec3::unit_z())
                } else {
                    Quaternion::rotation_3d(std::f32::consts::FRAC_PI_4, position.normalized())
                };

                Transform {
                    position,
                    rotation,
                    scale: Vec3::new(0.1, 0.1, 0.1),
                }
            })
        })
        .collect::<Vec<_>>();

    let physics_material = PhysicsMaterial {
        friction: 1.0,
        restfullness: 0.5,
    };

    let obj1 = PhysicsObject::new(
        RidgidBody::new(Vec3::new(0.5, 0.02, 0.0), Vec3::new(0.0, 0.0, 0.0), 5.0),
        Collider::SphereColider(SphereColider::new(1.0, physics_material)),
    );
    let obj2 = PhysicsObject::new(
        RidgidBody::new(Vec3::new(0.0, 0.02, 0.0), Vec3::zero(), 5.0),
        Collider::SphereColider(SphereColider::new(1.0, physics_material)),
    );
    let obj3 = PhysicsObject::new(
        RidgidBody::new(Vec3::new(0.0, 0.0, 0.0), Vec3::zero(), 5.0),
        Collider::SphereColider(SphereColider::new(1.0, physics_material)),
    );
    let obj4 = PhysicsObject::new(
        RidgidBody::new(Vec3::new(0.0, 0.0, 0.0), Vec3::zero(), 5.0),
        Collider::SphereColider(SphereColider::new(1.0, physics_material)),
    );

    let mut physics_objects: Vec<PhysicsObject> = vec![obj1, obj2, obj3, obj4]; //vec![obj1.clone(); 16];

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
                let dt = last_frame.elapsed().as_secs_f32();
                last_frame = std::time::Instant::now();
                let _frame_rate = 1.0 / dt; // TODO render on screen

                if allow_camera_update {
                    camera_controller.update_camera(dt, &mut context.renderer.camera);
                }

                update(start, dt, &mut instances, &mut physics_objects);

                context.renderer.update_camera();

                context
                    .renderer
                    .update_instances(&[(model, &instances[8..]), (model_pawn, &instances[..8])]);
                context
                    .renderer
                    .render([0.229, 0.507, 0.921, 1.0])
                    .expect("render error");
            }

            _ => (),
        }
    });
}
