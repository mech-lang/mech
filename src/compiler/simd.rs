/// Dense row-major matrix specialized for SIMD-friendly f32 kernels.
#[derive(Debug, Clone, PartialEq)]
pub struct MatrixF32 {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f32>,
}

impl MatrixF32 {
    pub fn new(rows: usize, cols: usize, data: Vec<f32>) -> Self {
        assert_eq!(
            rows * cols,
            data.len(),
            "rows * cols must equal data length"
        );
        Self { rows, cols, data }
    }

    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    pub fn add(&self, rhs: &Self) -> Self {
        assert_eq!(self.rows, rhs.rows, "row mismatch");
        assert_eq!(self.cols, rhs.cols, "column mismatch");

        let mut out = vec![0.0; self.data.len()];

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            if std::arch::is_x86_feature_detected!("avx") {
                // SAFETY: AVX support is checked at runtime and pointers are in-bounds.
                unsafe {
                    add_avx(&self.data, &rhs.data, &mut out);
                }
                return Self::new(self.rows, self.cols, out);
            }
        }

        for i in 0..self.data.len() {
            out[i] = self.data[i] + rhs.data[i];
        }

        Self::new(self.rows, self.cols, out)
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx")]
unsafe fn add_avx(lhs: &[f32], rhs: &[f32], out: &mut [f32]) {
    use std::arch::x86_64::*;

    let len = lhs.len();
    let mut i = 0;
    while i + 8 <= len {
        // SAFETY: Bounds checked by loop condition and unaligned ops are allowed.
        let a = unsafe { _mm256_loadu_ps(lhs.as_ptr().add(i)) };
        // SAFETY: Bounds checked by loop condition and unaligned ops are allowed.
        let b = unsafe { _mm256_loadu_ps(rhs.as_ptr().add(i)) };
        let c = _mm256_add_ps(a, b);
        // SAFETY: Bounds checked by loop condition and unaligned ops are allowed.
        unsafe { _mm256_storeu_ps(out.as_mut_ptr().add(i), c) };
        i += 8;
    }

    while i < len {
        out[i] = lhs[i] + rhs[i];
        i += 1;
    }
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn add_avx(lhs: &[f32], rhs: &[f32], out: &mut [f32]) {
    for i in 0..lhs.len() {
        out[i] = lhs[i] + rhs[i];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_add_works() {
        let a = MatrixF32::new(2, 4, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        let b = MatrixF32::new(2, 4, vec![8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0]);
        let c = a.add(&b);
        assert_eq!(c.data, vec![9.0; 8]);
    }
}
