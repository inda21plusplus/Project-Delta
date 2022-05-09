use super::{Mat3, PhysicsMaterial, Vec3};
pub(crate) mod collision;

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

    pub fn get_radius(&self, scale: Vec3) -> f32 {
        let scale = scale.x; // TODO FIX, this is left to the user to discover
        debug_assert!(self.radius >= 0.0);
        debug_assert!(scale >= 0.0);
        self.radius * scale
    }

    pub(crate) fn inv_inertia_tensor(&self) -> vek::Mat3<f32> {
        Mat3::broadcast_diagonal(((2.0 / 5.0) * self.radius * self.radius).recip())
    }
}
