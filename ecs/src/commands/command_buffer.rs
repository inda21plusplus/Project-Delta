use super::Command;
use std::vec::Drain;

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
    pub(crate) fn new() -> Self {
        Self {
            owner: Owner::Commands,
            commands: Vec::new(),
        }
    }

    pub fn push(&mut self, command: Command) {
        if self.owner == Owner::World {
            panic!("Cannot push command after taking the command buffer");
        }
        self.commands.push(command);
    }

    pub fn take(&mut self) -> Drain<Command> {
        if self.owner == Owner::World {
            panic!("Cannot take command buffer twice");
        }
        self.owner = Owner::World;
        self.commands.drain(..)
    }
}
