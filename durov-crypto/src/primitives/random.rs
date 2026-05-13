use crypto_bigint::{BoxedUint, RandomBits};
use durov_tl_types::buffer::Buffer;

pub fn random_bigint(bits: u32) -> BoxedUint {
    BoxedUint::random_bits(&mut rand::rng(), bits)
}

pub fn extend_random(buf: &mut Buffer, len: usize) {
    let start = buf.len();
    buf.resize_back(len);
    rand::fill(&mut buf[start..]);
}
