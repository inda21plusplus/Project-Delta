use std::{alloc::Layout, any::TypeId, borrow::Cow};

use crate::{Entity, World};

#[derive(Debug)]
pub enum Command {
    Despawn(Entity),
    AddComponent {
        entity: Entity,
        type_id: TypeId,
        component: *mut u8, // TODO: watch out for memory leak?
        name: Cow<'static, str>,
        layout: Layout,
        drop: unsafe fn(*mut u8),
    },
}

impl Command {
    pub(super) fn execute(self, world: &mut World) {
        match self {
            Command::Despawn(entity) => {
                world.despawn(entity);
            }
            Command::AddComponent {
                entity,
                component,
                type_id,
                name,
                layout,
                drop,
            } => {
                let comp_id = match world
                    .component_registry_mut()
                    .component_id_from_type_id(type_id)
                {
                    Some(id) => id,
                    None => unsafe {
                        world
                            .component_registry_mut()
                            .register_raw(type_id, name, layout, drop)
                    },
                };
                unsafe { world.add_raw(entity, component, comp_id) };
            }
        }
    }
}
