#![deny(warnings)]

pub mod component;
mod entity;
// mod world;

pub use entity::{Entities, Entity};
// pub use world::World;

#[cfg(test)]
mod tests {
    use std::{any, collections::HashSet};

    use crate::component::{ComponentRegistry, Storage, StorageType};

    use super::*;

    #[test]
    fn component_registry() {
        struct A(u8);
        struct B(&'static str);
        struct C(u16);

        let mut reg = ComponentRegistry::default();
        let a_id = reg.register::<A>();
        let b_id = reg.register::<B>();

        assert_eq!(any::type_name::<A>(), reg[a_id].info.name());
        assert_eq!(any::type_name::<B>(), reg[b_id].info.name());

        assert_eq!(Some(&reg[a_id].info), reg.component::<A>().map(|c| &c.info));
        assert_eq!(Some(&reg[b_id].info), reg.component::<B>().map(|c| &c.info));
        assert!(reg.component::<C>().is_none());

        assert_eq!(Some(a_id), reg.id::<A>());
        assert_eq!(Some(b_id), reg.id::<B>());
        assert_eq!(None, reg.id::<C>());
    }

    #[test]
    fn entities() {
        let mut entities = Entities::default();
        let a = entities.spawn();
        assert!(entities.exists(a));
        assert!(entities.delete(a));

        let b = entities.spawn();
        assert!(entities.exists(b));
        assert!(!entities.exists(a));
        assert_ne!(a, b);

        let c = entities.spawn();
        let d = entities.spawn();
        assert!(!entities.exists(a));
        assert!(entities.exists(b));
        assert!(entities.exists(c));
        assert!(entities.exists(d));

        assert!(entities.delete(c));

        assert!(!entities.exists(a));
        assert!(entities.exists(b));
        assert!(!entities.exists(c));
        assert!(entities.exists(d));

        let e = entities.spawn();

        assert!(!entities.exists(a));
        assert!(entities.exists(b));
        assert!(!entities.exists(c));
        assert!(entities.exists(d));
        assert!(entities.exists(e));

        assert_eq!(
            [b, d, e].into_iter().collect::<HashSet<Entity>>(),
            entities.iter().collect::<HashSet<Entity>>()
        );

        let arr = [a, b, c, d, e];
        let iter = arr.iter().enumerate();
        for (i, x) in iter.clone() {
            for (j, y) in iter.clone() {
                assert!((x == y) == (i == j));
            }
        }
    }

    #[test]
    fn vec_storage() {
        use std::{cell::Cell, rc::Rc};

        struct Counter(Rc<Cell<usize>>);
        impl Counter {
            fn new(rc: Rc<Cell<usize>>) -> Self {
                rc.set(rc.get() + 1);
                Self(rc)
            }
        }
        impl Drop for Counter {
            fn drop(&mut self) {
                self.0.set(self.0.get() - 1)
            }
        }
        let counter = Rc::new(Cell::new(0));

        {
            let mut storage = Storage::new::<Counter>(StorageType::VecStorage);
            let mut entities = Entities::default();
            let es: Vec<_> = (0..100).map(|_| entities.spawn()).collect();
            for i in (0..100).step_by(2) {
                assert_eq!(i / 2, counter.get());
                storage.set(es[i], Counter::new(counter.clone()));
            }
            assert_eq!(50, counter.get());
            for &e in &es[0..50] {
                // NOTE: half of these calls should overwrite (and therefore free) the Counters,
                // and the other half should put the counters previously unused memory
                storage.set(e, Counter::new(counter.clone()));
            }
            assert_eq!(75, counter.get());
        }
        assert_eq!(0, counter.get());
    }
}
