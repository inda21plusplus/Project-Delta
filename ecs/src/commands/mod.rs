use std::{
    alloc::Layout,
    any::{self, TypeId},
    borrow::Cow,
};

mod command;
mod command_buffer;

pub(crate) use command::Command;
pub(crate) use command_buffer::CommandBuffer;

use crate::{Entities, Entity};

/// Allows commands to be issued without exclusive access the world by deferring them to be run at
/// a later state, when `World::maintain` is called.
/// Issuing commands on a command buffer after calling `World::maintain` since creation will result
/// in a panic.
pub struct Commands<'b, 'e> {
    buffer: &'b mut CommandBuffer,
    entities: &'e Entities,
}

impl<'b, 'e> Commands<'b, 'e> {
    pub fn new(buffer: &'b mut CommandBuffer, entities: &'e Entities) -> Self {
        Self { buffer, entities }
    }

    pub fn spawn(&mut self) -> Entity {
        self.entities.spawn()
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.buffer.add(Command::Despawn(entity));
    }

    pub fn add<T: 'static>(&mut self, entity: Entity, component: T) {
        unsafe fn drop<T: 'static>(ptr: *mut u8) {
            ptr.cast::<T>().drop_in_place();
        }
        let component = Box::new(component);
        let component = Box::into_raw(component);
        let component = component as *mut u8;
        self.buffer.add(Command::AddComponent {
            entity,
            type_id: TypeId::of::<T>(),
            name: Cow::Borrowed(any::type_name::<T>()),
            component,
            layout: Layout::new::<T>(),
            drop: drop::<T>,
        });
    }
}
