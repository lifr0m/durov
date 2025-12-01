#[cfg(feature = "fast-buf")]
mod array;

#[cfg(feature = "fast-buf")]
use array::Array;
use std::mem;
use std::ops::{Deref, DerefMut};

/// Capacity on first alloc.
const DEFAULT_CAPACITY: usize = 256;

/// Capacity multiplier on realloc.
const CAPACITY_FACTOR: usize = 4;

/// Capacity is divided by this number and compared with required length.
/// If the result is bigger than required length, there will be no realloc.
const CAPACITY_DIVIDER: usize = 2;

/// Contiguous deque buffer.
///
/// Content is placed between head and tail like this:
///
/// ```text
/// [ ... head ### tail ... ]
/// ```
///
/// Head and tail start at the same point - in the center of allocated vector.
/// Pushing data to front makes head go left.
/// Pushing data to back makes tail go right.
/// If one of them reaches it's end, buffer is reallocated.
///
/// If user for example pushes only to front and drains only from back,
/// head reaches it's end and data needs to be repositioned.
/// In such cases data can be too small to make new allocation so it's
/// just moved back to the center of vector without actually reallocating.
pub struct Buffer {
    #[cfg(not(feature = "fast-buf"))]
    data: Vec<u8>,
    #[cfg(feature = "fast-buf")]
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
    /// Create empty buffer.
    pub fn new() -> Self {
        Self {
            #[cfg(not(feature = "fast-buf"))]
            data: Vec::new(),
            #[cfg(feature = "fast-buf")]
            data: Array::new(),

            head: 0,
            tail: 0,
        }
    }

    /// Empty buffer.
    pub fn clear(&mut self) {
        self.head = self.capacity() / 2;
        self.tail = self.head;
    }

    /// Extend buffer by `add_len` bytes at back.
    ///
    /// Extended part is not guaranteed to be zeroed.
    pub fn resize_back(&mut self, add_len: usize) {
        self.reserve_back(add_len);
        self.tail += add_len;
    }

    /// Extend buffer by `add_len` bytes at front.
    ///
    /// Extended part is not guaranteed to be zeroed.
    pub fn resize_front(&mut self, add_len: usize) {
        self.reserve_front(add_len);
        self.head -= add_len;
    }

    /// Get owned slice from `start` to `start + N`.
    pub fn array<const N: usize>(&self, start: usize) -> [u8; N] {
        let mut arr = [0; N];
        arr.copy_from_slice(&self[start..start + N]);
        arr
    }

    /// Add byte to back.
    pub fn push_back(&mut self, byte: u8) {
        self.reserve_back(1);
        self.data[self.tail] = byte;
        self.tail += 1;
    }

    /// Add byte to front.
    pub fn push_front(&mut self, byte: u8) {
        self.reserve_front(1);
        self.head -= 1;
        self.data[self.head] = byte;
    }

    /// Extend back by `other`.
    pub fn extend_back(&mut self, other: &[u8]) {
        self.reserve_back(other.len());
        self.data[self.tail..self.tail + other.len()].copy_from_slice(other);
        self.tail += other.len();
    }

    /// Extend front by `other`.
    pub fn extend_front(&mut self, other: &[u8]) {
        self.reserve_front(other.len());
        self.head -= other.len();
        self.data[self.head..self.head + other.len()].copy_from_slice(other);
    }

    /// Remove `len` bytes from back.
    pub fn truncate_back(&mut self, len: usize) {
        assert!(self.len() >= len);
        self.tail -= len;
    }

    /// Remove `len` bytes from front.
    pub fn truncate_front(&mut self, len: usize) {
        assert!(self.len() >= len);
        self.head += len;
    }

    /// Ensure there are enough space for adding `len` bytes to back.
    fn reserve_back(&mut self, len: usize) {
        if self.need_realloc_back(len) {
            self.realloc(len);
        }
    }

    /// Ensure there are enough space for adding `len` bytes to front.
    fn reserve_front(&mut self, len: usize) {
        if self.need_realloc_front(len) {
            self.realloc(len);
        }
    }

    /// Determine whether we need realloc/reposition to add `len` bytes to front.
    fn need_realloc_front(&self, len: usize) -> bool {
        // head - len < 0
        len > self.head
    }

    /// Determine whether we need realloc/reposition to add `len` bytes to back.
    fn need_realloc_back(&self, len: usize) -> bool {
        // tail + len > capacity
        self.tail + len > self.capacity()
    }

    /// Reposition data in current vector to center or
    /// allocate new vector and copy data to it.
    ///
    /// It's ensured that new capacity will be enough to add
    /// `add_len` bytes to back or front.
    fn realloc(&mut self, add_len: usize) {
        let old_cap = self.capacity();
        let old_head = self.head;
        let old_tail = self.tail;
        let len = self.len();

        let required_len = add_len + len + add_len;

        let mut new_cap = match old_cap {
            0 => DEFAULT_CAPACITY,
            _ => old_cap,
        };
        while new_cap < required_len {
            new_cap *= CAPACITY_FACTOR;
        }

        self.head = new_cap / 2 - len / 2;
        self.tail = self.head + len;

        if new_cap == old_cap && required_len <= new_cap / CAPACITY_DIVIDER {
            self.data.copy_within(old_head..old_tail, self.head);
        } else {
            #[cfg(not(feature = "fast-buf"))]
            let new_data = vec![0; new_cap];
            #[cfg(feature = "fast-buf")]
            let new_data = Array::alloc(new_cap);

            let old_data = mem::replace(&mut self.data, new_data);
            self.copy_from_slice(&old_data[old_head..old_tail]);
        }
    }

    /// Max space that can be used without realloc or reposition.
    fn capacity(&self) -> usize {
        self.data.len()
    }
}

impl Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.data[self.head..self.tail]
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data[self.head..self.tail]
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
        buf.resize_back(3);
        buf.resize_front(4);
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
        buf.push_back(42);
        buf.push_front(33);
        assert_eq!(buf[..], [33, 42]);
    }

    #[test]
    fn test_extend() {
        let mut buf = Buffer::new();
        buf.extend_back(&[42, 33]);
        buf.extend_front(&[88, 99]);
        assert_eq!(buf[..], [88, 99, 42, 33]);
    }

    #[test]
    fn test_truncate() {
        let mut buf = Buffer::new();
        buf.extend_back(&[3, 2, 7, 9, 5]);
        buf.truncate_back(1);
        buf.truncate_front(2);
        assert_eq!(buf[..], [7, 9]);
    }

    #[test]
    fn test_reserve_back() {
        let mut buf = Buffer::new();
        buf.reserve_back(1834);
        assert!(buf.capacity() >= 1834 * 2);
    }

    #[test]
    fn test_reserve_front() {
        let mut buf = Buffer::new();
        buf.reserve_front(1834);
        assert!(buf.capacity() >= 1834 * 2);
    }

    #[test]
    fn test_need_realloc_back() {
        let mut buf = Buffer::new();
        assert!(buf.need_realloc_back(1193));
        buf.reserve_back(1193);
        assert!(!buf.need_realloc_back(1193));
    }

    #[test]
    fn test_need_realloc_front() {
        let mut buf = Buffer::new();
        assert!(buf.need_realloc_front(1193));
        buf.reserve_front(1193);
        assert!(!buf.need_realloc_front(1193));
    }

    #[test]
    fn test_realloc() {
        let mut buf = Buffer::new();
        buf.extend_front(&[1, 2]);
        buf.extend_back(&[3, 4]);
        buf.realloc(2714);
        assert!(buf.capacity() >= 4 + 2714 * 2);
        assert_eq!(buf[..], [1, 2, 3, 4]);
    }
}
