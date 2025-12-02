use crate::buffer::Buffer;
use crate::constants::{FALSE_ID, TRUE_ID, VECTOR_ID};
use crate::utils::calc_pad_len;
use crate::BareVec;
use crypto_bigint::{I128, I256};
use std::sync::Arc;

pub trait Serialize {
    fn serialize(&self, dst: &mut Buffer);

    fn to_bytes(&self) -> Buffer {
        let mut buf = Buffer::new();
        self.serialize(&mut buf);
        buf
    }
}

impl<T: ?Sized + Serialize> Serialize for &T {
    fn serialize(&self, dst: &mut Buffer) {
        (*self).serialize(dst);
    }
}

impl Serialize for bool {
    fn serialize(&self, dst: &mut Buffer) {
        if *self {
            TRUE_ID.serialize(dst);
        } else {
            FALSE_ID.serialize(dst);
        }
    }
}

impl Serialize for i32 {
    fn serialize(&self, dst: &mut Buffer) {
        dst.extend_back(&self.to_le_bytes());
    }
}

impl Serialize for i64 {
    fn serialize(&self, dst: &mut Buffer) {
        dst.extend_back(&self.to_le_bytes());
    }
}

impl Serialize for f64 {
    fn serialize(&self, dst: &mut Buffer) {
        dst.extend_back(&self.to_le_bytes());
    }
}

impl Serialize for str {
    fn serialize(&self, dst: &mut Buffer) {
        self.as_bytes().serialize(dst);
    }
}

impl Serialize for String {
    fn serialize(&self, dst: &mut Buffer) {
        self.as_str().serialize(dst);
    }
}

impl<const N: usize> Serialize for [u8; N] {
    fn serialize(&self, dst: &mut Buffer) {
        dst.extend_back(self);
    }
}

impl Serialize for [u8] {
    fn serialize(&self, dst: &mut Buffer) {
        if self.len() <= 253 {
            dst.push_back(self.len() as u8);
            dst.extend_back(self);
            let pad_len = calc_pad_len(1 + self.len());
            let start = dst.len();
            dst.resize_back(pad_len);
            dst[start..].fill(0);
        } else {
            dst.push_back(254);
            dst.extend_back(&self.len().to_le_bytes()[..3]);
            dst.extend_back(self);
            let pad_len = calc_pad_len(4 + self.len());
            let start = dst.len();
            dst.resize_back(pad_len);
            dst[start..].fill(0);
        }
    }
}

impl Serialize for Vec<u8> {
    fn serialize(&self, dst: &mut Buffer) {
        self.as_slice().serialize(dst);
    }
}

impl<T: Serialize> Serialize for [T] {
    fn serialize(&self, dst: &mut Buffer) {
        VECTOR_ID.serialize(dst);
        (self.len() as i32).serialize(dst);
        self.iter().for_each(|e| e.serialize(dst));
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn serialize(&self, dst: &mut Buffer) {
        self.as_slice().serialize(dst);
    }
}

impl<T: Serialize> Serialize for BareVec<T> {
    fn serialize(&self, dst: &mut Buffer) {
        (self.0.len() as i32).serialize(dst);
        self.0.iter().for_each(|e| e.serialize(dst));
    }
}

impl Serialize for I128 {
    fn serialize(&self, dst: &mut Buffer) {
        dst.extend_back(&self.as_uint().to_le_bytes());
    }
}

impl Serialize for I256 {
    fn serialize(&self, dst: &mut Buffer) {
        dst.extend_back(&self.as_uint().to_le_bytes());
    }
}

impl<T: ?Sized + Serialize> Serialize for Box<T> {
    fn serialize(&self, dst: &mut Buffer) {
        self.as_ref().serialize(dst);
    }
}

impl<T: ?Sized + Serialize> Serialize for Arc<T> {
    fn serialize(&self, dst: &mut Buffer) {
        self.as_ref().serialize(dst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_bytes() {
        assert_eq!(Vec::<u8>::from([]).to_bytes()[..], [0, 0, 0, 0]);
        assert_eq!(Vec::<u8>::from([42]).to_bytes()[..], [1, 42, 0, 0]);
        assert_eq!(Vec::<u8>::from_iter(1..255).to_bytes()[..], [
            vec![254, 254, 0, 0],
            (1..255).collect(),
            vec![0, 0],
        ].concat());
    }
}
