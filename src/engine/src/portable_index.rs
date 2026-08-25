//! Host-independent conversion for scalar matrix selectors.
//!
//! Legacy source execution stores indices in `usize`, while resident artifacts
//! store them in `u64`.  Restrict both paths to the width available on every
//! supported host so the same selector cannot address different elements on
//! native and wasm32 runtimes.

pub(crate) const PORTABLE_INDEX_MAX: u64 = u32::MAX as u64;

pub(crate) trait ToPortableIndex {
    fn to_portable_index(self) -> Option<u64>;
}

macro_rules! impl_unsigned_portable_index {
    ($($kind:ty),+ $(,)?) => {
        $(
            impl ToPortableIndex for $kind {
                fn to_portable_index(self) -> Option<u64> {
                    let value = u128::from(self);
                    (value <= u128::from(PORTABLE_INDEX_MAX)).then_some(value as u64)
                }
            }
        )+
    };
}

macro_rules! impl_signed_portable_index {
    ($($kind:ty),+ $(,)?) => {
        $(
            impl ToPortableIndex for $kind {
                fn to_portable_index(self) -> Option<u64> {
                    let value = i128::from(self);
                    (value >= 0 && value <= i128::from(PORTABLE_INDEX_MAX))
                        .then_some(value as u64)
                }
            }
        )+
    };
}

impl_unsigned_portable_index!(u8, u16, u32, u64, u128);
impl_signed_portable_index!(i8, i16, i32, i64, i128);

impl ToPortableIndex for usize {
    fn to_portable_index(self) -> Option<u64> {
        let value = self as u128;
        (value <= u128::from(PORTABLE_INDEX_MAX)).then_some(value as u64)
    }
}

impl ToPortableIndex for f32 {
    fn to_portable_index(self) -> Option<u64> {
        let value = f64::from(self);
        (value.is_finite() && value >= 0.0 && value <= PORTABLE_INDEX_MAX as f64)
            .then_some(value.trunc() as u64)
    }
}

impl ToPortableIndex for f64 {
    fn to_portable_index(self) -> Option<u64> {
        (self.is_finite() && self >= 0.0 && self <= PORTABLE_INDEX_MAX as f64)
            .then_some(self.trunc() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_index_has_one_boundary_for_every_scalar_family() {
        assert_eq!(u32::MAX.to_portable_index(), Some(PORTABLE_INDEX_MAX));
        assert_eq!((u64::from(u32::MAX) + 1).to_portable_index(), None);
        assert_eq!((-1_i64).to_portable_index(), None);
        assert_eq!(1.75_f64.to_portable_index(), Some(1));
        assert_eq!((PORTABLE_INDEX_MAX as f64 + 1.0).to_portable_index(), None);
        assert_eq!(f64::NAN.to_portable_index(), None);
    }
}
