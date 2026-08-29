//! Compatibility construction of canonical function invocations.

use crate::{
    FunctionArgs, FunctionInvocation, FunctionInvocationLayout, LegacyValue, ValueCell,
    ValueDataDraft,
};

#[cfg(feature = "matrix")]
use crate::matrix::Matrix;

#[cfg(feature = "no_std")]
use alloc::{vec, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::vec::Vec;

impl From<FunctionArgs> for FunctionInvocation {
    fn from(args: FunctionArgs) -> Self {
        function_invocation_from_legacy(args)
    }
}

pub fn function_invocation_from_legacy(args: FunctionArgs) -> FunctionInvocation {
    let compatibility_args = args.clone();
    let (layout, output, inputs) = match args {
        FunctionArgs::Nullary(output) => (FunctionInvocationLayout::Nullary, output, Vec::new()),
        FunctionArgs::Unary(output, input) => {
            (FunctionInvocationLayout::Unary, output, vec![input])
        }
        FunctionArgs::Binary(output, first, second) => (
            FunctionInvocationLayout::Binary,
            output,
            vec![first, second],
        ),
        FunctionArgs::Ternary(output, first, second, third) => (
            FunctionInvocationLayout::Ternary,
            output,
            vec![first, second, third],
        ),
        FunctionArgs::Quaternary(output, first, second, third, fourth) => (
            FunctionInvocationLayout::Quaternary,
            output,
            vec![first, second, third, fourth],
        ),
        FunctionArgs::Variadic(output, inputs) => {
            (FunctionInvocationLayout::Variadic, output, inputs)
        }
    };
    FunctionInvocation::from_legacy_cells(
        layout,
        value_cell_from_legacy_function_value(output),
        inputs
            .into_iter()
            .map(value_cell_from_legacy_function_value)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        compatibility_args,
    )
}

macro_rules! exact_matrix_cell {
    ($matrix:expr) => {{
        match $matrix {
            #[cfg(feature = "matrix1")]
            Matrix::Matrix1(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "matrix2")]
            Matrix::Matrix2(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "matrix3")]
            Matrix::Matrix3(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "matrix4")]
            Matrix::Matrix4(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "matrix2x3")]
            Matrix::Matrix2x3(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "matrix3x2")]
            Matrix::Matrix3x2(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "row_vector2")]
            Matrix::RowVector2(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "row_vector3")]
            Matrix::RowVector3(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "row_vector4")]
            Matrix::RowVector4(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "vector2")]
            Matrix::Vector2(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "vector3")]
            Matrix::Vector3(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "vector4")]
            Matrix::Vector4(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "row_vectord")]
            Matrix::RowDVector(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "vectord")]
            Matrix::DVector(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "matrixd")]
            Matrix::DMatrix(reference) => inferred_matrix_reference_cell!(reference),
        }
    }};
}

macro_rules! inferred_matrix_reference_cell {
    ($reference:expr) => {{
        let reference = $reference;
        let extents = {
            let matrix = reference.borrow();
            (matrix.nrows(), matrix.ncols())
        };
        ValueCell::from_inferred_ref(reference, Some(extents))
            .expect("exact legacy matrix has a canonical cell representation")
    }};
}

fn value_cell_from_legacy_function_value(value: LegacyValue) -> ValueCell {
    match value {
        #[cfg(feature = "u8")]
        LegacyValue::U8(reference) => inferred_cell(reference),
        #[cfg(feature = "u16")]
        LegacyValue::U16(reference) => inferred_cell(reference),
        #[cfg(feature = "u32")]
        LegacyValue::U32(reference) => inferred_cell(reference),
        #[cfg(feature = "u64")]
        LegacyValue::U64(reference) => inferred_cell(reference),
        #[cfg(feature = "u128")]
        LegacyValue::U128(reference) => inferred_cell(reference),
        #[cfg(feature = "i8")]
        LegacyValue::I8(reference) => inferred_cell(reference),
        #[cfg(feature = "i16")]
        LegacyValue::I16(reference) => inferred_cell(reference),
        #[cfg(feature = "i32")]
        LegacyValue::I32(reference) => inferred_cell(reference),
        #[cfg(feature = "i64")]
        LegacyValue::I64(reference) => inferred_cell(reference),
        #[cfg(feature = "i128")]
        LegacyValue::I128(reference) => inferred_cell(reference),
        #[cfg(feature = "f32")]
        LegacyValue::F32(reference) => inferred_cell(reference),
        #[cfg(feature = "f64")]
        LegacyValue::F64(reference) => inferred_cell(reference),
        #[cfg(feature = "string")]
        LegacyValue::String(reference) => inferred_cell(reference),
        #[cfg(all(not(feature = "string"), feature = "variable_define"))]
        value @ LegacyValue::String(_) => ValueCell::new(value),
        #[cfg(feature = "bool")]
        LegacyValue::Bool(reference) => inferred_cell(reference),
        #[cfg(all(not(feature = "bool"), feature = "variable_define"))]
        value @ LegacyValue::Bool(_) => ValueCell::new(value),
        #[cfg(feature = "complex")]
        LegacyValue::C64(reference) => inferred_cell(reference),
        #[cfg(feature = "rational")]
        LegacyValue::R64(reference) => inferred_cell(reference),
        LegacyValue::Index(reference) => inferred_cell(reference),
        LegacyValue::Id(value) => {
            ValueCell::from_inferred_value_data(crate::SchemaBody::Id, ValueDataDraft::Id(value))
                .expect("legacy ID has a canonical cell representation")
        }
        #[cfg(feature = "matrix")]
        LegacyValue::MatrixIndex(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "bool"))]
        LegacyValue::MatrixBool(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "u8"))]
        LegacyValue::MatrixU8(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "u16"))]
        LegacyValue::MatrixU16(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "u32"))]
        LegacyValue::MatrixU32(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "u64"))]
        LegacyValue::MatrixU64(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "u128"))]
        LegacyValue::MatrixU128(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "i8"))]
        LegacyValue::MatrixI8(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "i16"))]
        LegacyValue::MatrixI16(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "i32"))]
        LegacyValue::MatrixI32(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "i64"))]
        LegacyValue::MatrixI64(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "i128"))]
        LegacyValue::MatrixI128(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "f32"))]
        LegacyValue::MatrixF32(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "f64"))]
        LegacyValue::MatrixF64(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "string"))]
        LegacyValue::MatrixString(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "rational"))]
        LegacyValue::MatrixR64(matrix) => exact_matrix_cell!(matrix),
        #[cfg(all(feature = "matrix", feature = "complex"))]
        LegacyValue::MatrixC64(matrix) => exact_matrix_cell!(matrix),
        value => ValueCell::new(value),
    }
}

fn inferred_cell<T>(reference: crate::Ref<T>) -> ValueCell
where
    T: crate::CanonicalCellBacking,
{
    ValueCell::from_inferred_ref(reference, None)
        .expect("exact legacy function value has a canonical cell representation")
}
