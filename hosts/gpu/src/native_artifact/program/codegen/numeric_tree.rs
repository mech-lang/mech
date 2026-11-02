//! Direct structured child transitions in the one numeric SSA/view bank.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::compiler::{backend, offset};
use crate::native_artifact::program::prepare::numeric_tree::Proof;
use cranelift_codegen::ir::{InstBuilder, MemFlags, types};
use cranelift_frontend::FunctionBuilder;

pub(super) fn control(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    shadow: usize,
    tree: &Proof,
    site: usize,
) -> Result<(), NativeCompileError> {
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
                    b.ins().store(MemFlags::new(), zero, bank, offset(at)?);
                    b.ins().store(
                        MemFlags::new(),
                        zero,
                        bank,
                        offset(at.checked_add(1).ok_or_else(|| {
                            backend("numeric tree retirement validity offset overflows")
                        })?)?,
                    );
                }
            }
            words.write(b, shadow + tree.active(index), zero);
            words.write(b, shadow + tree.latched(index), zero);
        }
    }
    Ok(())
}

pub(super) fn debit(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    shadow: usize,
    tree: &Proof,
    node: usize,
) {
    // Every debit is strictly positive and drawn from the same u64 fuel lane;
    // this count cannot overflow before the fuel check declines.
    let count = words.read(b, shadow + tree.traversals(node));
    let count = b.ins().iadd_imm(count, 1);
    words.write(b, shadow + tree.traversals(node), count);
}
