use std::{
    alloc::Layout,
    any::{self, TypeId},
    borrow::Cow,
};

mod command;

pub(crate) use command::Command;

use crate::{Entity, World};

#[derive(Debug, Clone, Copy)]
pub enum EntityRef {
    Entity(Entity),
    New(usize),
}

impl From<Entity> for EntityRef {
    fn from(entity: Entity) -> Self {
        EntityRef::Entity(entity)
    }
}

#[derive(Default)]
struct ExecutionContext {
    entities_created: Vec<Entity>,
}

/// Allows commands to be issued without exclusive access the world by deferring them to be run at
/// a later state, when `World::maintain` is called.
/// Issuing commands on a command buffer after calling `World::maintain` since creation will result
/// in a panic.
pub struct Commands {
    commands: Vec<Command>,
    entity_counter: usize,
}

impl Commands {
    /// Retrieves a `Commands` which can be used to issue commands to be run on this `World` when
    /// possible, which is when `maintain` is called.
    /// After `maintain` is called, issuing more commands to the `Commands` will result in a panic.
    pub fn new() -> Self {
        Self {
            commands: vec![],
            entity_counter: 0,
        }
    }

    /// Executes all commands on the `World` and clears the command buffer.
    pub fn apply(&mut self, world: &mut World) {
        let mut ctx = ExecutionContext::default();
        for command in self.commands.drain(..) {
            command.execute(world, &mut ctx);
        }
        self.entity_counter = 0;
    }

    pub fn spawn(&mut self) -> EntityRef {
        let id = self.entity_counter;
        self.entity_counter += 1;
        self.commands.push(Command::Spawn);
        EntityRef::New(id)
    }

    pub fn despawn<E: Into<EntityRef>>(&mut self, entity: E) {
        self.commands.push(Command::Despawn(entity.into()));
    }

    pub fn add<T: 'static, E: Into<EntityRef>>(&mut self, entity: E, component: T) {
        unsafe fn drop<T: 'static>(ptr: *mut u8) {
            ptr.cast::<T>().drop_in_place();
        }
        let component = Box::new(component);
        let component = Box::into_raw(component);
        let component = component as *mut u8;
        self.commands.push(Command::AddComponent {
            entity: entity.into(),
            type_id: TypeId::of::<T>(),
            name: Cow::Borrowed(any::type_name::<T>()),
            component,
            layout: Layout::new::<T>(),
            drop: drop::<T>,
        });
    }
}
