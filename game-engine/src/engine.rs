use std::time::{Duration, Instant};

use ecs::World;
use rendering::Renderer;

use crate::{physics_systems, Time};

const TIME_STEP: Duration = Duration::from_millis(20);

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
        let delta = now - self.last_update;

        if delta >= TIME_STEP {
            let time = if let Some(time) = self.world.resource_mut::<Time>() {
                time
            } else {
                self.world.add_resource(Time::default());
                self.world.resource_mut().unwrap()
            };
            time.time_since_startup += delta;
            time.dt = delta;

            physics_systems::update(&mut self.world, TIME_STEP.as_secs_f32());

            self.last_update += delta;
        }
    }
}
