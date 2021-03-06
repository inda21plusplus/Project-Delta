use std::{
    alloc::Layout,
    any::{self, TypeId},
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    fmt, ops,
    rc::Rc,
};

use super::{Storage, StorageType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(u16);

/// Basic metadata about a kind of component.
#[derive(Debug, PartialEq, Eq)]
pub struct ComponentInfo {
    name: Cow<'static, str>,
    type_id: Option<TypeId>,
    id: ComponentId,
    // TODO: maybe add some sort of is_thread_safe bool or require `Send + Sync` for all
    // components.
}

impl ComponentInfo {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn type_id(&self) -> Option<TypeId> {
        self.type_id
    }

    pub fn id(&self) -> ComponentId {
        self.id
    }
}

/// A kind of components registered in a `ComponentRegistry`. Includes both metadata about the kind
/// of component and all the components of this kind.
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

#[derive(Debug)]
pub struct ComponentEntryRef {
    ptr: *mut ComponentEntry,
    borrowed: Rc<RefCell<Vec<BorrowStatus>>>,
    mutable: bool,
}

impl ComponentEntryRef {
    pub fn get(&self) -> &ComponentEntry {
        unsafe { &*self.ptr }
    }

    pub fn get_mut(&mut self) -> &mut ComponentEntry {
        assert!(
            self.mutable,
            "Tried to get mutable access to immutable borrow to component entry"
        );
        unsafe { &mut *self.ptr }
    }

    pub fn mutable(&self) -> bool {
        self.mutable
    }

    fn try_new(
        ptr: *mut ComponentEntry,
        borrowed: Rc<RefCell<Vec<BorrowStatus>>>,
        mutable: bool,
    ) -> Option<Self> {
        let id = unsafe { (*ptr).info.id.0 as usize };
        borrowed.borrow_mut()[id].add_borrow(mutable).ok()?;

        Some(Self {
            ptr,
            borrowed,
            mutable,
        })
    }
}

impl Drop for ComponentEntryRef {
    fn drop(&mut self) {
        let id = unsafe { (*self.ptr).info.id.0 as usize };
        self.borrowed.borrow_mut()[id].remove_borrow(self.mutable);
    }
}

/// A registry for different kinds of components. Includes both metadata about the kinds of
/// components and all components themselves.
#[derive(Debug, Default)]
pub struct ComponentRegistry {
    // Indexed by ComponentId's
    entries: Vec<ComponentEntry>,

    rust_types: HashMap<TypeId, ComponentId>,

    borrowed: Rc<RefCell<Vec<BorrowStatus>>>,
}

impl ComponentRegistry {
    /// Registeres a rust type as a component kind. A rust type must *not* be registered twice in
    /// the same registry.
    pub fn register<T>(&mut self) -> ComponentId
    where
        T: 'static,
    {
        unsafe fn drop_ptr<T>(ptr: *mut u8) {
            ptr.cast::<T>().drop_in_place();
        }

        // Safety: if the type id and layout do not match here or `drop_ptr` is invalid, thats on
        // Rust, not us.
        unsafe {
            self.register_raw(
                TypeId::of::<T>(),
                Cow::Borrowed(any::type_name::<T>()),
                Layout::new::<T>(),
                drop_ptr::<T>,
            )
        }
    }

    /// Registeres a rust type as a component kind. A rust type must *not* be registered twice in
    /// the same registry.
    /// # Safety
    /// The `type_id` and `layout` must match and `drop` must be a valid drop function for the
    /// given `type_id`.
    pub unsafe fn register_raw(
        &mut self,
        type_id: TypeId,
        name: Cow<'static, str>,
        layout: Layout,
        drop: unsafe fn(*mut u8),
    ) -> ComponentId {
        let id = ComponentId(self.entries.len().try_into().unwrap());

        let old = self.rust_types.insert(type_id, id);
        debug_assert!(old.is_none());
        assert!(self.check_exclusive_access());

        let info = ComponentInfo {
            name,
            type_id: Some(type_id),
            id,
        };
        let storage = Storage::new(StorageType::VecStorage, layout, drop);

        self.entries.push(ComponentEntry::new(info, storage));
        self.borrowed.borrow_mut().push(BorrowStatus::default());

        id
    }

    // TODO: better name
    pub fn component_id_from_type_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.rust_types.get(&type_id).copied()
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

    pub fn entries_mut(&mut self) -> &mut [ComponentEntry] {
        assert!(self.check_exclusive_access());
        &mut self.entries
    }

    fn check_exclusive_access(&self) -> bool {
        self.borrowed.borrow().iter().all(|b| b.is_free())
    }

    /// Tries to borrow the entry for the component with the given id. Set `mutable` to `true` if
    /// the borrow may be used for writing and `false` if you are absolutely certain the borrow
    /// will not be used for writing.
    /// If the component is already borrowed in a way incompatible with the requested borrow,
    /// `None` is returned. Otherwise a mutable raw pointer to the entry and a function are
    /// returned. Call the function after the borrow will no longer be accessed to indicate that
    /// the component is available to be borrowed again.
    pub fn try_borrow(&self, comp_id: ComponentId, mutable: bool) -> Option<ComponentEntryRef> {
        let entry =
            &self.entries[comp_id.0 as usize] as *const ComponentEntry as *mut ComponentEntry;

        ComponentEntryRef::try_new(entry, self.borrowed.clone(), mutable)
    }
}

impl ops::Index<ComponentId> for ComponentRegistry {
    type Output = ComponentEntry;

    fn index(&self, id: ComponentId) -> &Self::Output {
        let index = id.0 as usize;
        assert!(self.borrowed.borrow()[index].is_readable());
        &self.entries[index]
    }
}

impl ops::IndexMut<ComponentId> for ComponentRegistry {
    fn index_mut(&mut self, id: ComponentId) -> &mut Self::Output {
        let index = id.0 as usize;
        assert!(self.borrowed.borrow()[index].is_free());
        &mut self.entries[index]
    }
}

#[derive(Default)]
struct BorrowStatus(i16);

impl BorrowStatus {
    fn is_free(&self) -> bool {
        self.0 == 0
    }
    fn is_readable(&self) -> bool {
        self.0 >= 0
    }
    fn add_borrow(&mut self, mutable: bool) -> Result<(), ()> {
        if mutable {
            self.add_writer()
        } else {
            self.add_reader()
        }
    }
    fn add_reader(&mut self) -> Result<(), ()> {
        if self.is_readable() {
            self.0 += 1;
            Ok(())
        } else {
            Err(())
        }
    }
    fn add_writer(&mut self) -> Result<(), ()> {
        if self.is_free() {
            self.0 -= 1;
            Ok(())
        } else {
            Err(())
        }
    }
    fn remove_borrow(&mut self, mutable: bool) {
        if mutable {
            assert!(self.0 < 0);
            self.0 += 1;
        } else {
            assert!(self.0 > 0);
            self.0 -= 1;
        }
    }
}

impl fmt::Debug for BorrowStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 == 0 {
            write!(f, "BorrowStatus(free)")
        } else if self.0 > 0 {
            write!(f, "BorrowStatus({} readers)", self.0)
        } else if self.0 == -1 {
            write!(f, "BorrowStatus(one writer)")
        } else {
            write!(f, "BorrowStatus(invalid: {})", self.0)
        }
    }
}
