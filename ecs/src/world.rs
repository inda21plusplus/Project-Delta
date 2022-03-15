use crate::component::ComponentRegistry;
use crate::{Entities, Entity};

pub struct EntityBuilder<'w> {
    entity: Entity,
    world: &'w mut World,
}

impl<'w> EntityBuilder<'w> {
    pub fn add<T: 'static>(self, component: T) -> Self {
        self.world.add(self.entity, component);
        self
    }
    pub fn entity(self) -> Entity {
        self.entity
    }
}

#[derive(Debug, Default)]
pub struct World {
    entities: Entities,
    component_registry: ComponentRegistry,
}

impl World {
    pub fn spawn(&mut self) -> EntityBuilder {
        EntityBuilder {
            entity: self.entities.spawn(),
            world: self,
        }
    }

    /// Returns true if the entity already had a component of type T.
    pub fn add<T: 'static>(&mut self, entity: Entity, component: T) -> bool {
        let component_id = self
            .component_registry
            .id::<T>()
            .unwrap_or_else(|| self.component_registry.register::<T>());

        self.component_registry[component_id]
            .storage
            .set(entity, component)
    }

    /// Returns true if the entity existed.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if self.entities.despawn(entity) {
            for component in self.component_registry.entries_mut() {
                component.storage.remove(entity);
            }
            true
        } else {
            false
        }
    }

    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        let id = self.component_registry.id::<T>()?;

        self.component_registry[id].storage.get_mut(entity)
    }

    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        let id = self.component_registry.id::<T>()?;

        self.component_registry[id].storage.get(entity)
    }
}
