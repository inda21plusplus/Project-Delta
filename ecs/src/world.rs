use crate::component::ComponentRegistry;
use crate::Entities;

pub struct World {
    entities: Entities,
    components: ComponentRegistry,
}
