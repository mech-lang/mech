//! Exact typed operand loans for admitted synchronous numeric kernels.
//! No source owner, pointer, permit or output escapes the selected call.
use super::*;

#[cfg(test)]
thread_local! {
    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
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
    run()
}

/// Canonical matrices are row-major; dense F64 transport is column-major.
/// Callers retain complete original input/schema/shape and resource admission.
/// Only an exact scalar schema and matching Core backing select typed reads.
pub(super) enum BorrowedNumericOperand<'a> {
    Bool(&'a [bool]),
    Index(&'a [u64]),
    U8(&'a [u8]),
    U16(&'a [u16]),
    U32(&'a [u32]),
    U64(&'a [u64]),
    U128(&'a [u128]),
    I8(&'a [i8]),
    I16(&'a [i16]),
    I32(&'a [i32]),
    I64(&'a [i64]),
    I128(&'a [i128]),
    F32(&'a [F32Bits]),
    F64(&'a [F64Bits]),
    Snapshot {
        value: &'a mech_core::Value,
        element: &'a SchemaBody,
    },
    DenseF64 {
        values: &'a [f64],
        rows: usize,
        columns: usize,
    },
}

impl<'a> BorrowedNumericOperand<'a> {
    pub(super) fn new(
        value: Option<&'a mech_core::Value>,
        schema: Option<&'a mech_core::Schema>,
        input: ResidentValueRef<'a>,
        (rows, columns): (usize, usize),
    ) -> Result<Self, ResidentKernelError> {
        if let (Some(value), Some(schema)) = (value, schema) {
            return Ok(Self::snapshot(
                value,
                comparison_element_body(schema.body()),
            ));
        }
        let ResidentValueRef::F64(values) = input else {
            return Err(ResidentKernelError::InvalidInput);
        };
        if values.len()
            != rows
                .checked_mul(columns)
                .ok_or(ResidentKernelError::InvalidShape)?
        {
            return Err(ResidentKernelError::InvalidShape);
        }
        Ok(Self::DenseF64 {
            values,
            rows,
            columns,
        })
    }

    pub(super) fn snapshot(value: &'a mech_core::Value, element: &'a SchemaBody) -> Self {
        let generic = || Self::Snapshot { value, element };
        #[cfg(test)]
        if DISABLED.with(std::cell::Cell::get) {
            return generic();
        }
        let view = match value.data() {
            ValueData::Matrix(matrix) => matrix.elements(),
            ValueData::Bool(cell) => SequenceView::Bool(core::slice::from_ref(cell)),
            ValueData::Index(cell) => SequenceView::Index(core::slice::from_ref(cell)),
            ValueData::U8(cell) => SequenceView::U8(core::slice::from_ref(cell)),
            ValueData::U16(cell) => SequenceView::U16(core::slice::from_ref(cell)),
            ValueData::U32(cell) => SequenceView::U32(core::slice::from_ref(cell)),
            ValueData::U64(cell) => SequenceView::U64(core::slice::from_ref(cell)),
            ValueData::U128(cell) => SequenceView::U128(core::slice::from_ref(cell)),
            ValueData::I8(cell) => SequenceView::I8(core::slice::from_ref(cell)),
            ValueData::I16(cell) => SequenceView::I16(core::slice::from_ref(cell)),
            ValueData::I32(cell) => SequenceView::I32(core::slice::from_ref(cell)),
            ValueData::I64(cell) => SequenceView::I64(core::slice::from_ref(cell)),
            ValueData::I128(cell) => SequenceView::I128(core::slice::from_ref(cell)),
            ValueData::F32(cell) => SequenceView::F32(core::slice::from_ref(cell)),
            ValueData::F64(cell) => SequenceView::F64(core::slice::from_ref(cell)),
            _ => return generic(),
        };
        match (element, view) {
            (SchemaBody::Bool, SequenceView::Bool(cells)) => Self::Bool(cells),
            (SchemaBody::Index, SequenceView::Index(cells)) => Self::Index(cells),
            (SchemaBody::UnsignedInteger(IntegerWidth::W8), SequenceView::U8(cells)) => {
                Self::U8(cells)
            }
            (SchemaBody::UnsignedInteger(IntegerWidth::W16), SequenceView::U16(cells)) => {
                Self::U16(cells)
            }
            (SchemaBody::UnsignedInteger(IntegerWidth::W32), SequenceView::U32(cells)) => {
                Self::U32(cells)
            }
            (SchemaBody::UnsignedInteger(IntegerWidth::W64), SequenceView::U64(cells)) => {
                Self::U64(cells)
            }
            (SchemaBody::UnsignedInteger(IntegerWidth::W128), SequenceView::U128(cells)) => {
                Self::U128(cells)
            }
            (SchemaBody::SignedInteger(IntegerWidth::W8), SequenceView::I8(cells)) => {
                Self::I8(cells)
            }
            (SchemaBody::SignedInteger(IntegerWidth::W16), SequenceView::I16(cells)) => {
                Self::I16(cells)
            }
            (SchemaBody::SignedInteger(IntegerWidth::W32), SequenceView::I32(cells)) => {
                Self::I32(cells)
            }
            (SchemaBody::SignedInteger(IntegerWidth::W64), SequenceView::I64(cells)) => {
                Self::I64(cells)
            }
            (SchemaBody::SignedInteger(IntegerWidth::W128), SequenceView::I128(cells)) => {
                Self::I128(cells)
            }
            (SchemaBody::FloatingPoint(mech_core::FloatWidth::W32), SequenceView::F32(cells)) => {
                Self::F32(cells)
            }
            (SchemaBody::FloatingPoint(mech_core::FloatWidth::W64), SequenceView::F64(cells)) => {
                Self::F64(cells)
            }
            _ => generic(),
        }
    }

    pub(super) fn at(&self, index: usize) -> Result<ValueDataDraft, ResidentKernelError> {
        match self {
            Self::Bool(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::Bool)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::Index(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::Index)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::U8(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::U8)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::U16(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::U16)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::U32(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::U32)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::U64(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::U64)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::U128(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::U128)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::I8(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::I8)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::I16(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::I16)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::I32(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::I32)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::I64(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::I64)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::I128(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::I128)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::F32(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::F32)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::F64(cells) => cells
                .get(index)
                .copied()
                .map(ValueDataDraft::F64)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::Snapshot { value, element } => {
                if let ValueData::Matrix(matrix) = value.data() {
                    let data = matrix
                        .elements()
                        .value_at(index)
                        .ok_or(ResidentKernelError::InvalidShape)?;
                    canonical_snapshot_data_draft(element, &data)
                        .map_err(|_| ResidentKernelError::InvalidInput)
                } else if index == 0 {
                    canonical_snapshot_data_draft(element, value.data())
                        .map_err(|_| ResidentKernelError::InvalidInput)
                } else {
                    Err(ResidentKernelError::InvalidShape)
                }
            }
            Self::DenseF64 {
                values,
                rows,
                columns,
            } => {
                if *columns == 0 || index >= values.len() {
                    return Err(ResidentKernelError::InvalidShape);
                }
                let row = index / columns;
                let column = index % columns;
                let offset = column
                    .checked_mul(*rows)
                    .and_then(|offset| offset.checked_add(row))
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let value = values
                    .get(offset)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                Ok(ValueDataDraft::F64(F64Bits::from_f64(*value)))
            }
        }
    }
}
