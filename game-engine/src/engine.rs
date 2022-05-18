use std::time::Instant;

use ecs::World;
use rendering::Renderer;

use crate::{physics_systems, time::TIME_STEP, Time};

pub struct Engine {
    pub renderer: Renderer,
    pub world: World,

    last_update: Option<Instant>,
}

impl Engine {
    pub fn new(renderer: Renderer) -> Self {
        Self {
            renderer,
            world: World::default(),
            last_update: None,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let mut last_update = if let Some(last_update) = self.last_update {
            last_update
        } else {
            now - TIME_STEP
        };
        let mut delta = now - last_update;

        let mut i = 0;
        while delta >= TIME_STEP && i < 2 {
            Time::system(&mut self.world);

            physics_systems::update(&mut self.world);

            last_update += delta;
            delta -= TIME_STEP;
            i += 1;
        }
        self.last_update = Some(last_update);
    }
}
