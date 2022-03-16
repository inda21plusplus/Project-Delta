use dense_bitset::BitSet;
use std::{
    alloc::{self, Layout},
    fmt,
    mem::{self, MaybeUninit},
    ptr::{self, NonNull},
};

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

    /// Returns true if `entity` previously had this kind of component.
    /// # Safety
    /// `Self` must contain `T`s
    pub unsafe fn set<T>(&mut self, index: usize, mut value: T) -> bool {
        let res = self.set_ptr(index, ((&mut value) as *mut T).cast());
        mem::forget(value);
        res
    }

    /// Returns true if `entity` previously had this kind of component.
    /// # Safety
    /// The value pointed to by `ptr` must not be a valid value for the type `self` stores.
    /// It must *not* freed by the caller.
    pub unsafe fn set_ptr(&mut self, index: usize, ptr: *mut u8) -> bool {
        match self {
            Self::VecStorage(s) => s.set(index, ptr),
        }
    }

    pub fn unset(&mut self, index: usize) -> bool {
        match self {
            Self::VecStorage(s) => s.unset(index),
        }
    }

    /// # Safety
    /// `Self` must be a storage for `T`s
    pub unsafe fn remove<T: 'static>(&mut self, index: usize) -> Option<T> {
        match self {
            Self::VecStorage(s) => s.remove(index),
        }
    }

    /// # Safety
    /// `Self` must be a storage for `T`s
    pub unsafe fn get<T>(&self, index: usize) -> Option<&T> {
        self.get_ptr(index).cast::<T>().as_ref()
    }

    /// # Safety
    /// `Self` must be a storage for `T`s
    pub unsafe fn get_mut<T>(&mut self, index: usize) -> Option<&mut T> {
        self.get_mut_ptr(index).cast::<T>().as_mut()
    }

    /// Returns null if nothing exists at `index`
    pub fn get_ptr(&self, index: usize) -> *const u8 {
        match self {
            Self::VecStorage(s) => s.get(index),
        }
    }

    /// Returns null if nothing exists at `index`
    pub fn get_mut_ptr(&mut self, index: usize) -> *mut u8 {
        match self {
            Self::VecStorage(s) => s.get_mut(index),
        }
    }
}

pub struct VecStorage {
    item_layout: Layout,
    drop: unsafe fn(*mut u8),
    cap: usize,
    // Is dangling when `cap * layout.size()` is zero. Points to an allocated buffer of
    // `cap * layout.size()` bytes otherwise.
    ptr: NonNull<u8>,
    // # Safety
    // May never contain any index `>= cap`
    occupied: BitSet,
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

        let res = self.unset(index);
        self.get_mut_unchecked(index)
            .copy_from_nonoverlapping(value, self.item_layout.size());
        self.occupied.insert(index);
        res
    }

    /// Runs the destructor for the component marks it as not occupied.
    /// Returns true if the component was removed.
    fn unset(&mut self, index: usize) -> bool {
        if !self.occupied.get(index) {
            return false;
        }
        self.occupied.remove(index);
        unsafe {
            (self.drop)(self.get_mut_unchecked(index));
        }
        true
    }

    /// Take out the component from `Self`. Does not run its destructor.
    /// # Safety
    /// `Self` must contain `T`s
    unsafe fn remove<T: 'static>(&mut self, index: usize) -> Option<T> {
        if !self.occupied.get(index) {
            return None;
        }
        self.occupied.remove(index);
        let ptr = self.get_unchecked(index).cast::<T>();
        let mut res: MaybeUninit<T> = MaybeUninit::uninit();
        res.as_mut_ptr().copy_from(ptr, 1);
        Some(res.assume_init())
    }

    /// Panics on allocation failiure.
    fn ensure_capacity(&mut self, cap: usize) {
        let old_cap = self.cap;
        if old_cap >= cap {
            return;
        }
        let cap = cap.next_power_of_two();
        self.cap = cap;
        let curr_layout = self.layout_with_cap(old_cap);
        let new_layout = self.layout_with_cap(cap);
        if new_layout.size() == 0 {
            return;
        }
        let new_data = unsafe {
            if old_cap == 0 {
                alloc::alloc(new_layout)
            } else {
                alloc::realloc(self.ptr.as_ptr(), curr_layout, new_layout.size())
            }
        };
        self.ptr = NonNull::new(new_data).expect("Failed to allocate component array");
    }

    fn clear(&mut self) {
        for i in 0..self.cap {
            self.unset(i);
        }
    }

    /// May be dangling but never null
    /// # Safety
    /// If `index >= self.cap` the result is undefined behaviour
    unsafe fn get_unchecked(&self, index: usize) -> *const u8 {
        (self.ptr.as_ptr() as *const u8).add(index * self.offset())
    }

    /// May be dangling but never null
    /// # Safety
    /// If `index >= self.cap` the result is undefined behaviour
    unsafe fn get_mut_unchecked(&mut self, index: usize) -> *mut u8 {
        self.ptr.as_ptr().add(index * self.offset())
    }

    /// Returns a null pointer if nothing exists as `index`
    fn get(&self, index: usize) -> *const u8 {
        self.occupied
            .get(index)
            .then(|| unsafe { self.get_unchecked(index) })
            .unwrap_or(ptr::null())
    }

    /// Returns a null pointer if nothing exists at `index`
    fn get_mut(&mut self, index: usize) -> *mut u8 {
        self.occupied
            .get(index)
            .then(|| unsafe { self.get_mut_unchecked(index) })
            .unwrap_or(ptr::null_mut())
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
