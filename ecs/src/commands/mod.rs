use std::{
    alloc::Layout,
    any::{self, TypeId},
    borrow::Cow,
    cell::RefCell,
    rc::Rc,
};

mod command;
mod command_buffer;

pub(crate) use command::Command;
pub(crate) use command_buffer::CommandBuffer;

use crate::Entity;

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
    inner: Rc<RefCell<CommandBuffer>>,
    entity_counter: usize,
}

impl Commands {
    pub(crate) fn new() -> (Rc<RefCell<CommandBuffer>>, Self) {
        let inner = Rc::new(RefCell::new(CommandBuffer::new()));
        (
            inner.clone(),
            Self {
                inner,
                entity_counter: 0,
            },
        )
    }

    pub fn spawn(&mut self) -> EntityRef {
        let id = self.entity_counter;
        self.entity_counter += 1;
        self.inner.borrow_mut().push(Command::Spawn);
        EntityRef::New(id)
    }

    pub fn despawn<E: Into<EntityRef>>(&mut self, entity: E) {
        self.inner
            .borrow_mut()
            .push(Command::Despawn(entity.into()));
    }

    pub fn add<T: 'static, E: Into<EntityRef>>(&mut self, entity: E, component: T) {
        unsafe fn drop<T: 'static>(ptr: *mut u8) {
            ptr.cast::<T>().drop_in_place();
        }
        let component = Box::new(component);
        let component = Box::into_raw(component);
        let component = component as *mut u8;
        self.inner.borrow_mut().push(Command::AddComponent {
            entity: entity.into(),
            type_id: TypeId::of::<T>(),
            name: Cow::Borrowed(any::type_name::<T>()),
            component,
            layout: Layout::new::<T>(),
            drop: drop::<T>,
        });
    }
}
