use std::collections::HashSet;

use crate::component::ComponentId;

// TODO: make this an actual `std::error::Error`
#[derive(Debug, PartialEq, Eq)]
pub enum QueryError {
    ConcurrentMutableAccess(ComponentId),
}

/// Represents a valid set of queries without multiple mutable access to the same type of component
#[derive(Debug, Clone, PartialEq)]
pub struct QuerySet {
    queries: Vec<Query>,
}

/// Represents a (possibly invalid) query for components.
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    Single {
        id: ComponentId,
        mutable: bool,
        optional: bool,
    },
    Multiple(Vec<Query>),
    // TODO: Add some way of querying for one of many components
}

impl QuerySet {
    pub fn new(queries: Vec<Query>) -> Result<Self, QueryError> {
        let mut mutable_acces_to = HashSet::new();
        for q in &queries {
            Self::validate(&mut mutable_acces_to, q)?;
        }
        Ok(Self { queries })
    }

    pub fn queries(&self) -> &[Query] {
        self.queries.as_ref()
    }

    fn validate(
        mutable_acces_to: &mut HashSet<ComponentId>,
        query: &Query,
    ) -> Result<(), QueryError> {
        match query {
            &Query::Single {
                id, mutable: true, ..
            } if !mutable_acces_to.insert(id) => Err(QueryError::ConcurrentMutableAccess(id)),
            Query::Multiple(qs) => qs
                .iter()
                .try_for_each(|q| Self::validate(mutable_acces_to, q)),
            _ => Ok(()),
        }
    }
}
