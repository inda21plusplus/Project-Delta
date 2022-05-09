use crate::World;

use super::Command;

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

    pub fn add(&mut self, command: Command) {
        self.commands.push(command);
    }

    /// Executes all commands on the `World` and clears the command buffer.
    pub fn apply(&mut self, world: &mut World) {
        for command in self.commands.drain(..) {
            command.execute(world);
        }
    }
}
