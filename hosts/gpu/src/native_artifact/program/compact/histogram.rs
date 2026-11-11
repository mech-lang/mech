//! The shared native UTF8 hash kernel, with compact owned slot storage and
//! native canonicalization. No managed call admission or Map finishing hook.
use super::{
    arrays::{self, Arrays},
    layout::{Layout, Type},
    packed,
    recipes::Recipe,
    strings,
};
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::{
    codegen::utf8_histogram::{self, Funding},
    runtime::maps::utf8_histogram::{Frame, Slot},
};
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, guard},
};
use cranelift_codegen::ir::{
    Block, Endianness, FuncRef, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value,
    condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{FuncId, Module};
use mech_engine::native_artifact::{NativeCollectionPrimitive as P, NativeMatchValue as V};
use std::mem::{offset_of, size_of};

pub(super) fn define<M: Module>(
    module: &mut M,
    actions: &[Recipe],
) -> Result<Option<FuncId>, NativeCompileError> {
    if !actions.iter().any(|a| {
        matches!(
            a,
            Recipe::Buffer {
                operation: P::StringHistogram,
                ..
            }
        )
    }) {
        return Ok(None);
    }
    // Exactly the managed kernel's native hash and SIMD equality instructions.
    // The compact contract supplies no work account, so it grants no prepaid
    // managed authority and cannot enter any managed admission/finish path.
    let (id, _, _) = utf8_histogram::define(
        module,
        "mech_compact_utf8_histogram",
        true,
        Funding::Unmetered,
        true,
        false,
    )?;
    Ok(Some(id))
}

fn address(b: &mut FunctionBuilder<'_>, base: Value, index: Value, stride: usize) -> Value {
    let offset = b.ins().imul_imm(index, stride as i64);
    b.ins().iadd(base, offset)
}
fn swap(
    b: &mut FunctionBuilder<'_>,
    base: Value,
    a: Value,
    c: Value,
    entry: &Type,
) -> Result<(), NativeCompileError> {
    let a = address(b, base, a, entry.packed_bytes.unwrap());
    let c = address(b, base, c, entry.packed_bytes.unwrap());
    let left = packed::load(b, a, entry)?;
    let right = packed::load(b, c, entry)?;
    packed::store(b, a, entry, &right)?;
    packed::store(b, c, entry, &left)
}
fn order(
    b: &mut FunctionBuilder<'_>,
    base: Value,
    a: Value,
    c: Value,
    stride: usize,
    endian: Endianness,
) -> Value {
    let a = address(b, base, a, stride);
    let c = address(b, base, c, stride);
    let ap = b.ins().load(types::I64, MemFlags::new(), a, 0);
    let an = b.ins().load(types::I64, MemFlags::new(), a, 8);
    let cp = b.ins().load(types::I64, MemFlags::new(), c, 0);
    let cn = b.ins().load(types::I64, MemFlags::new(), c, 8);
    strings::order(b, ap, an, cp, cn, endian)
}
fn sift(
    b: &mut FunctionBuilder<'_>,
    base: Value,
    root: Value,
    length: Value,
    entry: &Type,
    endian: Endianness,
) -> Result<(), NativeCompileError> {
    let head = b.create_block();
    b.append_block_param(head, types::I64);
    let child_block = b.create_block();
    let compare_right = b.create_block();
    let chosen = b.create_block();
    b.append_block_param(chosen, types::I64);
    let move_down = b.create_block();
    let done = b.create_block();
    b.ins().jump(head, &[root.into()]);
    b.switch_to_block(head);
    let root = b.block_params(head)[0];
    // Every entry extent comes from a checked native allocation. Thus
    // 2*root+1 and all byte offsets fit before the following length test.
    let child = b.ins().ishl_imm(root, 1);
    let child = b.ins().iadd_imm(child, 1);
    let exists = b.ins().icmp(IntCC::UnsignedLessThan, child, length);
    b.ins().brif(exists, child_block, &[], done, &[]);
    b.switch_to_block(child_block);
    let right = b.ins().iadd_imm(child, 1);
    let exists = b.ins().icmp(IntCC::UnsignedLessThan, right, length);
    b.ins()
        .brif(exists, compare_right, &[], chosen, &[child.into()]);
    b.switch_to_block(compare_right);
    let cmp = order(b, base, child, right, entry.packed_bytes.unwrap(), endian);
    let less = b.ins().icmp_imm(IntCC::SignedLessThan, cmp, 0);
    let largest = b.ins().select(less, right, child);
    b.ins().jump(chosen, &[largest.into()]);
    b.switch_to_block(chosen);
    let largest = b.block_params(chosen)[0];
    let cmp = order(b, base, root, largest, entry.packed_bytes.unwrap(), endian);
    let less = b.ins().icmp_imm(IntCC::SignedLessThan, cmp, 0);
    b.ins().brif(less, move_down, &[], done, &[]);
    b.switch_to_block(move_down);
    swap(b, base, root, largest, entry)?;
    b.ins().jump(head, &[largest.into()]);
    b.switch_to_block(done);
    Ok(())
}
/// In-place O(distinct * log(distinct)) canonical ordering, independent of
/// source size and input distribution. Counts travel with their exact keys.
fn canonicalize(
    b: &mut FunctionBuilder<'_>,
    base: Value,
    length: Value,
    entry: &Type,
    endian: Endianness,
) -> Result<(), NativeCompileError> {
    let zero = b.ins().iconst(types::I64, 0);
    let head = b.create_block();
    b.append_block_param(head, types::I64);
    let build = b.create_block();
    let built = b.create_block();
    let half = b.ins().ushr_imm(length, 1);
    b.ins().jump(head, &[half.into()]);
    b.switch_to_block(head);
    let at = b.block_params(head)[0];
    b.ins().brif(at, build, &[], built, &[]);
    b.switch_to_block(build);
    let at = b.ins().iadd_imm(at, -1);
    sift(b, base, at, length, entry, endian)?;
    b.ins().jump(head, &[at.into()]);
    b.switch_to_block(built);
    let head = b.create_block();
    b.append_block_param(head, types::I64);
    let extract = b.create_block();
    let done = b.create_block();
    b.ins().jump(head, &[length.into()]);
    b.switch_to_block(head);
    let end = b.block_params(head)[0];
    let more = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, end, 1);
    b.ins().brif(more, extract, &[], done, &[]);
    b.switch_to_block(extract);
    let end = b.ins().iadd_imm(end, -1);
    swap(b, base, zero, end, entry)?;
    sift(b, base, zero, end, entry, endian)?;
    b.ins().jump(head, &[end.into()]);
    b.switch_to_block(done);
    Ok(())
}

pub(super) fn emit(
    arrays: &Arrays,
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    layout: &Layout,
    input: V,
    output: V,
    error: Block,
    endian: Endianness,
    function: FuncRef,
    classes: Option<&super::utf8_classes::State>,
) -> Result<(), NativeCompileError> {
    let entry = layout.ty(output.get()).map_entry()?;
    if entry.packed_bytes != Some(size_of::<Slot>()) {
        return Err(backend("native UTF8 histogram entry/slot stride differs"));
    }
    let source = arrays::read(words, b, layout, input);
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let third = b.ins().udiv_imm(source[1], 3);
    let extra = super::maps::add(b, third, one, error);
    let required = super::maps::add(b, source[1], extra, error);
    let head = b.create_block();
    b.append_block_param(head, types::I64);
    let grow = b.create_block();
    let ready = b.create_block();
    b.append_block_param(ready, types::I64);
    let four = b.ins().iconst(types::I64, 4);
    b.ins().jump(head, &[four.into()]);
    b.switch_to_block(head);
    let capacity = b.block_params(head)[0];
    let enough = b
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, capacity, required);
    b.ins().brif(enough, ready, &[capacity.into()], grow, &[]);
    b.switch_to_block(grow);
    let capacity = super::maps::add(b, capacity, capacity, error);
    b.ins().jump(head, &[capacity.into()]);
    b.switch_to_block(ready);
    let capacity = b.block_params(ready)[0];
    let bank = arrays.allocate(b, capacity, entry, true, error)?;
    arrays.retain(b, bank[5]);
    let representatives = if classes.is_some() {
        let word = Type::derive(&mech_core::SchemaBody::UnsignedInteger(
            mech_core::IntegerWidth::W64,
        ))?;
        Some(arrays.allocate(b, source[1], &word, false, error)?)
    } else {
        None
    };
    let frame_slot = b.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        size_of::<Frame>() as u32,
        3,
    ));
    let frame = b.ins().stack_addr(types::I64, frame_slot, 0);
    let quarter = b.ins().ushr_imm(capacity, 2);
    let limit = b.ins().isub(capacity, quarter);
    for (offset, value) in [
        (offset_of!(Frame, cells), source[0]),
        (offset_of!(Frame, count), source[1]),
        (offset_of!(Frame, slots), bank[0]),
        (offset_of!(Frame, capacity), capacity),
        (offset_of!(Frame, limit), limit),
        (offset_of!(Frame, fuel), zero),
        (offset_of!(Frame, comparison), zero),
        (offset_of!(Frame, completed), zero),
        (offset_of!(Frame, distinct), zero),
        (offset_of!(Frame, payload), zero),
        (
            offset_of!(Frame, representatives),
            representatives.as_ref().map_or(zero, |view| view[0]),
        ),
    ] {
        b.ins().store(MemFlags::new(), value, frame, offset as i32);
    }
    let call = b.ins().call(function, &[frame]);
    let status = b.inst_results(call)[0];
    let valid = b.ins().icmp_imm(IntCC::Equal, status, 0);
    guard(b, valid, error, 0x19503);
    if let (Some(state), Some(view)) = (classes, &representatives) {
        state.publish(b, arrays, view, error);
    }
    let count = b.ins().load(
        types::I64,
        MemFlags::new(),
        frame,
        offset_of!(Frame, distinct) as i32,
    );
    let mut view = super::map_index::allocate(arrays, b, count, entry, error)?;
    let scan = b.create_block();
    b.append_block_param(scan, types::I64);
    b.append_block_param(scan, types::I64);
    let inspect = b.create_block();
    let copy = b.create_block();
    let next = b.create_block();
    b.append_block_param(next, types::I64);
    let done = b.create_block();
    b.append_block_param(done, types::I64);
    b.ins().jump(scan, &[zero.into(), zero.into()]);
    b.switch_to_block(scan);
    let at = b.block_params(scan)[0];
    let ordinal = b.block_params(scan)[1];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, capacity);
    b.ins().brif(more, inspect, &[], done, &[ordinal.into()]);
    b.switch_to_block(inspect);
    let slot = address(b, bank[0], at, size_of::<Slot>());
    let row = b.ins().load(
        types::I64,
        MemFlags::new(),
        slot,
        offset_of!(Slot, row_plus_one) as i32,
    );
    b.ins().brif(row, copy, &[], next, &[ordinal.into()]);
    b.switch_to_block(copy);
    let bounded = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, count);
    guard(b, bounded, error, 0x19503);
    let row = b.ins().iadd_imm(row, -1);
    let cell = address(b, source[0], row, 16);
    let ptr = b.ins().load(types::I64, MemFlags::new(), cell, 0);
    let len = b.ins().load(types::I64, MemFlags::new(), cell, 8);
    let total = b.ins().load(
        types::I64,
        MemFlags::new(),
        slot,
        offset_of!(Slot, count) as i32,
    );
    let target = address(b, view[0], ordinal, entry.packed_bytes.unwrap());
    packed::store(b, target, entry, &[ptr, len, total])?;
    let ordinal = b.ins().iadd_imm(ordinal, 1);
    b.ins().jump(next, &[ordinal.into()]);
    b.switch_to_block(next);
    let ordinal = b.block_params(next)[0];
    let at = b.ins().iadd_imm(at, 1);
    b.ins().jump(scan, &[at.into(), ordinal.into()]);
    b.switch_to_block(done);
    let copied = b.block_params(done)[0];
    let complete = b.ins().icmp(IntCC::Equal, copied, count);
    guard(b, complete, error, 0x19503);
    arrays.release(b, bank[5], error);
    canonicalize(b, view[0], count, entry, endian)?;
    view[1] = count;
    super::map_index::rebuild(b, &view, entry, error);
    arrays::write(words, b, layout.slot(output.get()), &view);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::AbiParam;
    use cranelift_frontend::FunctionBuilderContext;
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, default_libcall_names};
    #[test]
    fn native_canonicalization_preserves_utf8_counts_and_guarded_empty_spans() {
        let entry = Type::derive(&mech_core::SchemaBody::Tuple(
            vec![
                mech_core::SchemaBody::String,
                mech_core::SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64),
            ]
            .into_boxed_slice(),
        ))
        .unwrap();
        let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
        let endian = module.isa().endianness();
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 2];
        let mut fc = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
            let block = b.create_block();
            b.append_block_params_for_function_params(block);
            b.switch_to_block(block);
            let a = b.block_params(block).to_vec();
            canonicalize(&mut b, a[0], a[1], &entry, endian).unwrap();
            b.ins().return_(&[]);
            b.seal_all_blocks();
            b.finalize();
        }
        assert!(context.func.layout.blocks().all(|block| {
            context
                .func
                .layout
                .block_insts(block)
                .all(|inst| !context.func.dfg.insts[inst].opcode().is_call())
        }));
        let id = module
            .declare_function(
                "compact_native_map_canonicalize",
                Linkage::Export,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: exact two-word C ABI and live executable module.
        let sort: unsafe extern "C" fn(*mut [u64; 3], usize) =
            unsafe { std::mem::transmute(module.get_finalized_function(id)) };
        let mut keys = vec![
            "".to_owned(),
            "\0".into(),
            "é".into(),
            "e\u{301}".into(),
            "🦀".into(),
        ];
        keys.extend((0..257).map(|i| format!("{}{}東京", "a".repeat(i % 37), i)));
        for count in [0, 1, 2, 3, 4, 7, 16, 31, 64, 127, keys.len()] {
            for mode in 0..4 {
                let mut expected = (0..count)
                    .map(|i| {
                        [
                            if keys[i].is_empty() {
                                0
                            } else {
                                keys[i].as_ptr() as u64
                            },
                            keys[i].len() as u64,
                            (i as u64 + 1) * 17,
                        ]
                    })
                    .collect::<Vec<_>>();
                expected.sort_by_key(|entry| keys[(entry[2] / 17 - 1) as usize].as_str());
                let mut actual = expected.clone();
                match mode {
                    0 => actual.reverse(),
                    1 if count > 0 => actual.rotate_left(count / 3),
                    2 => {
                        for i in 0..count {
                            actual.swap(i, (i * 37 + 7) % count);
                        }
                    }
                    _ => {}
                }
                let guard = [u64::MAX; 3];
                actual.insert(0, guard);
                actual.push(guard);
                // SAFETY: native sort can access only the selected initialized
                // entries. All nonempty UTF8 owners survive the invocation.
                unsafe { sort(actual.as_mut_ptr().add(1), count) };
                assert_eq!(actual[0], guard);
                assert_eq!(actual[count + 1], guard);
                assert_eq!(&actual[1..count + 1], expected);
            }
        }
        // An empty result must not dereference its payload pointer.
        unsafe { sort(std::ptr::null_mut(), 0) };
    }
}
