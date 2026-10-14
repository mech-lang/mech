//! Numeric rewrite eligibility shared by native compiler pathways.

/// An exactly representable binary divisor and reciprocal have the same exact
/// real product/quotient before f64 rounding (including subnormal results).
pub fn exact_binary_reciprocal(bits: u64) -> Option<u64> {
    let magnitude = bits & !(1u64 << 63);
    let exponent = magnitude >> 52;
    let mantissa = magnitude & ((1u64 << 52) - 1);
    let power = if exponent == 0 {
        mantissa.count_ones() == 1
    } else {
        exponent < 0x7ff && mantissa == 0
    };
    let inverse = 1.0 / f64::from_bits(bits);
    (power && inverse.is_finite() && inverse != 0.0).then_some(inverse.to_bits())
}
