mod host_call;
mod resource_read;
mod resource_write;

pub use host_call::*;
pub use resource_read::*;
pub use resource_write::*;

#[cfg(feature = "compiler")]
use mech_core::{BytecodeCompilerContext, CompileConst, MResult, Register, ValRef, Value};

#[cfg(feature = "compiler")]
pub(super) fn compile_external_output(
    output: &ValRef,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let value = output.borrow();
    let pointer = external_value_pointer(&value, output.addr());
    let (register, initialize) = context.register_for_ptr_with_initialization_status(pointer);
    if initialize {
        let constant = compile_external_constant(&value, context)?;
        context.emit_const_load(register, constant);
    }
    Ok(register)
}

#[cfg(feature = "compiler")]
pub(super) fn compile_external_value(
    value: &Value,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let pointer = external_value_pointer(value, value as *const Value as usize);
    let (register, initialize) = context.register_for_ptr_with_initialization_status(pointer);
    if initialize {
        let constant = compile_external_constant(value, context)?;
        context.emit_const_load(register, constant);
    }
    Ok(register)
}

#[cfg(feature = "compiler")]
fn external_value_pointer(value: &Value, fallback: usize) -> usize {
    match value {
        // Mutable references are transparent to the bytecode value model. Use
        // the referenced value's stable pointer so a symbol that wraps another
        // node's output reuses that producer's register instead of compiling a
        // detached constant with the same current value.
        Value::MutableReference(reference) => {
            external_value_pointer(&reference.borrow(), reference.addr())
        }
        Value::Typed(value, _) => external_value_pointer(value, fallback),
        Value::Id(_) | Value::Kind(_) | Value::IndexAll | Value::EmptyKind(_) | Value::Empty => {
            fallback
        }
        _ => value.addr(),
    }
}

#[cfg(feature = "compiler")]
fn compile_external_constant(
    value: &Value,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<u32> {
    match value {
        Value::MutableReference(reference) => {
            compile_external_constant(&reference.borrow(), context)
        }
        _ => value.compile_const(context),
    }
}
