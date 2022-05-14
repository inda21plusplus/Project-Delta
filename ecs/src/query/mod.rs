use std::collections::HashSet;

use crate::{
    component::{ComponentEntryRef, ComponentId},
    entity::{Iter as EntityIter, IterCombinations as EntityIterCombinations},
    BorrowMutError, Entity, World,
};

pub mod macros;

pub use self::macros::*;

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
    pub optional: bool,
}

impl Query {
    pub fn new(components: Vec<ComponentQuery>) -> Result<Self, BorrowMutError> {
        let mut mutable_acces_to = HashSet::new();
        for c in components.iter().filter(|c| c.mutable) {
            if !mutable_acces_to.insert(c.id) {
                return Err(BorrowMutError::new(c.id));
            }
        }
        for c in components.iter().filter(|c| !c.mutable) {
            if mutable_acces_to.contains(&c.id) {
                return Err(BorrowMutError::new(c.id));
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
pub struct QueryResponse<'w, 'q> {
    world: &'w World,
    entries: Vec<ComponentEntryRef>,
    query: &'q Query,
}

impl<'w, 'q> QueryResponse<'w, 'q> {
    pub(crate) fn new(world: &'w World, query: &'q Query, entries: Vec<ComponentEntryRef>) -> Self {
        debug_assert!(query.components().len() == entries.len());
        Self {
            world,
            entries,
            query,
        }
    }

    /// Same as `try_get` but panics if `None` would be returned.
    /// # Safety
    /// See documentation for `try_get`
    pub unsafe fn get(&mut self, entity: Entity) -> Vec<*mut u8> {
        self.try_get(entity)
            .expect("The given entity does not match the query or has been despawned")
    }

    /// Returns a slice of pointers to the components requsted if `entity` matches the query.
    /// Otherwise `None` is returned. The order of the components are the same as in the query.
    ///
    /// # Safety
    /// All pointers returned are technically mutable **BUT** modifying the pointers to components
    /// not marked as mutable in the query is undefined behaviour.
    /// The pointers must not outlive this `QueryResponse`
    pub unsafe fn try_get(&mut self, entity: Entity) -> Option<Vec<*mut u8>> {
        self.world
            .entities()
            .id(entity)
            .and_then(|index| self.try_get_by_index(index))
    }

    unsafe fn try_get_by_index(&mut self, index: u32) -> Option<Vec<*mut u8>> {
        let mut res = Vec::with_capacity(self.entries.len());
        for (e, cq) in self.entries.iter().zip(self.query.components().iter()) {
            let ptr = e.get().storage.get_ptr(index as usize) as *mut u8;
            if ptr.is_null() && !cq.optional {
                return None;
            }
            res.push(ptr);
        }
        Some(res)
    }

    pub unsafe fn iter<'a>(&'a mut self) -> Iter<'a, 'w, 'q> {
        Iter::new(self)
    }

    pub unsafe fn iter_combinations<'a>(&'a mut self) -> IterCombinations<'a, 'w, 'q> {
        IterCombinations::new(self)
    }
}

pub struct Iter<'a, 'w, 'q> {
    res: &'a mut QueryResponse<'w, 'q>,
    entity_iter: EntityIter<'w>,
}

impl<'a, 'w, 'q> Iter<'a, 'w, 'q> {
    pub fn new(res: &'a mut QueryResponse<'w, 'q>) -> Self {
        let entity_iter = res.world.entities().iter();
        Self { res, entity_iter }
    }
}

// TODO: for sparse components this could be optimized
impl<'a, 'r, 'q> Iterator for Iter<'a, 'r, 'q> {
    type Item = (Entity, Vec<*mut u8>);

    fn next(&mut self) -> Option<Self::Item> {
        self.entity_iter.next().and_then(|e| unsafe {
            self.res
                .try_get_by_index(self.entity_iter.entities().id(e).unwrap())
                .map(|comps| (e, comps))
        })
    }
}

pub struct IterCombinations<'a, 'w, 'q> {
    res: &'a mut QueryResponse<'w, 'q>,
    entity_iter: EntityIterCombinations<'w>,
}

impl<'a, 'w, 'q> IterCombinations<'a, 'w, 'q> {
    pub fn new(res: &'a mut QueryResponse<'w, 'q>) -> Self {
        let entity_iter = res.world.entities().iter_combinations();
        Self { res, entity_iter }
    }
}

// TODO: for sparse components this could be optimized
impl<'a, 'r, 'q> Iterator for IterCombinations<'a, 'r, 'q> {
    type Item = ((Entity, Vec<*mut u8>), (Entity, Vec<*mut u8>));

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we know that `EntityIterCombinations` won't return the same entity in both
        // values of the tuple, so we know that the components are safe to access.
        loop {
            let (e1, e2) = self.entity_iter.next()?;
            if let (Some(comps1), Some(comps2)) = unsafe {
                (
                    self.res
                        .try_get_by_index(self.entity_iter.entities().id(e1).unwrap()),
                    self.res
                        .try_get_by_index(self.entity_iter.entities().id(e2).unwrap()),
                )
            } {
                return Some(((e1, comps1), (e2, comps2)));
            }
        }
    }
}
