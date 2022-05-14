use std::ptr::NonNull;

#[macro_export]
macro_rules! query_iter {
    ( $world:expr, $commands:ident: Commands, ($($query:tt)*) => $body:block ) => {{
        let mut command_buffer = $crate::CommandBuffer::new();
        let mut $commands = $crate::Commands::new(&mut command_buffer, $world.entities());

        $crate::query_iter!($world, ($($query)*) => $body);

        command_buffer.apply(&mut $world);
    }};
    ( $world:expr, ($($query:tt)*) => $body:block ) => {{
        #[allow(unused_mut)]
        let mut v = vec![];
        $crate::_query_definition!($world, v, ($($query)*));
        let q = $crate::query::Query::new(v).expect("Query violates rusts borrow rules");

        let mut res = $world.query(&q);

        #[allow(unused_variables)]
        for (e, comps) in unsafe { res.iter() } {
            let lt = ();
            $crate::_query_defvars!(comps, &lt, e, ($($query)*));
            $body
        }
    }};
}

#[macro_export]
macro_rules! query_iter_combs {
    ( $world:expr, ($($query:tt)*) => $body:block ) => {{
        #[allow(unused_mut)]
        let mut v = vec![];
        $crate::_query_definition!($world, v, ($($query)*));
        let q = $crate::query::Query::new(v).expect("Query violates rusts borrow rules");

        let mut res = $world.query(&q);

        #[allow(unused_variables)]
        for ((e1, comps1), (e2, comps2)) in unsafe { res.iter_combinations() } {
            let lt = ();
            $crate::_query_defvars_combs!(comps1, comps2, &lt, (e1, e2), ($($query)*));
            $body
        }
    }};
}

#[macro_export]
macro_rules! _query_definition {
    ( $world:expr, $vec:expr, ($name:tt: Entity, $($tail:tt)*) ) => {{
        $crate::_query_definition!($world, $vec, ($($tail)*));
    }};
    ( $world:expr, $vec:expr, ($name:tt: $type:ty, $($tail:tt)*) ) => {{
        $vec.push($crate::query::ComponentQuery {
            id: $world.component_registry().id::<$type>().expect(&format!(
                    "Tried querying for unregistered type {}",
                    std::any::type_name::<$type>(),
            )),
            mutable: false,
        });
        $crate::_query_definition!($world, $vec, ($($tail)*));
    }};
    ( $world:expr, $vec:expr, ($name:tt: mut $type:ty, $($tail:tt)*) ) => {{
        $vec.push($crate::query::ComponentQuery {
            id: $world.component_registry().id::<$type>().expect(&format!(
                    "Tried querying for unregistered type {}",
                    std::any::type_name::<$type>(),
            )),
            mutable: true,
        });
        $crate::_query_definition!($world, $vec, ($($tail)*));
    }};

    // Last entry
    ( $world:expr, $vec:expr, ($name:tt: Entity) ) => { };
    ( $world:expr, $vec:expr, ($name:tt: $type:ty) ) => {{
        $vec.push($crate::query::ComponentQuery {
            id: $world.component_registry().id::<$type>().expect(&format!(
                    "Tried querying for unregistered type {}",
                    std::any::type_name::<$type>(),
            )),
            mutable: false,
        });
    }};
    ( $world:expr, $vec:expr, ($name:tt: mut $type:ty) ) => {{
        $vec.push($crate::query::ComponentQuery {
            id: $world.component_registry().id::<$type>().expect(&format!(
                    "Tried querying for unregistered type {}",
                    std::any::type_name::<$type>(),
            )),
            mutable: true,
        });
    }};
}

#[macro_export]
macro_rules! _query_defvars {
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: Entity, $($tail:tt)*) ) => {
        let $name = $entity;
        $crate::_query_defvars!($comps[..], $lt, $entity, ($($tail)*));
    };
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: $type:ty, $($tail:tt)*) ) => {
        let $name = unsafe { $crate::query::_as_ref_lt($lt, $comps[0].cast::<$type>()) };
        $crate::_query_defvars!($comps[1..], $lt, $entity, ($($tail)*));
    };
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: mut $type:ty, $($tail:tt)*) ) => {
        let $name = unsafe { $crate::query::_as_mut_lt($lt, $comps[0].cast::<$type>()) };
        $crate::_query_defvars!($comps[1..], $lt, $entity, ($($tail)*));
    };

    // Last entry
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: Entity) ) => {
        let $name = $entity;
    };
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: $type:ty) ) => {
        let $name = unsafe { $crate::query::_as_ref_lt($lt, $comps[0].cast::<$type>()) };
    };
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: mut $type:ty) ) => {
        let $name = unsafe { $crate::query::_as_mut_lt($lt, $comps[0].cast::<$type>()) };
    };
}

#[macro_export]
macro_rules! _query_defvars_combs {
    ( $comps1:expr, $comps2:expr, $lt:expr, $entity:expr, ($name:tt: Entity, $($tail:tt)*) ) => {
        let $name = $entity;
        $crate::_query_defvars_combs!($comps1, $comps2, $lt, $entity, ($($tail)*));
    };
    ( $comps1:expr, $comps2:expr, $lt:expr, $entity:expr, ($name:tt: $type:ty, $($tail:tt)*) ) => {
        let $name = unsafe { (
            $crate::query::_as_ref_lt($lt, $comps1[0].cast::<$type>()),
            $crate::query::_as_ref_lt($lt, $comps2[0].cast::<$type>()),
        ) };
        $crate::_query_defvars_combs!($comps1[1..], $comps2[1..], $lt, $entity, ($($tail)*));
    };
    ( $comps1:expr, $comps2:expr, $lt:expr, $entity:expr, ($name:tt: mut $type:ty, $($tail:tt)*) ) => {
        let $name = unsafe { (
            $crate::query::_as_mut_lt($lt, $comps1[0].cast::<$type>()),
            $crate::query::_as_mut_lt($lt, $comps2[0].cast::<$type>()),
        ) };
        $crate::_query_defvars_combs!($comps1[1..], $comps2[1..], $lt, $entity, ($($tail)*));
    };

    // Last entry
    ( $comps1:expr, $comps2:expr, $lt:expr, $entity:expr, ($name:tt: Entity) ) => {
        let $name = $entity;
    };
    ( $comps1:expr, $comps2:expr, $lt:expr, $entity:expr, ($name:tt: $type:ty) ) => {
        let $name = unsafe { (
            $crate::query::_as_ref_lt($lt, $comps1[0].cast::<$type>()),
            $crate::query::_as_ref_lt($lt, $comps2[0].cast::<$type>()),
        ) };
    };
    ( $comps1:expr, $comps2:expr, $lt:expr, $entity:expr, ($name:tt: mut $type:ty) ) => {
        let $name = unsafe { (
            $crate::query::_as_mut_lt($lt, $comps1[0].cast::<$type>()),
            $crate::query::_as_mut_lt($lt, $comps2[0].cast::<$type>()),
        ) };
    };
}

/// Casts `ptr` to a reference with the lifetime `'a`.
/// # Safety
/// It is the responsibility of the caller to ensure that the lifetime `'a` outlives
/// the lifetime of the data pointed to by `ptr`.
/// # Note
/// This is used in macros exported by the crate.
#[allow(clippy::needless_lifetimes)]
pub unsafe fn _as_ref_lt<'a, T>(_lifetime: &'a (), ptr: NonNull<T>) -> &'a T {
    ptr.as_ref()
}

/// Casts `ptr` to a mutable reference with the lifetime `'a`.
/// # Safety
/// It is the responsibility of the caller to ensure that the lifetime `'a` outlives
/// the lifetime of the data pointed to by `ptr`.
/// # Note
/// This is used in macros exported by the crate.
#[allow(clippy::mut_from_ref, clippy::needless_lifetimes)]
pub unsafe fn _as_mut_lt<'a, T>(_lifetime: &'a (), mut ptr: NonNull<T>) -> &'a mut T {
    ptr.as_mut()
}
