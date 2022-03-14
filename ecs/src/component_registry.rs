use std::{
    alloc::Layout,
    any::{self, TypeId},
    collections::HashMap,
    ops,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentId(u16);

#[derive(Debug, PartialEq, Eq)]
pub struct ComponentInfo {
    name: String,
    layout: Layout,
}

impl ComponentInfo {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }
}

#[derive(Debug, Default)]
pub struct ComponentRegistry {
    // Indexed by ComponentId's
    components: Vec<ComponentInfo>,
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
            layout: Layout::new::<T>(),
        };
        self.components.push(info);
        self.rust_types.insert(TypeId::of::<T>(), id);
        id
    }

    pub fn id<T>(&self) -> Option<ComponentId>
    where
        T: 'static,
    {
        self.rust_types.get(&TypeId::of::<T>()).copied()
    }

    pub fn info<T>(&self) -> Option<&ComponentInfo>
    where
        T: 'static,
    {
        self.id::<T>().map(|id| &self[id])
    }
}

impl ops::Index<ComponentId> for ComponentRegistry {
    type Output = ComponentInfo;

    fn index(&self, index: ComponentId) -> &Self::Output {
        &self.components[index.0 as usize]
    }
}
