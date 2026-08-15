#![allow(dead_code)]

#[cfg(any(feature = "n_choose_k", feature = "dynamic-module"))]
pub mod n_choose_k {
    use core::ops::{Add, AddAssign, Div, Mul, Sub};
    use num_traits::{One, Zero};

    pub const MAX_SCALAR_STEPS: u128 = 1_000_000;

    pub fn supports_f64(n: f64, k: f64) -> bool {
        if !n.is_finite()
            || !k.is_finite()
            || n < 0.0
            || k < 0.0
            || n.fract() != 0.0
            || k.fract() != 0.0
            // Match the retained source adapter's exact f64-to-u128 bound.
            || n > u128::MAX as f64
            || k > u128::MAX as f64
        {
            return false;
        }
        k > n || k.min(n - k) <= MAX_SCALAR_STEPS as f64
    }

    pub fn scalar<T>(n: T, k: T) -> T
    where
        T: Copy
            + PartialOrd
            + Add<Output = T>
            + AddAssign
            + Sub<Output = T>
            + Mul<Output = T>
            + Div<Output = T>
            + Zero
            + One,
    {
        if k > n {
            return T::zero();
        }

        // Symmetry bounds the loop by min(k, n-k). Runtime factories validate
        // that this finite loop fits the bytecode v1 work limit before calling
        // the kernel.
        let k = if k > n - k { n - k } else { k };

        let mut result = T::one();
        let mut i = T::zero();

        while i < k {
            let numerator = n - i;
            let denominator = i + T::one();
            result = result * numerator / denominator;
            i += T::one();
        }

        result
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn scalar_f64_returns_expected_result() {
            assert_eq!(scalar(10.0_f64, 2.0_f64), 45.0);
        }

        #[test]
        fn scalar_f64_returns_zero_when_k_exceeds_n() {
            assert_eq!(scalar(2.0_f64, 10.0_f64), 0.0);
        }

        #[test]
        fn f64_support_contract_rejects_non_finite_fractional_and_unbounded_work() {
            for (n, k) in [
                (10.0, f64::INFINITY),
                (f64::NAN, 1.0),
                (10.0, 1.5),
                (-1.0, 0.0),
                (2_000_002.0, 1_000_001.0),
                (1.0e100, 1.0e50),
                (1.0e100, 1.0),
            ] {
                assert!(!supports_f64(n, k));
            }
            assert!(supports_f64(10.0, 2.0));
            assert!(supports_f64(2.0, 10.0));
            assert!(supports_f64(u128::MAX as f64, 1.0));
        }
    }
}
