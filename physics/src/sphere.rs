use common::{Mat3, Vec3};

pub mod collision;

use crate::PhysicsMaterial;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SphereCollider {
    pub local_position: Vec3,
    pub radius: f32,
    pub material: PhysicsMaterial,
}

impl SphereCollider {
    pub fn new(radius: f32, material: PhysicsMaterial) -> Self {
        Self {
            radius,
            material,
            local_position: Vec3::zero(),
        }
    }

    pub fn get_radius(&self, scale: Vec3) -> f32 {
        // TODO: add support for non-uniformly scaled "spheres"
        let scale = scale.x.abs().max(scale.y.abs()).max(scale.z.abs());

        debug_assert!(self.radius >= 0.0);
        debug_assert!(scale >= 0.0);

        self.radius * scale
    }

    // TODO: pay attention to scale
    pub(crate) fn inv_inertia_tensor(&self) -> Mat3 {
        Mat3::broadcast_diagonal(((2.0 / 5.0) * self.radius * self.radius).recip())
    }
}
