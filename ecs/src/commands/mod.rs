use std::{
    alloc::Layout,
    any::{self, TypeId},
    borrow::Cow,
};

mod command;
mod command_buffer;

pub(crate) use command::Command;
pub use command_buffer::CommandBuffer;

use crate::{Entities, Entity};

/// Allows commands to be issued without exclusive access the world by deferring them to be run at
/// a later state, when exclusive access to the world is available.
/// # Examples
/// ```
/// # struct Position(f32, f32);
/// # use ecs::{CommandBuffer, Commands, World};
/// let mut world = World::default();
/// let e1 = world.spawn();
///
/// let mut command_buffer = CommandBuffer::new();
/// let mut commands = Commands::new(&mut command_buffer, world.entities());
///
/// let e2 = commands.spawn();
/// commands.add(e2, Position(0.0, 0.0));
/// commands.despawn(e1);
///
/// assert!(world.entities().exists(e2)); // NOTE: some commands, like spawning entities happen immediately
/// assert!(world.entities().exists(e1)); // but others happen when the command buffer is applied
/// assert!(world.get::<Position>(e2).is_none());
///
/// command_buffer.apply(&mut world);
///
/// assert!(!world.entities().exists(e1));
/// assert!(world.entities().exists(e2));
/// ```
pub struct Commands<'b, 'e> {
    buffer: &'b mut CommandBuffer,
    entities: &'e Entities,
}

impl<'b, 'e> Commands<'b, 'e> {
    /// Creates a wrapper for issuing commands to the command buffer. The commands will be issued
    /// when `CommandBuffer::apply` is called.
    pub fn new(buffer: &'b mut CommandBuffer, entities: &'e Entities) -> Self {
        Self { buffer, entities }
    }

    /// Creates a new `entity`. See `Entities::spawn` for more information.
    pub fn spawn(&mut self) -> Entity {
        self.entities.spawn()
    }

    /// Despawns an entity, removing its components (if any).
    pub fn despawn(&mut self, entity: Entity) {
        self.buffer.add(Command::Despawn(entity));
    }

    /// Adds a component to an entity. If the type is not registered as a component, it gets
    /// registered automatically. Returns `true` if `entity` did not have this kind of component
    /// before and `entity` exists. If `entity` exists and the component was already present,
    /// the old component is dropped and replaced with the new one.
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
