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

#[cfg(any(feature = "sin", feature = "dynamic-module"))]
pub mod sin {
    pub fn scalar_f64(input: f64) -> f64 {
        libm::sin(input)
    }

    pub fn view_f64(input: &[f64], out: &mut [f64]) {
        for (src, dst) in input.iter().zip(out.iter_mut()) {
            *dst = scalar_f64(*src);
        }
    }
}

#[cfg(any(feature = "cos", feature = "dynamic-module"))]
pub mod cos {
    pub fn scalar_f64(input: f64) -> f64 {
        libm::cos(input)
    }

    pub fn view_f64(input: &[f64], out: &mut [f64]) {
        for (src, dst) in input.iter().zip(out.iter_mut()) {
            *dst = scalar_f64(*src);
        }
    }
}

#[cfg(any(feature = "tan", feature = "dynamic-module"))]
pub mod tan {
    pub fn scalar_f64(input: f64) -> f64 {
        libm::tan(input)
    }

    pub fn view_f64(input: &[f64], out: &mut [f64]) {
        for (src, dst) in input.iter().zip(out.iter_mut()) {
            *dst = scalar_f64(*src);
        }
    }
}

#[cfg(any(feature = "asin", feature = "dynamic-module"))]
pub mod asin {
    pub fn scalar_f64(input: f64) -> f64 {
        libm::asin(input)
    }

    pub fn view_f64(input: &[f64], out: &mut [f64]) {
        for (src, dst) in input.iter().zip(out.iter_mut()) {
            *dst = scalar_f64(*src);
        }
    }
}

#[cfg(any(feature = "acos", feature = "dynamic-module"))]
pub mod acos {
    pub fn scalar_f64(input: f64) -> f64 {
        libm::acos(input)
    }

    pub fn view_f64(input: &[f64], out: &mut [f64]) {
        for (src, dst) in input.iter().zip(out.iter_mut()) {
            *dst = scalar_f64(*src);
        }
    }
}

#[cfg(any(feature = "atan", feature = "dynamic-module"))]
pub mod atan {
    pub fn scalar_f64(input: f64) -> f64 {
        libm::atan(input)
    }

    pub fn view_f64(input: &[f64], out: &mut [f64]) {
        for (src, dst) in input.iter().zip(out.iter_mut()) {
            *dst = scalar_f64(*src);
        }
    }
}
