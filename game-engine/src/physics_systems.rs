use common::{Transform, Vec3};
use ecs::{query_iter, query_iter_combs, World};

use physics::{collide, Collider, Gravity, Rigidbody};

use crate::Time;

pub fn update(world: &mut World) {
    let gravity = world
        .resource::<Gravity>()
        .map(|g| g.0)
        .unwrap_or_else(Vec3::zero);
    let dt = world.resource::<Time>().unwrap().dt().as_secs_f32();

    query_iter!(world, (transform: mut Transform, rb: mut Rigidbody, collider: Option<Collider>) => {
        rb.add_force(gravity / rb.mass, dt);

        // simulate one step in the simulation
        rb.step(dt, transform, collider);
    });

    // TODO: this should apply to pairs of entities where at least one of them has a rigidbody, not
    // necessarily both.
    query_iter_combs!(world, ((tr1, tr2): mut Transform, (rb1, rb2): mut Rigidbody, (c1, c2): Collider) => {
        collide(tr2, rb2, c2, tr1, rb1, c1);
    });
}
