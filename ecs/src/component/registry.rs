use std::{
    any::{self, TypeId},
    collections::HashMap,
    ops,
};

use super::{Storage, StorageType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(u16);

#[derive(Debug, PartialEq, Eq)]
pub struct ComponentInfo {
    name: String,
    type_id: Option<TypeId>,
}

impl ComponentInfo {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}

#[derive(Debug)]
pub struct ComponentEntry {
    pub info: ComponentInfo,
    pub storage: Storage,
}

impl ComponentEntry {
    pub fn new(info: ComponentInfo, storage: Storage) -> Self {
        Self { info, storage }
    }
}

#[derive(Debug, Default)]
pub struct ComponentRegistry {
    // Indexed by ComponentId's
    entries: Vec<ComponentEntry>,
    rust_types: HashMap<TypeId, ComponentId>,
}

impl ComponentRegistry {
    pub fn register<T>(&mut self) -> ComponentId
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        let id = self.entries.len();
        let id = ComponentId(id.try_into().unwrap());
        debug_assert!(self.rust_types.insert(type_id, id).is_none());

        let info = ComponentInfo {
            name: any::type_name::<T>().to_string(),
            type_id: Some(type_id),
        };
        // TODO: detect which storage type should be used, or *maybe* creating components from rust
        // struct will always want the same kind of storage since they will probably be on most
        // components?
        let storage = Storage::new::<T>(StorageType::VecStorage);
        self.entries.push(ComponentEntry::new(info, storage));
        id
    }

    pub fn id<T>(&self) -> Option<ComponentId>
    where
        T: 'static,
    {
        self.rust_types.get(&TypeId::of::<T>()).copied()
    }

    pub fn component<T>(&self) -> Option<&ComponentEntry>
    where
        T: 'static,
    {
        self.id::<T>().map(|id| &self[id])
    }

    /// Get a mutable reference to the component registry's entries.
    pub fn entries_mut(&mut self) -> &mut [ComponentEntry] {
        &mut self.entries
    }
}

impl ops::Index<ComponentId> for ComponentRegistry {
    type Output = ComponentEntry;

    fn index(&self, index: ComponentId) -> &Self::Output {
        &self.entries[index.0 as usize]
    }
}

impl ops::IndexMut<ComponentId> for ComponentRegistry {
    fn index_mut(&mut self, index: ComponentId) -> &mut Self::Output {
        &mut self.entries[index.0 as usize]
    }
}