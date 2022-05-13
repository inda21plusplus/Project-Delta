use std::{alloc::Layout, any::TypeId, borrow::Cow, mem, ptr};

use crate::{Entity, World};

#[derive(Debug)]
pub enum Command {
    Despawn(Entity),
    AddComponent {
        entity: Entity,
        type_id: TypeId,
        component: *mut u8, // Is set to null after ownership is transferred to the world
        name: Cow<'static, str>,
        layout: Layout,
        drop: unsafe fn(*mut u8),
    },
}

impl Command {
    pub(super) fn execute(mut self, world: &mut World) {
        match &mut self {
            &mut Command::Despawn(entity) => {
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
                let name = mem::take(name);
                let comp_id = match world
                    .component_registry_mut()
                    .component_id_from_type_id(*type_id)
                {
                    Some(id) => id,
                    None => unsafe {
                        world
                            .component_registry_mut()
                            .register_raw(*type_id, name, *layout, *drop)
                    },
                };
                unsafe { world.add_raw(*entity, *component, comp_id) };
                *component = ptr::null_mut();
            }
        }
    }
}

impl Drop for Command {
    fn drop(&mut self) {
        match self {
            &mut Command::AddComponent {
                component, drop, ..
            } if !component.is_null() => unsafe {
                drop(component);
            },
            _ => {}
        }
    }
}
