//! Pure native String map/join loop over entry-bound owners and lookup table.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::compiler::backend;
use crate::native_artifact::program::codegen::ascii_map as ascii_map_codegen;
use crate::native_artifact::program::codegen::ascii_map::{ScalarTable, Tables};
use crate::native_artifact::program::runtime::maps::ascii::MappingKind;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, Endianness, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_artifact::NativeStringMapRegion;

mod batches;

mod simd;

pub(in crate::native_artifact) fn define<M: Module>(
    module: &mut M,
    region: &NativeStringMapRegion,
) -> Result<(FuncId, u64, u64), NativeCompileError> {
    let vectorized =
        module.isa().name() == "aarch64" && module.isa().endianness() == Endianness::Little;
    define_impl(module, region, vectorized)
}

fn define_impl<M: Module>(
    module: &mut M,
    region: &NativeStringMapRegion,
    vectorized: bool,
) -> Result<(FuncId, u64, u64), NativeCompileError> {
    let full = define_leaf(module, region, vectorized, MappingKind::Full)?;
    let projected = define_leaf(module, region, vectorized, MappingKind::Projected)?;
    ascii_map_codegen::dispatch(
        module,
        &format!("mech_string_map_region_{}", region.collection()),
        full,
        projected,
    )
}

fn define_leaf<M: Module>(
    module: &mut M,
    region: &NativeStringMapRegion,
    vectorized: bool,
    kind: MappingKind,
) -> Result<(FuncId, u64, u64), NativeCompileError> {
    let mut signature = module.make_signature();
    signature.params = vec![AbiParam::new(types::I64); 5];
    signature.returns.push(AbiParam::new(types::I64));
    let function = module
        .declare_function(
            &format!(
                "mech_string_map_region_{}_{}",
                region.collection(),
                kind.suffix()
            ),
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
        let scalar = ScalarTable::new(&mut b, table, kind);
        let start = if vectorized {
            simd::emit(&mut b, source, length, table, output, kind)
        } else {
            b.ins().iconst(types::I64, 0)
        };
        let offset = batches::emit(&mut b, source, length, &scalar, output, start);
        b.ins().jump(header, &[offset.into()]);
        b.switch_to_block(header);
        let index = b.block_params(header)[0];
        let more = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
        b.ins().brif(more, lookup, &[], done, &[]);
        b.switch_to_block(lookup);
        let address = b.ins().iadd(source, index);
        let byte = b.ins().load(types::I8, MemFlags::new(), address, 0);
        let key = b.ins().uextend(types::I64, byte);
        let value = scalar.lookup(&mut b, key);
        let present = b.ins().icmp_imm(IntCC::UnsignedLessThan, value, 128);
        b.ins().brif(present, write, &[], done, &[]);
        b.switch_to_block(write);
        let address = b.ins().iadd(output, index);
        b.ins().store(MemFlags::new(), value, address, 0);
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
    let stack = crate::native_artifact::compiler::compiled_stack_bytes(&compiled.buffer)?;
    Ok((function, stack, compiled.code_info().total_size as u64))
}

#[cfg(all(test, feature = "canonical-native-program-host"))]
mod tests;
