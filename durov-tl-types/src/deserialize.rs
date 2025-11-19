use crate::cursor::Cursor;
use crate::utils::calc_pad_len;
use crate::BareVec;
use crypto_bigint::{Encoding, I128, I256, U128, U256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("read: {0}")]
    Read(#[from] crate::cursor::Error),

    #[error("string decode: {0}")]
    StringDecode(#[from] std::string::FromUtf8Error),

    #[error("mismatching id: expected {expected}, received {received}")]
    IdMismatch {
        expected: i32,
        received: i32,
    },

    #[error("unknown id: {0}")]
    UnknownId(i32),
}

pub trait Deserialize
where
    Self: Sized,
{
    fn deserialize(src: &mut Cursor) -> Result<Self, Error>;

    fn from_bytes(src: &[u8]) -> Result<Self, Error> {
        let mut cur = Cursor::new(src);
        Self::deserialize(&mut cur)
    }
}

impl Deserialize for i32 {
    fn deserialize(src: &mut Cursor) -> Result<Self, Error> {
        let mut val = [0; 4];
        src.read(&mut val)?;
        Ok(Self::from_le_bytes(val))
    }
}

impl Deserialize for i64 {
    fn deserialize(src: &mut Cursor) -> Result<Self, Error> {
        let mut val = [0; 8];
        src.read(&mut val)?;
        Ok(Self::from_le_bytes(val))
    }
}

impl Deserialize for f64 {
    fn deserialize(src: &mut Cursor) -> Result<Self, Error> {
        let mut val = [0; 8];
        src.read(&mut val)?;
        Ok(Self::from_le_bytes(val))
    }
}

impl Deserialize for String {
    fn deserialize(src: &mut Cursor) -> Result<Self, Error> {
        let data = Vec::<u8>::deserialize(src)?;
        Ok(String::from_utf8(data)?)
    }
}

impl Deserialize for Vec<u8> {
    fn deserialize(src: &mut Cursor) -> Result<Self, Error> {
        let mut val = [0; 1];
        src.read(&mut val)?;

        if val[0] <= 253 {
            let len = val[0] as usize;

            let mut val = vec![0; len];
            src.read(&mut val)?;

            let pad_len = calc_pad_len(1 + len);
            let mut trash = [0; 1];
            for _ in 0..pad_len {
                src.read(&mut trash)?;
            }

            Ok(val)
        } else {
            let mut len = [0; 8];
            src.read(&mut len[..3])?;
            let len = usize::from_le_bytes(len);

            let mut val = vec![0; len];
            src.read(&mut val)?;

            let pad_len = calc_pad_len(4 + len);
            let mut trash = [0; 1];
            for _ in 0..pad_len {
                src.read(&mut trash)?;
            }

            Ok(val)
        }
    }
}

impl<T: Deserialize> Deserialize for Vec<T> {
    fn deserialize(src: &mut Cursor) -> Result<Self, Error> {
        let mut id = [0; 4];
        src.read(&mut id)?;
        let id = i32::from_le_bytes(id);

        if id != crate::constants::VECTOR_ID {
            return Err(Error::IdMismatch {
                expected: crate::constants::VECTOR_ID,
                received: id,
            });
        }

        let mut len = [0; 4];
        src.read(&mut len)?;
        let len = i32::from_le_bytes(len) as usize;

        (0..len)
            .map(|_| T::deserialize(src))
            .collect()
    }
}

impl<T: Deserialize> Deserialize for BareVec<T> {
    fn deserialize(src: &mut Cursor) -> Result<Self, Error> {
        let mut len = [0; 4];
        src.read(&mut len)?;
        let len = i32::from_le_bytes(len) as usize;

        (0..len)
            .map(|_| T::deserialize(src))
            .collect::<Result<_, _>>()
            .map(|vec| BareVec(vec))
    }
}

impl Deserialize for I128 {
    fn deserialize(src: &mut Cursor) -> Result<Self, Error> {
        let mut val = [0; 16];
        src.read(&mut val)?;
        Ok(*U128::from_le_bytes(val).as_int())
    }
}

impl Deserialize for I256 {
    fn deserialize(src: &mut Cursor) -> Result<Self, Error> {
        let mut val = [0; 32];
        src.read(&mut val)?;
        Ok(*U256::from_le_bytes(val).as_int())
    }
}

impl<T: Deserialize> Deserialize for Box<T> {
    fn deserialize(src: &mut Cursor) -> Result<Self, Error> {
        let val = T::deserialize(src)?;
        Ok(Box::new(val))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_bytes() {
        assert_eq!(Vec::<u8>::from_bytes(&[0, 0, 0, 0]).unwrap(), []);
        assert_eq!(Vec::<u8>::from_bytes(&[1, 42, 0, 0]).unwrap(), [42]);
        assert_eq!(Vec::<u8>::from_bytes(&[
            vec![254, 254, 0, 0],
            (1..255).collect(),
            vec![0, 0],
        ].concat()).unwrap(), Vec::from_iter(1..255));
    }
}
