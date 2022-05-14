use common::{Mat3, Quaternion, Vec3};

pub(crate) mod collision;
pub(crate) mod mesh;
pub(crate) mod sat;

use crate::{clamp, macros::debug_assert_finite, PhysicsMaterial};

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

    pub fn inv_inertia_tensor(&self) -> Mat3 {
        // https://www.wolframalpha.com/input?i=inertia+tensor+box
        let d = self.scale * self.scale;

        let x = d.y + d.z;
        let y = d.x + d.z;
        let z = d.x + d.y;

        Mat3::with_diagonal(1.0 / 12.0 * Vec3 { x, y, z })
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
