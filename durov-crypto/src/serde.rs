use crypto_bigint::BoxedUint;

pub fn serialize_boxed_bigint(n: &BoxedUint) -> Vec<u8> {
    n.to_be_bytes_trimmed_vartime()
        .into_vec()
}
