use crypto_bigint::modular::{MontyForm, MontyParams};
use crypto_bigint::{Odd, Uint};

pub fn pow_mod<const LIMBS: usize>(
    n: &Uint<LIMBS>,
    exp: &Uint<LIMBS>,
    modulus: Odd<Uint<LIMBS>>,
) -> Uint<LIMBS> {
    let params = MontyParams::new(modulus);
    let monty = MontyForm::new(n, params);
    monty.pow(exp)
        .to_montgomery()
}
