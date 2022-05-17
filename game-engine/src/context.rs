use ecs::World;
use rendering::Renderer;

use crate::physics_systems;

pub struct Context {
    pub renderer: Renderer,
    pub world: World,
}

impl Context {
    pub fn update(&mut self, dt: f32) {
        // TODO: perhaps it would be nice to control dt here. No nothing if enough time hasn't
        // passed and perhaps do something if we seem to be running too slow. (Or just accept slow
        // motion.
        // let dt = 60f32.recip();

        physics_systems::update(&mut self.world, dt);
    }
}
