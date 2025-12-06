use crypto_bigint::{BoxedUint, I128, I256};
use crypto_primes::Flavor;
pub use durov_crypto::*;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::schemas::mtproto as tl;
use durov_tl_types::serialize::Serialize;
use rsa::traits::PublicKeyParts;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("rsa: {0}")]
    Rsa(#[from] rsa::Error),

    #[error("unexpected pq size: {0} bytes")]
    UnexpectedPqSize(usize),

    #[error("pq is prime: {0}")]
    PrimePq(u64),

    #[error("rsa_pad data argument is too long: {0} bytes")]
    RsaPadDataTooLong(usize),

    #[error("invalid server dh inner data")]
    InvalidServerDhInnerData,

    #[error("answer hash mismatch: expected {expected:?}, received {received:?}")]
    AnswerHashMismatch {
        expected: [u8; 20],
        received: [u8; 20],
    },

    #[error("dh extra check 1 failed: p {p}, n {n}")]
    DhExtraCheck1Failed {
        p: BoxedUint,
        n: BoxedUint,
    },

    #[error("dh extra check 2 failed: p {p}, n {n}")]
    DhExtraCheck2Failed {
        p: BoxedUint,
        n: BoxedUint,
    },
}

pub enum Direction {
    ClientToServer,
    ServerToClient,
}

pub fn compute_pubkey_fingerprint(pubkey: &rsa::RsaPublicKey) -> i64 {
    let pubkey = tl::types::RsaPublicKey {
        n: serialize_bigint(pubkey.n()),
        e: serialize_bigint(pubkey.e()),
    };
    let hash = sha1([&pubkey.to_bytes()]);
    let data = make_arr([&hash[12..]]);
    i64::from_le_bytes(data)
}

pub fn deserialize_pq(data: &[u8]) -> Result<u64, Error> {
    if data.len() == 8 {
        let data = make_arr([data]);
        Ok(u64::from_be_bytes(data))
    } else {
        Err(Error::UnexpectedPqSize(data.len()))
    }
}

pub fn ensure_pq_composite(pq: u64) -> Result<(), Error> {
    if !crypto_primes::is_prime(Flavor::Any, &BoxedUint::from(pq)) {
        Ok(())
    } else {
        Err(Error::PrimePq(pq))
    }
}

pub fn factorize_pq(pq: u64) -> (u64, u64) {
    let p = factorize(pq as u128) as u64;
    let q = pq / p;
    if p < q { (p, q) } else { (q, p) }
}

pub fn serialize_pq(num: u64) -> Vec<u8> {
    let num = BoxedUint::from(num);
    serialize_bigint(&num)
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
        let key_aes_encrypted = deserialize_bigint(&key_aes_encrypted, 2048)
            .unwrap();

        if key_aes_encrypted < **server_pubkey.n() {
            break key_aes_encrypted;
        }
    };

    let encrypted_data = rsa::hazmat::rsa_encrypt(server_pubkey, &key_aes_encrypted)?;
    let encrypted_data = serialize_bigint(&encrypted_data);

    Ok(encrypted_data)
}

pub fn compute_new_nonce_hash(new_nonce: I256, byte: &[u8], auth_key_aux_id: &[u8]) -> [u8; 16] {
    let hash = sha1([
        &new_nonce.as_uint().to_le_bytes(),
        byte,
        auth_key_aux_id,
    ]);
    make_arr([&hash[4..]])
}

pub fn decrypt_answer(new_nonce: I256, server_nonce: I128, encrypted_answer: Vec<u8>)
    -> Result<(tl::enums::ServerDhInnerData, [u8; 32], [u8; 32]), Error>
{
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

pub fn ensure_dh_extra_1(p: &BoxedUint, n: &BoxedUint) -> Result<(), Error> {
    let one = BoxedUint::one();

    if *n > one && *n < p - one {
        Ok(())
    } else {
        Err(Error::DhExtraCheck1Failed {
            p: p.clone(),
            n: n.clone(),
        })
    }
}

pub fn ensure_dh_extra_2(p: &BoxedUint, n: &BoxedUint) -> Result<(), Error> {
    let lower = BoxedUint::one_with_precision(2048).shl(2048 - 64);
    let upper = p - BoxedUint::one_with_precision(2048).shl(2048 - 64);

    if lower < *n && *n < upper {
        Ok(())
    } else {
        Err(Error::DhExtraCheck2Failed {
            p: p.clone(),
            n: n.clone(),
        })
    }
}

pub fn encrypt_data(
    g_b: &BoxedUint,
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
            g_b: serialize_bigint(g_b),
        }
    );
    let data = data.to_bytes();

    let mut data_with_hash = make_buf([
        &sha1([&data]),
        &data,
    ]);
    let pad_len = calc_pad_len(data_with_hash.len(), 16);
    extend_random(&mut data_with_hash, pad_len);

    let mut encrypted_data = data_with_hash;
    aes256_ige_encrypt(&mut encrypted_data, tmp_aes_key, tmp_aes_iv);

    encrypted_data.to_vec()
}

pub fn compute_server_salt(new_nonce: I256, server_nonce: I128) -> i64 {
    let data = xor_new(
        sub_str(&new_nonce.as_uint().to_le_bytes(), 0, 8),
        sub_str(&server_nonce.as_uint().to_le_bytes(), 0, 8),
    );
    i64::from_le_bytes(data)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_pq_composite() {
        ensure_pq_composite(1372318559046200203).unwrap();
        ensure_pq_composite(1141464581).unwrap_err();
    }

    #[test]
    fn test_factorize_pq() {
        assert_eq!(factorize_pq(1372318559046200203), (1141464581, 1202243663));
    }
}
