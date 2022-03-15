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

impl SphereColider {
    fn get_radius(radius: f32, scale: Vec3) -> f32 {
        // debug_assert!(radius >= 0.0);
        radius * scale.x.abs() // TODO FIX, this is left to the user to discover
    }

    pub fn collide(
        &self,
        own_rb: &mut RidgidBody,
        own_transform: &mut Transform,
        other: &Collider,
        other_rb: &mut RidgidBody,
        other_transform: &mut Transform,
    ) {
        let self_position = get_world_position(
            own_transform.position,
            own_transform.rotation,
            self.local_position,
        );

        //own_rb.velocity -= 0.0001*(own_transform.position);

        let self_radius = SphereColider::get_radius(self.radius, own_transform.scale);
        match other {
            Collider::SphereColider(s) => {
                let R = 0.5;
                let other_position = get_world_position(
                    other_transform.position,
                    other_transform.rotation,
                    s.local_position,
                );
                let other_radius = SphereColider::get_radius(s.radius, other_transform.scale);

                let diff = other_position - self_position;

                let distance_squared = diff.magnitude_squared();
                let total_radius = other_radius + self_radius;
                if distance_squared <= total_radius * total_radius {
                    let normal = diff.normalized();
                    let distance = distance_squared.sqrt() - total_radius;
                    let pop = normal * distance * 0.51;
                    own_transform.position += pop;
                    other_transform.position -= pop;

                    let m1 = own_rb.mass; //  (mass of ball 1)
                    let m2 = other_rb.mass; //  (mass of ball 2)
                    let r1 = self_radius; //  (radius of ball 1)
                    let r2 = other_radius; //  (radius of ball 2)
                    let mut b1 = self_position; //  (coordinate of ball 1)
                    let mut b2 = other_position; //  (coordinate of ball 2)
                    let mut v1 = own_rb.velocity; //  (velocity of ball 1)
                    let mut v2 = other_rb.velocity; //  (velocity of ball 2)

                    let r12 = r1 + r2;
                    let m21 = m2 / m1;
                    let b21 = b2 - b1;
                    let v21 = v2 - v1;

                    let v_cm = (m1 * v1 + m2 * v2) / (m1 + m2);

                    //     **** calculate relative distance and relative speed ***
                    let d = b21.magnitude();
                    let v = v21.magnitude();

                    //     **** shift coordinate system so that ball 1 is at the origin ***
                    b2 = b21;

                    //     **** boost coordinate system so that ball 2 is resting ***
                    v1 = -v21;

                    //     **** find the polar coordinates of the location of ball 2 ***
                    let theta2 = (b2.z / d).acos();
                    let phi2 = if b2.x == 0.0 && b2.y == 0.0 {
                        0.0
                    } else {
                        b2.y.atan2(b2.x)
                    };
                    let st = (theta2).sin();
                    let ct = (theta2).cos();
                    let sp = (phi2).sin();
                    let cp = (phi2).cos();

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
                    let thetav = (fvz1r).acos();
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
                    let t = (d * (thetav.cos()) - r12 * (1.0 - dr * dr).sqrt()) / v;

                    //     **** update positions and reverse the coordinate shift ***
                    b2 += v2 * t + b1;
                    b1 += (v1 + v2) * t;

                    //  ***  update velocities ***

                    let a = (thetav + alpha).tan();

                    let dvz2 = 2.0 * (vz1r + a * (cbeta * vx1r + sbeta * vy1r))
                        / ((1.0 + a * a) * (1.0 + m21));

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

                    v1 = (v1 - v_cm) * R + v_cm;
                    v2 = (v2 - v_cm) * R + v_cm;
                    own_rb.velocity = v1;
                    other_rb.velocity = v2;
                }
            }
            _ => unimplemented!(),
        }
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

impl RidgidBody {
    fn add_impulse(&mut self, force: Vec3) {
        self.velocity += force / self.mass;
    }

    fn step(&mut self, dt: f32, transform: &mut Transform) {
        transform.position += self.velocity * dt;
        self.velocity += self.acceleration * dt;
    }

    fn collide(
        &mut self,
        own_transform: &mut Transform,
        own_coliders: &Vec<Collider>,
        other_transform: &mut Transform,
        other_rb: &mut RidgidBody,
        other_coliders: &Vec<Collider>,
    ) {
        for own in own_coliders {
            match own {
                Collider::SphereColider(s) => {
                    for other in other_coliders {
                        s.collide(self, own_transform, other, other_rb, other_transform)
                    }
                }
                Collider::BoxColider(_) => todo!(),
            }
        }
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
            phx_first[i].rb.collide(
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
                    .update_instances(&[(model, &instances[1..]), (model_pawn, &instances[..1])]);
                context
                    .renderer
                    .render([0.229, 0.507, 0.921, 1.0])
                    .expect("render error");
            }

            _ => (),
        }
    });
}
