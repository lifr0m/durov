pub fn sub_str(data: &[u8], index: usize, len: usize) -> &[u8] {
    &data[index..index + len]
}

pub fn sub_str_mut(data: &mut [u8], index: usize, len: usize) -> &mut [u8] {
    &mut data[index..index + len]
}

pub fn make_arr<const L: usize, const N: usize>(data: [&[u8]; N]) -> [u8; L] {
    let mut arr = [0; L];
    let mut pos = 0;
    for elem in data {
        sub_str_mut(&mut arr, pos, elem.len())
            .copy_from_slice(elem);
        pos += elem.len();
    }
    arr
}

pub fn make_vec<const N: usize>(data: [&[u8]; N]) -> Vec<u8> {
    let mut vec = Vec::new();
    for elem in data {
        vec.extend(elem);
    }
    vec
}

pub fn calc_pad_len(len: usize, step: usize) -> usize {
    (step - len % step) % step
}
