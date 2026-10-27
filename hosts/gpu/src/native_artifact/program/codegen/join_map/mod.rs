//! Direct generated mapping/copying from an owning selected line chunk.
//! Original join/map admissions fund the complete bounded leaf. A missing
//! mapping records its first ordinal, then copies the remaining source raw;
//! only the original later map boundary reports that source-level failure.
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

pub(in crate::native_artifact) fn symbol(collection: u32) -> String {
    format!("mech_join_map_copy_region_{collection}")
}

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
    ascii_map_codegen::dispatch(module, &symbol(region.collection()), full, projected)
}

fn define_leaf<M: Module>(
    module: &mut M,
    region: &NativeStringMapRegion,
    vectorized: bool,
    kind: MappingKind,
) -> Result<(FuncId, u64, u64), NativeCompileError> {
    let mut signature = module.make_signature();
    // Owned geometry, exact output extent, immutable map table, output spare
    // capacity, and whether this chunk still precedes any missing key.
    signature.params = vec![AbiParam::new(types::I64); 5];
    signature.returns.push(AbiParam::new(types::I64));
    let function = module
        .declare_function(
            &format!("{}_{}", symbol(region.collection()), kind.suffix()),
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
        let geometry_kind = b.create_block();
        let lone_check = b.create_block();
        let offsets_check = b.create_block();
        let setup = b.create_block();
        let row_header = b.create_block();
        let row_load = b.create_block();
        let lone_row = b.create_block();
        let offsets_row = b.create_block();
        let first_row = b.create_block();
        let previous_row = b.create_block();
        let offsets_bounds = b.create_block();
        let cr_load = b.create_block();
        let row_ready = b.create_block();
        let row_bound = b.create_block();
        let map_start = b.create_block();
        let map_vector = b.create_block();
        let map_vector_write = b.create_block();
        let map_vector_next = b.create_block();
        let map_scalar = b.create_block();
        let map_key = b.create_block();
        let map_lookup = b.create_block();
        let map_write = b.create_block();
        let map_missing = b.create_block();
        let copy_start = b.create_block();
        let copy_vector = b.create_block();
        let copy_vector_next = b.create_block();
        let copy_scalar = b.create_block();
        let copy_byte = b.create_block();
        let row_done = b.create_block();
        let finish = b.create_block();
        let publish = b.create_block();
        let error = b.create_block();
        b.append_block_params_for_function_params(entry);
        for ty in [types::I64, types::I64, types::I8, types::I64] {
            b.append_block_param(row_header, ty);
        }
        b.append_block_param(offsets_bounds, types::I64);
        for _ in 0..2 {
            b.append_block_param(row_ready, types::I64);
        }
        for block in [map_vector, map_scalar, copy_vector, copy_scalar] {
            b.append_block_param(block, types::I64);
        }
        for _ in 0..2 {
            b.append_block_param(copy_start, types::I64);
        }
        b.append_block_param(row_done, types::I8);
        b.append_block_param(row_done, types::I64);

        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let (geometry, payload, table, output, enabled) =
            (args[0], args[1], args[2], args[3], args[4]);
        let source = b.ins().load(types::I64, MemFlags::new(), geometry, 0);
        let source_len = b.ins().load(types::I64, MemFlags::new(), geometry, 8);
        let offsets = b.ins().load(types::I64, MemFlags::new(), geometry, 16);
        let first = b.ins().load(types::I64, MemFlags::new(), geometry, 24);
        let count = b.ins().load(types::I64, MemFlags::new(), geometry, 32);
        let offset_count = b.ins().load(types::I64, MemFlags::new(), geometry, 40);
        let zero = b.ins().iconst(types::I64, 0);
        let yes = b.ins().iconst(types::I8, 1);
        let no = b.ins().iconst(types::I8, 0);
        let end = b.ins().iadd(first, count);
        let wraps = b.ins().icmp(IntCC::UnsignedLessThan, end, first);
        let bad_enabled = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, enabled, 1);
        let invalid = b.ins().bor(wraps, bad_enabled);
        b.ins().brif(invalid, error, &[], geometry_kind, &[]);
        b.switch_to_block(geometry_kind);
        let lone = b.ins().icmp_imm(IntCC::Equal, offsets, 0);
        b.ins().brif(lone, lone_check, &[], offsets_check, &[]);
        b.switch_to_block(lone_check);
        let a = b.ins().icmp_imm(IntCC::NotEqual, first, 0);
        let c = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, count, 1);
        let invalid = b.ins().bor(a, c);
        let c = b.ins().icmp_imm(IntCC::NotEqual, offset_count, 0);
        let invalid = b.ins().bor(invalid, c);
        b.ins().brif(invalid, error, &[], setup, &[]);
        b.switch_to_block(offsets_check);
        let invalid = b.ins().icmp(IntCC::UnsignedGreaterThan, end, offset_count);
        b.ins().brif(invalid, error, &[], setup, &[]);
        b.switch_to_block(setup);
        // The outer dispatch selected a complete prepared representation once.
        // Both scalar and vector paths use its identical mapping algebra.
        let scalar = ScalarTable::new(&mut b, table, kind);
        let tables = vectorized.then(|| Tables::new(&mut b, table, kind));
        let active = b.ins().ireduce(types::I8, enabled);
        let missing = b.ins().select(active, payload, zero);
        b.ins().jump(
            row_header,
            &[first.into(), zero.into(), active.into(), missing.into()],
        );
        b.switch_to_block(row_header);
        let state = b.block_params(row_header).to_vec();
        let (ordinal, written, active, missing) = (state[0], state[1], state[2], state[3]);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, end);
        b.ins().brif(more, row_load, &[], finish, &[]);
        b.switch_to_block(row_load);
        b.ins().brif(lone, lone_row, &[], offsets_row, &[]);
        b.switch_to_block(lone_row);
        // A lone EOF row has no removed LF or terminal CRLF pair.
        b.ins().jump(row_ready, &[zero.into(), source_len.into()]);
        b.switch_to_block(offsets_row);
        let at = b.ins().ishl_imm(ordinal, 2);
        let address = b.ins().iadd(offsets, at);
        let lf = b.ins().load(types::I32, MemFlags::new(), address, 0);
        let lf = b.ins().uextend(types::I64, lf);
        let initial = b.ins().icmp_imm(IntCC::Equal, ordinal, 0);
        b.ins().brif(initial, first_row, &[], previous_row, &[]);
        b.switch_to_block(first_row);
        b.ins().jump(offsets_bounds, &[zero.into()]);
        b.switch_to_block(previous_row);
        let prior = b.ins().load(types::I32, MemFlags::new(), address, -4);
        let prior = b.ins().uextend(types::I64, prior);
        let start = b.ins().iadd_imm(prior, 1);
        b.ins().jump(offsets_bounds, &[start.into()]);
        b.switch_to_block(offsets_bounds);
        let start = b.block_params(offsets_bounds)[0];
        let backwards = b.ins().icmp(IntCC::UnsignedGreaterThan, start, lf);
        let outside = b
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, lf, source_len);
        let invalid = b.ins().bor(backwards, outside);
        // Guard both the CR probe and the subsequent row's complete source span.
        let valid_offsets = b.create_block();
        b.ins().brif(invalid, error, &[], valid_offsets, &[]);
        b.switch_to_block(valid_offsets);
        let nonempty = b.ins().icmp(IntCC::UnsignedGreaterThan, lf, start);
        b.ins().brif(
            nonempty,
            cr_load,
            &[],
            row_ready,
            &[start.into(), lf.into()],
        );
        b.switch_to_block(cr_load);
        let address = b.ins().iadd(source, lf);
        let cr = b.ins().load(types::I8, MemFlags::new(), address, -1);
        let is_cr = b.ins().icmp_imm(IntCC::Equal, cr, 13);
        let trim = b.ins().uextend(types::I64, is_cr);
        let stop = b.ins().isub(lf, trim);
        b.ins().jump(row_ready, &[start.into(), stop.into()]);
        b.switch_to_block(row_ready);
        let start = b.block_params(row_ready)[0];
        let stop = b.block_params(row_ready)[1];
        let length = b.ins().isub(stop, start);
        let following = b.ins().iadd(written, length);
        let wraps = b.ins().icmp(IntCC::UnsignedLessThan, following, written);
        let exceeds = b.ins().icmp(IntCC::UnsignedGreaterThan, following, payload);
        let invalid = b.ins().bor(wraps, exceeds);
        b.ins().brif(invalid, error, &[], row_bound, &[]);
        b.switch_to_block(row_bound);
        let input = b.ins().iadd(source, start);
        let target = b.ins().iadd(output, written);
        b.ins().brif(
            active,
            map_start,
            &[],
            copy_start,
            &[zero.into(), missing.into()],
        );
        b.switch_to_block(map_start);
        let last = b.ins().iadd_imm(length, -16);
        if let Some(_) = &tables {
            let fits = b
                .ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, length, 16);
            b.ins()
                .brif(fits, map_vector, &[zero.into()], map_scalar, &[zero.into()]);
        } else {
            b.ins().jump(map_scalar, &[zero.into()]);
        }
        b.switch_to_block(map_vector);
        let vector_index = b.block_params(map_vector)[0];
        if let Some(tables) = &tables {
            let address = b.ins().iadd(input, vector_index);
            let keys = b.ins().load(types::I8X16, MemFlags::new(), address, 0);
            let (mapped, invalid) = tables.map(&mut b, keys);
            b.ins().brif(
                invalid,
                map_scalar,
                &[vector_index.into()],
                map_vector_write,
                &[],
            );
            b.switch_to_block(map_vector_write);
            let address = b.ins().iadd(target, vector_index);
            b.ins().store(MemFlags::new(), mapped, address, 0);
            let next = b.ins().iadd_imm(vector_index, 16);
            let done = b.ins().icmp(IntCC::Equal, next, length);
            b.ins().brif(
                done,
                row_done,
                &[yes.into(), missing.into()],
                map_vector_next,
                &[],
            );
            b.switch_to_block(map_vector_next);
            let full = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, next, last);
            // The bounded final vector may overlap already mapped bytes.
            // Immutable input and identical replacements make that harmless;
            // it avoids scalarizing a short tail without any row overread.
            let next = b.ins().select(full, next, last);
            b.ins().jump(map_vector, &[next.into()]);
        } else {
            // Unreachable in scalar builds; keep every constructed block sealed.
            b.ins().jump(error, &[]);
            b.switch_to_block(map_vector_write);
            b.ins().jump(error, &[]);
            b.switch_to_block(map_vector_next);
            b.ins().jump(error, &[]);
        }
        b.switch_to_block(map_scalar);
        let index = b.block_params(map_scalar)[0];
        let more = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
        b.ins()
            .brif(more, map_key, &[], row_done, &[yes.into(), missing.into()]);
        b.switch_to_block(map_key);
        let address = b.ins().iadd(input, index);
        let key = b.ins().load(types::I8, MemFlags::new(), address, 0);
        let key = b.ins().uextend(types::I64, key);
        let ascii = b.ins().icmp_imm(IntCC::UnsignedLessThan, key, 128);
        b.ins().brif(ascii, map_lookup, &[], error, &[]);
        b.switch_to_block(map_lookup);
        let value = scalar.lookup(&mut b, key);
        let present = b.ins().icmp_imm(IntCC::UnsignedLessThan, value, 128);
        b.ins().brif(present, map_write, &[], map_missing, &[]);
        b.switch_to_block(map_write);
        let address = b.ins().iadd(target, index);
        b.ins().store(MemFlags::new(), value, address, 0);
        let next = b.ins().iadd_imm(index, 1);
        b.ins().jump(map_scalar, &[next.into()]);
        b.switch_to_block(map_missing);
        let first_missing = b.ins().iadd(written, index);
        b.ins()
            .jump(copy_start, &[index.into(), first_missing.into()]);
        b.switch_to_block(copy_start);
        let copy_index = b.block_params(copy_start)[0];
        let copy_missing = b.block_params(copy_start)[1];
        let remaining = b.ins().isub(length, copy_index);
        let width = if vectorized { 16 } else { 8 };
        let copy_last = b.ins().iadd_imm(length, -width);
        let fits = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, width);
        b.ins().brif(
            fits,
            copy_vector,
            &[copy_index.into()],
            copy_scalar,
            &[copy_index.into()],
        );
        b.switch_to_block(copy_vector);
        let index = b.block_params(copy_vector)[0];
        let address = b.ins().iadd(input, index);
        let ty = if vectorized { types::I8X16 } else { types::I64 };
        let bytes = b.ins().load(ty, MemFlags::new(), address, 0);
        let address = b.ins().iadd(target, index);
        b.ins().store(MemFlags::new(), bytes, address, 0);
        let next = b.ins().iadd_imm(index, width);
        let done = b.ins().icmp(IntCC::Equal, next, length);
        b.ins().brif(
            done,
            row_done,
            &[no.into(), copy_missing.into()],
            copy_vector_next,
            &[],
        );
        b.switch_to_block(copy_vector_next);
        let full = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, next, copy_last);
        let next = b.ins().select(full, next, copy_last);
        b.ins().jump(copy_vector, &[next.into()]);
        b.switch_to_block(copy_scalar);
        let index = b.block_params(copy_scalar)[0];
        let more = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
        b.ins().brif(
            more,
            copy_byte,
            &[],
            row_done,
            &[no.into(), copy_missing.into()],
        );
        b.switch_to_block(copy_byte);
        let address = b.ins().iadd(input, index);
        let byte = b.ins().load(types::I8, MemFlags::new(), address, 0);
        let address = b.ins().iadd(target, index);
        b.ins().store(MemFlags::new(), byte, address, 0);
        let next = b.ins().iadd_imm(index, 1);
        b.ins().jump(copy_scalar, &[next.into()]);
        b.switch_to_block(row_done);
        let next_active = b.block_params(row_done)[0];
        let next_missing = b.block_params(row_done)[1];
        let next_ordinal = b.ins().iadd_imm(ordinal, 1);
        b.ins().jump(
            row_header,
            &[
                next_ordinal.into(),
                following.into(),
                next_active.into(),
                next_missing.into(),
            ],
        );
        b.switch_to_block(finish);
        let complete = b.ins().icmp(IntCC::Equal, written, payload);
        b.ins().brif(complete, publish, &[], error, &[]);
        b.switch_to_block(publish);
        let at =
            (crate::native_artifact::program::protocol::abi::JOIN_MAP_INITIALIZED_WORD * 8) as i32;
        b.ins().store(MemFlags::new(), written, geometry, at);
        b.ins().return_(&[missing]);
        b.switch_to_block(error);
        let invalid = b.ins().iconst(types::I64, -1);
        b.ins().return_(&[invalid]);
        b.seal_all_blocks();
        b.finalize();
    }
    module
        .define_function(function, &mut context)
        .map_err(backend)?;
    let compiled = context
        .compiled_code()
        .ok_or_else(|| backend("joined String copy code missing"))?;
    let stack = crate::native_artifact::compiler::compiled_stack_bytes(&compiled.buffer)?;
    Ok((function, stack, compiled.code_info().total_size as u64))
}

#[cfg(all(test, feature = "canonical-native-program-host"))]
mod tests;
