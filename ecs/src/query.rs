use std::collections::HashSet;

use crate::{component::ComponentId, World};

// TODO: make this an actual `std::error::Error`
#[derive(Debug, PartialEq, Eq)]
pub enum QueryError {
    ConcurrentMutableAccess(ComponentId),
}

/// Represents a valid query for components without multiple mutable access to the same type of
/// component.
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
}

pub enum MaybeMut<'a, T> {
    Const(&'a T),
    Mut(&'a mut T),
}

pub struct QueryResponse<'c, 'q> {
    entries: Vec<MaybeMut<'c, ComponentEntry>>,
    query: &'q Query,
}

// impl<'c, 'q> QueryResponse<'c, 'q> {
//     pub fn new_const(world: &'w World, query: &'q Query) -> Self {
//         Self {
//             world: MaybeMut::Const(world),
//             query,
//         }
//     }
// }
