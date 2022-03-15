use std::{
    any::{self, TypeId},
    collections::HashMap,
    ops,
};

use super::{Storage, StorageType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentId(u16);

#[derive(Debug, PartialEq, Eq)]
pub struct ComponentInfo {
    name: String,
}

impl ComponentInfo {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}

#[derive(Debug)]
pub struct Component {
    pub info: ComponentInfo,
    pub storage: Storage,
}

impl Component {
    pub fn new(info: ComponentInfo, storage: Storage) -> Self {
        Self { info, storage }
    }
}

#[derive(Debug, Default)]
pub struct ComponentRegistry {
    // Indexed by ComponentId's
    components: Vec<Component>,
    rust_types: HashMap<TypeId, ComponentId>,
}

impl ComponentRegistry {
    pub fn register<T>(&mut self) -> ComponentId
    where
        T: 'static,
    {
        let id = self.components.len();
        let id = ComponentId(id.try_into().unwrap());
        let info = ComponentInfo {
            name: any::type_name::<T>().to_string(),
        };
        // TODO: detect which storage type should be used, or *maybe* creating components from rust
        // struct will always want the same kind of storage since they will probably be on most
        // components?
        let storage = Storage::new::<T>(StorageType::VecStorage);
        self.components.push(Component::new(info, storage));
        self.rust_types.insert(TypeId::of::<T>(), id);
        id
    }

    pub fn id<T>(&self) -> Option<ComponentId>
    where
        T: 'static,
    {
        self.rust_types.get(&TypeId::of::<T>()).copied()
    }

    pub fn component<T>(&self) -> Option<&Component>
    where
        T: 'static,
    {
        self.id::<T>().map(|id| &self[id])
    }
}

impl ops::Index<ComponentId> for ComponentRegistry {
    type Output = Component;

    fn index(&self, index: ComponentId) -> &Self::Output {
        &self.components[index.0 as usize]
    }
}
