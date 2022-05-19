use std::time::Duration;

use ecs::World;

pub const TIME_STEP: Duration = Duration::from_millis(10);

#[derive(Default)]
pub struct Time {
    pub(crate) time_since_startup: Duration,
    pub(crate) dt: Duration,
}

impl Time {
    pub(crate) fn system(world: &mut World) {
        let time = if let Some(time) = world.resource_mut::<Time>() {
            time
        } else {
            world.add_resource(Time::default());
            world.resource_mut().unwrap()
        };
        time.time_since_startup += TIME_STEP;
        time.dt = TIME_STEP;
    }

    pub fn dt(&self) -> Duration {
        self.dt
    }

    pub fn time_since_startup(&self) -> Duration {
        self.time_since_startup
    }
}
