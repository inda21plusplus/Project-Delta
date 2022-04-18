use std::{collections::HashSet, marker::PhantomData, ptr::NonNull};

use crate::{
    component::{ComponentEntryRef, ComponentId, ComponentRegistry},
    BorrowMutError, Entity,
};

/// Casts `ptr` to a reference with the lifetime `'a`.
/// # Safety
/// It is the responsibility of the caller to ensure that the lifetime `'a` outlives
/// the lifetime of the data pointed to by `ptr`.
#[allow(clippy::needless_lifetimes)]
pub unsafe fn as_ref_lt<'a, T>(_lifetime: &'a (), ptr: NonNull<T>) -> &'a T {
    ptr.as_ref()
}

/// Casts `ptr` to a mutable reference with the lifetime `'a`.
/// # Safety
/// It is the responsibility of the caller to ensure that the lifetime `'a` outlives
/// the lifetime of the data pointed to by `ptr`.
#[allow(clippy::mut_from_ref, clippy::needless_lifetimes)]
pub unsafe fn as_mut_lt<'a, T>(_lifetime: &'a (), mut ptr: NonNull<T>) -> &'a mut T {
    ptr.as_mut()
}

#[macro_export]
macro_rules! _query_definition {
    ( $world:expr, $vec:expr, ($name:ident: $type:ty, $($tail:tt)*) ) => {{
        $vec.push(ComponentQuery {
            id: $world.component_id::<$type>().unwrap(),
            mutable: false,
        });
        _query_definition!($world, $vec, ($($tail)*));
    }};
    ( $world:expr, $vec:expr, ($name:ident: mut $type:ty, $($tail:tt)*) ) => {{
        $vec.push(ComponentQuery {
            id: $world.component_id::<$type>().unwrap(),
            mutable: true,
        });
        _query_definition!($world, $vec, ($($tail)*));
    }};
    ( $world:expr, $vec:expr, ($name:ident: $type:ty) ) => {{
        $vec.push(ComponentQuery {
            id: $world.component_id::<$type>().unwrap(),
            mutable: false,
        });
    }};
    ( $world:expr, $vec:expr, ($name:ident: mut $type:ty) ) => {{
        $vec.push(ComponentQuery {
            id: $world.component_id::<$type>().unwrap(),
            mutable: true,
        });
    }};
}

#[macro_export]
macro_rules! _query_defvars {
    ( $comps:expr, $lt:expr, ($name:ident: $type:ty, $($tail:tt)*) ) => {
        let $name = unsafe { $crate::query::as_ref_lt($lt, $comps[0].cast::<$type>()) };
        _query_defvars!($comps[1..], $lt, ($($tail)*));
    };
    ( $comps:expr, $lt:expr, ($name:ident: mut $type:ty, $($tail:tt)*) ) => {
        let $name = unsafe { $crate::query::as_mut_lt($lt, $comps[0].cast::<$type>()) };
        _query_defvars!($comps[1..], $lt, ($($tail)*));
    };
    ( $comps:expr, $lt:expr, ($name:ident: $type:ty) ) => {
        let $name = unsafe { $crate::query::as_ref_lt($lt, $comps[0].cast::<$type>()) };
    };
    ( $comps:expr, $lt:expr, ($name:ident: mut $type:ty) ) => {
        let $name = unsafe { $crate::query::as_mut_lt($lt, $comps[0].cast::<$type>()) };
    };
}

#[macro_export]
macro_rules! query_iter {
    ( $world:expr, ($($query:tt)*) => $body:block ) => {{
        let mut v = vec![];
        _query_definition!($world, v, ($($query)*));
        let q = Query::new(v).expect("Query violates rusts borrow rules");

        let mut res = $world.query(&q);

        for comps in unsafe { res.iter() } {
            let lt = ();
            $crate::_query_defvars!(comps, &lt, ($($query)*));
            $body
        }
    }};
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
pub struct QueryResponse<'r, 'q> {
    _world_marker: PhantomData<&'r ComponentRegistry>,
    entries: Vec<ComponentEntryRef>,
    query: &'q Query,
}

impl<'r, 'q> QueryResponse<'r, 'q> {
    pub(crate) fn new(
        _registry: &'r ComponentRegistry,
        query: &'q Query,
        entries: Vec<ComponentEntryRef>,
    ) -> Self {
        debug_assert!(query.components().len() == entries.len());
        Self {
            _world_marker: PhantomData,
            entries,
            query,
        }
    }

    /// Same as `try_get` but panics if `None` would be returned.
    /// # Safety
    /// See documentation for `try_get`
    pub unsafe fn get(&mut self, entity: Entity) -> Vec<NonNull<u8>> {
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
    pub unsafe fn try_get(&mut self, entity: Entity) -> Option<Vec<NonNull<u8>>> {
        self.try_get_by_index(entity.get_id_unchecked())
    }

    unsafe fn try_get_by_index(&mut self, index: u32) -> Option<Vec<NonNull<u8>>> {
        let mut res = Vec::with_capacity(self.entries.len());
        for (e, cq) in self.entries.iter_mut().zip(self.query.components().iter()) {
            res.push(if cq.mutable {
                NonNull::new(e.get_mut().storage.get_mut_ptr(index as usize))?
            } else {
                NonNull::new(e.get().storage.get_ptr(index as usize) as *mut _)?
            });
        }
        Some(res)
    }

    /// Returns the last index of an entity that has at least one component in the query. There
    /// might not actually be a hit for this query at this index, but there is definitly no hits
    /// after this index.
    fn last_index_worth_checking(&self) -> Option<u32> {
        self.entries
            .iter()
            .flat_map(|e| e.get().storage.last_set_index())
            .max()
            .map(|max| max as u32)
    }

    pub unsafe fn iter<'a>(&'a mut self) -> Iter<'a, 'r, 'q> {
        Iter::new(self, self.last_index_worth_checking())
    }
}

pub struct Iter<'a, 'r, 'q> {
    index: u32,
    last: Option<u32>,
    res: &'a mut QueryResponse<'r, 'q>,
}

impl<'a, 'r, 'q> Iter<'a, 'r, 'q> {
    pub fn new(res: &'a mut QueryResponse<'r, 'q>, last: Option<u32>) -> Self {
        Self {
            index: 0,
            last,
            res,
        }
    }
}

// TODO: for sparse components this could be optimized
impl<'a, 'r, 'q> Iterator for Iter<'a, 'r, 'q> {
    type Item = Vec<NonNull<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.last? {
            let index = self.index;
            self.index += 1;
            let res = unsafe { self.res.try_get_by_index(index) };
            if res.is_some() {
                return res;
            }
        }
        None
    }
}
