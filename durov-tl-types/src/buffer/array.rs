use std::ops::{Deref, DerefMut};
use std::{alloc, ptr, slice};

pub struct Array {
    ptr: *mut u8,
    len: usize,
}

unsafe impl Send for Array {}

unsafe impl Sync for Array {}

impl Array {
    pub fn new() -> Self {
        Self {
            ptr: ptr::dangling_mut(),
            len: 0,
        }
    }

    pub fn alloc(len: usize) -> Self {
        assert!(len > 0);

        let layout = Self::layout(len);
        let ptr = unsafe { alloc::alloc(layout) };

        if ptr.is_null() {
            alloc::handle_alloc_error(layout);
        }

        Self { ptr, len }
    }

    fn layout(len: usize) -> alloc::Layout {
        alloc::Layout::array::<u8>(len)
            .unwrap()
    }
}

impl Drop for Array {
    fn drop(&mut self) {
        if self.len > 0 {
            let layout = Self::layout(self.len);
            unsafe { alloc::dealloc(self.ptr, layout) };
        }
    }
}

impl Deref for Array {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl DerefMut for Array {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}
