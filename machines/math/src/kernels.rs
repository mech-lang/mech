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

#[cfg(any(feature = "sqrt", feature = "dynamic-module"))]
pub mod sqrt {
    pub fn scalar_f64(input: f64) -> f64 {
        libm::sqrt(input)
    }

    pub fn view_f64(input: &[f64], out: &mut [f64]) {
        for (src, dst) in input.iter().zip(out.iter_mut()) {
            *dst = scalar_f64(*src);
        }
    }
}

#[cfg(any(feature = "floor", feature = "dynamic-module"))]
pub mod floor {
    pub fn scalar_f64(input: f64) -> f64 {
        libm::floor(input)
    }

    pub fn view_f64(input: &[f64], out: &mut [f64]) {
        for (src, dst) in input.iter().zip(out.iter_mut()) {
            *dst = scalar_f64(*src);
        }
    }
}

#[cfg(any(feature = "ceil", feature = "dynamic-module"))]
pub mod ceil {
    pub fn scalar_f64(input: f64) -> f64 {
        libm::ceil(input)
    }

    pub fn view_f64(input: &[f64], out: &mut [f64]) {
        for (src, dst) in input.iter().zip(out.iter_mut()) {
            *dst = scalar_f64(*src);
        }
    }
}

#[cfg(any(feature = "atan2", feature = "dynamic-module"))]
pub mod atan2 {
    pub fn scalar_f64(lhs: f64, rhs: f64) -> f64 {
        libm::atan2(lhs, rhs)
    }
}
