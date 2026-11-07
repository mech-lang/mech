//! Direct structured child transitions in the one numeric SSA/view bank.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::compiler::{backend, offset};
use crate::native_artifact::program::prepare::numeric_tree::Proof;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;
use std::collections::BTreeMap;

pub(super) fn control(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    shadow: usize,
    tree: &Proof,
    site: usize,
    wide_stores: bool,
) -> Result<usize, NativeCompileError> {
    let mut wide_pairs = 0;
    for (index, node) in tree.nodes.iter().enumerate() {
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        if index > 0 && site == node.tail.initialize() as usize {
            words.write(b, shadow + tree.active(index), one);
            words.write(b, shadow + tree.latched(index), zero);
        }
        if site == node.tail.latch() as usize {
            words.write(b, shadow + tree.latched(index), one);
        }
        if index > 0 && site == node.tail.exit() as usize {
            // A cold first change may have reified this activation's scalar
            // carrier. Retire exactly those homes at its source return edge.
            if !node.scalar_bindings.is_empty() {
                let bank = words.read(b, shadow + tree.bank());
                for binding in &node.scalar_bindings {
                    let at = binding
                        .physical
                        .checked_mul(2)
                        .ok_or_else(|| backend("numeric tree retirement offset overflows"))?;
                    clear_scalar_pair(b, bank, at, zero, wide_stores)?;
                    wide_pairs += usize::from(wide_stores);
                }
            }
            words.write(b, shadow + tree.active(index), zero);
            words.write(b, shadow + tree.latched(index), zero);
        }
    }
    Ok(wide_pairs)
}

pub(super) fn debit(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    shadow: usize,
    tree: &Proof,
    node: usize,
    projected: bool,
    fuel_derived: bool,
) {
    if projected && tree.projected_child_debits() && node != 0 {
        if fuel_derived {
            // Every original fuel debit remains above this call. Its exact
            // child total is reconstructed before any observable boundary.
            return;
        }
        let selected = &tree.nodes[node];
        for (slot, cost) in [
            (tree.child_compute(), selected.compute),
            (tree.child_comparison(), selected.comparison),
        ] {
            // The unchanged header checks fund this debit before execution.
            // All prices are nonnegative and their complete sum is <= the
            // initial u64 allowance, so this exact partial sum cannot wrap.
            if cost != 0 {
                let total = words.read(b, shadow + slot);
                let total = b.ins().iadd_imm(total, cost as i64);
                words.write(b, shadow + slot, total);
            }
        }
        return;
    }
    // Every debit is strictly positive and drawn from the same u64 fuel lane;
    // this count cannot overflow before the fuel check declines.
    let count = words.read(b, shadow + tree.traversals(node));
    let count = b.ins().iadd_imm(count, 1);
    words.write(b, shadow + tree.traversals(node), count);
}

/// Emission-only ablation keeps the original independent counters hot and
/// computes their same exact projection only when exporting a cold boundary.
/// Production maintains these two totals directly and emits no conversion.
pub(super) fn settled_debits(
    b: &mut FunctionBuilder<'_>,
    tree: &Proof,
    projected: bool,
    mut read: impl FnMut(&mut FunctionBuilder<'_>, usize) -> Result<Value, NativeCompileError>,
) -> Result<BTreeMap<usize, Value>, NativeCompileError> {
    let mut result = BTreeMap::new();
    if !tree.projected_child_debits() {
        return Ok(result);
    }
    if projected {
        let zero = b.ins().iconst(types::I64, 0);
        for word in tree.counters..tree.bank() {
            if !tree.live_counter_word(word, true) {
                result.insert(word, zero);
            }
        }
        return Ok(result);
    }
    let mut compute = b.ins().iconst(types::I64, 0);
    let mut comparison = compute;
    for (index, node) in tree.nodes.iter().enumerate().skip(1) {
        let count = read(b, tree.traversals(index))?;
        // The same successful funding checks bound every product and partial
        // sum by the original allowance. No arithmetic or debit is speculated.
        let cost = b.ins().imul_imm(count, node.compute as i64);
        compute = b.ins().iadd(compute, cost);
        if node.comparison != 0 {
            let cost = b.ins().imul_imm(count, node.comparison as i64);
            comparison = b.ins().iadd(comparison, cost);
        }
    }
    result.insert(tree.child_compute(), compute);
    result.insert(tree.child_comparison(), comparison);
    Ok(result)
}

// Preserve both original address validations before covering exactly the same
// 16 bytes with an unaligned vector store. The bank remains private to the
// current invocation; source retirement order and all other effects stay put.
fn clear_scalar_pair(
    b: &mut FunctionBuilder<'_>,
    bank: Value,
    at: usize,
    zero: Value,
    wide: bool,
) -> Result<(), NativeCompileError> {
    let first = offset(at)?;
    let second = offset(
        at.checked_add(1)
            .ok_or_else(|| backend("numeric tree retirement validity offset overflows"))?,
    )?;
    if wide {
        let zeroes = b.ins().splat(types::I64X2, zero);
        b.ins().store(MemFlags::new(), zeroes, bank, first);
    } else {
        b.ins().store(MemFlags::new(), zero, bank, first);
        b.ins().store(MemFlags::new(), zero, bank, second);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::AbiParam;
    use cranelift_frontend::FunctionBuilderContext;
    use cranelift_module::{Linkage, Module, default_libcall_names};

    #[test]
    fn wide_retirement_stores_cover_exact_unaligned_pairs_and_preserve_neighbors() {
        for at in [0, 2, 12] {
            for wide in [false, true] {
                let mut module = cranelift_jit::JITModule::new(
                    cranelift_jit::JITBuilder::with_flags(
                        &[("opt_level", "speed")],
                        default_libcall_names(),
                    )
                    .unwrap(),
                );
                let mut context = module.make_context();
                context.func.signature.params = vec![AbiParam::new(types::I64)];
                let id = module
                    .declare_function("clear_pair", Linkage::Local, &context.func.signature)
                    .unwrap();
                let mut fc = FunctionBuilderContext::new();
                {
                    let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
                    let entry = b.create_block();
                    b.append_block_params_for_function_params(entry);
                    b.switch_to_block(entry);
                    let bank = b.block_params(entry)[0];
                    let zero = b.ins().iconst(types::I64, 0);
                    clear_scalar_pair(&mut b, bank, at, zero, wide).unwrap();
                    b.ins().return_(&[]);
                    b.seal_all_blocks();
                    b.finalize();
                }
                module.define_function(id, &mut context).unwrap();
                module.finalize_definitions().unwrap();
                // SAFETY: live matching JIT code writes only these two valid
                // words. The second input deliberately has only 8-byte alignment.
                let entry: unsafe extern "C" fn(*mut u64) =
                    unsafe { core::mem::transmute(module.get_finalized_function(id)) };
                for shift in [0, 1] {
                    let mut values = [u64::MAX; 17];
                    let mut expected = values;
                    expected[shift + at..shift + at + 2].fill(0);
                    unsafe { entry(values.as_mut_ptr().add(shift)) };
                    assert_eq!(values, expected);
                }
            }
        }
    }

    #[test]
    fn wide_retirement_stores_keep_both_original_offset_refusals() {
        for at in [i32::MAX as usize / 8, i32::MAX as usize / 8 + 1, usize::MAX] {
            let mut errors = Vec::new();
            for wide in [false, true] {
                let mut function = cranelift_codegen::ir::Function::new();
                let mut fc = FunctionBuilderContext::new();
                let mut b = FunctionBuilder::new(&mut function, &mut fc);
                let entry = b.create_block();
                b.switch_to_block(entry);
                let bank = b.ins().iconst(types::I64, 0);
                let zero = b.ins().iconst(types::I64, 0);
                errors.push(format!(
                    "{:?}",
                    clear_scalar_pair(&mut b, bank, at, zero, wide).unwrap_err()
                ));
            }
            assert_eq!(errors[0], errors[1]);
        }
    }
}
