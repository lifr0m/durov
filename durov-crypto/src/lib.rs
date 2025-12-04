mod ciphers;
mod hashes;
mod logic;
mod modular;
mod primes;
mod utils;
mod random;
mod serde;
mod prime_g;

pub use ciphers::*;
use crypto_bigint::U2048;
pub use hashes::*;
pub use logic::*;
pub use modular::*;
pub use prime_g::*;
pub use primes::*;
pub use random::*;
pub use serde::*;
use thiserror::Error;
pub use utils::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid prime: {0:?}")]
    InvalidPrime(Vec<u8>),

    #[error("invalid g: {0}")]
    InvalidG(i32),

    #[error("unsafe prime: {0}")]
    UnsafePrime(Box<U2048>),

    #[error("unsafe prime {p} on g {g}")]
    UnsafePrimeOnG {
        p: Box<U2048>,
        g: i32,
    },
}
