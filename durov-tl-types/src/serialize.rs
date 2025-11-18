use crate::utils::calc_pad_len;
use crypto_bigint::{U128, U256};

pub trait Serialize {
    fn serialize(&self, dst: &mut impl Extend<u8>);

    fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::new();
        self.serialize(&mut vec);
        vec
    }
}

impl Serialize for i32 {
    fn serialize(&self, dst: &mut impl Extend<u8>) {
        dst.extend(self.to_le_bytes());
    }
}

impl Serialize for i64 {
    fn serialize(&self, dst: &mut impl Extend<u8>) {
        dst.extend(self.to_le_bytes());
    }
}

impl Serialize for f64 {
    fn serialize(&self, dst: &mut impl Extend<u8>) {
        dst.extend(self.to_le_bytes());
    }
}

impl Serialize for str {
    fn serialize(&self, dst: &mut impl Extend<u8>) {
        self.as_bytes().serialize(dst);
    }
}

impl Serialize for String {
    fn serialize(&self, dst: &mut impl Extend<u8>) {
        self.as_str().serialize(dst);
    }
}

impl Serialize for [u8] {
    fn serialize(&self, dst: &mut impl Extend<u8>) {
        if self.len() <= 253 {
            dst.extend([self.len() as u8]);
            dst.extend(self.iter().copied());
            let pad_len = calc_pad_len(1 + self.len());
            (0..pad_len).for_each(|_| dst.extend([0]));
        } else {
            dst.extend([254]);
            dst.extend(self.len().to_le_bytes()[..3].iter().copied());
            dst.extend(self.iter().copied());
            let pad_len = calc_pad_len(4 + self.len());
            (0..pad_len).for_each(|_| dst.extend([0]));
        }
    }
}

impl Serialize for Vec<u8> {
    fn serialize(&self, dst: &mut impl Extend<u8>) {
        self.as_slice().serialize(dst);
    }
}

impl<T: Serialize> Serialize for [T] {
    fn serialize(&self, dst: &mut impl Extend<u8>) {
        dst.extend(crate::constants::VECTOR_ID.to_le_bytes());
        dst.extend((self.len() as i32).to_le_bytes());
        self.iter().for_each(|e| e.serialize(dst));
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn serialize(&self, dst: &mut impl Extend<u8>) {
        self.as_slice().serialize(dst);
    }
}

impl<T: Serialize> Serialize for crate::BareVec<T> {
    fn serialize(&self, dst: &mut impl Extend<u8>) {
        dst.extend((self.0.len() as i32).to_le_bytes());
        self.0.iter().for_each(|e| e.serialize(dst));
    }
}

impl Serialize for U128 {
    fn serialize(&self, dst: &mut impl Extend<u8>) {
        dst.extend(self.to_le_bytes());
    }
}

impl Serialize for U256 {
    fn serialize(&self, dst: &mut impl Extend<u8>) {
        dst.extend(self.to_le_bytes());
    }
}

impl<T: Serialize> Serialize for Box<T> {
    fn serialize(&self, dst: &mut impl Extend<u8>) {
        self.as_ref().serialize(dst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_bytes() {
        assert_eq!(Vec::<u8>::from([]).to_bytes(), [0, 0, 0, 0]);
        assert_eq!(Vec::<u8>::from([42]).to_bytes(), [1, 42, 0, 0]);
        assert_eq!(Vec::<u8>::from_iter(1..255).to_bytes(), [
            vec![254, 254, 0, 0],
            (1..255).collect(),
            vec![0, 0],
        ].concat());
    }
}
