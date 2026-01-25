#![allow(non_snake_case)]

use crate::tl;
use crypto_bigint::Odd;
use durov_crypto::*;
use pbkdf2::pbkdf2_hmac_array;
use sha2::Sha512;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("durov crypto: {0}")]
    DurovCrypto(#[from] durov_crypto::Error),

    #[error("param not provided: {0}")]
    ParamNotProvided(&'static str),

    #[error("unknown algo provided")]
    UnknownAlgo,
}

pub fn compute_srp_check(pwd: tl::enums::account::Password, password: &str)
    -> Result<tl::enums::InputCheckPasswordSrp, Error>
{
    let tl::enums::account::Password::Password(pwd) = pwd;

    if !pwd.has_password {
        return Ok(tl::types::InputCheckPasswordEmpty {}.into());
    }
    let algo = pwd.current_algo
        .ok_or(Error::ParamNotProvided("current_algo"))?;

    let tl::enums::PasswordKdfAlgo::PasswordKdfAlgoSha256Sha256Pbkdf2Hmacsha512Iter100000Sha256ModPow(algo) = algo else {
        return Err(Error::UnknownAlgo);
    };

    let p = deserialize_bigint(&algo.p, 2048)?;
    ensure_prime_safe(&p)?;
    let p = Odd::new(p).unwrap();

    ensure_g_safe(&p, algo.g)?;
    let g = deserialize_bigint(&algo.g.to_be_bytes(), 2048)?;

    let salt1 = &algo.salt1;
    let salt2 = &algo.salt2;

    let g_b = pwd.srp_B
        .ok_or(Error::ParamNotProvided("srp_B"))?;
    let g_b = deserialize_bigint(&g_b, 2048)?;

    let a = random_bigint(2048);
    let g_a = g.pow_mod(&a, &p);

    let k = H([
        &serialize_bigint_padded(&p),
        &serialize_bigint_padded(&g),
    ]);
    let k = deserialize_bigint(&k, 2048)?;

    let u = H([
        &serialize_bigint_padded(&g_a),
        &serialize_bigint_padded(&g_b),
    ]);
    let u = deserialize_bigint(&u, 2048)?;

    let x = PH2(password.as_bytes(), salt1, salt2);
    let x = deserialize_bigint(&x, 2048)?;

    let v = g.pow_mod(&x, &p);
    let k_v = k.mul_mod(&v, p.as_nz_ref());

    let t = g_b.sub_mod(&k_v, p.as_nz_ref());
    let s_a = t.pow_mod(&(a + u * x), &p);

    let k_a = H([
        &serialize_bigint_padded(&s_a),
    ]);

    let M1 = H([
        &xor_new::<32>(
            &H([
                &serialize_bigint_padded(&p),
            ]),
            &H([
                &serialize_bigint_padded(&g),
            ]),
        ),
        &H([salt1]),
        &H([salt2]),
        &serialize_bigint_padded(&g_a),
        &serialize_bigint_padded(&g_b),
        &k_a,
    ]);

    Ok(tl::types::InputCheckPasswordSrp {
        srp_id: pwd.srp_id
            .ok_or(Error::ParamNotProvided("srp_id"))?,
        A: serialize_bigint(&g_a),
        M1: M1.to_vec(),
    }.into())
}

fn H<const N: usize>(data: [&[u8]; N]) -> [u8; 32] {
    sha256(data)
}

fn SH(data: &[u8], salt: &[u8]) -> [u8; 32] {
    H([salt, data, salt])
}

fn PH1(password: &[u8], salt1: &[u8], salt2: &[u8]) -> [u8; 32] {
    SH(
        &SH(password, salt1),
        salt2,
    )
}

fn PH2(password: &[u8], salt1: &[u8], salt2: &[u8]) -> [u8; 32] {
    SH(
        &pbkdf2_hmac_array::<Sha512, 64>(
            &PH1(password, salt1, salt2),
            salt1,
            100_000,
        ),
        salt2,
    )
}
