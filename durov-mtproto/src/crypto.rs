mod utils;
mod hashes;
mod logic;
mod ciphers;
mod primes;
mod modular;

use crate::tl;
pub use ciphers::*;
use crypto_bigint::{BoxedUint, Odd, Random, I128, I256, U2048, U64};
use crypto_primes::Flavor;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::serialize::Serialize;
pub use hashes::*;
pub use logic::*;
pub use modular::*;
pub use primes::*;
use rsa::traits::PublicKeyParts;
use thiserror::Error;
pub use utils::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unexpected pq size: {0} bytes")]
    UnexpectedPqSize(usize),

    #[error("pq is prime: {0}")]
    PrimePq(i64),

    #[error("rsa_pad data argument is too long: {0} bytes")]
    RsaPadDataTooLong(usize),

    #[error("rsa: {0}")]
    Rsa(#[from] rsa::Error),

    #[error("invalid server dh inner data")]
    InvalidServerDhInnerData,

    #[error("answer hash mismatch: expected {expected:?}, received {received:?}")]
    AnswerHashMismatch {
        expected: [u8; 20],
        received: [u8; 20],
    },

    #[error("invalid dh prime: {0:?}")]
    InvalidDhPrime(Vec<u8>),

    #[error("unsafe prime: {0}")]
    UnsafeDhPrime(Box<U2048>),

    #[error("invalid dh g: {0}")]
    InvalidDhG(i32),

    #[error("unsafe dh prime {p} on g {g}")]
    UnsafeDhPrimeOnG {
        p: Box<U2048>,
        g: i32,
    },

    #[error("invalid g_a: {0:?}")]
    InvalidGa(Vec<u8>),

    #[error("dh extra check 1 failed: p {p}, n {n}")]
    DhExtraCheck1Failed {
        p: Box<U2048>,
        n: Box<U2048>,
    },

    #[error("dh extra check 2 failed on: p {p}, n {n}")]
    DhExtraCheck2Failed {
        p: Box<U2048>,
        n: Box<U2048>,
    },
}

pub enum Direction {
    ClientToServer,
    ServerToClient,
}

pub fn random_bigint<T: Random>() -> T {
    T::random(&mut rand::rng())
}

fn serialize_boxed_bigint(n: &BoxedUint) -> Vec<u8> {
    n.to_be_bytes_trimmed_vartime()
        .into_vec()
}

pub fn compute_rsa_pubkey_fingerprint(pubkey: &rsa::RsaPublicKey) -> i64 {
    let pubkey = tl::types::RsaPublicKey {
        n: serialize_boxed_bigint(pubkey.n()),
        e: serialize_boxed_bigint(pubkey.e()),
    };
    let hash = sha1([&pubkey.to_bytes()]);
    let data = make_arr([&hash[12..]]);
    i64::from_le_bytes(data)
}

pub fn parse_pq(data: &[u8]) -> Result<i64, Error> {
    if data.len() == 8 {
        let mut pq = [0; 8];
        pq.copy_from_slice(data);
        Ok(i64::from_be_bytes(pq))
    } else {
        Err(Error::UnexpectedPqSize(data.len()))
    }
}

pub fn ensure_pq_composite(pq: i64) -> Result<(), Error> {
    if !crypto_primes::is_prime(Flavor::Any, &U64::from(pq as u64)) {
        Ok(())
    } else {
        Err(Error::PrimePq(pq))
    }
}

pub fn serialize_p_q(n: i64) -> Vec<u8> {
    let n = BoxedUint::from(n as u64);
    serialize_boxed_bigint(&n)
}

pub fn factorize_pq(pq: i64) -> (i64, i64) {
    let p = factorize(pq as i128)
        .expect("can't factorize pq")
        as i64;
    let q = pq / p;

    if p < q {
        (p, q)
    } else {
        (q, p)
    }
}

pub fn rsa_pad(data: &[u8], server_pubkey: &rsa::RsaPublicKey) -> Result<Vec<u8>, Error> {
    if data.len() > 144 {
        return Err(Error::RsaPadDataTooLong(data.len()));
    }

    let mut data_with_padding = make_arr::<192, _>([data]);
    rand::fill(&mut data_with_padding[data.len()..]);

    let mut data_pad_reversed = data_with_padding;
    data_pad_reversed.reverse();

    let key_aes_encrypted = loop {
        let mut temp_key = [0; 32];
        rand::fill(&mut temp_key);

        let data_with_hash = make_arr::<224, _>([
            &data_pad_reversed,
            &sha256([&temp_key, &data_with_padding]),
        ]);

        let mut aes_encrypted = data_with_hash;
        aes256_ige_encrypt(&mut aes_encrypted, temp_key, [0; 32]);

        let mut temp_key_xor = temp_key;
        xor(&mut temp_key_xor, &sha256([&aes_encrypted]));

        let key_aes_encrypted = make_arr::<256, _>([
            &temp_key_xor,
            &aes_encrypted,
        ]);
        let key_aes_encrypted = BoxedUint::from_be_slice_vartime(&key_aes_encrypted);

        if key_aes_encrypted < **server_pubkey.n() {
            break key_aes_encrypted;
        }
    };

    let encrypted_data = serialize_boxed_bigint(
        &rsa::hazmat::rsa_encrypt(server_pubkey, &key_aes_encrypted)?,
    );

    Ok(encrypted_data)
}

pub fn compute_new_nonce_hash(
    new_nonce: I256,
    byte: &[u8],
    auth_key_aux_id: &[u8],
) -> [u8; 16] {
    let hash = sha1([
        &new_nonce.as_uint().to_le_bytes(),
        byte,
        auth_key_aux_id,
    ]);
    make_arr([&hash[4..]])
}

pub fn decrypt_answer(
    new_nonce: I256,
    server_nonce: I128,
    encrypted_answer: Vec<u8>,
) -> Result<(tl::enums::ServerDhInnerData, [u8; 32], [u8; 32]), Error> {
    let tmp_aes_key = make_arr([
        &sha1([
            &new_nonce.as_uint().to_le_bytes(),
            &server_nonce.as_uint().to_le_bytes(),
        ]),
        sub_str(
            &sha1([
                &server_nonce.as_uint().to_le_bytes(),
                &new_nonce.as_uint().to_le_bytes(),
            ]),
            0,
            12,
        ),
    ]);
    let tmp_aes_iv = make_arr([
        sub_str(
            &sha1([
                &server_nonce.as_uint().to_le_bytes(),
                &new_nonce.as_uint().to_le_bytes(),
            ]),
            12,
            8,
        ),
        &sha1([
            &new_nonce.as_uint().to_le_bytes(),
            &new_nonce.as_uint().to_le_bytes(),
        ]),
        sub_str(
            &new_nonce.as_uint().to_le_bytes(),
            0,
            4,
        ),
    ]);

    let mut answer_with_hash = encrypted_answer;
    aes256_ige_decrypt(&mut answer_with_hash, tmp_aes_key, tmp_aes_iv);

    let (answer, len) = tl::enums::ServerDhInnerData::from_bytes_with_size(&answer_with_hash[20..])
        .map_err(|_| Error::InvalidServerDhInnerData)?;

    let hash = sha1([
        sub_str(&answer_with_hash, 20, len),
    ]);
    if hash != answer_with_hash[..20] {
        return Err(Error::AnswerHashMismatch {
            expected: hash,
            received: answer_with_hash[..20].try_into().unwrap(),
        });
    }

    Ok((answer, tmp_aes_key, tmp_aes_iv))
}

pub fn parse_dh_prime(dh_prime: Vec<u8>) -> Result<U2048, Error> {
    if dh_prime.len() == U2048::BYTES {
        Ok(U2048::from_be_slice(&dh_prime))
    } else {
        Err(Error::InvalidDhPrime(dh_prime))
    }
}

pub fn ensure_dh_prime_safe(p: &U2048) -> Result<(), Error> {
    if U2048::ONE.shl(2047) < *p && crypto_primes::is_prime(Flavor::Safe, p) {
        Ok(())
    } else {
        Err(Error::UnsafeDhPrime(Box::new(*p)))
    }
}

pub fn ensure_dh_g_safe(p: &U2048, g: i32) -> Result<(), Error> {
    if match g {
        2 => dh_g_check_helper(p, 8, [7]),
        3 => dh_g_check_helper(p, 3, [2]),
        4 => true,
        5 => dh_g_check_helper(p, 5, [1, 4]),
        6 => dh_g_check_helper(p, 24, [19, 23]),
        7 => dh_g_check_helper(p, 7, [3, 5, 6]),
        _ => return Err(Error::InvalidDhG(g)),
    } {
        Ok(())
    } else {
        Err(Error::UnsafeDhPrimeOnG {
            p: Box::new(*p),
            g,
        })
    }
}

fn dh_g_check_helper<const N: usize>(p: &U2048, n: u32, results: [u32; N]) -> bool {
    let r = p % U2048::from(n);
    let results = results.map(U2048::from);
    results.contains(&r)
}

pub fn parse_g(g: i32) -> U2048 {
    U2048::from(g as u32)
}

pub fn parse_g_a(g_a: Vec<u8>) -> Result<U2048, Error> {
    if g_a.len() == U2048::BYTES {
        Ok(U2048::from_be_slice(&g_a))
    } else {
        Err(Error::InvalidGa(g_a))
    }
}

pub fn compute_g_b(p: U2048, g: &U2048, b: &U2048) -> U2048 {
    pow_mod(g, b, Odd::new(p).unwrap())
}

pub fn ensure_dh_extra_1(p: &U2048, n: &U2048) -> Result<(), Error> {
    let one = U2048::ONE;

    if *n > one && *n < *p - one {
        Ok(())
    } else {
        Err(Error::DhExtraCheck1Failed {
            p: Box::new(*p),
            n: Box::new(*n),
        })
    }
}

pub fn ensure_dh_extra_2(p: &U2048, n: &U2048) -> Result<(), Error> {
    let lower = U2048::ONE.shl(2048 - 64);
    let upper = *p - U2048::ONE.shl(2048 - 64);

    if lower < *n && *n < upper {
        Ok(())
    } else {
        Err(Error::DhExtraCheck2Failed {
            p: Box::new(*p),
            n: Box::new(*n),
        })
    }
}

pub fn encrypt_data(
    g_b: &U2048,
    nonce: I128,
    server_nonce: I128,
    tmp_aes_key: [u8; 32],
    tmp_aes_iv: [u8; 32],
    prev_auth_key_aux_id: Option<i64>,
) -> Vec<u8> {
    let data = tl::enums::ClientDhInnerData::ClientDhInnerData(
        tl::types::ClientDhInnerData {
            nonce,
            server_nonce,
            retry_id: prev_auth_key_aux_id.unwrap_or(0),
            g_b: g_b.to_be_bytes().to_vec(),
        }
    );
    let data = data.to_bytes();

    let payload_len = 20 + data.len();
    let pad_len = calc_pad_len(payload_len, 16);
    let mut padding = vec![0; pad_len];
    rand::fill(&mut padding);

    let data_with_hash = make_vec([
        &sha1([&data]),
        &data,
        &padding,
    ]);

    let mut encrypted_data = data_with_hash;
    aes256_ige_encrypt(&mut encrypted_data, tmp_aes_key, tmp_aes_iv);

    encrypted_data
}

pub fn compute_auth_key(p: U2048, g_a: &U2048, b: &U2048) -> [u8; 256] {
    pow_mod(g_a, b, Odd::new(p).unwrap())
        .to_be_bytes()
}

pub fn compute_server_salt(new_nonce: I256, server_nonce: I128) -> [u8; 8] {
    xor_new(
        sub_str(&new_nonce.as_uint().to_le_bytes(), 0, 8),
        sub_str(&server_nonce.as_uint().to_le_bytes(), 0, 8),
    )
}

pub fn compute_auth_key_id(auth_key: &[u8]) -> i64 {
    let hash = sha1([auth_key]);
    let data = make_arr([&hash[12..]]);
    i64::from_le_bytes(data)
}

pub fn compute_auth_key_aux_id(auth_key: &[u8]) -> i64 {
    let hash = sha1([auth_key]);
    let data = make_arr([&hash[..8]]);
    i64::from_le_bytes(data)
}

pub fn compute_msg_key(
    auth_key: &[u8],
    direction: Direction,
    plaintext: &[u8],
    random_padding: &[u8],
) -> [u8; 16] {
    let x = match direction {
        Direction::ClientToServer => 0,
        Direction::ServerToClient => 8,
    };

    let msg_key_large = sha256([
        sub_str(auth_key, 88 + x, 32),
        plaintext,
        random_padding,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_pq_composite() {
        ensure_pq_composite(1372318559046200203)
            .unwrap();
    }

    #[test]
    fn test_factorize_pq() {
        assert_eq!(factorize_pq(1372318559046200203), (1141464581, 1202243663));
    }
}
