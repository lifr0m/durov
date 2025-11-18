pub fn calc_pad_len(len: usize) -> usize {
    (4 - len % 4) % 4
}
