use crypto_bigint::Random;
use durov_tl_types::buffer::Buffer;

pub fn random_bigint<T: Random>() -> T {
    T::random(&mut rand::rng())
}

pub fn extend_random(buf: &mut Buffer, len: usize) {
    let start = buf.len();
    buf.resize_back(len);
    rand::fill(&mut buf[start..]);
}
