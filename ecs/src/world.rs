use std::cell::RefCell;
use std::mem;
use std::rc::Rc;

use crate::commands::CommandBuffer;
use crate::component::{ComponentId, ComponentRegistry};
use crate::query::QueryResponse;
use crate::{BorrowMutError, Commands, Entities, Entity, Query};

pub struct ResourceId(ComponentId);

#[derive(Debug)]
pub struct World {
    entities: Entities,
    component_registry: ComponentRegistry,
    // The current storage implementation for components waste very little if the only component
    // that's added to is is the first created entity, so we can simply have one entity that holds
    // all entities. You could see it as a bit of a hack and there are better ways to implement
    // resources, but this is at least very simple. NOTE however that iterating through all
    // entities would also yield this one which is not desirable.
    resource_holder: Entity,
    command_buffers: Vec<Rc<RefCell<CommandBuffer>>>,
}

impl Default for World {
    fn default() -> Self {
        let mut entities = Entities::default();
        let resource_holder = entities.spawn();
        Self {
            entities,
            component_registry: Default::default(),
            resource_holder,
            command_buffers: vec![],
        }
    }
}

impl World {
    /// Retrieves a `Commands` which can be used to issue commands to be run on this `World` when
    /// possible, which is when `maintain` is called.
    /// After `maintain` is called, issuing more commands to the `Commands` will result in a panic.
    pub fn commands(&mut self) -> Commands {
        let (buffer, commands) = Commands::new();

        self.command_buffers.push(buffer);

        commands
    }

    /// Must be called after every frame.
    /// At the moment this runs all deferred commands, but more will be done in the future.
    pub fn maintain(&mut self) {
        // Temporarily take the vec here to we can let the command borrow the world mutably
        let mut buffers = mem::take(&mut self.command_buffers);
        eprintln!("Running {:?}", buffers);
        for command_buffer in buffers.drain(..) {
            for command in command_buffer.borrow_mut().take() {
                command.execute(self);
            }
        }
        // This is just to avoid some allocations
        self.command_buffers = buffers;
    }

    pub fn spawn(&mut self) -> Entity {
        self.entities.spawn()
    }

    pub fn add_resource<T: 'static>(&mut self, resource: T) -> ResourceId {
        let resource_id = self.component_registry.register::<T>();
        self.add(self.resource_holder, resource);
        ResourceId(resource_id)
    }

    pub fn resource<T: 'static>(&self) -> Option<&T> {
        self.get(self.resource_holder)
    }

    pub fn resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.get_mut(self.resource_holder)
    }

    pub fn component_id<T: 'static>(&self) -> Option<ComponentId> {
        self.component_registry.id::<T>()
    }

    /// Adds a component to an entity. If the type is not registered as a component, it gets
    /// registered automatically. Returns `true` if `entity` did not have this kind of component
    /// before and `entity` exists.
    pub fn add<T: 'static>(&mut self, entity: Entity, component: T) -> bool {
        let comp_id = self
            .component_registry
            .id::<T>()
            .unwrap_or_else(|| self.component_registry.register::<T>());

        self.entities
            .id(entity)
            .map(|id| unsafe {
                self.component_registry[comp_id]
                    .storage
                    .set::<T>(id as usize, component)
            })
            .unwrap_or(false)
    }

    /// The component type must already be registered in the component registry.
    /// Returns `true` if `entity` did not have this kind of component
    /// before and `entity` exists.
    /// If this returns `true`, ownership of `component` is transferred to the world and must
    /// therefore not be dropped or used after this call.
    /// If this returns `false`, ownership is kept by the caller and must be dropped or used.
    /// TODO: better documentation
    pub unsafe fn add_raw(
        &mut self,
        entity: Entity,
        component: *mut u8,
        component_id: ComponentId,
    ) -> bool {
        self.entities
            .id(entity)
            .map(|id| {
                self.component_registry[component_id]
                    .storage
                    .set_ptr(id as usize, component)
            })
            .unwrap_or(false)
    }

    /// Removes a component from an entity, returning it or `None` if the entity did not exist or
    /// did not have a component of the specified type.
    pub fn remove<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        let comp_id = self.component_registry.id::<T>()?;

        let id = self.entities.id(entity)?;
        unsafe {
            self.component_registry[comp_id]
                .storage
                .remove::<T>(id as usize)
        }
    }

    /// Returns `true` if the entity existed.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if entity == self.resource_holder {
            // This could happen if someone were to query for a resource (they're currently just
            // components), get the entity, and try to delete it.
            return false;
        }
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
    ) -> Result<QueryResponse<'a, 'q>, BorrowMutError> {
        let mut entries = Vec::with_capacity(query.components().len());
        for c in query.components() {
            match self.component_registry.try_borrow(c.id, c.mutable) {
                Some(entry) => entries.push(entry),
                None => return Err(BorrowMutError::new(c.id)),
            }
        }
        Ok(QueryResponse::new(self, query, entries))
    }

    /// Tries to query for a set of components. If thats not possible (see `try_query`) this
    /// function panics.
    pub fn query<'a, 'q>(&'a self, query: &'q Query) -> QueryResponse<'a, 'q> {
        self.try_query(query).unwrap()
    }

    /// Panics if the component currently is mutably borrowed in a query
    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        let comp_id = self.component_registry.id::<T>()?;

        self.entities
            .id(entity)
            .and_then(|id| unsafe { self.component_registry[comp_id].storage.get(id as usize) })
    }

    /// Panics if the component currently is borrowed in a query
    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        let comp_id = self.component_registry.id::<T>()?;

        self.entities.id(entity).and_then(|id| unsafe {
            self.component_registry[comp_id]
                .storage
                .get_mut(id as usize)
        })
    }

    /// Get a reference to the world's entities.
    pub fn entities(&self) -> &Entities {
        &self.entities
    }

    /// Get a mutable reference to the world's component registry.
    pub fn component_registry_mut(&mut self) -> &mut ComponentRegistry {
        &mut self.component_registry
    }
}
