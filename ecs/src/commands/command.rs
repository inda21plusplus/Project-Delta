use std::{alloc::Layout, any::TypeId, borrow::Cow};

use crate::World;

use super::{EntityRef, ExecutionContext};

#[derive(Debug)]
pub enum Command {
    Spawn,
    Despawn(EntityRef),
    AddComponent {
        entity: EntityRef,
        type_id: TypeId,
        component: *mut u8, // TODO: watch out for memory leak?
        name: Cow<'static, str>,
        layout: Layout,
        drop: unsafe fn(*mut u8),
    },
}

impl Command {
    pub(super) fn execute(self, world: &mut World, ctx: &mut ExecutionContext) {
        match self {
            Command::Spawn => {
                ctx.entities_created.push(world.spawn());
            }
            Command::Despawn(entity) => {
                let entity = match entity {
                    EntityRef::Entity(entity) => entity,
                    EntityRef::New(index) => ctx.entities_created[index],
                };
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
                let entity = match entity {
                    EntityRef::Entity(entity) => entity,
                    EntityRef::New(index) => ctx.entities_created[index],
                };
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
