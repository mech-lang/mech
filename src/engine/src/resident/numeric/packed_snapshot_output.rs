//! Reuse Core's exact typed constructors after maintained kernel admission.
//! No source Value, pointer, permit or partially produced output is retained.
use super::*;

#[cfg(test)]
thread_local! {
    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static CONSTRUCTIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(super) fn without<T>(run: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            DISABLED.with(|v| v.set(self.0));
        }
    }
    let _reset = Reset(DISABLED.with(|v| v.replace(true)));
    super::borrowed_numeric::without(run)
}
#[cfg(test)]
pub(super) fn constructions() -> usize {
    CONSTRUCTIONS.with(|v| v.replace(0))
}

/// The caller has already admitted its original complete staging envelope.
/// One initialized typed result bank fits within the original draft bank.
/// Unsupported scalar families keep the canonical draft finalizer.
pub(super) fn try_construct(
    kernel: &BoundResidentKernel,
    shape: &ShapeInstance,
    count: usize,
    mut cell: impl FnMut(usize) -> Result<ValueDataDraft, ResidentKernelError>,
) -> Option<Result<mech_core::Value, ResidentKernelError>> {
    #[cfg(test)]
    if DISABLED.with(std::cell::Cell::get) {
        return None;
    }
    let metadata = kernel.snapshot_output()?;
    let schemas = kernel.shared_snapshot_schemas()?;
    let SchemaBody::Matrix { element, .. } = schemas.get(metadata.schema)?.body() else {
        return None;
    };
    macro_rules! construct {
        ($variant:ident, $constructor:ident) => {
            Some((|| {
                #[cfg(test)]
                CONSTRUCTIONS.with(|v| v.set(v.get() + 1));
                let mut cells = Vec::with_capacity(count);
                for index in 0..count {
                    let ValueDataDraft::$variant(value) = cell(index)? else {
                        return Err(ResidentKernelError::InvalidOutput);
                    };
                    cells.push(value);
                }
                let next = mech_core::Value::$constructor(
                    metadata.schema,
                    shape.clone(),
                    cells.into_boxed_slice(),
                    Arc::clone(schemas),
                )
                .map_err(|_| ResidentKernelError::InvalidOutput)?;
                if next.schema_key() != metadata.schema_key {
                    return Err(ResidentKernelError::InvalidOutput);
                }
                Ok(next)
            })())
        };
    }
    use mech_core::{FloatWidth as F, IntegerWidth as W};
    match element.as_ref() {
        SchemaBody::Bool => construct!(Bool, from_bool_matrix),
        SchemaBody::Index => construct!(Index, from_index_matrix),
        SchemaBody::UnsignedInteger(W::W8) => construct!(U8, from_u8_matrix),
        SchemaBody::UnsignedInteger(W::W16) => construct!(U16, from_u16_matrix),
        SchemaBody::UnsignedInteger(W::W32) => construct!(U32, from_u32_matrix),
        SchemaBody::UnsignedInteger(W::W64) => construct!(U64, from_u64_matrix),
        SchemaBody::UnsignedInteger(W::W128) => construct!(U128, from_u128_matrix),
        SchemaBody::SignedInteger(W::W8) => construct!(I8, from_i8_matrix),
        SchemaBody::SignedInteger(W::W16) => construct!(I16, from_i16_matrix),
        SchemaBody::SignedInteger(W::W32) => construct!(I32, from_i32_matrix),
        SchemaBody::SignedInteger(W::W64) => construct!(I64, from_i64_matrix),
        SchemaBody::SignedInteger(W::W128) => construct!(I128, from_i128_matrix),
        SchemaBody::FloatingPoint(F::W32) => construct!(F32, from_f32_matrix),
        SchemaBody::FloatingPoint(F::W64) => construct!(F64, from_f64_matrix),
        _ => None,
    }
}

/// Exact F64 banks need no per-cell generic draft. The caller retains the
/// original complete admission and current operands, as with `try_construct`.
pub(super) fn try_construct_f64(
    kernel: &BoundResidentKernel,
    shape: &ShapeInstance,
    count: usize,
    mut cell: impl FnMut(usize) -> Result<F64Bits, ResidentKernelError>,
) -> Option<Result<mech_core::Value, ResidentKernelError>> {
    #[cfg(test)]
    if DISABLED.with(std::cell::Cell::get) {
        return None;
    }
    let metadata = kernel.snapshot_output()?;
    let schemas = kernel.shared_snapshot_schemas()?;
    let SchemaBody::Matrix { element, .. } = schemas.get(metadata.schema)?.body() else {
        return None;
    };
    if element.as_ref() != &SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) {
        return None;
    }
    Some((|| {
        #[cfg(test)]
        CONSTRUCTIONS.with(|v| v.set(v.get() + 1));
        let mut cells = Vec::with_capacity(count);
        for index in 0..count {
            cells.push(cell(index)?);
        }
        let next = mech_core::Value::from_f64_matrix(
            metadata.schema,
            shape.clone(),
            cells.into_boxed_slice(),
            Arc::clone(schemas),
        )
        .map_err(|_| ResidentKernelError::InvalidOutput)?;
        if next.schema_key() != metadata.schema_key {
            return Err(ResidentKernelError::InvalidOutput);
        }
        Ok(next)
    })())
}
