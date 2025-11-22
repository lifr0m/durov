use crate::connection::DcType;
use crate::crypto;
use crate::crypto::compute_server_salt;
use crypto_bigint::{I128, I256, U2048};
use durov_tl_types::schemas::mtproto as tl;
use durov_tl_types::serialize::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("crypto: {0}")]
    Crypto(#[from] crypto::Error),

    #[error("nonce mismatch: expected {expected}, received {received}")]
    NonceMismatch {
        expected: I128,
        received: I128,
    },

    #[error("server nonce mismatch: expected {expected}, received {received}")]
    ServerNonceMismatch {
        expected: I128,
        received: I128,
    },

    #[error("new nonce hash mismatch: expected {expected:?}, received {received:?}")]
    NewNonceHashMismatch {
        expected: [u8; 16],
        received: [u8; 16],
    },

    #[error("rsa pubkey fingerprint mismatch: expected {expected}, received {received:?}")]
    RsaPubkeyFingerprintMismatch {
        expected: i64,
        received: Vec<i64>,
    },

    #[error("restart handshake")]
    Restart,

    #[error("retry starting from step 4")]
    RetryStep4,
}

pub struct Step1 {
    pub req: tl::functions::ReqPqMulti,
    pub nonce: I128,
}

pub struct Step2 {
    pub req: tl::functions::ReqDhParams,
    pub server_nonce: I128,
    pub new_nonce: I256,
}

pub struct Step3 {
    pub tmp_aes_key: [u8; 32],
    pub tmp_aes_iv: [u8; 32],
    pub p: U2048,
    pub g: U2048,
    pub g_a: U2048,
    pub server_time: i32,
}

pub struct Step4 {
    pub req: tl::functions::SetClientDhParams,
    pub auth_key: [u8; 256],
}

pub struct Step5 {
    pub server_salt: [u8; 8],
}

pub fn step1() -> Step1 {
    let nonce = crypto::random_bigint();
    let req = tl::functions::ReqPqMulti { nonce };
    Step1 { req, nonce }
}

pub fn step2(
    res: tl::enums::ResPq,
    nonce: I128,
    dc: i32,
    dc_type: DcType,
    server_pubkey: &rsa::RsaPublicKey,
) -> Result<Step2, Error> {
    let tl::enums::ResPq::ResPq(res) = res;

    ensure_nonce_equal(nonce, res.nonce)?;

    let fingerprint = crypto::compute_rsa_pubkey_fingerprint(server_pubkey);

    if !res.server_public_key_fingerprints.contains(&fingerprint) {
        return Err(Error::RsaPubkeyFingerprintMismatch {
            expected: fingerprint,
            received: res.server_public_key_fingerprints,
        });
    }

    let pq = crypto::parse_pq(&res.pq)?;
    crypto::ensure_pq_composite(pq)?;
    let (p, q) = crypto::factorize_pq(pq);

    let new_nonce = crypto::random_bigint();

    let data = tl::enums::PQInnerData::PQInnerDataDc(
        tl::types::PQInnerDataDc {
            pq: res.pq,
            p: crypto::serialize_p_q(p),
            q: crypto::serialize_p_q(q),
            nonce,
            server_nonce: res.server_nonce,
            new_nonce,
            dc: match dc_type {
                DcType::Production => dc,
                DcType::Test => dc + 10_000,
                DcType::Media => -dc,
            },
        }
    );

    let req = tl::functions::ReqDhParams {
        nonce,
        server_nonce: res.server_nonce,
        p: crypto::serialize_p_q(p),
        q: crypto::serialize_p_q(q),
        public_key_fingerprint: fingerprint,
        encrypted_data: crypto::rsa_pad(&data.to_bytes(), server_pubkey)?,
    };

    Ok(Step2 { req, server_nonce: res.server_nonce, new_nonce })
}

pub fn step3(
    res: tl::enums::ServerDhParams,
    nonce: I128,
    server_nonce: I128,
    new_nonce: I256,
) -> Result<Step3, Error> {
    let res = match res {
        tl::enums::ServerDhParams::ServerDhParamsOk(res) => res,
        tl::enums::ServerDhParams::ServerDhParamsFail(res) => {
            ensure_nonce_equal(nonce, res.nonce)?;
            ensure_server_nonce_equal(server_nonce, res.server_nonce)?;
            ensure_new_nonce_hash_equal(new_nonce, res.new_nonce_hash, 0, 0)?;

            return Err(Error::Restart);
        }
    };

    ensure_nonce_equal(nonce, res.nonce)?;
    ensure_server_nonce_equal(server_nonce, res.server_nonce)?;

    let (answer, tmp_aes_key, tmp_aes_iv) = crypto::decrypt_answer(
        new_nonce,
        server_nonce,
        res.encrypted_answer,
    )?;
    let tl::enums::ServerDhInnerData::ServerDhInnerData(answer) = answer;

    ensure_nonce_equal(nonce, answer.nonce)?;
    ensure_server_nonce_equal(server_nonce, answer.server_nonce)?;

    let p = crypto::parse_dh_prime(answer.dh_prime)?;
    crypto::ensure_dh_prime_safe(&p)?;
    crypto::ensure_dh_g_safe(&p, answer.g)?;

    let g = crypto::parse_g(answer.g);
    let g_a = crypto::parse_g_a(answer.g_a)?;

    crypto::ensure_dh_extra_1(&p, &g)?;
    crypto::ensure_dh_extra_1(&p, &g_a)?;
    crypto::ensure_dh_extra_2(&p, &g_a)?;

    Ok(Step3 { tmp_aes_key, tmp_aes_iv, p, g, g_a, server_time: answer.server_time })
}

#[allow(clippy::too_many_arguments)]
pub fn step4(
    nonce: I128,
    server_nonce: I128,
    tmp_aes_key: [u8; 32],
    tmp_aes_iv: [u8; 32],
    p: U2048,
    g: &U2048,
    g_a: &U2048,
    prev_auth_key_aux_id: Option<i64>,
) -> Result<Step4, Error> {
    let b = crypto::random_bigint();
    let g_b = crypto::compute_g_b(p, g, &b);

    crypto::ensure_dh_extra_1(&p, &g_b)?;
    crypto::ensure_dh_extra_2(&p, &g_b)?;

    let req = tl::functions::SetClientDhParams {
        nonce,
        server_nonce,
        encrypted_data: crypto::encrypt_data(
            &g_b,
            nonce,
            server_nonce,
            tmp_aes_key,
            tmp_aes_iv,
            prev_auth_key_aux_id,
        ),
    };

    let auth_key = crypto::compute_auth_key(p, g_a, &b);

    Ok(Step4 { req, auth_key })
}

pub fn step5(
    res: tl::enums::SetClientDhParamsAnswer,
    nonce: I128,
    server_nonce: I128,
    new_nonce: I256,
    auth_key: &[u8],
) -> Result<Step5, Error> {
    let auth_key_aux_id = crypto::compute_auth_key_aux_id(auth_key);

    let res = match res {
        tl::enums::SetClientDhParamsAnswer::DhGenOk(res) => res,
        tl::enums::SetClientDhParamsAnswer::DhGenRetry(res) => {
            ensure_nonce_equal(nonce, res.nonce)?;
            ensure_server_nonce_equal(server_nonce, res.server_nonce)?;
            ensure_new_nonce_hash_equal(new_nonce, res.new_nonce_hash2, 2, auth_key_aux_id)?;

            return Err(Error::RetryStep4);
        }
        tl::enums::SetClientDhParamsAnswer::DhGenFail(res) => {
            ensure_nonce_equal(nonce, res.nonce)?;
            ensure_server_nonce_equal(server_nonce, res.server_nonce)?;
            ensure_new_nonce_hash_equal(new_nonce, res.new_nonce_hash3, 3, auth_key_aux_id)?;

            return Err(Error::RetryStep4);
        }
    };

    ensure_nonce_equal(nonce, res.nonce)?;
    ensure_server_nonce_equal(server_nonce, res.server_nonce)?;
    ensure_new_nonce_hash_equal(new_nonce, res.new_nonce_hash1, 1, auth_key_aux_id)?;

    let server_salt = compute_server_salt(new_nonce, res.server_nonce);

    Ok(Step5 { server_salt })
}

fn ensure_nonce_equal(nonce: I128, res_nonce: I128) -> Result<(), Error> {
    if nonce == res_nonce {
        Ok(())
    } else {
        Err(Error::NonceMismatch {
            expected: nonce,
            received: res_nonce,
        })
    }
}

fn ensure_server_nonce_equal(nonce: I128, res_nonce: I128) -> Result<(), Error> {
    if nonce == res_nonce {
        Ok(())
    } else {
        Err(Error::ServerNonceMismatch {
            expected: nonce,
            received: res_nonce,
        })
    }
}

fn ensure_new_nonce_hash_equal(
    nonce: I256,
    res_hash: I128,
    byte: u8,
    auth_key_aux_id: i64,
) -> Result<(), Error> {
    let byte: &[u8] = match byte {
        0 => &[],
        _ => &[byte],
    };
    let auth_key_aux_id: &[u8] = match auth_key_aux_id {
        0 => &[],
        _ => &auth_key_aux_id.to_le_bytes(),
    };

    let hash = crypto::compute_new_nonce_hash(nonce, byte, auth_key_aux_id);
    let res_hash = res_hash.as_uint().to_le_bytes();

    if hash == res_hash {
        Ok(())
    } else {
        Err(Error::NewNonceHashMismatch {
            expected: hash,
            received: res_hash,
        })
    }
}
