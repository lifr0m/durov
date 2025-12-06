use crate::Error;
use crypto_bigint::BoxedUint;

pub fn serialize_bigint(num: &BoxedUint) -> Vec<u8> {
    num.to_be_bytes_trimmed_vartime()
        .into_vec()
}

pub fn serialize_bigint_padded(num: &BoxedUint) -> Vec<u8> {
    num.to_be_bytes()
        .into_vec()
}

pub fn deserialize_bigint(data: &[u8], bits: u32) -> Result<BoxedUint, Error> {
    Ok(BoxedUint::from_be_slice(data, bits)?)
}
