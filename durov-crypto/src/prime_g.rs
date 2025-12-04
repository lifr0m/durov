use crate::Error;
use crypto_bigint::U2048;
use crypto_primes::Flavor;

pub fn parse_prime(prime: &[u8]) -> Result<U2048, Error> {
    if prime.len() == U2048::BYTES {
        Ok(U2048::from_be_slice(prime))
    } else {
        Err(Error::InvalidPrime(prime.to_vec()))
    }
}

pub fn parse_g(g: i32) -> U2048 {
    U2048::from(g as u32)
}

pub fn ensure_prime_safe(p: &U2048) -> Result<(), Error> {
    if U2048::ONE.shl(2047) < *p && crypto_primes::is_prime(Flavor::Safe, p) {
        Ok(())
    } else {
        Err(Error::UnsafePrime(Box::new(*p)))
    }
}

pub fn ensure_g_safe(p: &U2048, g: i32) -> Result<(), Error> {
    if match g {
        2 => check_g(p, 8, [7]),
        3 => check_g(p, 3, [2]),
        4 => true,
        5 => check_g(p, 5, [1, 4]),
        6 => check_g(p, 24, [19, 23]),
        7 => check_g(p, 7, [3, 5, 6]),
        _ => return Err(Error::InvalidG(g)),
    } {
        Ok(())
    } else {
        Err(Error::UnsafePrimeOnG {
            p: Box::new(*p),
            g,
        })
    }
}

fn check_g<const N: usize>(p: &U2048, n: u32, results: [u32; N]) -> bool {
    let r = p % U2048::from(n);
    let results = results.map(U2048::from);
    results.contains(&r)
}
