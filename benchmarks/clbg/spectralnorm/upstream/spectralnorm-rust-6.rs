// The Computer Language Benchmarks Game
// http://benchmarksgame.alioth.debian.org/
//
// contributed by the Rust Project Developers
// contributed by Matt Brubeck
// contributed by TeXitoi
// modified by Tung Duong
// contributed by Cristi Cobzarenco (@cristicbz)
// contributed by Andre Bogus

extern crate rayon;

// This would usually be done within a library.
#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
mod simd {
    use std::arch::x86_64::*;
    use std::ops::*;

    use rayon::prelude::*;

    #[derive(Clone, Copy)]
    pub struct F64x2(__m128d);

    impl F64x2 {
        pub fn splat(x: f64) -> F64x2 { F64x2(unsafe { _mm_set1_pd(x) }) }
        pub fn new(a: f64, b: f64) -> F64x2 {
            F64x2(unsafe { _mm_set_pd(b, a) })
        }
        pub fn write_to_slice_unaligned(self, slice: &mut [f64]) {
            unsafe { _mm_storeu_pd(slice.as_mut_ptr(), self.0) }
        }
        pub fn hadd(self, rhs: F64x2) -> F64x2 {
            F64x2(unsafe { _mm_hadd_pd(self.0, rhs.0) })
        }
        pub fn sum(self) -> f64 {
            let mut s = [0f64; 2];
            self.write_to_slice_unaligned(&mut s);
            s[0] + s[1]
        }
    }

    impl Add for F64x2 {
        type Output = Self;
        fn add(self, rhs: Self) -> Self {
            F64x2(unsafe { _mm_add_pd(self.0, rhs.0) })
        }
    }
    impl Mul for F64x2 {
        type Output = Self;
        fn mul(self, rhs: Self) -> Self {
            F64x2(unsafe { _mm_mul_pd(self.0, rhs.0) })
        }
    }
    impl Div for F64x2 {
        type Output = Self;
        fn div(self, rhs: Self) -> Self {
            F64x2(unsafe { _mm_div_pd(self.0, rhs.0) })
        }
    }

    pub fn spectralnorm(n: usize) -> f64 {
        // Group all vectors in pairs of two for SIMD convenience.
        assert!(n % 2 == 0, "only even lengths are accepted");
        let mut u = vec![F64x2::splat(1.0); n / 2];
        let mut v = vec![F64x2::splat(0.0); n / 2];
        let mut tmp = vec![F64x2::splat(0.0); n / 2];

        for _ in 0..10 {
            mult_at_av(&u, &mut v, &mut tmp);
            mult_at_av(&v, &mut u, &mut tmp);
        }

        (dot(&u, &v) / dot(&v, &v)).sqrt()
    }

    fn mult_at_av(v: &[F64x2], out: &mut [F64x2], tmp: &mut [F64x2]) {
        mult(v, tmp, a);
        mult(tmp, out, |i, j| a(j, i));
    }

    fn mult<F>(v: &[F64x2], out: &mut [F64x2], a: F)
               where F: Fn([usize; 2], [usize; 2]) -> F64x2 + Sync {
        // Parallelize along the output vector, with each pair of slots as a
        // parallelism unit.
        out.par_iter_mut().enumerate().for_each(|(i, slot)| {
            // We're computing everything in chunks of two so the indces of
            // slot[0] and slot[1] are 2*i and 2*i + 1.
            let i = 2 * i;
            let (i0, i1) = ([i; 2], [i + 1; 2]);

            // Each slot in the pair gets its own sum, which is further
            // computed in two f64 lanes (which are summed at the end)
            let (mut sum0, mut sum1) = (F64x2::splat(0.0), F64x2::splat(0.0));
            for (j, &x) in v.iter().enumerate() {
                let j = [2 * j, 2 * j  + 1];
                sum0 = sum0 + x / a(i0, j);
                sum1 = sum1 + x / a(i1, j);
            }

            // Sum the two lanes for each slot.
            *slot = F64x2::hadd(sum0, sum1);
        });
    }

    fn a(i: [usize; 2], j: [usize; 2]) -> F64x2 {
       F64x2::new(((i[0] + j[0]) * (i[0] + j[0] + 1) / 2 + i[0] + 1) as f64,
        ((i[1] + j[1]) * (i[1] + j[1] + 1) / 2 + i[1] + 1) as f64)
    }

    fn dot(v: &[F64x2], u: &[F64x2]) -> f64 {
        // Vectorised form of dot product: (1) compute dot across two lanes.
        let r = u.iter()
                 .zip(v)
                 .map(|(&x, &y)| x * y)
                 .fold(F64x2::splat(0.0), |s, x| s + x);

        // (2) sum the two lanes.
        r.sum()
    }

}

#[cfg(not(all(target_arch = "x86_64", target_feature = "sse2")))]
mod simd {
    pub fn spectralnorm(n: usize) -> f64 {
        panic!("This only works with SSE2");
    }
}

// now here's the actual program
fn main() {
    let n = std::env::args().nth(1)
        .and_then(|n| n.parse().ok())
        .unwrap_or(100);
    let answer = simd::spectralnorm(n);
    println!("{:.9}", answer);
}
