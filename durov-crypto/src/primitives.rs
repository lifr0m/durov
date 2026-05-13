mod ciphers;
mod hashes;
mod logic;
mod primes;
mod random;
mod safety;
mod serde;
mod helpers;

pub use ciphers::*;
use crypto_bigint::BoxedUint;
pub use hashes::*;
pub use helpers::*;
pub use logic::*;
pub use primes::*;
pub use random::*;
pub use safety::*;
pub use serde::*;
use thiserror::Error;

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

pub enum Direction {
    ClientToServer,
    ServerToClient,
}
