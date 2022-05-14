use collision::collide;
use common::{Quaternion, Transform, Vec3};

use macros::debug_assert_finite;

mod r#box;
mod collision;
mod raycast;
mod rigidbody;
mod sphere;

pub use collision::Collider;
pub use r#box::BoxCollider;
pub use raycast::RayCastHit;
pub use rigidbody::Rigidbody;
pub use sphere::SphereCollider;

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

pub fn update(dt: f32, transforms: &mut Vec<Transform>, phx_objects: &mut Vec<PhysicsObject>) {
    let phx_length = phx_objects.len();

    for i in 0..phx_length {
        phx_objects[i].rb.is_colliding_this_frame = false;

        // this needs to be changed somehow if we want multible colliders on a dynamic object
        let tensor = phx_objects[i].colliders[0].inv_inertia_tensor();

        // simulate one step in the simulation
        phx_objects[i].rb.step(dt, &mut transforms[i], tensor);
    }
    for i in 0..phx_length {
        let (phx_first, phx_last) = phx_objects.split_at_mut(i + 1);

        let (trans_first, trans_last) = transforms.split_at_mut(i + 1);

        // pop colliders and apply force on all colliding objects
        for (transform, phx_obj) in trans_last.iter_mut().zip(phx_last.iter_mut()) {
            // simply dont care about collison if both are static
            if phx_first[i].rb.is_static && phx_obj.rb.is_static {
                continue;
            }
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
    for i in 0..phx_length {
        phx_objects[i].rb.is_colliding = phx_objects[i].rb.is_colliding_this_frame;
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PhysicsObject {
    pub rb: Rigidbody,
    pub colliders: Vec<Collider>,
}

impl PhysicsObject {
    pub fn new(rb: Rigidbody, collider: Collider) -> Self {
        Self {
            rb,
            colliders: vec![collider],
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

/// returns the world position of a collider given transform and collider
fn get_position(transform: &Transform, collider: &Collider) -> Vec3 {
    get_world_position(
        transform.position,
        transform.scale,
        transform.rotation,
        match collider {
            Collider::Sphere(c) => c.local_position,
            Collider::Box(c) => c.local_position,
        },
    )
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct PhysicsMaterial {
    pub friction: f32,
    pub restfullness: f32, // bounciness
}
