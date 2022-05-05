use crate::World;

use super::{Command, ExecutionContext};

#[derive(Debug)]
pub(crate) struct CommandBuffer {
    owner: Owner,
    commands: Vec<Command>,
}

#[derive(Debug, PartialEq, Eq)]
enum Owner {
    Commands,
    World,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self {
            owner: Owner::Commands,
            commands: Vec::new(),
        }
    }

    pub fn execute(&mut self, world: &mut World) {
        if self.owner == Owner::World {
            panic!("Cannot take command buffer twice");
        }
        self.owner = Owner::World;

        let mut ctx = ExecutionContext::default();
        for command in self.commands.drain(..) {
            command.execute(world, &mut ctx);
        }
    }

    pub fn push(&mut self, command: Command) {
        if self.owner == Owner::World {
            panic!("Cannot push command after taking the command buffer");
        }
        self.commands.push(command);
    }
}
