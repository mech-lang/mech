//! Fuse original pointwise expressions into product epilogues or sum terms.
//! All operation selection is compile time. No native operation dispatch/table.
use super::{hooks::Hooks, numeric_binary::address};
use crate::native_artifact::program::{
    prepare::{
        numeric_binary::Kind,
        numeric_product_epilogues::{Candidate, Source},
    },
    protocol::{
        GlobalHelper,
        abi::{NumericBinaryFrame, NumericProductFrame, NumericProductLeaf},
    },
};
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, f64_arithmetic, guard},
};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeMatchValue;
use std::mem::offset_of;
struct Leaf {
    pointer: Value,
    scalar: Value,
    mode: Value,
    inline: bool,
}
pub(super) struct Epilogue<'a> {
    recipe: &'a Candidate,
    leaves: Vec<Leaf>,
    cancelled: Block,
}
/// A lane mask for nonfinite values. Callers choose the validation boundary;
/// ordinary product/epilogue guards retain their original immediate check.
pub(super) fn nonfinite_mask(b: &mut FunctionBuilder<'_>, value: Value, vector: bool) -> Value {
    let kind = if vector {
        types::I64.by(2).unwrap()
    } else {
        types::I64
    };
    let bits = b.ins().bitcast(kind, MemFlags::new(), value);
    let mask = b.ins().iconst(types::I64, 0x7ff0000000000000);
    let mask = if vector {
        b.ins().splat(kind, mask)
    } else {
        mask
    };
    let exponent = b.ins().band(bits, mask);
    b.ins().icmp(IntCC::Equal, exponent, mask)
}
pub(super) fn finite(b: &mut FunctionBuilder<'_>, value: Value, vector: bool, cancelled: Block) {
    let bad = nonfinite_mask(b, value, vector);
    let bad = if vector { b.ins().vany_true(bad) } else { bad };
    let good = b.create_block();
    b.ins().brif(bad, cancelled, &[], good, &[]);
    b.switch_to_block(good);
}

impl Leaf {
    fn emit(
        &self,
        b: &mut FunctionBuilder<'_>,
        row: Value,
        column: Value,
        index: Value,
        vector: bool,
    ) -> Value {
        let kind = if vector {
            types::F64.by(2).unwrap()
        } else {
            types::F64
        };
        if self.inline {
            let v = b.ins().bitcast(types::F64, MemFlags::new(), self.scalar);
            return if vector { b.ins().splat(kind, v) } else { v };
        }
        let plain = b.create_block();
        let matrix = b.create_block();
        let done = b.create_block();
        b.append_block_param(done, kind);
        let scalar = b.ins().icmp_imm(IntCC::Equal, self.mode, 0);
        b.ins().brif(scalar, plain, &[], matrix, &[]);
        b.switch_to_block(plain);
        let v = b.ins().bitcast(types::F64, MemFlags::new(), self.scalar);
        let v = if vector { b.ins().splat(kind, v) } else { v };
        b.ins().jump(done, &[v.into()]);
        b.switch_to_block(matrix);
        let col = b.ins().icmp_imm(IntCC::Equal, self.mode, 2);
        let coord = b.ins().select(col, row, column);
        let exact = b.ins().icmp_imm(IntCC::Equal, self.mode, 1);
        let at = b.ins().select(exact, index, coord);
        let pointer = address(b, self.pointer, at);
        if vector {
            let repeated = b.create_block();
            let contiguous = b.create_block();
            b.ins().brif(col, repeated, &[], contiguous, &[]);
            b.switch_to_block(repeated);
            let v = b.ins().load(types::F64, MemFlags::new(), pointer, 0);
            let v = b.ins().splat(kind, v);
            b.ins().jump(done, &[v.into()]);
            b.switch_to_block(contiguous);
            let v = b.ins().load(kind, MemFlags::new(), pointer, 0);
            b.ins().jump(done, &[v.into()]);
        } else {
            let v = b.ins().load(kind, MemFlags::new(), pointer, 0);
            b.ins().jump(done, &[v.into()]);
        }
        b.switch_to_block(done);
        b.block_params(done)[0]
    }
}
impl Epilogue<'_> {
    pub(super) fn transposed_left(&self) -> bool {
        self.recipe.transposed_left()
    }

    fn expression(
        &self,
        b: &mut FunctionBuilder<'_>,
        mut values: Vec<Option<Value>>,
        end: usize,
        row: Value,
        column: Value,
        index: Value,
        vector: bool,
    ) -> Value {
        let start = values.len();
        let mut external = vec![None; self.leaves.len()];
        for (at, node) in self.recipe.nodes.iter().take(end).enumerate().skip(start) {
            let args = node.inputs.map(|source| match source {
                Source::Node(n) => {
                    values[n].expect("proved cell source excludes virtual transpose")
                }
                Source::External(n) => match external[n] {
                    Some(v) => v,
                    None => {
                        let v = self.leaves[n].emit(b, row, column, index, vector);
                        external[n] = Some(v);
                        v
                    }
                },
            });
            let Kind::Elementwise(op) = node.kind else {
                unreachable!("source-closed cell expression")
            };
            let value = f64_arithmetic(b, op, args[0], args[1]);
            // Every consumed intermediate must be finite; otherwise discard
            // private output and replay the unchanged original source, whose
            // input validation and NaN payload behavior remain authoritative.
            if at + 1 < self.recipe.nodes.len() {
                finite(b, value, vector, self.cancelled);
            }
            values.push(Some(value));
        }
        values.last().copied().flatten().expect("final scalar cell")
    }
    pub(super) fn apply(
        &self,
        b: &mut FunctionBuilder<'_>,
        product: Value,
        row: Value,
        column: Value,
        index: Value,
        vector: bool,
    ) -> Value {
        finite(b, product, vector, self.cancelled);
        // The virtual view has no per-cell scalar. Keep its node slot empty;
        // the closed source proof permits it only as the product's operand.
        let mut values = vec![None; self.recipe.product_index().expect("selected product")];
        values.push(Some(product));
        self.expression(
            b,
            values,
            self.recipe.nodes.len(),
            row,
            column,
            index,
            vector,
        )
    }
    pub(super) fn term(
        &self,
        b: &mut FunctionBuilder<'_>,
        row: Value,
        column: Value,
        index: Value,
        vector: bool,
    ) -> Value {
        self.expression(
            b,
            Vec::new(),
            self.recipe.nodes.len() - 1,
            row,
            column,
            index,
            vector,
        )
    }
}
fn get(b: &mut FunctionBuilder<'_>, frame: Value, offset: usize) -> Value {
    b.ins()
        .load(types::I64, MemFlags::new(), frame, offset as i32)
}
pub(in crate::native_artifact) fn emit_entry(
    b: &mut FunctionBuilder<'_>,
    hooks: &Hooks,
    site: usize,
    recipe: &Candidate,
    locals: Value,
    is_scalar: impl Fn(NativeMatchValue) -> bool,
    row_pairs: bool,
    row_quartets: bool,
    target: Block,
    error: Block,
) -> Result<(), NativeCompileError> {
    if recipe.entry as usize != site
        || (recipe.reduction_axis().is_none() && recipe.product_index().is_none())
        || recipe.nodes.len() < 2
    {
        return Err(backend("product epilogue lost source entry"));
    }
    hooks.library_operation(
        b,
        site,
        GlobalHelper::NumericProductPrepare.raw(),
        &recipe.scalars.iter().map(|(_, id)| *id).collect::<Vec<_>>(),
        locals,
        &is_scalar,
        error,
    )?;
    let frame = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let native = b.create_block();
    let ordinary = b.create_block();
    let cancelled = b.create_block();
    let complete = b.create_block();
    b.ins().brif(frame, native, &[], ordinary, &[]);
    b.switch_to_block(native);
    let left = get(b, frame, offset_of!(NumericBinaryFrame, left));
    let right = get(b, frame, offset_of!(NumericBinaryFrame, right));
    let output = get(b, frame, offset_of!(NumericBinaryFrame, output));
    let count = get(b, frame, offset_of!(NumericBinaryFrame, count));
    let columns = get(b, frame, offset_of!(NumericBinaryFrame, columns));
    let inner = get(b, frame, offset_of!(NumericBinaryFrame, inner));
    let leaves = recipe
        .external
        .iter()
        .enumerate()
        .map(|(at, &id)| {
            let base = offset_of!(NumericProductFrame, leaves)
                + at * std::mem::size_of::<NumericProductLeaf>();
            Leaf {
                pointer: get(b, frame, base + offset_of!(NumericProductLeaf, pointer)),
                scalar: get(b, frame, base + offset_of!(NumericProductLeaf, scalar)),
                mode: get(b, frame, base + offset_of!(NumericProductLeaf, mode)),
                inline: is_scalar(id),
            }
        })
        .collect();
    let epilogue = Epilogue {
        recipe,
        leaves,
        cancelled,
    };
    if let Some(axis) = recipe.reduction_axis() {
        super::numeric_reduction_producers::emit(
            b, output, count, inner, axis, &epilogue, complete,
        );
    } else {
        super::numeric_product::emit_epilogue(
            b,
            left,
            right,
            output,
            count,
            columns,
            inner,
            &epilogue,
            row_pairs,
            row_quartets,
            complete,
        );
    }
    for (block, cancel, ready, target) in [(complete, 0, 1, target), (cancelled, 1, 0, ordinary)] {
        b.switch_to_block(block);
        let status = b.ins().iconst(types::I64, cancel);
        hooks.call(
            b,
            site,
            GlobalHelper::NumericProductFinish,
            [frame, count, status],
            error,
        );
        let actual = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
        let valid = b.ins().icmp_imm(IntCC::Equal, actual, ready);
        guard(
            b,
            valid,
            error,
            crate::native_artifact::abi::STATUS_INTEGRITY,
        );
        b.ins().jump(target, &[]);
    }
    b.switch_to_block(ordinary);
    Ok(())
}
