use std::time::Instant;

use ecs::World;
use rendering::Renderer;

use crate::{physics_systems, time::TIME_STEP, Time};

pub struct Engine {
    pub renderer: Renderer,
    pub world: World,

    last_update: Instant,
}

impl Engine {
    pub fn new(renderer: Renderer) -> Self {
        Self {
            renderer,
            world: World::default(),
            last_update: Instant::now(),
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let mut delta = now - self.last_update;

        let mut i = 0;
        while delta >= TIME_STEP && i < 2 {
            Time::system(&mut self.world);

            physics_systems::update(&mut self.world, TIME_STEP.as_secs_f32());

            self.last_update += delta;
            delta -= TIME_STEP;
            i += 1;
        }
    }
}
