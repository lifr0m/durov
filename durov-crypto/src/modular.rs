use crypto_bigint::modular::{BoxedMontyForm, BoxedMontyParams};
use crypto_bigint::{BoxedUint, Odd};

pub fn pow_mod(num: &BoxedUint, exp: &BoxedUint, modulus: &Odd<BoxedUint>) -> BoxedUint {
    let params = BoxedMontyParams::new(modulus.clone());
    let num = BoxedMontyForm::new(num.clone(), params);
    num.pow(exp).retrieve()
}
