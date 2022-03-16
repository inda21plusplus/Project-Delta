#![deny(warnings)]

pub mod component;
mod entity;
mod query;
mod world;

pub use entity::{Entities, Entity};
pub use query::{Query, QueryError, QuerySet};
pub use world::World;

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
        assert!(entities.despawn(a));

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

        assert!(entities.despawn(c));

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
                unsafe {
                    storage.set(
                        entities.id(es[i]).unwrap() as usize,
                        Counter::new(counter.clone()),
                    );
                }
            }
            assert_eq!(50, counter.get());
            for &e in &es[..50] {
                let index = entities.id(e).unwrap() as usize;
                unsafe {
                    let existed = storage.set(index, Counter::new(counter.clone()));
                    if index % 2 == 0 {
                        assert!(existed);
                    } else {
                        assert!(!existed);
                    }
                }
            }
            assert_eq!(75, counter.get());
            for &e in &es[50..] {
                let index = entities.id(e).unwrap() as usize;
                unsafe {
                    let c: Option<Counter> = storage.remove(index);
                    if index % 2 == 0 {
                        assert!(c.is_some());
                    } else {
                        assert!(c.is_none());
                    }
                }
            }
            assert_eq!(50, counter.get());
        }
        assert_eq!(0, counter.get());
    }

    #[test]
    fn world() {
        let mut world = World::default();

        #[derive(Debug, PartialEq)]
        struct Position {
            x: f32,
            y: f32,
            z: f32,
        }
        struct Health(u8);
        #[derive(Debug, PartialEq)]
        enum Rarity {
            Common,
            Rare,
        }

        let player = world.spawn();
        world.add(
            player,
            Position {
                x: 0.,
                y: 0.,
                z: 0.,
            },
        );
        world.add(player, Health(100));

        assert_eq!(
            Some(&Position {
                x: 0.,
                y: 0.,
                z: 0.
            }),
            world.get::<Position>(player)
        );

        assert!(world.get_mut::<Rarity>(player).is_none());

        let common_sword = world.spawn();
        world.add(
            common_sword,
            Position {
                x: 1.,
                y: 0.,
                z: 1.,
            },
        );
        world.add(common_sword, Rarity::Common);

        assert!(world.get_mut::<Rarity>(player).is_none());

        let rare_sword = world.spawn();
        world.add(
            rare_sword,
            Position {
                x: 1.,
                y: 1.,
                z: 1.,
            },
        );
        world.add(rare_sword, Rarity::Rare);

        assert!(world.get_mut::<Rarity>(player).is_none());

        assert_eq!(Some(&Rarity::Common), world.get::<Rarity>(common_sword));
        assert_eq!(Some(&Rarity::Rare), world.get::<Rarity>(rare_sword));

        assert_eq!(
            Some(&Position {
                x: 0.,
                y: 0.,
                z: 0.
            }),
            world.get::<Position>(player)
        );

        world.despawn(player);

        world.get_mut::<Position>(rare_sword).unwrap().x += 1.;

        assert_eq!(
            Some(&Position {
                x: 2.,
                y: 1.,
                z: 1.
            }),
            world.get::<Position>(rare_sword)
        );
    }

    #[test]
    fn world2() {
        let mut world = World::default();

        #[derive(Debug, PartialEq, Clone, Copy)]
        struct Health(u8);
        #[derive(Debug, PartialEq, Clone, Copy)]
        struct Hunger(u8);

        let player1 = world.spawn();
        world.add(player1, Health(100));
        world.add(player1, Hunger(20));
        assert_eq!(Some(Hunger(20)), world.remove(player1));
        assert_eq!(None, world.remove::<Hunger>(player1));
        world.despawn(player1);

        let player2 = world.spawn();
        world.add(player2, Health(50));
        assert!(world.get::<Health>(player1).is_none());
        assert_eq!(Some(Health(50)), world.get::<Health>(player2).copied());
    }

    #[test]
    fn zero_sized_components() {
        let mut world = World::default();

        struct Marker;

        let e1 = world.spawn();
        world.add(e1, Marker);
        let e2 = world.spawn();
        let e3 = world.spawn();
        world.add(e3, Marker);
        let e4 = world.spawn();

        assert!(world.get::<Marker>(e1).is_some());
        assert!(world.get::<Marker>(e2).is_none());
        assert!(world.get::<Marker>(e3).is_some());
        assert!(world.get::<Marker>(e4).is_none());

        world.remove::<Marker>(e1);

        assert!(world.get::<Marker>(e1).is_none());
        assert!(world.get::<Marker>(e2).is_none());
        assert!(world.get::<Marker>(e3).is_some());
        assert!(world.get::<Marker>(e4).is_none());

        world.add::<Marker>(e2, Marker);

        assert!(world.get::<Marker>(e1).is_none());
        assert!(world.get::<Marker>(e2).is_some());
        assert!(world.get::<Marker>(e3).is_some());
        assert!(world.get::<Marker>(e4).is_none());

        world.add::<Marker>(e2, Marker);

        assert!(world.get::<Marker>(e1).is_none());
        assert!(world.get::<Marker>(e2).is_some());
        assert!(world.get::<Marker>(e3).is_some());
        assert!(world.get::<Marker>(e4).is_none());
    }

    #[test]
    fn validate_empty_query() {
        assert!(QuerySet::new(vec![]).is_ok());
    }

    #[test]
    fn validate_multiple_const_query() {
        let mut comp_reg = ComponentRegistry::default();
        struct A;
        struct B;
        struct C;
        let a = comp_reg.register::<A>();
        let b = comp_reg.register::<B>();
        let c = comp_reg.register::<C>();
        assert!(QuerySet::new(vec![
            Query::Single {
                id: a,
                mutable: true,
                optional: false,
            },
            Query::Multiple(vec![
                Query::Single {
                    id: b,
                    mutable: false,
                    optional: false,
                },
                Query::Single {
                    id: c,
                    mutable: false,
                    optional: true,
                }
            ]),
            Query::Single {
                id: b,
                mutable: false,
                optional: false,
            }
        ])
        .is_ok());
    }

    #[test]
    fn validate_multiple_mutable_query() {
        let mut comp_reg = ComponentRegistry::default();
        struct A;
        struct B;
        let a = comp_reg.register::<A>();
        let b = comp_reg.register::<B>();
        assert_eq!(
            Err(QueryError::ConcurrentMutableAccess(b)),
            QuerySet::new(vec![
                Query::Single {
                    id: a,
                    mutable: true,
                    optional: false,
                },
                Query::Multiple(vec![
                    Query::Single {
                        id: a,
                        mutable: false,
                        optional: false,
                    },
                    Query::Single {
                        id: b,
                        mutable: true,
                        optional: true,
                    }
                ]),
                Query::Single {
                    id: b,
                    mutable: true,
                    optional: true,
                }
            ])
        );
    }
}