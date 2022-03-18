use std::{collections::HashSet, marker::PhantomData, ptr::NonNull};

use crate::{
    component::{ComponentEntryRef, ComponentId, ComponentRegistry},
    Entity,
};

// TODO: make this an actual `std::error::Error`
#[derive(Debug, PartialEq, Eq)]
pub enum QueryError {
    ConcurrentMutableAccess(ComponentId),
}

/// Represents a valid query for components without multiple mutable access to the same type of
/// component.
/// NOTE: there's currently no way of for example having one query for `mut A` on entities with a
/// `B` and another for `mut A` on entities without a `B`, even though that would be safe.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    components: Vec<ComponentQuery>,
}

/// Represents one part of a query.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentQuery {
    pub id: ComponentId,
    pub mutable: bool,
    // TODO: add optional queries: `optional: bool,`
}

impl Query {
    pub fn new(components: Vec<ComponentQuery>) -> Result<Self, QueryError> {
        let mut mutable_acces_to = HashSet::new();
        for c in components.iter().filter(|c| c.mutable) {
            if !mutable_acces_to.insert(c.id) {
                return Err(QueryError::ConcurrentMutableAccess(c.id));
            }
        }
        for c in components.iter().filter(|c| !c.mutable) {
            if mutable_acces_to.contains(&c.id) {
                return Err(QueryError::ConcurrentMutableAccess(c.id));
            }
        }
        Ok(Self { components })
    }

    /// Returns `true` if no part of this query requires mutable access
    pub fn is_immutable(&self) -> bool {
        self.components.iter().all(|c| !c.mutable)
    }

    /// Get a reference to the query's components.
    pub fn components(&self) -> &[ComponentQuery] {
        self.components.as_ref()
    }
}

#[derive(Debug)]
pub struct QueryResponse<'r, 'q> {
    _world_marker: PhantomData<&'r ComponentRegistry>,
    entries: Vec<ComponentEntryRef>,
    query: &'q Query,
    current_res: Vec<NonNull<u8>>,
}

impl<'r, 'q> QueryResponse<'r, 'q> {
    pub fn new(
        _registry: &'r ComponentRegistry,
        query: &'q Query,
        entries: Vec<ComponentEntryRef>,
    ) -> Self {
        debug_assert!(query.components().len() == entries.len());
        let len = entries.len();
        Self {
            _world_marker: PhantomData,
            entries,
            query,
            current_res: vec![NonNull::dangling(); len],
        }
    }

    /// Returns a slice of pointers to the components requsted if `entity` matches the query.
    /// Otherwise `None` is returned. The order of the components are the same as in the query.
    ///
    /// # Safety
    /// All pointers returned are technically mutable **BUT** modifying the pointers to components
    /// not marked as mutable in the query is undefined behaviour.
    /// The pointers must not outlive this `QueryResponse`
    pub unsafe fn try_get(&mut self, entity: Entity) -> Option<&[NonNull<u8>]> {
        for (i, (e, cq)) in self
            .entries
            .iter_mut()
            .zip(self.query.components().iter())
            .enumerate()
        {
            self.current_res[i] = if cq.mutable {
                NonNull::new(
                    e.get_mut()
                        .storage
                        .get_mut_ptr(entity.get_id_unchecked() as usize),
                )?
            } else {
                NonNull::new(e.get().storage.get_ptr(entity.get_id_unchecked() as usize) as *mut _)?
            }
        }
        Some(&self.current_res)
    }

    /// Same as `try_get` but panics if `None` would be returned.
    /// # Safety
    /// See documentation for `try_get`
    pub unsafe fn get(&mut self, entity: Entity) -> &[NonNull<u8>] {
        self.try_get(entity)
            .expect("The given entity does not match the query or has been despawned")
    }
}
