#[cfg(any(feature = "round", feature = "dynamic-module"))]
pub mod round {
    pub fn scalar_f64(input: f64) -> f64 {
        libm::round(input)
    }

    pub fn view_f64(input: &[f64], out: &mut [f64]) {
        for (src, dst) in input.iter().zip(out.iter_mut()) {
            *dst = scalar_f64(*src);
        }
    }
}
