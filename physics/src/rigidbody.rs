use common::{Mat3, Transform, Vec3};

use crate::{macros::debug_assert_finite, Collider};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Rigidbody {
    pub angular_momentum: Vec3,
    pub linear_momentum: Vec3,

    //pub center_of_mass_offset: Vec3,
    pub mass: f32,

    // TODO: remove this. A static rigidbody is the same as no rigidbody
    pub is_static: bool,
}

impl Default for Rigidbody {
    fn default() -> Self {
        Rigidbody::new(1.0)
    }
}

impl Rigidbody {
    pub fn new(mass: f32) -> Self {
        Self {
            mass,
            angular_momentum: Vec3::zero(),
            linear_momentum: Vec3::zero(),
            is_static: false,
        }
    }

    pub fn new_static() -> Self {
        Self {
            is_static: true,
            ..Default::default()
        }
    }

    pub fn velocity(&self) -> Vec3 {
        self.linear_momentum / self.mass
    }

    pub fn angular_velocity(&self, inv_tensor_world: Mat3) -> Vec3 {
        inv_tensor_world * self.angular_momentum
    }

    pub fn add_impulse(&mut self, impulse: Vec3) {
        if self.is_static {
            return;
        }

        self.linear_momentum += impulse;
    }

    pub fn add_force(&mut self, force: Vec3, dt: f32) {
        self.add_impulse(force * dt);
    }

    /// Applies this rigidbody's velocities to `transform`. `collider` is used to calculate
    /// the angular inertia.
    // TODO: if we saved the angular velocity we wouldn't need to take the `collider` here.
    pub fn step(&self, dt: f32, transform: &mut Transform, collider: Option<&Collider>) {
        // NOTE: not 100% sure but `identity` seems to be reasonable
        let inv_inertia_tensor = collider
            .map(|c| c.inv_inertia_tensor())
            .unwrap_or_else(Mat3::identity);

        if self.is_static {
            return;
        }

        debug_assert_finite!(self.velocity());
        debug_assert_finite!(transform.position);

        // TODO: https://en.wikipedia.org/wiki/Verlet_integration

        transform.position += self.velocity() * dt;

        // apply rotation
        let i_inv = Mat3::from(transform.rotation)
            * inv_inertia_tensor
            * Mat3::from(transform.rotation).transposed();
        let angular_velocity = self.angular_velocity(i_inv);

        transform.rotation.rotate_x(angular_velocity.x * dt);
        transform.rotation.rotate_y(angular_velocity.y * dt);
        transform.rotation.rotate_z(angular_velocity.z * dt);
    }
}
