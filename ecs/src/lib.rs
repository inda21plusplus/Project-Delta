#![deny(warnings)]

mod commands;
pub mod component;
mod entity;
mod error;
#[macro_use]
mod query;
mod world;

pub use commands::Commands;
pub use entity::{Entities, Entity};
pub use error::BorrowMutError;
pub use query::{as_mut_lt, as_ref_lt, ComponentQuery, Query};
pub use world::World;

#[cfg(test)]
mod tests {
    use std::{
        alloc::Layout,
        any,
        cell::Cell,
        collections::HashSet,
        mem, ptr,
        rc::Rc,
        time::{Duration, Instant},
    };

    use crate::{
        commands::CommandBuffer,
        component::{ComponentRegistry, Storage, StorageType},
    };

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
        let counter = Rc::new(Cell::new(0));

        unsafe fn drop_counter(counter: *mut u8) {
            ptr::drop_in_place(counter as *mut Counter)
        }

        {
            let mut storage = unsafe {
                Storage::new(
                    StorageType::VecStorage,
                    Layout::new::<Counter>(),
                    drop_counter,
                )
            };
            let entities = Entities::default();
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
                assert!(
                    unsafe { storage.set(index, Counter::new(counter.clone())) }
                        == (index % 2 == 1)
                );
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
        assert!(Query::new(vec![]).is_ok());
    }

    #[test]
    fn validate_multiple_const_query() {
        let mut comp_reg = ComponentRegistry::default();
        struct A;
        struct B;
        let a = comp_reg.register::<A>();
        let b = comp_reg.register::<B>();
        let q = Query::new(vec![
            ComponentQuery {
                id: a,
                mutable: true,
            },
            ComponentQuery {
                id: b,
                mutable: false,
            },
            ComponentQuery {
                id: b,
                mutable: false,
            },
        ]);
        assert!(q.is_ok(), "{:?}; a={:?}, b={:?}", q, a, b);
    }

    #[test]
    fn validate_multiple_mutable_query() {
        let mut comp_reg = ComponentRegistry::default();
        struct A;
        struct B;
        let a = comp_reg.register::<A>();
        let b = comp_reg.register::<B>();
        assert_eq!(
            Err(BorrowMutError::new(b)),
            Query::new(vec![
                ComponentQuery {
                    id: a,
                    mutable: false,
                },
                ComponentQuery {
                    id: b,
                    mutable: true,
                },
                ComponentQuery {
                    id: b,
                    mutable: false,
                }
            ])
        );
    }

    #[test]
    fn read_and_write_using_query() {
        let mut world = World::default();
        let a = world.spawn();
        world.add(a, 0usize);
        let b = world.spawn();
        world.add(b, 1usize);
        world.add(b, 2f32);
        let usize_id = world.component_id::<usize>().unwrap();
        let f32_id = world.component_id::<f32>().unwrap();

        let usize_query = Query::new(vec![ComponentQuery {
            id: usize_id,
            mutable: false,
        }])
        .unwrap();
        let both_query = Query::new(vec![
            ComponentQuery {
                id: usize_id,
                mutable: true,
            },
            ComponentQuery {
                id: f32_id,
                mutable: false,
            },
        ])
        .unwrap();
        {
            let mut res = world.query(&usize_query);
            assert_eq!(*unsafe { res.get(a)[0].cast::<usize>().as_ref() }, 0);
            assert_eq!(*unsafe { res.get(b)[0].cast::<usize>().as_ref() }, 1);
        }
        {
            let mut res = world.query(&both_query);
            assert!(unsafe { res.try_get(a) }.is_none());
            let (int, float) = unsafe {
                if let [int, float] = res.get(b)[..] {
                    (int.cast::<usize>().as_mut(), float.cast::<f32>().as_ref())
                } else {
                    panic!()
                }
            };
            *int += 2;
            assert_eq!(2., *float);
        }
        {
            let mut res = world.query(&usize_query);
            assert_eq!(*unsafe { res.get(a)[0].cast::<usize>().as_ref() }, 0);
            assert_eq!(*unsafe { res.get(b)[0].cast::<usize>().as_ref() }, 3);
        }
    }

    #[test]
    fn multiple_queries_at_the_same_time() {
        let mut world = World::default();
        struct Name(String);
        struct Health(u8);
        let chungus = world.spawn();
        world.add(chungus, Name("Big chungus".into()));
        world.add(chungus, Health(200));
        let ant = world.spawn();
        world.add(ant, Name("Mr. Ant".into()));
        world.add(ant, Health(8));

        let name_query = Query::new(vec![ComponentQuery {
            id: world.component_id::<Name>().unwrap(),
            mutable: false,
        }])
        .unwrap();
        let mut_name_query = Query::new(vec![ComponentQuery {
            id: world.component_id::<Name>().unwrap(),
            mutable: true,
        }])
        .unwrap();
        let health_query = Query::new(vec![ComponentQuery {
            id: world.component_id::<Health>().unwrap(),
            mutable: true,
        }])
        .unwrap();
        let r1 = world.query(&name_query);
        let r2 = world.query(&name_query);
        let r3 = world.query(&health_query);
        mem::drop(r1);
        mem::drop(r2);
        let r4 = world.query(&mut_name_query);
        mem::drop(r3);
        mem::drop(r4);

        let r5 = world.query(&name_query);
        let r6 = world.query(&name_query);
        assert_eq!(
            BorrowMutError::new(world.component_id::<Name>().unwrap()),
            world.try_query(&mut_name_query).unwrap_err()
        );
        mem::drop(r6);
        assert_eq!(
            BorrowMutError::new(world.component_id::<Name>().unwrap()),
            world.try_query(&mut_name_query).unwrap_err()
        );
        mem::drop(r5);
        assert!(world.try_query(&mut_name_query).is_ok());
    }

    #[test]
    fn mutable_queries_must_be_exclusive() {
        let mut world = World::default();
        struct Name(String);
        struct Health(u8);
        let name_id = world.component_registry_mut().register::<Name>();
        let health_id = world.component_registry_mut().register::<Health>();

        let q1 = Query::new(vec![
            ComponentQuery {
                id: name_id,
                mutable: true,
            },
            ComponentQuery {
                id: health_id,
                mutable: false,
            },
        ])
        .unwrap();
        let q2 = Query::new(vec![ComponentQuery {
            id: health_id,
            mutable: true,
        }])
        .unwrap();

        let r = world.query(&q1);
        assert_eq!(
            BorrowMutError::new(name_id),
            world.try_query(&q1).unwrap_err(),
        );
        mem::drop(r);

        let r = world.query(&q2);
        assert_eq!(
            BorrowMutError::new(health_id),
            world.try_query(&q1).unwrap_err(),
        );
        mem::drop(r);

        let r = world.query(&q1);
        assert_eq!(
            BorrowMutError::new(health_id),
            world.try_query(&q2).unwrap_err(),
        );
        mem::drop(r);
    }

    #[test]
    fn type_safe_macros() {
        let mut world = World::default();
        struct Name(String);
        struct Speed(f32);
        let sanic = world.spawn();
        world.add(sanic, Name("Sanic".into()));
        world.add(sanic, Speed(100.0));
        let mario = world.spawn();
        world.add(mario, Name("Mario".into()));
        world.add(mario, Speed(200.0)); // copilot thinks mario is faster than sanic

        query_iter!(world, (name: Name, speed: mut Speed) => {
            match name.0.as_ref() {
                "Mario" => assert_eq!(speed.0, 200.0),
                "Sanic" => {
                    assert_eq!(speed.0, 100.0);
                    speed.0 = 300.0; // copilot thinks he's faster than mario
                }
                _ => panic!("Unexpected name"),
            }
        });

        query_iter!(world, (entity: Entity, name: Name, speed: Speed) => {
            match name.0.as_ref() {
                "Mario" => {
                    assert_eq!(entity, mario);
                    assert_eq!(speed.0, 200.0)
                }
                "Sanic" => {
                    assert_eq!(entity, sanic);
                    assert_eq!(speed.0, 300.0);
                }
                _ => panic!("Unexpected name"),
            }
        });

        let mut found_sanic = false;
        let mut found_mario = false;
        query_iter!(world, (entity: Entity) => {
            if found_sanic {
                assert_eq!(entity, mario);
                found_mario = true;
            } else {
                assert_eq!(entity, sanic);
                found_sanic = true;
            }
        });
        assert!(found_sanic && found_mario);
    }

    #[test]
    fn iterate_over_query() {
        let mut world = World::default();
        struct Position(f32);
        struct Velocity(f32);
        let pos_id = world.component_registry_mut().register::<Position>();
        let vel_id = world.component_registry_mut().register::<Velocity>();

        for i in 0..1000 {
            let entity = world.spawn();
            world.add(entity, Position(i as f32));
            world.add(entity, Velocity(1.5));
        }

        let q = Query::new(vec![
            ComponentQuery {
                id: pos_id,
                mutable: true,
            },
            ComponentQuery {
                id: vel_id,
                mutable: false,
            },
        ])
        .unwrap();
        let mut q = world.query(&q);
        for (pos, vel) in unsafe {
            q.iter().map(|(_e, comps)| {
                if let [pos, vel] = comps[..] {
                    (
                        pos.cast::<Position>().as_mut(),
                        vel.cast::<Velocity>().as_ref(),
                    )
                } else {
                    panic!();
                }
            })
        } {
            pos.0 += vel.0;
        }
        mem::drop(q);

        let q = Query::new(vec![ComponentQuery {
            id: pos_id,
            mutable: false,
        }])
        .unwrap();
        let mut q = world.query(&q);
        for (i, pos) in unsafe {
            q.iter()
                .map(|(_e, comps)| {
                    if let [pos] = comps[..] {
                        pos.cast::<Position>().as_ref()
                    } else {
                        panic!();
                    }
                })
                .enumerate()
        } {
            assert_eq!(i as f32 + 1.5, pos.0);
        }
    }

    #[test]
    fn resources() {
        let mut world = World::default();
        struct Time {
            now: Instant,
            dt: Duration,
        }
        struct Gravity {
            accel_x: f32,
            accel_y: f32,
        }
        world.add_resource(Time {
            now: Instant::now(),
            dt: Duration::from_millis(16),
        });
        world.add_resource(Gravity {
            accel_x: 0.,
            accel_y: -9.818,
        });

        assert_eq!(16_000, world.resource::<Time>().unwrap().dt.as_micros());
        assert_eq!(0., world.resource::<Gravity>().unwrap().accel_x);
        assert_eq!(-9.818, world.resource::<Gravity>().unwrap().accel_y);

        let time = world.resource_mut::<Time>().unwrap();
        let now = Instant::now();
        time.dt = Duration::from_millis(15);
        time.now = now;

        let dt = world.resource::<Time>().unwrap().dt.as_micros();
        assert_eq!(15_000, dt);
    }

    #[test]
    #[should_panic]
    fn borrowing_borrowed_component_panics() {
        let mut world = World::default();
        let id = world.component_registry_mut().register::<usize>();
        let entity = world.spawn();

        let q = Query::new(vec![ComponentQuery { id, mutable: true }]).unwrap();
        // This creates a mutable borrow on `usize`s
        let q = world.query(&q);
        // And this another one
        world.get::<usize>(entity);
        // While the first borrow still exists
        mem::drop(q);
    }

    #[test]
    fn command_buffer_despawn_entities() {
        struct Health(u8);

        let mut world = World::default();
        let a = world.spawn();
        world.add(a, Health(100));
        let b = world.spawn();
        world.add(b, Health(10));
        let c = world.spawn();
        world.add(c, Health(100));
        let d = world.spawn();
        world.add(d, Health(10));

        let mut command_buffer = CommandBuffer::new();
        let mut commands = Commands::new(&mut command_buffer, world.entities());
        query_iter!(world, (entity: Entity, health: mut Health) => {
            health.0 -= 10;
            if health.0 == 0 {
                commands.despawn(entity);
            }
        });
        assert!(world.entities().exists(a));
        assert!(world.entities().exists(b));
        assert!(world.entities().exists(c));
        assert!(world.entities().exists(d));
        command_buffer.apply(&mut world);
        assert!(world.entities().exists(a));
        assert!(!world.entities().exists(b));
        assert!(world.entities().exists(c));
        assert!(!world.entities().exists(d));
    }

    #[test]
    fn add_component_to_old_entity_through_commands() {
        let mut world = World::default();
        let e1 = world.spawn();
        let e2 = world.spawn();
        let counter = Rc::new(Cell::new(0));
        let mut command_buffer = CommandBuffer::new();
        let mut commands = Commands::new(&mut command_buffer, world.entities());

        commands.add(e1, Counter::named(counter.clone(), "a"));
        assert_eq!(counter.get(), 1);
        commands.add(e1, Counter::named(counter.clone(), "b"));
        assert_eq!(counter.get(), 2);
        commands.add(e2, Counter::named(counter.clone(), "c"));
        assert_eq!(counter.get(), 3);
        commands.despawn(e2);
        assert_eq!(counter.get(), 3);
        command_buffer.apply(&mut world);
        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn add_component_to_newly_created_entity_through_commands() {
        let mut world = World::default();
        let counter = Rc::new(Cell::new(0));
        let mut command_buffer = CommandBuffer::new();
        let mut commands = Commands::new(&mut command_buffer, world.entities());

        let e1 = commands.spawn();
        commands.add(e1, Counter::named(counter.clone(), "a"));
        assert_eq!(counter.get(), 1);
        commands.add(e1, Counter::named(counter.clone(), "b"));
        assert_eq!(counter.get(), 2);

        let e2 = commands.spawn();
        commands.add(e2, Counter::named(counter.clone(), "c"));
        commands.despawn(e2);
        assert_eq!(counter.get(), 3);

        command_buffer.apply(&mut world);
        assert_eq!(counter.get(), 1);
        query_iter!(world, (c: Counter) => {
            assert_eq!(c.1, "b");
        });
    }

    #[derive(Debug)]
    struct Counter(Rc<Cell<usize>>, &'static str);
    impl Counter {
        fn new(rc: Rc<Cell<usize>>) -> Self {
            Self::named(rc, "")
        }
        fn named(rc: Rc<Cell<usize>>, name: &'static str) -> Self {
            rc.set(rc.get() + 1);
            Self(rc, name)
        }
    }
    impl Drop for Counter {
        fn drop(&mut self) {
            self.0.set(self.0.get() - 1)
        }
    }
}
