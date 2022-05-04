use std::{
    alloc::Layout,
    any::{self, TypeId},
    borrow::Cow,
    cell::RefCell,
    ptr,
    rc::Rc,
};

mod command;
mod command_buffer;

pub(crate) use command::Command;
pub(crate) use command_buffer::CommandBuffer;

use crate::Entity;

/// Allows commands to be issued without exclusive acces the world by deferring them to be run
/// at a later state, when `World::maintain` is called.
/// Issuing commands on a command buffer after calling `World::maintain` since creation will result
/// in a panic.
pub struct Commands {
    inner: Rc<RefCell<CommandBuffer>>,
}

impl Commands {
    pub(crate) fn new() -> (Rc<RefCell<CommandBuffer>>, Self) {
        let inner = Rc::new(RefCell::new(CommandBuffer::new()));
        (inner.clone(), Self { inner })
    }

    pub fn spawn(&mut self) {
        self.inner.borrow_mut().push(Command::Spawn);
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.inner.borrow_mut().push(Command::Despawn(entity));
    }

    pub fn add<T: 'static>(&mut self, entity: Entity, component: T) {
        unsafe fn drop<T: 'static>(ptr: *mut u8) {
            eprintln!("Dropping {:?} of type {}", ptr, any::type_name::<T>());
            ptr::drop_in_place(ptr as *mut T);
        }
        let component = Box::new(component);
        let component = Box::into_raw(component);
        let component = component as *mut u8;
        self.inner.borrow_mut().push(Command::AddComponent {
            entity,
            type_id: TypeId::of::<T>(),
            name: Cow::Borrowed(any::type_name::<T>()),
            component,
            layout: Layout::new::<T>(),
            drop: drop::<T>,
        });
    }
}
