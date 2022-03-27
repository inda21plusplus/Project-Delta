use camera_controller::CameraController;
use game_engine::{renderer::Transform, Context};
use rand::Rng;

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

#[must_use]
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

#[inline]
#[must_use]
fn squared(v: f32) -> f32 {
    v * v
}

#[must_use]
fn clamp(v: Vec3, min: Vec3, max: Vec3) -> Vec3 {
    let mut ret = Vec3::zero();
    for i in 0..3 {
        let min = min[i];
        let max = max[i];
        ret[i] = f32::clamp(v[i], min.min(max), min.max(max))
    }
    ret
}

#[must_use]
fn get_closest_point(
    other_loc: Vec3,
    cube_loc: Vec3,
    cube_scale: Vec3,
    cube_rotation: Quaternion,
) -> Vec3 {
    // this rotates the other so the cube is aligned with the world axis (aka Quaterion Identity)
    let other_loc = cube_rotation.inverse() * (other_loc - cube_loc) + cube_loc;

    let b_min = cube_loc - cube_scale;
    let b_max = cube_loc + cube_scale;

    //https://developer.mozilla.org/en-US/docs/Games/Techniques/3D_collision_detection
    let closest = clamp(other_loc, b_min, b_max);

    // rotate back to world space
    cube_rotation * (closest - cube_loc) + cube_loc
}

/// returns (1,0,0) (0,1,0) (0,0,1) with rotation aka positive normals
#[must_use]
fn get_axis(t: &Transform, c: &BoxColider) -> (Vec3, Vec3, Vec3) {
    let rotation = t.rotation * c.local_rotation;
    (
        rotation * Vec3::unit_x(),
        rotation * Vec3::unit_y(),
        rotation * Vec3::unit_z(),
    )
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

fn get_vertex(w: &Vec3, t: &Transform, c: &BoxColider) -> Vec<Vec3> {
    let s = c.scale * t.scale;
    let r = t.rotation * c.local_rotation;
    let mut vec: Vec<Vec3> = Vec::with_capacity(8);

    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                vec.push(w + r * Vec3::new(s.x * x, s.y * y, s.z * z))
            }
        }
    }

    vec
}

fn overlap(a_min: f32, a_max: f32, b_min: f32, b_max: f32) -> f32 {
    debug_assert!(a_min <= a_max, "a min < max");
    debug_assert!(b_min <= b_max, "b min < max");

    if a_min < b_min {
        if a_max < b_min {
            0.0
        } else {
            a_max - b_min
        }
    } else if b_max < a_min {
        0.0
    } else {
        a_min - b_max
    }
}

fn get_min_max_vert(normal: Vec3, verts: &Vec<Vec3>) -> (f32, f32) {
    let mut proj_min = f32::MAX;
    let mut proj_max = f32::MIN;
    for vert in verts {
        let val = vert.dot(normal);
        if val < proj_min {
            proj_min = val;
        }

        if val > proj_max {
            proj_max = val;
        }
    }
    (proj_min, proj_max)
}

/// SAT algo on 3d
/// https://hitokageproduction.com/article/11
/// https://github.com/irixapps/Unity-Separating-Axis-SAT/
/// https://youtu.be/7Ik2vowGcU0
/// The seperated axis theorem tldr:
/// If 2 shapes colide then all the shadows along all the axis must overlap
#[must_use]
fn proj_has_overlap(axis: &Vec<Vec3>, a_verts: &Vec<Vec3>, b_verts: &Vec<Vec3>) -> bool {
    for normal in axis {
        if *normal == Vec3::zero() {
            return true;
        }
        let (a_min, a_max) = get_min_max_vert(*normal, a_verts);
        let (b_min, b_max) = get_min_max_vert(*normal, b_verts);
        let overlap = overlap(a_min, a_max, b_min, b_max).abs();

        if overlap <= 0.0 {
            return false;
        }
    }

    true
}

/// same as proj_has_overlap with more return info
#[must_use]
fn proj_has_overlap_extra(
    axis: &Vec<Vec3>,
    a_verts: &Vec<Vec3>,
    b_verts: &Vec<Vec3>,
) -> Option<(f32, Vec3)> {
    let mut min_overlap = f32::INFINITY;
    let mut penetration = Vec3::zero();
    for normal in axis {
        if *normal == Vec3::zero() {
            return Some((min_overlap, penetration));
        }
        let (a_min, a_max) = get_min_max_vert(*normal, a_verts);
        let (b_min, b_max) = get_min_max_vert(*normal, b_verts);
        let overlap = overlap(a_min, a_max, b_min, b_max);
        let abs_overlap = overlap.abs();
        if abs_overlap <= 0.0 {
            return None;
        }

        if abs_overlap < min_overlap.abs() {
            min_overlap = overlap;
            penetration = *normal;
        }
    }

    Some((min_overlap, penetration))
}

#[must_use]
fn get_axis_and_verts(
    w1: &Vec3,
    w2: &Vec3,
    t1: &Transform,
    t2: &Transform,
    bc1: &BoxColider,
    bc2: &BoxColider,
) -> (Vec<Vec3>, Vec<Vec3>, Vec<Vec3>) {
    let (a0, a1, a2) = get_axis(&t1, bc1);
    let (b0, b1, b2) = get_axis(&t2, bc2);

    let axis = vec![
        a0,
        a1,
        a2,
        b0,
        b1,
        b2,
        a0.cross(b0),
        a0.cross(b1),
        a0.cross(b2),
        a1.cross(b0),
        a1.cross(b1),
        a1.cross(b2),
        a2.cross(b0),
        a2.cross(b1),
        a2.cross(b2),
    ];

    let a_vex = get_vertex(w1, &t1, bc1);
    let b_vex = get_vertex(w2, &t2, bc2);
    (axis, a_vex, b_vex)
}

/// Returns true if 2 objects are colliding
#[must_use]
pub fn is_colliding(c1: &Collider, t1: &mut Transform, c2: &Collider, t2: &mut Transform) -> bool {
    let w1 = get_position(t1, c1);
    let w2 = get_position(t2, c2);

    match c1 {
        Collider::SphereColider(sc1) => match c2 {
            Collider::SphereColider(sc2) => {
                let r1 = sc1.get_radius(t1.scale);
                let r2 = sc2.get_radius(t2.scale);

                debug_assert!(r1 > 0.0, "r1 = {}", r1);
                debug_assert!(r2 > 0.0, "r2 = {}", r2);

                let total_radius = r1 + r2;

                w1.distance_squared(w2) <= total_radius * total_radius
            }
            Collider::BoxColider(bc2) => {
                let r_squared = squared(sc1.get_radius(t1.scale));
                debug_assert!(r_squared > 0.0, "r^2 = {}", r_squared);

                let scale = t2.scale * bc2.scale;
                debug_assert!(scale.are_all_positive(), "Scale is negative");
                debug_assert!(is_finite(&scale), "Scale is Nan = {}", scale);

                let closest_point = get_closest_point(w1, w2, scale, t2.rotation);
                closest_point.distance_squared(w1) < r_squared
            }
        },
        Collider::BoxColider(bc1) => match c2 {
            Collider::BoxColider(bc2) => {
                let (axis, a_verts, b_verts) = get_axis_and_verts(&w1, &w2, &t1, &t2, bc1, bc2);
                proj_has_overlap(&axis, &a_verts, &b_verts)
                    || proj_has_overlap(&axis, &b_verts, &a_verts)
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
            Collider::BoxColider(b2) => collide_box_vs_box(b1, rb1, t1, w1, b2, rb2, t2, w2),
        },
    }
}

#[inline]
#[must_use]
/// using https://en.wikipedia.org/wiki/Elastic_collision on a 1d plane where m is mass and v is velocity
fn standard_elastic_collision(m1: f32, v1: f32, m2: f32, v2: f32) -> (f32, f32) {
    let u1: f32 = (m1 * v1 - m2 * v1 + 2.0 * m2 * v2) / (m1 + m2);
    let u2: f32 = (2.0 * m1 * v1 - m1 * v2 + m2 * v2) / (m1 + m2);

    //todo https://en.wikipedia.org/wiki/Inelastic_collision
    //todo https://en.wikipedia.org/wiki/Coefficient_of_restitution
    (u1, u2)
}

#[must_use]
fn standard_elastic_collision_3(m1: f32, v1: &Vec3, m2: f32, v2: &Vec3) -> (Vec3, Vec3) {
    let (v1x, v2x) = standard_elastic_collision(m1, v1.x, m2, v2.x);
    let (v1y, v2y) = standard_elastic_collision(m1, v1.y, m2, v2.y);
    let (v1z, v2z) = standard_elastic_collision(m1, v1.z, m2, v2.z);
    (Vec3::new(v1x, v1y, v1z), Vec3::new(v2x, v2y, v2z))
}

#[inline]
#[must_use]
fn proj(on: Vec3, vec: Vec3) -> Vec3 {
    vec.dot(on) * on / on.magnitude_squared()
}

#[allow(dead_code)]
fn pause_and_wait_for_input() {
    let mut stdout = std::io::stdout();
    std::io::Write::write(&mut stdout, b"Paused\n").unwrap();
    std::io::Write::flush(&mut stdout).unwrap();
    std::io::Read::read(&mut std::io::stdin(), &mut [0]).unwrap();
}

pub fn collide_box_vs_box(
    c1: &BoxColider,
    rb1: &mut RidgidBody,
    t1: &mut Transform,
    w1: Vec3, // world position
    c2: &BoxColider,
    rb2: &mut RidgidBody,
    t2: &mut Transform,
    w2: Vec3, // world position
) {
    let (axis, a_verts, b_verts) = get_axis_and_verts(&w1, &w2, t1, t2, c1, c2);
    let mut min_overlap: Option<(f32, Vec3)> = None;

    min_overlap = proj_has_overlap_extra(&axis, &b_verts, &a_verts);
    if min_overlap.is_none() {
        min_overlap = proj_has_overlap_extra(&axis, &a_verts, &b_verts);
    }
    debug_assert!(min_overlap.is_some());

    if let Some((overlap, overlap_axis)) = min_overlap {
        let re1 = c1.material.restfullness;
        let re2 = c2.material.restfullness;

        // TODO MAKE BETTER
        let normal = overlap_axis.normalized(); // this is not perfect
        pop_coliders(normal * overlap * 2.0, t1, t2, &rb1, &rb2);
        standard_collision(
            normal,
            rb1,
            rb2,
            normal * overlap,
            -normal * overlap,
            re1,
            re2,
        );
        // t1.position = w2 + _axis.normalized() * overlap;
    }
}

pub fn collide_sphere_vs_box(
    c1: &SphereColider,
    rb1: &mut RidgidBody,
    t1: &mut Transform,
    w1: Vec3, // world position
    c2: &BoxColider,
    rb2: &mut RidgidBody,
    t2: &mut Transform,
    w2: Vec3, // world position
) {
    let re1 = c1.material.restfullness;
    let re2 = c2.material.restfullness;

    let r = c1.get_radius(t1.scale);
    debug_assert!(r > 0.0);

    let scale = t2.scale * c2.scale;
    debug_assert!(is_finite(&scale));

    let closest_point = get_closest_point(w1, w2, scale, t2.rotation);
    let overlap_distance = r - closest_point.distance(w1);
    debug_assert!(overlap_distance >= 0.0);

    let normal = (w1 - closest_point).normalized();
    let point_of_contact = closest_point;

    pop_coliders(normal * overlap_distance, t1, t2, &rb1, &rb2);
    standard_collision(
        normal,
        rb1,
        rb2,
        point_of_contact - w1,
        point_of_contact - w2,
        re1,
        re2,
    );
}

macro_rules! assert_delta {
    ($x:expr, $y:expr, $d:expr) => {
        if !($x - $y < $d || $y - $x < $d) {
            panic!();
        }
    };
}
fn standard_collision(
    normal: Vec3,
    rb1: &mut RidgidBody,
    rb2: &mut RidgidBody,
    // offset from point of contact
    o1: Vec3,
    o2: Vec3,
    // not used atm, restfullness
    _re1: f32,
    _re2: f32,
) {
    //assert_delta!(normal.magnitude(),1.0, 0.1f32);
    if !is_finite(&normal) {
        return;
    }

    let v1 = rb1.velocity;
    let v2 = rb2.velocity;

    let m1 = rb1.mass;
    let m2 = rb2.mass;

    debug_assert!(m1 > 0.0);
    debug_assert!(m2 > 0.0);

    // proj the velocities on the normal, this way you can move the frame of
    // refrence and think of the two objects are coliding head on
    let real_v1 = proj(normal, v1);
    let real_v2 = proj(normal, v2);

    let bouncy_ness = 0.6;
    let friction = 0.7;
    if rb1.is_static {
        rb2.velocity = (v2 - (1.0 + bouncy_ness) * real_v2) * friction;
        return;
    } else if rb2.is_static {
        rb1.velocity = (v1 - (1.0 + bouncy_ness) * real_v1) * friction;
        return;
    }

    // using a perfectly elastic collision on each axis
    let (new_v1, new_v2) = standard_elastic_collision_3(m1, &real_v1, m2, &real_v2);

    // inital velocity - velocity used to colide "head on" + velocity after coliding "head on"
    rb1.velocity = v1 - real_v1;
    rb2.velocity = v2 - real_v2;

    rb1.add_impulse_at_location(new_v1, o1);
    rb2.add_impulse_at_location(new_v2, o2);
}

/// where normal_distance is the normal pointing at c1 from c2 with the length of the intercetion
pub fn pop_coliders(
    normal_distance: Vec3,
    t1: &mut Transform,
    t2: &mut Transform,
    rb1: &RidgidBody,
    rb2: &RidgidBody,
) {
    //debug_assert!(normal_distance.magnitude_squared() > 0.0); //TODO
    // cant move static coliders
    if rb1.is_static && rb2.is_static {
        return;
    }

    const POP_SIZE: f32 = 1.1;
    let pop = normal_distance * POP_SIZE;
    if rb1.is_static {
        t2.position -= pop;
    } else if rb2.is_static {
        t1.position += pop;
    } else {
        t2.position -= pop * 0.5;
        t1.position += pop * 0.5;
    }
}

pub fn collide_sphere_vs_sphere(
    c1: &SphereColider,
    rb1: &mut RidgidBody,
    t1: &mut Transform,
    w1: Vec3, // world position
    c2: &SphereColider,
    rb2: &mut RidgidBody,
    t2: &mut Transform,
    w2: Vec3, // world position
) {
    let re1 = c1.material.restfullness;
    let re2 = c2.material.restfullness;

    let r1 = c1.get_radius(t1.scale);
    let r2 = c2.get_radius(t2.scale);

    // pop
    let diff = w2 - w1;
    let distance_pop = diff.magnitude() - r1 - r2;
    let normal = diff.normalized();
    pop_coliders(distance_pop * normal, t1, t2, &rb1, &rb2);
    standard_collision(normal, rb1, rb2, -normal * r1, normal * r2, re1, re2);
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
    fn get_radius_dbg(&self, t: &Transform, direction: Vec3) -> f32 {
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

    assert_eq!(box_c.get_radius_dbg(&t, Vec3::new(1.0, 0.0, 0.0)), 1.0);

    assert_eq!(
        box_c.get_radius_dbg(&t, Vec3::new(100.0, 0.0, 0.0)),
        1.0,
        "can not take non normalized input"
    );

    assert_eq!(
        box_c.get_radius_dbg(&t, Vec3::new(1.0, 1.0, 0.0)),
        2.0f32.sqrt()
    );

    let max_radius = 3.0f32.sqrt();
    let min_radius = 1.0f32.sqrt();

    assert_delta!(
        box_c.get_radius_dbg(&t, Vec3::new(1.0, 1.0, 1.0)),
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
            let size = box_c.get_radius_dbg(&t, forward);

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
    fn new(velocity: Vec3, acceleration: Vec3, angular_velocity: Vec3, mass: f32) -> Self {
        Self {
            velocity,
            acceleration,
            mass,
            angular_velocity,
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
) -> bool {
    let mut has_colided = false;
    for c1 in cc1 {
        for c2 in cc2 {
            if is_colliding(c1, t1, c2, t2) {
                //println!("Colliding!");
                has_colided = true;

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
    has_colided
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
        return;
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
        if self.is_static {
            return;
        }

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

fn update(
    is_paused: &mut bool,
    dt: f32,
    transforms: &mut Vec<Transform>,
    phx_objects: &mut Vec<PhysicsObject>,
) {

    let real_dt = dt;
    let phx_length = phx_objects.len();
    for i in 0..phx_length {
        let (phx_first, phx_last) = phx_objects.split_at_mut(i + 1);
        //if phx_first[i].rb.is_static || !phx_first[i].rb.is_active {
        //    continue; // we dont care about non active or static objects
        //}

        let (trans_first, trans_last) = transforms.split_at_mut(i + 1);

        phx_first[i].rb.step(real_dt, &mut trans_first[i]);
        let mut has_colided = false;
        for (transform, phx_obj) in trans_last.iter_mut().zip(phx_last.iter_mut()) {
            if collide(
                &mut phx_first[i].rb,
                &mut trans_first[i],
                &phx_first[i].colliders,
                transform,
                &mut phx_obj.rb,
                &phx_obj.colliders,
            ) {
                has_colided = true;
            }
        }
        if has_colided {
            *is_paused = true;
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
        rotation: Quaternion::identity(),
        scale: Vec3::new(10.0, 1.0, 10.0),
    }];
    let cubes = 15;
    let spheres = 15;
    let mut rng = rand::thread_rng();

    for _ in 0..(cubes + spheres) {
        let scale = rng.gen_range(1.0..1.5);
        instances.push(Transform {
            position: Vec3::new(
                rng.gen_range(-10.0..10.0),
                rng.gen_range(1.0..20.0),
                rng.gen_range(-10.0..10.0),
            ),
            rotation: Quaternion::identity(),
            scale: Vec3::new(scale, scale, scale),
        })
    }
    let physics_material = PhysicsMaterial {
        friction: 1.0,
        restfullness: 1.0,
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

                if allow_camera_update {
                    camera_controller.update_camera(dt, &mut context.renderer.camera);
                }
                if !pause_physics || !can_pause_phx {
                    update(
                        &mut pause_physics,
                        dt,
                        &mut instances,
                        &mut physics_objects,
                    );
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
