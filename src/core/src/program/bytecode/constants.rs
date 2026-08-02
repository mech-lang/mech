use crate::{MResult, Ref, Value};

#[cfg(feature = "no_std")]
use alloc::vec::Vec;

use super::{ByteReader, MatrixStorage, RuntimeType, checked_usize, invalid, owned_utf8};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedConstant {
    pub runtime_type: RuntimeType,
    pub alignment: u8,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstantEntry {
    pub type_id: u32,
    pub encoding: u8,
    pub alignment: u8,
    pub flags: u16,
    pub offset: u64,
    pub length: u64,
}

pub fn decode_constants(
    types: &[RuntimeType],
    entries: &[ConstantEntry],
    blob: &[u8],
) -> MResult<Vec<Value>> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(entries.len())
        .map_err(|_| invalid::<()>("unable to allocate decoded constants").unwrap_err())?;
    for entry in entries {
        values.push(decode_constant(types, entry, blob)?);
    }
    Ok(values)
}

fn decode_constant(types: &[RuntimeType], entry: &ConstantEntry, blob: &[u8]) -> MResult<Value> {
    if entry.encoding != 1 {
        return invalid("unsupported bytecode constant encoding");
    }
    if entry.flags != 0 {
        return invalid("constant entry flags must be zero");
    }
    if !matches!(entry.alignment, 1 | 2 | 4 | 8 | 16) {
        return invalid("invalid constant alignment");
    }
    if entry.offset % u64::from(entry.alignment) != 0 {
        return invalid("misaligned constant entry");
    }
    let start = usize::try_from(entry.offset)
        .map_err(|_| invalid::<()>("constant offset exceeds address space").unwrap_err())?;
    let length = usize::try_from(entry.length)
        .map_err(|_| invalid::<()>("constant length exceeds address space").unwrap_err())?;
    let end = start
        .checked_add(length)
        .ok_or_else(|| invalid::<()>("constant range overflow").unwrap_err())?;
    let bytes = blob
        .get(start..end)
        .ok_or_else(|| invalid::<()>("constant entry is outside ConstantBlob").unwrap_err())?;
    let type_id = checked_usize(u64::from(entry.type_id), "constant type ID")?;
    let ty = types
        .get(type_id)
        .ok_or_else(|| invalid::<()>("constant type ID is out of range").unwrap_err())?;
    match ty {
        RuntimeType::Empty => {
            if !bytes.is_empty() {
                return invalid("Empty constant must have zero payload bytes");
            }
            Ok(Value::Empty)
        }
        RuntimeType::Bool => {
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            {
                let value = match bytes {
                    [0] => false,
                    [1] => true,
                    _ => return invalid("Bool constant must be exactly 0x00 or 0x01"),
                };
                Ok(Value::Bool(Ref::new(value)))
            }
            #[cfg(not(any(feature = "bool", feature = "variable_define")))]
            {
                invalid("Bool constants are unavailable in this runtime")
            }
        }
        RuntimeType::String => {
            #[cfg(any(feature = "string", feature = "variable_define"))]
            {
                let value = owned_utf8(bytes, "String constant")?;
                Ok(Value::String(Ref::new(value)))
            }
            #[cfg(not(any(feature = "string", feature = "variable_define")))]
            {
                invalid("String constants are unavailable in this runtime")
            }
        }
        RuntimeType::Index => {
            if bytes.len() != 8 {
                return invalid("Index constant must contain eight bytes");
            }
            let raw = u64::from_le_bytes(bytes.try_into().unwrap());
            let value = usize::try_from(raw)
                .map_err(|_| invalid::<()>("Index constant exceeds usize").unwrap_err())?;
            Ok(Value::Index(Ref::new(value)))
        }
        RuntimeType::F64 => {
            #[cfg(feature = "f64")]
            {
                if bytes.len() != 8 {
                    return invalid("F64 constant must contain eight bytes");
                }
                Ok(Value::F64(Ref::new(f64::from_bits(u64::from_le_bytes(
                    bytes.try_into().unwrap(),
                )))))
            }
            #[cfg(not(feature = "f64"))]
            {
                invalid("F64 constants are unavailable in this runtime")
            }
        }
        RuntimeType::Matrix {
            element,
            storage,
            rows,
            cols,
        } if **element == RuntimeType::F64 => decode_f64_matrix(*storage, *rows, *cols, bytes),
        _ => invalid(format!(
            "constant decoding is unsupported for runtime type {ty:?}"
        )),
    }
}

fn decode_f64_matrix(storage: MatrixStorage, rows: u32, cols: u32, bytes: &[u8]) -> MResult<Value> {
    #[cfg(all(feature = "matrix", feature = "f64"))]
    {
        let mut reader = ByteReader::new(bytes);
        let encoded_rows = reader.read_u32("matrix constant rows")?;
        let encoded_cols = reader.read_u32("matrix constant columns")?;
        if (encoded_rows, encoded_cols) != (rows, cols) || !storage.validate_dimensions(rows, cols)
        {
            return invalid("matrix constant shape disagrees with RuntimeType");
        }
        let row_count = checked_usize(u64::from(rows), "matrix row count")?;
        let column_count = checked_usize(u64::from(cols), "matrix column count")?;
        let element_count = row_count
            .checked_mul(column_count)
            .ok_or_else(|| invalid::<()>("matrix element count overflow").unwrap_err())?;
        let element_bytes = element_count
            .checked_mul(8)
            .ok_or_else(|| invalid::<()>("matrix element byte length overflow").unwrap_err())?;
        if reader.remaining() != element_bytes {
            return invalid("matrix constant element count disagrees with payload length");
        }
        let mut elements = Vec::new();
        elements.try_reserve_exact(element_count).map_err(|_| {
            invalid::<()>("unable to allocate matrix constant elements").unwrap_err()
        })?;
        for _ in 0..element_count {
            elements.push(f64::from_bits(reader.read_u64("matrix constant element")?));
        }
        if !reader.is_empty() {
            return invalid("matrix constant has trailing bytes");
        }
        let value = match storage {
            #[cfg(feature = "matrix1")]
            MatrixStorage::Matrix1 => {
                crate::matrix::Matrix::Matrix1(Ref::new(na::Matrix1::from_row_slice(&elements)))
            }
            #[cfg(feature = "matrix2")]
            MatrixStorage::Matrix2 => {
                crate::matrix::Matrix::Matrix2(Ref::new(na::Matrix2::from_row_slice(&elements)))
            }
            #[cfg(feature = "matrix3")]
            MatrixStorage::Matrix3 => {
                crate::matrix::Matrix::Matrix3(Ref::new(na::Matrix3::from_row_slice(&elements)))
            }
            #[cfg(feature = "matrix4")]
            MatrixStorage::Matrix4 => {
                crate::matrix::Matrix::Matrix4(Ref::new(na::Matrix4::from_row_slice(&elements)))
            }
            #[cfg(feature = "matrix2x3")]
            MatrixStorage::Matrix2x3 => {
                crate::matrix::Matrix::Matrix2x3(Ref::new(na::Matrix2x3::from_row_slice(&elements)))
            }
            #[cfg(feature = "matrix3x2")]
            MatrixStorage::Matrix3x2 => {
                crate::matrix::Matrix::Matrix3x2(Ref::new(na::Matrix3x2::from_row_slice(&elements)))
            }
            #[cfg(feature = "row_vector2")]
            MatrixStorage::RowVector2 => crate::matrix::Matrix::RowVector2(Ref::new(
                na::RowVector2::from_row_slice(&elements),
            )),
            #[cfg(feature = "row_vector3")]
            MatrixStorage::RowVector3 => crate::matrix::Matrix::RowVector3(Ref::new(
                na::RowVector3::from_row_slice(&elements),
            )),
            #[cfg(feature = "row_vector4")]
            MatrixStorage::RowVector4 => crate::matrix::Matrix::RowVector4(Ref::new(
                na::RowVector4::from_row_slice(&elements),
            )),
            #[cfg(feature = "vector2")]
            MatrixStorage::Vector2 => {
                crate::matrix::Matrix::Vector2(Ref::new(na::Vector2::from_column_slice(&elements)))
            }
            #[cfg(feature = "vector3")]
            MatrixStorage::Vector3 => {
                crate::matrix::Matrix::Vector3(Ref::new(na::Vector3::from_column_slice(&elements)))
            }
            #[cfg(feature = "vector4")]
            MatrixStorage::Vector4 => {
                crate::matrix::Matrix::Vector4(Ref::new(na::Vector4::from_column_slice(&elements)))
            }
            #[cfg(feature = "row_vectord")]
            MatrixStorage::RowVectorD => crate::matrix::Matrix::RowDVector(Ref::new(
                na::RowDVector::from_row_slice(&elements),
            )),
            #[cfg(feature = "vectord")]
            MatrixStorage::VectorD => {
                crate::matrix::Matrix::DVector(Ref::new(na::DVector::from_column_slice(&elements)))
            }
            #[cfg(feature = "matrixd")]
            MatrixStorage::MatrixD => crate::matrix::Matrix::DMatrix(Ref::new(
                na::DMatrix::from_row_slice(row_count, column_count, &elements),
            )),
            _ => {
                return invalid(format!(
                    "matrix storage {storage:?} is unavailable in this runtime"
                ));
            }
        };
        Ok(Value::MatrixF64(value))
    }
    #[cfg(not(all(feature = "matrix", feature = "f64")))]
    {
        let _ = (storage, rows, cols, bytes);
        invalid("F64 matrix constants are unavailable in this runtime")
    }
}
