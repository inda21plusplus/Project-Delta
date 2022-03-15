use dense_bitset::BitSet;
use std::{
    alloc::{self, Layout},
    fmt, mem,
    ptr::NonNull,
};

use crate::Entity;

#[derive(Debug)]
pub enum Storage {
    VecStorage(VecStorage),
}

#[derive(Debug)]
pub enum StorageType {
    VecStorage,
}

impl Storage {
    pub fn new<T: 'static>(storage_type: StorageType) -> Self {
        unsafe fn drop_ptr<T>(ptr: *mut u8) {
            ptr.cast::<T>().drop_in_place();
        }

        match storage_type {
            StorageType::VecStorage => {
                Self::VecStorage(VecStorage::new(Layout::new::<T>(), drop_ptr::<T>))
            }
        }
    }

    // Returns true if `entity` previously had this kind of component.
    pub fn set<T>(&mut self, entity: Entity, mut value: T) -> bool {
        let res = unsafe { self.set_ptr(entity, ((&mut value) as *mut T).cast()) };
        mem::forget(value);
        res
    }

    // Returns true if `entity` previously had this kind of component.
    // #SAFETY:
    // The value pointed to by `ptr` must not be freed by the caller.
    pub unsafe fn set_ptr(&mut self, entity: Entity, ptr: *mut u8) -> bool {
        let index = entity.id as usize;
        match self {
            Self::VecStorage(s) => s.set(index, ptr),
        }
    }

    pub fn remove(&mut self, entity: Entity) -> bool {
        let index = entity.id as usize;
        match self {
            Self::VecStorage(s) => s.remove(index),
        }
    }

    pub fn get<T>(&self, entity: Entity) -> Option<&T> {
        let index = entity.id as usize;
        unsafe {
            match self {
                Self::VecStorage(s) => s.get(index).cast::<T>().as_ref(),
            }
        }
    }

    pub fn get_mut<T>(&mut self, entity: Entity) -> Option<&mut T> {
        let index = entity.id as usize;
        unsafe {
            match self {
                Self::VecStorage(s) => s.get_mut(index).map(|p| p.cast::<T>().as_mut()),
            }
        }
    }
}

pub struct VecStorage {
    occupied: BitSet,
    item_layout: Layout,
    drop: unsafe fn(*mut u8),
    cap: usize,
    // Is dangling when `cap` is zero. Points to an allocated buffer of `cap` *
    // `layout.size()`
    ptr: NonNull<u8>,
}

impl VecStorage {
    fn new(item_layout: Layout, drop: unsafe fn(*mut u8)) -> Self {
        Self {
            occupied: BitSet::default(),
            item_layout,
            drop,
            cap: 0,
            ptr: NonNull::dangling(),
        }
    }

    /// `self` effectively takes ownership over the value pointed to by `value` and should not be
    /// freed by the caller. Returns true if there was a component at `index` before.
    unsafe fn set(&mut self, index: usize, value: *mut u8) -> bool {
        self.ensure_capacity(index + 1);

        let res = self.remove(index);
        self.get_mut_unchecked(index)
            .as_ptr()
            .copy_from_nonoverlapping(value, self.item_layout.size());
        self.occupied.insert(index);
        res
    }

    /// Returns true if the component was removed
    fn remove(&mut self, index: usize) -> bool {
        if !self.occupied.get(index) {
            return false;
        }
        unsafe {
            (self.drop)(self.get_mut_unchecked(index).as_ptr());
        }
        self.occupied.remove(index);
        true
    }

    fn ensure_capacity(&mut self, cap: usize) {
        if self.cap >= cap {
            return;
        }
        let cap = cap.next_power_of_two();
        let curr_layout = self.layout_with_cap(self.cap);
        let new_layout = self.layout_with_cap(cap);
        let new_data = unsafe {
            if self.cap == 0 {
                alloc::alloc(new_layout)
            } else {
                alloc::realloc(self.ptr.as_ptr(), curr_layout, new_layout.size())
            }
        };
        self.ptr = NonNull::new(new_data).expect("Failed to allocate component array");
        self.cap = cap;
    }

    fn clear(&mut self) {
        while self.cap > 0 {
            self.cap -= 1;
            let i = self.cap;
            self.remove(i);
        }
    }

    /// Returns a null pointer if nothing exists as `index`
    fn get(&self, index: usize) -> *const u8 {
        let offset = self.offset();
        if self.occupied.get(index) {
            unsafe { (self.ptr.as_ptr() as *const u8).add(index * offset) }
        } else {
            0 as *const u8
        }
    }

    /// May be dangling
    fn get_mut_unchecked(&mut self, index: usize) -> NonNull<u8> {
        let offset = self.offset();
        unsafe { NonNull::new_unchecked(self.ptr.as_ptr().add(index * offset)) }
    }

    /// Returns `None` if nothing exists at `index`
    fn get_mut(&mut self, index: usize) -> Option<NonNull<u8>> {
        if self.occupied.get(index) {
            Some(self.get_mut_unchecked(index))
        } else {
            None
        }
    }

    fn layout_with_cap(&self, cap: usize) -> Layout {
        repeat(&self.item_layout, cap).expect("Failed to get memory layout of components")
    }

    fn offset(&self) -> usize {
        self.item_layout.size() + padding_needed_for(&self.item_layout, self.item_layout.align())
    }
}

impl Drop for VecStorage {
    fn drop(&mut self) {
        self.clear();
        let layout = self.layout_with_cap(self.cap);
        if layout.size() == 0 {
            return;
        }
        unsafe {
            alloc::dealloc(self.ptr.as_ptr(), layout);
        }
    }
}

impl fmt::Debug for VecStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VecStorage {{ {} items }}",
            self.occupied.element_count()
        )
    }
}

// TODO: replace these with the methods on `Layout` when those become stable

// From: https://doc.rust-lang.org/src/core/alloc/layout.rs.html#299
fn repeat(layout: &Layout, n: usize) -> Option<Layout> {
    // This cannot overflow. Quoting from the invariant of Layout:
    // > `size`, when rounded up to the nearest multiple of `align`,
    // > must not overflow (i.e., the rounded value must be less than
    // > `usize::MAX`)
    let padded_size = layout.size() + padding_needed_for(layout, layout.align());
    let alloc_size = padded_size.checked_mul(n)?;

    // SAFETY: layout.align is already known to be valid and alloc_size has been
    // padded already.
    unsafe {
        Some(Layout::from_size_align_unchecked(
            alloc_size,
            layout.align(),
        ))
    }
}

// From: https://doc.rust-lang.org/src/core/alloc/layout.rs.html#241
const fn padding_needed_for(layout: &Layout, align: usize) -> usize {
    let len = layout.size();

    let len_rounded_up = len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
    len_rounded_up.wrapping_sub(len)
}
