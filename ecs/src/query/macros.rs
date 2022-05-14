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
    ( $world:expr, $commands:ident: Commands, ($($query:tt)*) => $body:block ) => {{
        let mut command_buffer = $crate::CommandBuffer::new();
        let mut $commands = $crate::Commands::new(&mut command_buffer, $world.entities());

        query_iter_combs!($world, ($($query)*) => $body);

        command_buffer.apply(&mut $world);
    }};
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
    // entity
    ( $world:expr, $vec:expr, ($name:tt: Entity, $($tail:tt)*) ) => {{
        $crate::_query_definition!($world, $vec, ($($tail)*));
    }};
    // opt
    ( $world:expr, $vec:expr, ($name:tt: Option<$type:ty>, $($tail:tt)*) ) => {{
        $vec.push(ComponentQuery {
            id: $world.component_registry().id::<$type>().unwrap(),
            mutable: false,
            optional: false,
        });
        _query_definition!($world, $vec, ($($tail)*));
    }};
    // opt mut
    ( $world:expr, $vec:expr, ($name:tt: mut Option<$type:ty>, $($tail:tt)*) ) => {{
        $vec.push(ComponentQuery {
            id: $world.component_registry().id::<$type>().unwrap(),
            mutable: true,
            optional: true,
        });
        _query_definition!($world, $vec, ($($tail)*));
    }};
    // comp
    ( $world:expr, $vec:expr, ($name:tt: $type:ty, $($tail:tt)*) ) => {{
        $vec.push($crate::query::ComponentQuery {
            id: $world.component_registry().id::<$type>().expect(&format!(
                    "Tried querying for unregistered type {}",
                    std::any::type_name::<$type>(),
            )),
            mutable: false,
            optional: false,
        });
        $crate::_query_definition!($world, $vec, ($($tail)*));
    }};
    // mut
    ( $world:expr, $vec:expr, ($name:tt: mut $type:ty, $($tail:tt)*) ) => {{
        $vec.push($crate::query::ComponentQuery {
            id: $world.component_registry().id::<$type>().expect(&format!(
                    "Tried querying for unregistered type {}",
                    std::any::type_name::<$type>(),
            )),
            mutable: true,
            optional: false,
        });
        $crate::_query_definition!($world, $vec, ($($tail)*));
    }};

    // Last entry
    ( $world:expr, $vec:expr, ($name:tt: Entity) ) => { };
    // opt
    ( $world:expr, $vec:expr, ($name:tt: Option<$type:ty>) ) => {{
        $vec.push(ComponentQuery {
            id: $world.component_registry().id::<$type>().unwrap(),
            mutable: false,
            optional: true,
        });
    }};
    // mut opt
    ( $world:expr, $vec:expr, ($name:tt: mut Option<$type:ty>) ) => {{
        $vec.push(ComponentQuery {
            id: $world.component_registry().id::<$type>().expect(&format!("{}", std::any::type_name::<$type>())),
            mutable: true,
            optional: true,
        });
    }};
    // comp
    ( $world:expr, $vec:expr, ($name:tt: $type:ty) ) => {{
        $vec.push($crate::query::ComponentQuery {
            id: $world.component_registry().id::<$type>().expect(&format!(
                    "Tried querying for unregistered type {}",
                    std::any::type_name::<$type>(),
            )),
            mutable: false,
            optional: false,
        });
    }};
    // mut
    ( $world:expr, $vec:expr, ($name:tt: mut $type:ty) ) => {{
        $vec.push($crate::query::ComponentQuery {
            id: $world.component_registry().id::<$type>().expect(&format!(
                    "Tried querying for unregistered type {}",
                    std::any::type_name::<$type>(),
            )),
            mutable: true,
            optional: false,
        });
    }};
}

#[macro_export]
macro_rules! _query_defvars {
    // Entity
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: Entity, $($tail:tt)*) ) => {
        let $name = $entity;
        $crate::_query_defvars!($comps[..], $lt, $entity, ($($tail)*));
    };
    // opt
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: Option<$type:ty>, $($tail:tt)*) ) => {
        let $name = unsafe { $crate::query::_as_opt_ref_lt($lt, $comps[0].cast::<$type>()) };
        _query_defvars!($comps[1..], $lt, $entity, ($($tail)*));
    };
    // mut opt
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: mut Option<$type:ty>, $($tail:tt)*) ) => {
        let $name = unsafe { $crate::query::_as_opt_mut_lt($lt, $comps[0].cast::<$type>()) };
        _query_defvars!($comps[1..], $lt, $entity, ($($tail)*));
    };
    // comp
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: $type:ty, $($tail:tt)*) ) => {
        let $name = unsafe { $crate::query::_as_ref_lt($lt, $comps[0].cast::<$type>()) };
        $crate::_query_defvars!($comps[1..], $lt, $entity, ($($tail)*));
    };
    // mut
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: mut $type:ty, $($tail:tt)*) ) => {
        let $name = unsafe { $crate::query::_as_mut_lt($lt, $comps[0].cast::<$type>()) };
        $crate::_query_defvars!($comps[1..], $lt, $entity, ($($tail)*));
    };

    // Last entry
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: Entity) ) => {
        let $name = $entity;
    };
    // opt
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: Option<$type:ty>) ) => {
        let $name = unsafe { $crate::query::_as_opt_ref_lt($lt, $comps[0].cast::<$type>()) };
    };
    // opt mut
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: mut Option<$type:ty>) ) => {
        let $name = unsafe { $crate::query::_as_opt_mut_lt($lt, $comps[0].cast::<$type>()) };
    };
    // comp
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: $type:ty) ) => {
        let $name = unsafe { $crate::query::_as_ref_lt($lt, $comps[0].cast::<$type>()) };
    };
    // mut
    ( $comps:expr, $lt:expr, $entity:expr, ($name:ident: mut $type:ty) ) => {
        let $name = unsafe { $crate::query::_as_mut_lt($lt, $comps[0].cast::<$type>()) };
    };
}

#[macro_export]
macro_rules! _query_defvars_combs {
    // entity
    ( $comps1:expr, $comps2:expr, $lt:expr, $entity:expr, ($name:tt: Entity, $($tail:tt)*) ) => {
        let $name = $entity;
        $crate::_query_defvars_combs!($comps1, $comps2, $lt, $entity, ($($tail)*));
    };
    // opt
    ( $comps1:expr, $comps2:expr, $lt:expr, $entity:expr, ($name:tt: Option<$type:ty>, $($tail:tt)*) ) => {
        let $name = unsafe { (
            $crate::query::_as_opt_ref_lt($lt, $comps1[0].cast::<$type>()),
            $crate::query::_as_opt_ref_lt($lt, $comps2[0].cast::<$type>()),
        ) };
        _query_defvars_combs!($comps1[1..], $comps2[1..], $lt, $entity, ($($tail)*));
    };
    // opt mut
    ( $comps1:expr, $comps2:expr, $lt:expr, $entity:expr, ($name:tt: mut Option<$type:ty>, $($tail:tt)*) ) => {
        let $name = unsafe { (
            $crate::query::_as_opt_mut_lt($lt, $comps1[0].cast::<$type>()),
            $crate::query::_as_opt_mut_lt($lt, $comps2[0].cast::<$type>()),
        ) };
        _query_defvars_combs!($comps[1..], $lt, $entity, ($($tail)*));
    };
    // comp
    ( $comps1:expr, $comps2:expr, $lt:expr, $entity:expr, ($name:tt: $type:ty, $($tail:tt)*) ) => {
        let $name = unsafe { (
            $crate::query::_as_ref_lt($lt, $comps1[0].cast::<$type>()),
            $crate::query::_as_ref_lt($lt, $comps2[0].cast::<$type>()),
        ) };
        $crate::_query_defvars_combs!($comps1[1..], $comps2[1..], $lt, $entity, ($($tail)*));
    };
    // mut
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
    // comp
    ( $comps1:expr, $comps2:expr, $lt:expr, $entity:expr, ($name:tt: $type:ty) ) => {
        let $name = unsafe { (
            $crate::query::_as_ref_lt($lt, $comps1[0].cast::<$type>()),
            $crate::query::_as_ref_lt($lt, $comps2[0].cast::<$type>()),
        ) };
    };
    // mut
    ( $comps1:expr, $comps2:expr, $lt:expr, $entity:expr, ($name:tt: mut $type:ty) ) => {
        let $name = unsafe { (
            $crate::query::_as_mut_lt($lt, $comps1[0].cast::<$type>()),
            $crate::query::_as_mut_lt($lt, $comps2[0].cast::<$type>()),
        ) };
    };
}

#[allow(clippy::needless_lifetimes, clippy::missing_safety_doc)]
pub unsafe fn _as_ref_lt<'a, T>(_lifetime: &'a (), ptr: *const T) -> &'a T {
    &*ptr
}

#[allow(
    clippy::mut_from_ref,
    clippy::needless_lifetimes,
    clippy::missing_safety_doc
)]
pub unsafe fn _as_mut_lt<'a, T>(_lifetime: &'a (), ptr: *mut T) -> &'a mut T {
    &mut *ptr
}

use std::ptr::NonNull;

#[allow(clippy::needless_lifetimes, clippy::missing_safety_doc)]
pub unsafe fn _as_opt_ref_lt<'a, T>(_lifetime: &'a (), ptr: *const T) -> Option<&'a T> {
    NonNull::new(ptr as *mut T).map(|ptr| ptr.as_ref())
}

#[allow(
    clippy::mut_from_ref,
    clippy::needless_lifetimes,
    clippy::missing_safety_doc
)]
pub unsafe fn _as_opt_mut_lt<'a, T>(_lifetime: &'a (), ptr: *mut T) -> Option<&'a mut T> {
    NonNull::new(ptr).map(|mut ptr| ptr.as_mut())
}
