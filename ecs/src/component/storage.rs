use dense_bitset::BitSet;
use std::{alloc::Layout, mem::MaybeUninit};

use crate::Entity;

#[derive(Debug)]
pub struct Storage {
    occupied: BitSet,

    layout: Layout,
    drop: unsafe fn(*mut u8),

    cap: usize,
    len: usize,
    // Is uninitialized when `cap` is zero. Points to an allocated buffer of `cap` *
    // `layout.size()`
    ptr: MaybeUninit<*mut u8>,
}

unsafe fn drop_ptr<T>(ptr: *mut u8) {
    ptr.cast::<T>().drop_in_place();
}

impl Storage {
    pub fn new<T>() -> Self {
        unsafe {
            Self {
                occupied: BitSet::default(),

                layout: Layout::new::<T>(),
                drop: drop_ptr::<T>,

                cap: 0,
                len: 0,
                ptr: MaybeUninit::uninit(),
            }
        }
    }

    pub fn set<T>(&mut self, entity: Entity, component: T) {
        let index = entity.id as usize;
        assert!(!self.occupied.get(index));
        self.occupied.insert(index);

        self.ensure_capacity(index);
    }

    pub fn ensure_capacity(&mut self, cap: usize) {
        if self.cap >= cap {
            return;
        }
        let cap = cap.next_power_of_two();
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        todo!()
    }
}
