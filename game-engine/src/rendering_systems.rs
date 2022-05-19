use std::collections::HashMap;

use common::Transform;
use ecs::{query_iter, World};
use rendering::{model::ModelIndex, Light, Renderer, WorldId};

pub fn render(renderer: &mut Renderer, world: &mut World) {
    let world_id = if let Some(id) = world.resource::<WorldId>() {
        *id
    } else {
        return;
    };

    let mut transforms = HashMap::<ModelIndex, Vec<Transform>>::new();
    query_iter!(world, (transform: Transform, model: ModelIndex) => {
        transforms.entry(*model).or_insert_with(Vec::new).push(*transform);
    });

    let mut lights = vec![];
    query_iter!(world, (light: Light, transform: Transform) => {
        lights.push((*light, transform.position));
    });
    renderer.set_lights(world_id, lights);

    renderer.update_instances(world_id, transforms.iter().map(|(&i, ts)| (i, ts.as_ref())));
}
