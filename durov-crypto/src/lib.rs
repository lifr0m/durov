mod ciphers;
mod hashes;
mod logic;
mod modular;
mod primes;
mod utils;
mod random;
mod serde;
mod safety;

pub use ciphers::*;
use crypto_bigint::BoxedUint;
pub use hashes::*;
pub use logic::*;
pub use modular::*;
pub use primes::*;
pub use random::*;
pub use safety::*;
pub use serde::*;
use thiserror::Error;
pub use utils::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid bigint: {0}")]
    InvalidBigInt(#[from] crypto_bigint::DecodeError),

    #[error("unsafe prime: {0}")]
    UnsafePrime(BoxedUint),

    #[error("invalid g: {0}")]
    InvalidG(i32),

    #[error("unsafe prime {p} on g {g}")]
    UnsafePrimeOnG {
        p: BoxedUint,
        g: i32,
    },
}
