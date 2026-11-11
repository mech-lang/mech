//! One physical fixed-element layout for native loads, stores and comparisons.
use super::layout::{Kind, Type};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;

fn native(bytes: u8) -> Result<cranelift_codegen::ir::Type, NativeCompileError> {
    match bytes {
        1 => Ok(types::I8),
        2 => Ok(types::I16),
        4 => Ok(types::I32),
        8 => Ok(types::I64),
        _ => Err(backend("compact packed scalar width is unsupported")),
    }
}
pub(super) fn load(
    b: &mut FunctionBuilder<'_>,
    address: Value,
    ty: &Type,
) -> Result<Vec<Value>, NativeCompileError> {
    match &ty.kind {
        Kind::Utf8 => Ok(vec![
            b.ins().load(types::I64, MemFlags::new(), address, 0),
            b.ins().load(types::I64, MemFlags::new(), address, 8),
        ]),
        Kind::Scalar { bytes, signed, .. } => {
            let value = b.ins().load(native(*bytes)?, MemFlags::new(), address, 0);
            Ok(vec![if *bytes == 8 {
                value
            } else if *signed {
                b.ins().sextend(types::I64, value)
            } else {
                b.ins().uextend(types::I64, value)
            }])
        }
        Kind::Product(fields) => {
            let mut result = vec![];
            for field in fields {
                let at = field
                    .packed_offset
                    .ok_or_else(|| backend("compact packed field has no physical offset"))?;
                let address = b.ins().iadd_imm(address, at as i64);
                result.extend(load(b, address, &field.ty)?);
            }
            Ok(result)
        }
        _ => Err(backend(
            "compact packed element requires fixed scalar/product",
        )),
    }
}
pub(super) fn store(
    b: &mut FunctionBuilder<'_>,
    address: Value,
    ty: &Type,
    values: &[Value],
) -> Result<(), NativeCompileError> {
    if values.len() != ty.words {
        return Err(backend("compact packed store semantic width mismatch"));
    }
    match &ty.kind {
        Kind::Utf8 => {
            b.ins().store(MemFlags::new(), values[0], address, 0);
            b.ins().store(MemFlags::new(), values[1], address, 8);
        }
        Kind::Scalar { bytes, .. } => {
            let value = if *bytes == 8 {
                values[0]
            } else {
                b.ins().ireduce(native(*bytes)?, values[0])
            };
            b.ins().store(MemFlags::new(), value, address, 0);
        }
        Kind::Product(fields) => {
            for field in fields {
                let at = field
                    .packed_offset
                    .ok_or_else(|| backend("compact packed field has no physical offset"))?;
                let address = b.ins().iadd_imm(address, at as i64);
                store(
                    b,
                    address,
                    &field.ty,
                    &values[field.offset..field.offset + field.ty.words],
                )?;
            }
        }
        _ => {
            return Err(backend(
                "compact packed element requires fixed scalar/product",
            ));
        }
    }
    Ok(())
}
