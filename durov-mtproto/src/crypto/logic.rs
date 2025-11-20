pub fn xor(a: &mut [u8], b: &[u8]) {
    for (x, y) in a.iter_mut().zip(b) {
        *x ^= y;
    }
}

pub fn xor_new<const L: usize>(a: &[u8], b: &[u8]) -> [u8; L] {
    let mut r = [0; L];
    r.copy_from_slice(a);
    xor(&mut r, b);
    r
}
