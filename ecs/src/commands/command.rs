use crate::{Entity, World};

#[derive(Debug)]
pub enum Command {
    Spawn, // TODO: add components to spawned entity
    Despawn(Entity),
}

impl Command {
    pub fn execute(self, world: &mut World) {
        match self {
            Command::Spawn => {
                world.spawn();
            }
            Command::Despawn(entity) => {
                world.despawn(entity);
            }
        }
    }
}
