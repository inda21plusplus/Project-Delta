use crate::{renderer::Transform, physics::{macros::debug_assert_finite, clamp}};
pub(crate) mod collision;
pub(crate) mod sat;
pub(crate) mod mesh;

use super::{Vec3, Quaternion, PhysicsMaterial};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct BoxColider {
    pub local_position: Vec3,
    pub local_rotation: Quaternion,
    pub scale: Vec3,
    pub material: PhysicsMaterial,
}

impl BoxColider {
    pub fn new(scale: Vec3, material: PhysicsMaterial) -> Self {
        Self {
            local_position: Vec3::zero(),
            local_rotation: Quaternion::identity(),
            scale,
            material,
        }
    }


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

        debug_assert_finite!(direction);

        let outside_normal = self.get_side(t, direction);

        debug_assert_finite!(outside_normal);

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

/// get the closest point on a cube to another point
#[must_use]
pub fn get_closest_point(
    other_loc: Vec3,
    cube_loc: Vec3,
    cube_scale: Vec3,
    cube_rotation: Quaternion,
) -> Vec3 {
    debug_assert_finite!(other_loc);
    debug_assert_finite!(cube_loc);
    debug_assert_finite!(cube_scale);

    // this rotates the other so the cube is aligned with the world axis (aka Quaterion Identity)
    let other_loc = cube_rotation.inverse() * (other_loc - cube_loc) + cube_loc;

    let b_min = cube_loc - cube_scale;
    let b_max = cube_loc + cube_scale;

    //https://developer.mozilla.org/en-US/docs/Games/Techniques/3D_collision_detection
    let closest = clamp(other_loc, b_min, b_max);

    // rotate back to world space
    cube_rotation * (closest - cube_loc) + cube_loc
}