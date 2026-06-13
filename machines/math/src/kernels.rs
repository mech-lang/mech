#[cfg(any(feature = "round", feature = "dynamic-module"))]
pub mod round {
    pub fn scalar_f64(input: f64) -> f64 {
        libm::round(input)
    }
}
