//! Unbuilt generic numeric prototype; no dependency on the runtime or scores.
//! Algebraic policy is explicit and uses pinned nightly's float_algebraic API.
//! Indexing, allocation, shape checks and comparisons always stay strict.
#![cfg_attr(any(feature = "algebraic-add", feature = "algebraic-all"), feature(float_algebraic))]

pub trait Arithmetic {
    fn add(left: f64, right: f64) -> f64;
    fn sub(left: f64, right: f64) -> f64;
    fn mul(left: f64, right: f64) -> f64;
    fn div(left: f64, right: f64) -> f64;
}

pub struct Strict;
impl Arithmetic for Strict {
    #[inline(always)]
    fn add(left: f64, right: f64) -> f64 { left + right }
    #[inline(always)]
    fn sub(left: f64, right: f64) -> f64 { left - right }
    #[inline(always)]
    fn mul(left: f64, right: f64) -> f64 { left * right }
    #[inline(always)]
    fn div(left: f64, right: f64) -> f64 { left / right }
}

#[cfg(feature = "algebraic-add")]
pub struct AlgebraicReduction;
#[cfg(feature = "algebraic-add")]
impl Arithmetic for AlgebraicReduction {
    #[inline(always)]
    fn add(left: f64, right: f64) -> f64 { left.algebraic_add(right) }
    #[inline(always)]
    fn sub(left: f64, right: f64) -> f64 { left - right }
    #[inline(always)]
    fn mul(left: f64, right: f64) -> f64 { left * right }
    #[inline(always)]
    fn div(left: f64, right: f64) -> f64 { left / right }
}

#[cfg(feature = "algebraic-all")]
pub struct AlgebraicBroad;
#[cfg(feature = "algebraic-all")]
impl Arithmetic for AlgebraicBroad {
    #[inline(always)]
    fn add(left: f64, right: f64) -> f64 { left.algebraic_add(right) }
    #[inline(always)]
    fn sub(left: f64, right: f64) -> f64 { left - right }
    #[inline(always)]
    fn mul(left: f64, right: f64) -> f64 { left.algebraic_mul(right) }
    #[inline(always)]
    fn div(left: f64, right: f64) -> f64 { left.algebraic_div(right) }
}

/// Callers supply the original seed: resident scalar rows use +0.0 whereas
/// Rust iterator Sum uses -0.0. Strict controls must preserve that distinction.
pub fn sum<P: Arithmetic>(input: &[f64], seed: f64) -> f64 {
    input.iter().fold(seed, |total, &value| P::add(total, value))
}

pub fn dot<P: Arithmetic>(left: &[f64], right: &[f64], seed: f64) -> f64 {
    assert_eq!(left.len(), right.len());
    left.iter().zip(right).fold(seed, |total, (&left, &right)| {
        P::add(total, P::mul(left, right))
    })
}

/// Coefficients are arbitrary inputs supplied by the existing source graph.
/// This helper contains no matrix formula or benchmark-specific recurrence.
pub fn weighted_row<P: Arithmetic, const DIVIDE: bool>(
    input: &[f64], coefficients: &[f64], seed: f64,
) -> f64 {
    assert_eq!(input.len(), coefficients.len());
    input.iter().zip(coefficients).fold(seed, |total, (&value, &coefficient)| {
        let term = if DIVIDE { P::div(value, coefficient) } else { P::mul(value, coefficient) };
        P::add(total, term)
    })
}

#[derive(Clone, Copy)]
pub enum Binary { Add, Sub, Mul, Div }

/// Monomorphized policy exposes the same independent elementwise slice loop.
pub fn elementwise<P: Arithmetic>(operation: Binary, left: &[f64], right: &[f64], output: &mut [f64]) {
    assert_eq!(left.len(), right.len());
    assert_eq!(left.len(), output.len());
    match operation {
        Binary::Add => for ((out, &a), &b) in output.iter_mut().zip(left).zip(right) { *out = P::add(a, b); },
        Binary::Sub => for ((out, &a), &b) in output.iter_mut().zip(left).zip(right) { *out = P::sub(a, b); },
        Binary::Mul => for ((out, &a), &b) in output.iter_mut().zip(left).zip(right) { *out = P::mul(a, b); },
        Binary::Div => for ((out, &a), &b) in output.iter_mut().zip(left).zip(right) { *out = P::div(a, b); },
    }
}
