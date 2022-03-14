mod component_registry;
mod entity;

pub use component_registry::ComponentRegistry;
pub use entity::{Entities, Entity};

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, mem::size_of};

    use super::*;

    #[test]
    fn component_registry() {
        struct A(u8);
        struct B(&'static str);
        struct C(u16);

        let mut reg = ComponentRegistry::default();
        let a_id = reg.register::<A>();
        let b_id = reg.register::<B>();

        assert_eq!(size_of::<A>(), reg[a_id].layout().size());
        assert_eq!(size_of::<B>(), reg[b_id].layout().size());

        assert_eq!(Some(&reg[a_id]), reg.info::<A>());
        assert_eq!(Some(&reg[b_id]), reg.info::<B>());
        assert_eq!(None, reg.info::<C>());

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
}
