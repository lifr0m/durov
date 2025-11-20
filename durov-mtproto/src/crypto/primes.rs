use std::cmp::min;

pub fn factorize(n: i128) -> Option<i128> {
    for c in [3, 17, 113, 317] {
        let x_0 = rand::random::<i128>() % n;
        if let Some(g) = factorize_n(n, x_0, c) {
            return Some(g);
        }
    }
    None
}

/// Richard Brent's modification of Pollard's rho algorithm.
///
/// https://maths-people.anu.edu.au/%7Ebrent/pd/rpb051i.pdf
#[allow(non_snake_case)]
fn factorize_n(N: i128, x_0: i128, c: i128) -> Option<i128> {
    let f = |x| (i128::wrapping_mul(x, x) + c) % N;
    let m = 743;
    let mut y = x_0;
    let mut r = 1;
    let mut q = 1;
    let mut x;
    let mut G;
    let mut ys;
    loop {
        x = y;
        for _ in 1..=r {
            y = f(y);
        }
        let mut k = 0;
        loop {
            ys = y;
            for _ in 1..=min(m, r - k) {
                y = f(y);
                q = q * i128::abs(x - y) % N;
            }
            G = gcd(q, N);
            k += m;
            if (k >= r) || (G > 1) {
                break;
            }
        }
        r *= 2;
        if G > 1 {
            break;
        }
    }
    if G == N {
        loop {
            ys = f(ys);
            G = gcd(i128::abs(x - ys), N);
            if G > 1 {
                break;
            }
        }
    }
    if G == N {
        None
    } else {
        Some(G)
    }
}

/// Daniel Lemire's and Ralph Corderoy's optimized Binary GCD algorithm.
///
/// https://en.algorithmica.org/hpc/algorithms/gcd/
fn gcd(mut a: i128, mut b: i128) -> i128 {
    if a == 0 {
        return b;
    }
    if b == 0 {
        return a;
    }

    let mut az = a.trailing_zeros();
    let bz = b.trailing_zeros();
    let shift = min(az, bz);
    b >>= bz;

    while a != 0 {
        a >>= az;
        let diff = b - a;
        az = diff.trailing_zeros();
        b = min(a, b);
        a = diff.abs();
    }

    b << shift
}
