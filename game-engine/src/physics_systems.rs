use common::{Transform, Vec3};
use ecs::{query_iter, query_iter_combs, World};

use physics::{collide, Collider, Gravity, Rigidbody};

pub fn update(world: &mut World, dt: f32) {
    let gravity = world
        .resource::<Gravity>()
        .map(|g| g.0)
        .unwrap_or_else(Vec3::zero);

    query_iter!(world, (transform: mut Transform, rb: mut Rigidbody, collider: Option<Collider>) => {
        rb.add_force(gravity / rb.mass, dt);

        // simulate one step in the simulation
        rb.step(dt, transform, collider);
    });

    // TODO: this should apply to pairs of entities where at least one of them has a rigidbody, not
    // necessarily both.
    query_iter_combs!(world, ((tr1, tr2): mut Transform, (rb1, rb2): mut Rigidbody, (c1, c2): Collider) => {
        collide(tr1, rb1, c1, tr2, rb2, c2);
    });
}
