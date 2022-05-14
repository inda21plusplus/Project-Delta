use common::{Mat3, Transform, Vec3};
use ecs::{query_iter, query_iter_combs, World};

use crate::{collision::collide, Collider, Gravity, Rigidbody};

pub fn update(world: &mut World, dt: f32) {
    let gravity = world
        .resource::<Gravity>()
        .map(|g| g.0)
        .unwrap_or_else(Vec3::zero);

    query_iter!(world, (transform: mut Transform, rb: mut Rigidbody, collider: Option<Collider>) => {
        rb.add_force(gravity / rb.mass, dt);

        // NOTE: not 100% sure but `identity` seems to be reasonable
        let tensor = collider
            .map(|c| c.inv_inertia_tensor())
            .unwrap_or_else(Mat3::identity);

        // simulate one step in the simulation
        rb.step(dt, transform, tensor);
    });

    // TODO: this should apply to pairs of entities where at least one of them has a rigidbody, not
    // necessarily both.
    query_iter_combs!(world, ((tr1, tr2): mut Transform, (rb1, rb2): mut Rigidbody, (c1, c2): Collider) => {
        if rb1.is_static && rb2.is_static {
            continue;
        }
        collide(
            tr1,
            rb1,
            c1,
            tr2,
            rb2,
            c2,
        );
    });
}
