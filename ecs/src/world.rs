use crate::component::{ComponentId, ComponentRegistry};
use crate::query::QueryResponse;
use crate::{Entities, Entity, Query, QueryError};
use std::ptr;

#[derive(Debug, Default)]
pub struct World {
    entities: Entities,
    component_registry: ComponentRegistry,
}

impl World {
    pub fn spawn(&mut self) -> Entity {
        self.entities.spawn()
    }

    pub fn register_component<T: 'static>(&mut self) -> ComponentId {
        self.component_registry.register::<T>()
    }

    pub fn component_id<T: 'static>(&self) -> Option<ComponentId> {
        self.component_registry.id::<T>()
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

    /// Tries to query for a set of components. If this tries to borrow access to a component which
    /// has already been handed out (unless every borrow is immutable), a `QueryError` indicating
    /// one (of the possible many) components which was already inaccessible.
    pub fn try_query<'a, 'q>(
        &'a self,
        query: &'q Query,
    ) -> Result<QueryResponse<'a, 'q>, QueryError> {
        let mut entries = Vec::with_capacity(query.components().len());
        for c in query.components() {
            match self.component_registry.try_borrow(c.id, c.mutable) {
                Some(entry) => entries.push(entry),
                None => return Err(QueryError::ConcurrentMutableAccess(c.id)),
            }
        }
        Ok(QueryResponse::new(&self.component_registry, query, entries))
    }

    /// Tries to query for a set of components. If thats not possible (see `try_query`) this
    /// function panics.
    pub fn query<'a, 'q>(&'a self, query: &'q Query) -> QueryResponse<'a, 'q> {
        self.try_query(query).unwrap()
    }

    // NOTE: pub unsafe fn try_query_unsafe<'q>(&self, ...) -> ...QueryResponse<'static, 'q> ...
    // can be added if it's not possible to know at compile time that the response wont outlive the
    // world

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
