//! Pure native String map/join loop over entry-bound owners and lookup table.
use super::{NativeCompileError, backend};
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_artifact::NativeStringMapRegion;

#[path = "program_string_map_batches.rs"]
mod batches;

pub(in crate::native_artifact) fn define<M: Module>(
    module: &mut M,
    region: &NativeStringMapRegion,
) -> Result<(FuncId, u64, u64), NativeCompileError> {
    let mut signature = module.make_signature();
    signature.params = vec![AbiParam::new(types::I64); 5];
    signature.returns.push(AbiParam::new(types::I64));
    let function = module
        .declare_function(
            &format!("mech_string_map_region_{}", region.collection()),
            Linkage::Local,
            &signature,
        )
        .map_err(backend)?;
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        let header = b.create_block();
        let lookup = b.create_block();
        let write = b.create_block();
        let done = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(header, types::I64);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let (source, length, table, output) = (args[0], args[1], args[2], args[3]);
        let offset = batches::emit(&mut b, source, length, table, output);
        b.ins().jump(header, &[offset.into()]);
        b.switch_to_block(header);
        let index = b.block_params(header)[0];
        let more = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
        b.ins().brif(more, lookup, &[], done, &[]);
        b.switch_to_block(lookup);
        let address = b.ins().iadd(source, index);
        let byte = b.ins().load(types::I8, MemFlags::new(), address, 0);
        let key = b.ins().uextend(types::I64, byte);
        let offset = b.ins().ishl_imm(key, 1);
        let address = b.ins().iadd(table, offset);
        let value = b.ins().load(types::I16, MemFlags::new(), address, 0);
        let present = b.ins().icmp_imm(IntCC::UnsignedLessThan, value, 128);
        b.ins().brif(present, write, &[], done, &[]);
        b.switch_to_block(write);
        let address = b.ins().iadd(output, index);
        let byte = b.ins().ireduce(types::I8, value);
        b.ins().store(MemFlags::new(), byte, address, 0);
        let next = b.ins().iadd_imm(index, 1);
        b.ins().jump(header, &[next.into()]);
        b.switch_to_block(done);
        // A short prefix identifies the FIRST missing key. The owner callback
        // delegates that one failure to the existing selected Map operation.
        b.ins().return_(&[index]);
        b.seal_all_blocks();
        b.finalize();
    }
    module
        .define_function(function, &mut context)
        .map_err(backend)?;
    let compiled = context
        .compiled_code()
        .ok_or_else(|| backend("String map code missing"))?;
    let stack = super::super::compiler::compiled_stack_bytes(&compiled.buffer)?;
    Ok((function, stack, compiled.code_info().total_size as u64))
}
