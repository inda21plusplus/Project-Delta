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
use vek::{
    num_integer::{sqrt, Roots},
    num_traits::ToPrimitive,
    Ray,
};

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

                debug_assert!(r1 > 0.0, "r1 = {}", r1);
                debug_assert!(r2 > 0.0, "r2 = {}", r2);

                let total_radius = r1 + r2;

                w1.distance_squared(w2) <= total_radius * total_radius
            }
            Collider::BoxColider(b2) => {
                let r1 = b1.get_radius(t1.scale);
                let r2 = b2.get_radius(t2, w1 - w2);

                debug_assert!(r1 > 0.0, "r1 = {}", r1);
                debug_assert!(r2 > 0.0, "r2 = {}", r2);

                let total_radius = r1 + r2;

                w1.distance_squared(w2) <= total_radius * total_radius
            }
        },
        Collider::BoxColider(b1) => match c2 {
            Collider::BoxColider(b2) => {
                todo!("box vs box")
            }
            Collider::SphereColider(_) => is_colliding(c2, t2, c1, t1), // reuse code
        },
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
            Collider::BoxColider(b2) => collide_sphere_vs_box(b1, rb1, t1, w1, b2, rb2, t2, w2),
        },
        Collider::BoxColider(b1) => match c2 {
            Collider::SphereColider(b2) => collide_sphere_vs_box(b2, rb2, t2, w2, b1, rb1, t1, w1),
            Collider::BoxColider(_b2) => {
                todo!("box vs box")
            }
        },
    }
}

#[inline]
/// using https://en.wikipedia.org/wiki/Elastic_collision on a 1d plane where m is mass and v is velocity
fn standard_elastic_collision(m1: f32, v1: f32, m2: f32, v2: f32) -> (f32, f32) {
    let u1: f32 = (m1 * v1 - m2 * v1 + 2.0 * m2 * v2) / (m1 + m2);
    let u2: f32 = (2.0 * m1 * v1 - m1 * v2 + m2 * v2) / (m1 + m2);

    //todo https://en.wikipedia.org/wiki/Inelastic_collision
    //todo https://en.wikipedia.org/wiki/Coefficient_of_restitution
    (u1, u2)
}

fn standard_elastic_collision_3(m1: f32, v1: &Vec3, m2: f32, v2: &Vec3) -> (Vec3, Vec3) {
    let (v1x, v2x) = standard_elastic_collision(m1, v1.x, m2, v2.x);
    let (v1y, v2y) = standard_elastic_collision(m1, v1.y, m2, v2.y);
    let (v1z, v2z) = standard_elastic_collision(m1, v1.z, m2, v2.z);
    (Vec3::new(v1x, v1y, v1z), Vec3::new(v2x, v2y, v2z))
}

#[inline]
fn proj(on: Vec3, vec: Vec3) -> Vec3 {
    vec.dot(on) * on / on.magnitude_squared()
}

#[cfg(debug_assertions)]
macro_rules! pause {
    () => {
        pause_and_wait_for_input()
    };
}

#[cfg(not(debug_assertions))]
macro_rules! pause {
    () => {};
}

#[allow(dead_code)]
fn pause_and_wait_for_input() {
    let mut stdout = std::io::stdout();
    std::io::Write::write(&mut stdout, b"Paused\n").unwrap();
    std::io::Write::flush(&mut stdout).unwrap();
    std::io::Read::read(&mut std::io::stdin(), &mut [0]).unwrap();
}

pub fn collide_sphere_vs_box(
    c1: &SphereColider,
    rb1: &mut RidgidBody,
    t1: &mut Transform,
    mut w1: Vec3, // world position
    c2: &BoxColider,
    rb2: &mut RidgidBody,
    t2: &mut Transform,
    mut w2: Vec3, // world position
) {
    let re1 = c1.material.restfullness;
    let re2 = c2.material.restfullness;

    let r1 = c1.get_radius(t1.scale);
    let r2 = c2.get_radius(t2, w1 - w2);

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

    let m1 = rb1.mass;
    let m2 = rb2.mass;

    println!("General collision!");
    let v1 = rb1.velocity;
    let v2 = rb2.velocity;

    let normal = c2.get_side(t2, w1 - w2);

    // proj the velocities on the normal, this way you can move the frame of
    // refrence and think of the two objects are coliding head on
    let real_v1 = proj(normal, v1);
    let real_v2 = proj(normal, v2);

    // using a perfectly elastic collision on each axis
    let (new_v1, new_v2) = standard_elastic_collision_3(m1, &real_v1, m2, &real_v2);

    // inital velocity - velocity used to colide "head on" + velocity after coliding "head on"
    rb1.velocity = v1 - real_v1;
    rb2.velocity = v2 - real_v2;

    let location_normal = w2 - w1;

    rb1.add_impulse_at_location(new_v1, -location_normal * r1);
    rb2.add_impulse_at_location(new_v2, location_normal * r2);
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

    // these both colision methods should return the exact same result
    if true {
        // this is used because it can be used for non-sphere colisions
        println!("General collision!");
        let v1 = rb1.velocity;
        let v2 = rb2.velocity;

        let diff = w2 - w1;
        let normal = diff.normalized();

        // proj the velocities on the normal, this way you can move the frame of
        // refrence and think of the two objects are coliding head on
        let real_v1 = proj(normal, v1);
        let real_v2 = proj(normal, v2);

        // using a perfectly elastic collision on each axis
        let (new_v1, new_v2) = standard_elastic_collision_3(m1, &real_v1, m2, &real_v2);

        // inital velocity - velocity used to colide "head on" + velocity after coliding "head on"
        rb1.velocity = v1 - real_v1;
        rb2.velocity = v2 - real_v2;

        rb1.add_impulse_at_location(new_v1, -normal * r1);
        rb2.add_impulse_at_location(new_v2, normal * r2);
    } else {
        println!("Sphere collision!");

        let mut v1 = rb1.velocity;
        let mut v2 = rb2.velocity;

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
}

impl SphereColider {
    fn get_radius(&self, scale: Vec3) -> f32 {
        // debug_assert!(radius >= 0.0);
        self.radius * scale.x.abs() // TODO FIX, this is left to the user to discover
    }
}

impl BoxColider {
    #[inline]
    /// gets the distance from the box center acording to its bounds, can take non normalized input
    pub fn get_radius(&self, t: &Transform, direction: Vec3) -> f32 {
        debug_assert!(!t.scale.is_approx_zero(), "Scale too close to 0");

        debug_assert!(
            !direction.is_approx_zero(),
            "Direction magintude is too close to 0, {} | {:?}",
            direction.magnitude(),
            direction
        );

        debug_assert!(
            direction.x.is_finite() && direction.y.is_finite() && direction.z.is_finite(),
            "direction is not finite, {:?}",
            direction
        );

        let outside_normal = self.get_side(t, direction);

        debug_assert!(
            is_finite(&outside_normal),
            "outside_normal is not finite, {:?} at direction = {:?}",
            outside_normal,
            direction,
        );

        let plane_point = outside_normal * t.scale;
        let inside_normal = -outside_normal;

        let real_direction = t.rotation * direction.normalized();

        //https://en.wikipedia.org/wiki/Line%E2%80%93plane_intersection
        plane_point.dot(inside_normal) / (real_direction.dot(inside_normal))
    }

    /// If you raycast from the center of a box, witch side does the ray intercet with
    /// this function returns the side with the normal pointed out of the box of the side the ray colides with
    /// so (1,0.1,0.1) => (1,0,0) as it hits the side with that normal, the result is allways normalized
    /// if the direction is exactly 45 degrees, it prioritizes x then y then z
    pub fn get_side(&self, t: &Transform, direction: Vec3) -> Vec3 {
        let real_direction = t.rotation * direction;
        let scale = self.scale * t.scale;
        let dir = real_direction.normalized() / scale;

        let x = dir.x.abs();
        let y = dir.y.abs();
        let z = dir.z.abs();

        // this simply returns the axis with the largest scalar as a the normalized vector
        if x >= y && x >= z {
            Vec3::new(dir.x / x, 0.0, 0.0)
        } else if y >= x && y >= z {
            Vec3::new(0.0, dir.y / y, 0.0)
        } else {
            Vec3::new(0.0, 0.0, dir.z / z)
        }
    }
}

macro_rules! assert_delta {
    ($x:expr, $y:expr, $d:expr) => {
        if !($x - $y < $d || $y - $x < $d) {
            panic!();
        }
    };
}

#[test]
fn get_radius_test() {
    let t = Transform {
        position: Vec3::new(0.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        scale: Vec3::new(1.0, 1.0, 1.0),
    };

    let box_c = BoxColider::new(
        Vec3::new(1.0, 1.0, 1.0),
        PhysicsMaterial {
            friction: 1.0,
            restfullness: 1.0,
        },
    );

    assert_eq!(box_c.get_radius(&t, Vec3::new(1.0, 0.0, 0.0)), 1.0);

    assert_eq!(
        box_c.get_radius(&t, Vec3::new(100.0, 0.0, 0.0)),
        1.0,
        "can not take non normalized input"
    );

    assert_eq!(
        box_c.get_radius(&t, Vec3::new(1.0, 1.0, 0.0)),
        2.0f32.sqrt()
    );

    let max_radius = 3.0f32.sqrt();
    let min_radius = 1.0f32.sqrt();

    assert_delta!(
        box_c.get_radius(&t, Vec3::new(1.0, 1.0, 1.0)),
        max_radius,
        0.0001f32
    );

    for pitch_deg in 0..360 {
        for yaw_deg in 0..360 {
            let pitch = pitch_deg.to_f32().unwrap().to_radians();
            let yaw = yaw_deg.to_f32().unwrap().to_radians();

            let forward = Vec3::new(
                yaw.sin() * pitch.cos(),
                pitch.sin(),
                yaw.cos() * pitch.cos(),
            );
            let size = box_c.get_radius(&t, forward);

            assert!(!size.is_nan(), "size is nan");
            assert!(size.is_finite(), "size is inf");
            assert!(size >= min_radius, "size is less than inner radius of box");
            assert!(size <= max_radius, "size is above maximum")
        }
    }
}

#[test]
/// simple test to check that BoxColider::get_side returns the correct results given a cube
fn get_side_test() {
    let t = Transform {
        position: Vec3::new(0.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        scale: Vec3::new(1.0, 1.0, 1.0),
    };

    let box_c = BoxColider::new(
        Vec3::new(1.0, 1.0, 1.0),
        PhysicsMaterial {
            friction: 1.0,
            restfullness: 1.0,
        },
    );

    let same_dirs = vec![
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
    ];

    for small_offset in &same_dirs {
        let offset = small_offset / 10.0;
        for dir in &same_dirs {
            let side_dir = box_c.get_side(&t, *dir * 0.5 + offset);

            assert_eq!(side_dir, *dir); // assert correct direction
            assert_eq!(side_dir.magnitude_squared(), 1.0) // assert normalized
        }
    }

    // checks for nans and inf
    for pitch_deg in 0..360 {
        for yaw_deg in 0..360 {
            let pitch = pitch_deg.to_f32().unwrap().to_radians();
            let yaw = yaw_deg.to_f32().unwrap().to_radians();

            let forward = Vec3::new(
                yaw.sin() * pitch.cos(),
                pitch.sin(),
                yaw.cos() * pitch.cos(),
            );

            let side = box_c.get_side(&t, forward);
            assert!(side.x.is_finite(), "{:?}", side);
            assert!(side.y.is_finite(), "{:?}", side);
            assert!(side.z.is_finite(), "{:?}", side);
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

impl BoxColider {
    fn new(scale: Vec3, material: PhysicsMaterial) -> Self {
        Self {
            local_position: Vec3::zero(),
            local_rotation: Quaternion::identity(),
            scale,
            material,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RayCastHit {
    pub hit: Vec3,    // world position
    pub normal: Vec3, // normalized
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RidgidBody {
    pub velocity: Vec3,
    pub acceleration: Vec3, // can be used for gravity

    pub angular_velocity: Vec3, // Spin angular velocity in rad per seconds around that axis (Quaternion::rotate_3d)
    pub torque: Vec3,           // torque to angular_velocity is what acceleration is to velocity

    pub center_of_mass_offset: Vec3, // also used for instant center of rotation https://en.wikipedia.org/wiki/Instant_centre_of_rotation
    pub is_active_time: f32,
    pub mass: f32,
    pub is_using_global_gravity: bool,
    //is_trigger : bool,
    pub is_static: bool,
    pub is_active: bool, // TODO after object is not moving then it becomes disabled to oprimize
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
            torque: Vec3::zero(),
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

fn is_finite(v: &Vec3) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
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
                println!("Colliding!");
                pause!();

                solve_colliding(c1, rb1, t1, c2, rb2, t2);

                debug_assert!(is_finite(&rb1.velocity), "rb1 velocity = {}", rb1.velocity);
                debug_assert!(is_finite(&rb2.velocity), "rb2 velocity = {}", rb2.velocity);

                debug_assert!(
                    is_finite(&rb1.angular_velocity),
                    "rb1 angular_velocity = {}",
                    rb1.angular_velocity
                );
                debug_assert!(
                    is_finite(&rb2.angular_velocity),
                    "rb2 angular_velocity = {}",
                    rb2.angular_velocity
                );

                rb1.is_active = true;
                rb2.is_active = true;
                rb1.is_active_time = 0.0;
                rb2.is_active_time = 0.0;
            }
        }
    }
}

impl RidgidBody {
    fn add_impulse(&mut self, force: Vec3) {
        self.velocity += force / self.mass;
    }

    fn add_impulse_at_location(&mut self, velocity: Vec3, location: Vec3) {
        debug_assert!(is_finite(&velocity), "velocity = {}", velocity);
        debug_assert!(is_finite(&location), "location = {}", location);

        //debug_assert!(velocity.magnitude_squared() != 0.0, "velocity is too close to 0 = {}", velocity);

        // if zero velocity is applied then nothing happends
        if velocity.magnitude_squared() == 0.0 { 
            return;
        }

        // Bullet Block Explained! https://youtu.be/BLYoyLcdGPc no velocity is lost due to angular velocity irl,
        // so it is not removed here
        self.velocity += velocity;

        //https://en.wikipedia.org/wiki/Angular_velocity

        // just random shit
        let offset = self.center_of_mass_offset + location;
        let normal = offset;

        let rotation_around = -(normal.normalized().cross(velocity.normalized())).normalized();
        debug_assert!(
            is_finite(&rotation_around),
            "rotation_around = {} normal {} velocity {}",
            rotation_around,
            normal,
            velocity
        );

        let torque = rotation_around * 10.0; //velocity *  / offset.magnitude();

        self.angular_velocity += torque;

        // TODO idk what angular_velocity is
    }

    fn step(&mut self, dt: f32, transform: &mut Transform) {
        // apply acceleration
        self.velocity += self.acceleration * dt;
        self.angular_velocity += self.torque * dt;

        // apply rotation
        transform.rotation.rotate_x(self.angular_velocity.x * dt);
        transform.rotation.rotate_y(self.angular_velocity.y * dt);
        transform.rotation.rotate_z(self.angular_velocity.z * dt);

        // update position
        transform.position += self.velocity * dt;

        self.is_active_time += dt;
    }
}

fn update(dt: f32, transforms: &mut Vec<Transform>, phx_objects: &mut Vec<PhysicsObject>) {
    let real_dt = dt * 0.3;
    let phx_length = phx_objects.len();
    for i in 0..phx_length {
        let (phx_first, phx_last) = phx_objects.split_at_mut(i + 1);
        if phx_first[i].rb.is_static || !phx_first[i].rb.is_active {
            continue; // we dont care about non active or static objects
        }

        let (trans_first, trans_last) = transforms.split_at_mut(i + 1);

        phx_first[i].rb.step(real_dt, &mut trans_first[i]);
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
                    Quaternion::identity() //  Quaternion::rotation_3d(0.0, Vec3::unit_z())
                } else {
                    Quaternion::identity() //Quaternion::rotation_3d(std::f32::consts::FRAC_PI_4, position.normalized())
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
        restfullness: 1.0,
    };

    let obj1 = PhysicsObject::new(
        RidgidBody::new(Vec3::new(0.5, 0.02, 0.002), Vec3::new(0.0, 0.0, 0.0), 10.0),
        Collider::SphereColider(SphereColider::new(1.0, physics_material)),
    );
    //let obj2 = PhysicsObject::new(
    //    RidgidBody::new(Vec3::new(0.0, 0.01, 0.0), Vec3::zero(), 5.0),
    //    Collider::SphereColider(SphereColider::new(1.0, physics_material)),
    //);

    let obj2 = PhysicsObject::new(
        RidgidBody::new(Vec3::new(0.0, 0.00, 0.0), Vec3::zero(), 5.0),
        Collider::BoxColider(BoxColider::new(Vec3::new(1.0, 1.0, 1.0), physics_material)),
    );
    /*let obj3 = PhysicsObject::new(
        RidgidBody::new(Vec3::new(0.0, 0.0, 0.0), Vec3::zero(), 5.0),
        Collider::SphereColider(SphereColider::new(1.0, physics_material)),
    );
    let obj4 = PhysicsObject::new(
        RidgidBody::new(Vec3::new(0.0, 0.0, 0.0), Vec3::zero(), 5.0),
        Collider::SphereColider(SphereColider::new(1.0, physics_material)),
    );*/

    let mut physics_objects: Vec<PhysicsObject> = vec![obj1, obj2]; //obj3, obj4 vec![obj1.clone(); 16];

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
                let _frame_rate = 1.0 / dt; // TODO render on screen

                if allow_camera_update {
                    camera_controller.update_camera(dt, &mut context.renderer.camera);
                }

                update(dt, &mut instances, &mut physics_objects);
                last_frame = std::time::Instant::now();

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
