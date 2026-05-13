pub use crate::primitives::*;

pub fn compute_auth_key_id(auth_key: &[u8]) -> i64 {
    let hash = sha1([auth_key]);
    let data = make_arr([&hash[12..]]);
    i64::from_le_bytes(data)
}

pub fn compute_msg_key(
    auth_key: &[u8],
    direction: Direction,
    plaintext_with_padding: &[u8],
) -> [u8; 16] {
    let x = match direction {
        Direction::ClientToServer => 0,
        Direction::ServerToClient => 8,
    };

    let msg_key_large = sha256([
        sub_str(auth_key, 88 + x, 32),
        plaintext_with_padding,
    ]);
    make_arr([
        sub_str(&msg_key_large, 8, 16),
    ])
}

pub fn compute_aes_key_iv(
    auth_key: &[u8],
    msg_key: &[u8],
    direction: Direction,
) -> ([u8; 32], [u8; 32]) {
    let x = match direction {
        Direction::ClientToServer => 0,
        Direction::ServerToClient => 8,
    };

    let sha256_a = sha256([
        msg_key,
        sub_str(auth_key, x, 36),
    ]);
    let sha256_b = sha256([
        sub_str(auth_key, 40 + x, 36),
        msg_key,
    ]);
    let aes_key = make_arr([
        sub_str(&sha256_a, 0, 8),
        sub_str(&sha256_b, 8, 16),
        sub_str(&sha256_a, 24, 8),
    ]);
    let aes_iv = make_arr([
        sub_str(&sha256_b, 0, 8),
        sub_str(&sha256_a, 8, 16),
        sub_str(&sha256_b, 24, 8),
    ]);

    (aes_key, aes_iv)
}
