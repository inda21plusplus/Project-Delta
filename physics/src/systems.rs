use common::{Transform, Vec3};
use ecs::{query_iter, query_iter_combs, World};

use crate::{collision::collide, Collider, Rigidbody};

pub fn update(world: &mut World, dt: f32) {
    // TODO: this should also apply to entities without a collider, but we need to somehow come up
    // with a default inertia tensor.
    query_iter!(world, (transform: mut Transform, rb: mut Rigidbody, collider: Collider) => {
        rb.is_colliding_this_frame = false;

        // this needs to be changed somehow if we want multible colliders on a dynamic object
        let tensor = collider.inv_inertia_tensor();

        // TODO: customizable gravity
        rb.add_force(Vec3::new(0., -9.82, 0.) / rb.mass, dt);

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
            &vec![*c1],
            tr2,
            rb2,
            &vec![*c2],
        );
    });

    query_iter!(world, (rb: mut Rigidbody) => {
        rb.is_colliding = rb.is_colliding_this_frame;
    });
}
