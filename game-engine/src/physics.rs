use std::sync::atomic::{AtomicUsize, Ordering};

use crate::physics::macros::debug_assert_finite;

use self::{
    macros::debug_assert_normalized,
    r#box::{collision::raycast_box, BoxColider},
    sphere::{collision::raycast_sphere, SphereColider},
};

pub mod r#box;
pub mod collision;
pub mod sphere;

use common::{Mat3, Quaternion, Ray, Transform, Vec3};

type Tri = [Vec3; 3];

mod macros {
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

    macro_rules! debug_assert_normalized {
        ($normal:expr) => {
            debug_assert!(
                $normal.magnitude() > 0.99 && $normal.magnitude() < 1.01,
                "normal {} is not normalized, n = {} |n| = {}",
                stringify!($normal),
                $normal,
                $normal.magnitude()
            )
        };
    }

    pub(crate) use debug_assert_finite;
    pub(crate) use debug_assert_normalized;
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Collider {
    SphereColider(SphereColider),
    BoxColider(BoxColider),
}

impl Collider {
    pub fn inv_inertia_tensor(&self) -> Mat3 {
        match self {
            Collider::SphereColider(a) => a.inv_inertia_tensor(),
            Collider::BoxColider(a) => a.inv_inertia_tensor(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RayCastHit {
    pub distance: f32,
    /// normalized
    pub normal: Vec3,
}

impl RayCastHit {
    pub fn new(distance: f32, normal: Vec3) -> Self {
        Self { distance, normal }
    }
}

static BODY_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RidgidBody {
    pub id: usize,
    pub acceleration: Vec3, // can be used for gravity

    pub angular_momentum: Vec3,
    pub linear_momentum: Vec3,

    //pub center_of_mass_offset: Vec3,
    pub mass: f32,

    pub is_static: bool,

    pub is_colliding: bool, // not used atm
    pub is_colliding_this_frame: bool,
}

impl Default for RidgidBody {
    fn default() -> Self {
        let v = Vec3::default();
        RidgidBody::new(v, 1.0)
    }
}

impl RidgidBody {
    pub fn new(acceleration: Vec3, mass: f32) -> Self {
        Self {
            id: BODY_ID.fetch_add(1, Ordering::SeqCst),
            acceleration,
            mass,
            angular_momentum: Vec3::zero(),
            linear_momentum: Vec3::zero(),
            is_static: false,
            is_colliding: false,
            is_colliding_this_frame: false,
        }
    }

    pub fn velocity(&self) -> Vec3 {
        self.linear_momentum / self.mass
    }

    pub fn angular_velocity(&self, inv_tensor_world: Mat3) -> Vec3 {
        inv_tensor_world * self.angular_momentum
    }
}

#[inline]
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

/// returns the overlap between [a_min,a_max] and [b_min,b_max], will return a negative value if range is inverted, overlap(a,b) = -overlap(b,a)
fn overlap(a_min: f32, a_max: f32, b_min: f32, b_max: f32) -> f32 {
    debug_assert!(a_min <= a_max, "a min <= max");
    debug_assert!(b_min <= b_max, "b min <= max");

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
    pub friction: f32,
    pub restfullness: f32, // bounciness
}

pub fn raycast(t: &Transform, cols: &Vec<Collider>, ray: Ray) -> Option<RayCastHit> {
    debug_assert_normalized!(ray.direction);
    debug_assert_finite!(ray.origin);

    let mut distance = f32::INFINITY;
    let mut normal = Vec3::zero();

    for c in cols {
        let w = get_position(t, c);
        if let Some(hit) = raycast_collider(t, c, Ray::new(ray.origin - w, ray.direction)) {
            if hit.distance < distance {
                distance = hit.distance;
                normal = hit.normal;
                debug_assert_normalized!(hit.normal);
            }
        }
    }

    if distance < f32::INFINITY {
        Some(RayCastHit::new(distance, normal))
    } else {
        None
    }
}

/// rotation, collider, ray -> distance, normal
pub fn raycast_collider(t: &Transform, c: &Collider, ray: Ray) -> Option<RayCastHit> {
    match c {
        Collider::SphereColider(s) => raycast_sphere(t, s, ray),
        Collider::BoxColider(b) => raycast_box(t, b, ray),
    }
}
