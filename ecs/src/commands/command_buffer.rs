use crate::World;

use super::Command;

// TODO: optimize this by not making `Command` an enum and so that these can be packed with less
// padding. This would also allow us to keep components with a size only known at runtime directly
// in here instead of having a pointer to an extra allocation.
#[derive(Debug, Default)]
pub struct CommandBuffer {
    commands: Vec<Command>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        CommandBuffer {
            commands: Vec::new(),
        }
    }

    /// Add a command to the buffer.
    pub fn add(&mut self, command: Command) {
        self.commands.push(command);
    }

    /// Executes all commands on the `World` and clears the command buffer. This is when the
    /// commands are actually executed.
    pub fn apply(&mut self, world: &mut World) {
        for command in self.commands.drain(..) {
            command.execute(world);
        }
    }
}
