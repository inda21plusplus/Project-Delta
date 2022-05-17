use std::time::Duration;

#[derive(Default)]
pub struct Time {
    pub(crate) time_since_startup: Duration,
    pub(crate) dt: Duration,
}

impl Time {
    pub fn dt(&self) -> Duration {
        self.dt
    }

    pub fn time_since_startup(&self) -> Duration {
        self.time_since_startup
    }
}
