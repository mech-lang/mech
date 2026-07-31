#![allow(dead_code)]

#[cfg(any(feature = "n_choose_k", feature = "dynamic-module"))]
pub mod n_choose_k {
    use core::ops::{Add, AddAssign, Div, Mul, Sub};
    use num_traits::{One, Zero};

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
    }
}
