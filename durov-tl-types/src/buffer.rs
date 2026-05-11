mod array;

use array::Array;
use bytes::buf::UninitSlice;
use bytes::BufMut;
use std::ops::{Deref, DerefMut};
use std::{ptr, slice};

const DEFAULT_CAPACITY: usize = 256;

const CAPACITY_FACTOR: usize = 4;

pub struct Buffer {
    data: Array,
    head: usize,
    tail: usize,
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            data: Array::alloc(0),
            head: 0,
            tail: 0,
        }
    }

    pub fn clear(&mut self) {
        self.head = self.cap() / 2;
        self.tail = self.cap() / 2;
    }

    pub fn resize_front(&mut self, len: usize) {
        self.reserve_front(len);
        unsafe { ptr::write_bytes(self.head_ptr().sub(len), 0, len) };
        self.head -= len;
    }

    pub fn resize_back(&mut self, len: usize) {
        self.reserve_back(len);
        unsafe { ptr::write_bytes(self.tail_ptr(), 0, len) };
        self.tail += len;
    }

    pub fn array<const N: usize>(&self, start: usize) -> [u8; N] {
        let mut arr = [0; N];
        arr.copy_from_slice(&self[start..start + N]);
        arr
    }

    pub fn push_front(&mut self, byte: u8) {
        self.reserve_front(1);
        unsafe { ptr::write(self.head_ptr().sub(1), byte) };
        self.head -= 1;
    }

    pub fn push_back(&mut self, byte: u8) {
        self.reserve_back(1);
        unsafe { ptr::write(self.tail_ptr(), byte) };
        self.tail += 1;
    }

    pub fn extend_front(&mut self, data: &[u8]) {
        self.reserve_front(data.len());
        unsafe { ptr::copy_nonoverlapping(data.as_ptr(), self.head_ptr().sub(data.len()), data.len()) };
        self.head -= data.len();
    }

    pub fn extend_back(&mut self, data: &[u8]) {
        self.reserve_back(data.len());
        unsafe { ptr::copy_nonoverlapping(data.as_ptr(), self.tail_ptr(), data.len()) };
        self.tail += data.len();
    }

    pub fn truncate_front(&mut self, len: usize) {
        assert!(len <= self.len());
        self.head += len;
    }

    pub fn truncate_back(&mut self, len: usize) {
        assert!(len <= self.len());
        self.tail -= len;
    }

    fn reserve_front(&mut self, len: usize) {
        if len > self.spare_cap_front() {
            self.grow(len);
        }
    }

    fn reserve_back(&mut self, len: usize) {
        if len > self.spare_cap_back() {
            self.grow(len);
        }
    }

    fn grow(&mut self, len: usize) {
        let min_cap = len + self.len() + len;

        let mut cap = match self.cap() {
            0 => DEFAULT_CAPACITY,
            _ => self.cap(),
        };
        while cap < min_cap {
            cap *= CAPACITY_FACTOR;
        }

        let data = Array::alloc(cap);
        let head = data.len() / 2 - self.len() / 2;
        let tail = data.len() / 2 + self.len().div_ceil(2);

        unsafe { ptr::copy_nonoverlapping(self.head_ptr(), data.ptr().add(head), self.len()) };

        *self = Self { data, head, tail };
    }

    fn head_ptr(&self) -> *mut u8 {
        unsafe { self.ptr().add(self.head) }
    }

    fn tail_ptr(&self) -> *mut u8 {
        unsafe { self.ptr().add(self.tail) }
    }

    fn spare_cap_front(&self) -> usize {
        self.head
    }

    fn spare_cap_back(&self) -> usize {
        self.cap() - self.tail
    }

    fn ptr(&self) -> *mut u8 {
        self.data.ptr()
    }

    fn cap(&self) -> usize {
        self.data.len()
    }
}

impl Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.head_ptr(), self.tail - self.head) }
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { slice::from_raw_parts_mut(self.head_ptr(), self.tail - self.head) }
    }
}

unsafe impl BufMut for Buffer {
    fn remaining_mut(&self) -> usize {
        usize::MAX
    }

    unsafe fn advance_mut(&mut self, cnt: usize) {
        if self.spare_cap_back() < cnt {
            panic!("advance out of bounds: the len is {} but advancing by {}", self.spare_cap_back(), cnt);
        }
        self.tail += cnt;
    }

    fn chunk_mut(&mut self) -> &mut UninitSlice {
        if self.spare_cap_back() == 0 {
            self.grow(64);
        }
        unsafe { UninitSlice::from_raw_parts_mut(self.tail_ptr(), self.spare_cap_back()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let buf = Buffer::new();
        assert!(buf.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut buf = Buffer::new();
        buf.extend_back(&[3, 4, 1]);
        buf.clear();
        assert!(buf.is_empty());
    }

    #[test]
    fn test_resize() {
        let mut buf = Buffer::new();
        buf.resize_front(3);
        buf.resize_back(4);
        assert_eq!(buf.len(), 7);
    }

    #[test]
    fn test_array() {
        let mut buf = Buffer::new();
        buf.extend_back(&[42, 55, 33, 99]);
        assert_eq!(buf.array(1), [55, 33]);
    }

    #[test]
    fn test_push() {
        let mut buf = Buffer::new();
        buf.push_front(42);
        buf.push_back(33);
        assert_eq!(buf[..], [42, 33]);
    }

    #[test]
    fn test_extend() {
        let mut buf = Buffer::new();
        buf.extend_front(&[42, 33]);
        buf.extend_back(&[88, 99]);
        assert_eq!(buf[..], [42, 33, 88, 99]);
    }

    #[test]
    fn test_truncate() {
        let mut buf = Buffer::new();
        buf.extend_back(&[3, 2, 7, 9, 5]);
        buf.truncate_front(1);
        buf.truncate_back(2);
        assert_eq!(buf[..], [2, 7]);
    }

    #[test]
    fn test_grow() {
        let mut buf = Buffer::new();
        buf.extend_front(&[1, 2]);
        buf.extend_back(&[3, 4, 5]);
        buf.grow(2714);
        assert!(buf.spare_cap_front() >= 2714);
        assert!(buf.spare_cap_back() >= 2714);
        assert_eq!(buf[..], [1, 2, 3, 4, 5]);
    }
}
