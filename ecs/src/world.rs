use crate::component::{ComponentId, ComponentRegistry};
use crate::{Entities, Entity};
use std::ptr;

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

    pub fn add<T: 'static>(&mut self, entity: Entity, component: T) {
        let comp_id = self
            .component_registry
            .id::<T>()
            .unwrap_or_else(|| self.component_registry.register::<T>());

        self.entities.id(entity).map(|id| unsafe {
            self.component_registry[comp_id]
                .storage
                .set::<T>(id as usize, component)
        });
    }

    pub fn remove<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        let comp_id = self.component_registry.id::<T>()?;

        let id = self.entities.id(entity)?;
        unsafe {
            self.component_registry[comp_id]
                .storage
                .remove::<T>(id as usize)
        }
    }

    /// Returns true if the entity existed.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        self.entities
            .id(entity)
            .map(|id| {
                self.entities.despawn_unchecked(id);
                for component in self.component_registry.entries_mut() {
                    component.storage.unset(id as usize);
                }
            })
            .is_some()
    }

    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        let comp_id = self.component_registry.id::<T>()?;

        self.entities
            .id(entity)
            .and_then(|id| unsafe { self.component_registry[comp_id].storage.get(id as usize) })
    }

    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        let comp_id = self.component_registry.id::<T>()?;

        self.entities.id(entity).and_then(|id| unsafe {
            self.component_registry[comp_id]
                .storage
                .get_mut(id as usize)
        })
    }

    /// Returns null if `entity` no longer exists or if `entity` does not have the requested
    /// component. `World` keeps ownership of the component
    pub fn get_ptr(&self, entity: Entity, comp_id: ComponentId) -> *const u8 {
        self.entities
            .id(entity)
            .map(|id| {
                self.component_registry[comp_id]
                    .storage
                    .get_ptr(id as usize)
            })
            .unwrap_or(ptr::null())
    }

    /// Returns null if `entity` no longer exists or if `entity` does not have the requested
    /// component. `World` keeps ownership of the component
    pub fn get_mut_ptr(&mut self, entity: Entity, comp_id: ComponentId) -> *mut u8 {
        self.entities
            .id(entity)
            .map(|id| {
                self.component_registry[comp_id]
                    .storage
                    .get_mut_ptr(id as usize)
            })
            .unwrap_or(ptr::null_mut())
    }
}
