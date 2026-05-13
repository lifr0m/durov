use crate::primitives::Error;
use crypto_bigint::{BoxedUint, NonZero};
use crypto_primes::Flavor;

pub fn ensure_prime_safe(p: &BoxedUint) -> Result<(), Error> {
    let lower = BoxedUint::one_with_precision(2048).shl(2047);
    let upper = BoxedUint::one_with_precision(2049).shl(2048);

    if lower < *p && *p < upper && crypto_primes::is_prime(Flavor::Safe, p) {
        Ok(())
    } else {
        Err(Error::UnsafePrime(p.clone()))
    }
}

pub fn ensure_g_safe(p: &BoxedUint, g: i32) -> Result<(), Error> {
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
            p: p.clone(),
            g,
        })
    }
}

fn check_g<const N: usize>(p: &BoxedUint, n: u32, results: [u32; N]) -> bool {
    let n = BoxedUint::from(n);
    let n = NonZero::new(n).unwrap();
    let rem = p % n;
    let results = results.map(BoxedUint::from);
    results.contains(&rem)
}
