use std::sync::atomic::{AtomicUsize, Ordering};

use crate::physics::macros::debug_assert_finite;

use self::{r#box::BoxColider, sphere::SphereColider};

pub mod r#box;
pub mod collision;
pub mod sphere;

use common::{Mat3, Quaternion, Ray, Transform, Vec3};

type Tri = [Vec3; 3];

mod macros {
    macro_rules! assert_delta {
        ($x:expr, $y:expr, $d:expr) => {
            if !($x - $y < $d || $y - $x < $d) {
                panic!();
            }
        };
    }

    #[cfg(debug_assertions)]
    macro_rules! pause {
        () => {
            pause_and_wait_for_input()
        };
    }

    macro_rules! debug_assert_finite {
        ($vec:expr) => {
            debug_assert!(
                $vec.x.is_finite() && $vec.y.is_finite() && $vec.z.is_finite(),
                "{} = {}",
                stringify!($vec),
                $vec
            )
        };
    }

    #[cfg(not(debug_assertions))]
    macro_rules! pause {
        () => {};
    }

    #[must_use]
    macro_rules! squared {
        ($x:expr) => {
            $x * $x
        };
    }

    pub(crate) use assert_delta;
    pub(crate) use debug_assert_finite;
    pub(crate) use pause;
    pub(crate) use squared;
}

#[allow(dead_code)]
fn pause_and_wait_for_input() {
    let mut stdout = std::io::stdout();
    std::io::Write::write(&mut stdout, b"Paused\n").unwrap();
    std::io::Write::flush(&mut stdout).unwrap();
    std::io::Read::read(&mut std::io::stdin(), &mut [0]).unwrap();
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Collider {
    SphereColider(SphereColider),
    BoxColider(BoxColider),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RayCastHit {
    pub hit: Vec3,    // world position
    pub normal: Vec3, // normalized
}

static BODY_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RidgidBody {
    pub id: usize,
    pub last_frame_location: Vec3, // used for lerp location
    //pub velocity: Vec3,
    pub acceleration: Vec3, // can be used for gravity

    pub angular_momentum: Vec3,
    pub linear_momentum: Vec3,

    //pub angular_velocity: Vec3, // Spin angular velocity in rad per seconds around that axis (Quaternion::rotate_3d)
    //pub torque: Vec3,           // torque to angular_velocity is what acceleration is to velocity
    pub center_of_mass_offset: Vec3, // also used for instant center of rotation https://en.wikipedia.org/wiki/Instant_centre_of_rotation
    pub is_active_time: f32,
    pub mass: f32,
    pub is_using_global_gravity: bool,
    //is_trigger : bool,
    pub is_static: bool,
    pub is_active: bool, // TODO after object is not moving then it becomes disabled to oprimize
    pub is_colliding: bool,
    pub is_colliding_this_frame: bool,
    pub drag: f32,
}

impl Default for RidgidBody {
    fn default() -> Self {
        let v = Vec3::default();
        RidgidBody::new(v, v, v, 1.0)
    }
}

impl RidgidBody {
    pub fn new(velocity: Vec3, acceleration: Vec3, angular_velocity: Vec3, mass: f32) -> Self {
        Self {
            id: BODY_ID.fetch_add(1, Ordering::SeqCst),
            //velocity,
            acceleration,
            mass,
            angular_momentum: Vec3::zero(),
            linear_momentum: Vec3::zero(),
            //angular_velocity,
            is_active: true,
            is_using_global_gravity: false,
            is_active_time: 0.0f32,
            center_of_mass_offset: Vec3::zero(),
            is_static: false,
            //torque: Vec3::zero(),
            last_frame_location: Vec3::zero(),
            is_colliding: false,
            is_colliding_this_frame: false,
            drag: 0.5, // TODO make public
        }
    }

    pub fn velocity(&self) -> Vec3 {
        self.linear_momentum * self.mass.recip()
    }

    pub fn angular_velocity(&self, inv_tensor_world: Mat3) -> Vec3 {
        inv_tensor_world * self.angular_momentum
    }
}

#[inline]
#[must_use]
fn proj(on: Vec3, vec: Vec3) -> Vec3 {
    vec.dot(on) * on / on.magnitude_squared()
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

/// returns the overlap between [a_min,a_max] and [b_min,b_max], will return a negative value if range is inverted, overlap(a,b) = -overlap(b,a)
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

/// clamps a vector between 2 others vectors
#[must_use]
fn clamp(v: Vec3, min: Vec3, max: Vec3) -> Vec3 {
    debug_assert_finite!(min);
    debug_assert_finite!(max);

    let mut ret = Vec3::zero();
    for i in 0..3 {
        let min = min[i];
        let max = max[i];

        ret[i] = f32::clamp(v[i], min, max)
    }
    ret
}

/// returns the world position
fn get_world_position(pos: Vec3, scale: Vec3, rotation: Quaternion, local_position: Vec3) -> Vec3 {
    pos + rotation * local_position * scale
}

/// returns the world position of a collider given transform and colider
#[must_use]
fn get_position(transform: &Transform, collider: &Collider) -> Vec3 {
    get_world_position(
        transform.position,
        transform.scale,
        transform.rotation,
        match collider {
            Collider::SphereColider(c) => c.local_position,
            Collider::BoxColider(c) => c.local_position,
        },
    )
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct PhysicsMaterial {
    //static_friction: f32,
    pub friction: f32,
    pub restfullness: f32, // bounciness
}
impl Collider {
    pub fn inv_inertia_tensor(&self) -> Mat3 {
        match self {
            Collider::SphereColider(a) => a.inv_inertia_tensor(),
            Collider::BoxColider(a) => a.inv_inertia_tensor(),
        }
    }
}
