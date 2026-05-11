use std::{alloc, ptr};

pub struct Array {
    ptr: *mut u8,
    len: usize,
}

unsafe impl Send for Array {}

unsafe impl Sync for Array {}

impl Array {
    pub fn alloc(size: usize) -> Self {
        Self {
            ptr: alloc(size),
            len: size,
        }
    }

    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl Drop for Array {
    fn drop(&mut self) {
        dealloc(self.ptr, self.len);
    }
}

fn alloc(size: usize) -> *mut u8 {
    if size == 0 {
        return ptr::dangling_mut();
    }

    let layout = alloc::Layout::array::<u8>(size)
        .unwrap();
    let ptr = unsafe { alloc::alloc(layout) };

    if ptr.is_null() {
        alloc::handle_alloc_error(layout);
    }

    ptr
}

fn dealloc(ptr: *mut u8, size: usize) {
    if size == 0 {
        return;
    }

    let layout = alloc::Layout::array::<u8>(size)
        .unwrap();
    unsafe { alloc::dealloc(ptr, layout) };
}
