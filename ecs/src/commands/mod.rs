use std::{cell::RefCell, rc::Rc};

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
}
