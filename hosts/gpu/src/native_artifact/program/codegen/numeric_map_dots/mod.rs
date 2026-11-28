//! Source-closed ordered dots over current immutable map loans. All guards
//! precede stream loads; a decline resumes the original checked source DAG.
use super::numeric_dot_core::{self, Stream};
use crate::native_artifact::program::{
    prepare::numeric_maps::{
        Region,
        dots::{Batch, Dot, Expr, Index, Operand, Predicate},
    },
    runtime::NativeNumericMapFrame as Frame,
};
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, primitive},
};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeProgramArtifactPlan as Plan;
use std::{collections::BTreeMap, mem::offset_of};
mod batch;
pub(super) mod cells;
mod streams;
mod windows;

fn require(b: &mut FunctionBuilder<'_>, good: Value, slow: Block) {
    let next = b.create_block();
    b.ins().brif(good, next, &[], slow, &[]);
    b.switch_to_block(next);
}
struct Context<'a> {
    plan: &'a Plan,
    captures: &'a BTreeMap<u32, Value>,
    slow: Block,
    arithmetic: Block,
}
impl Context<'_> {
    fn word(&self, b: &mut FunctionBuilder<'_>, id: u32) -> Result<Value, NativeCompileError> {
        if let Some(&value) = self.captures.get(&id) {
            return Ok(value);
        }
        if let Some(value) = self.plan.constants().iter().find(|c| c.value().get() == id) {
            if let mech_core::ValueData::U64(n) = value.constant().data() {
                return Ok(b.ins().iconst(types::I64, *n as i64));
            }
        }
        Err(backend("ordered source dot lost invariant word"))
    }
}
fn finish(b: &mut FunctionBuilder<'_>, frame: Value, count: Value, complete: Block) {
    b.ins().store(
        MemFlags::new(),
        count,
        frame,
        offset_of!(Frame, next) as i32,
    );
    b.ins().jump(complete, &[]);
}
/// Emits a fast branch only when the exact source closes over an admitted dot
/// or independent-output parent. The current block on return is the old DAG.
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    region: &Region,
    plan: &Plan,
    captures: &BTreeMap<u32, Value>,
    matrices: &[(Value, Value, Value, Value)],
    start: Value,
    count: Value,
    output: Value,
    complete: Block,
) -> Result<(), NativeCompileError> {
    let dot = Dot::derive(plan, region, None);
    let batch = if dot.is_none() {
        Batch::derive(plan, region)
    } else {
        None
    };
    if dot.is_none() && batch.is_none() {
        return Ok(());
    }
    let slow = b.create_block();
    let arithmetic = b.create_block();
    b.append_block_param(arithmetic, types::I64);
    let context = Context {
        plan,
        captures,
        slow,
        arithmetic,
    };
    let empty = b.ins().icmp_imm(IntCC::Equal, count, 0);
    let work = b.create_block();
    let done = b.create_block();
    b.ins().brif(empty, done, &[], work, &[]);
    b.switch_to_block(work);
    if let Some(dot) = dot {
        let mut streams = Vec::new();
        for operand in &dot.operands {
            let (pointer, cells, rows, columns) = matrices[operand.matrix];
            streams.push(
                streams::prepare(
                    b,
                    &context,
                    operand,
                    [pointer, cells, rows, columns],
                    start,
                    count,
                    None,
                )?
                .stream(b, None, false),
            );
        }
        let result = numeric_dot_core::emit(b, &streams[..1], streams[1], count, false)[0];
        super::numeric_product_epilogues::finite(b, result, false, slow);
        let word = b.ins().bitcast(types::I64, MemFlags::new(), result);
        b.ins()
            .store(MemFlags::new(), word, frame, offset_of!(Frame, fold) as i32);
    } else if let Some(batch) = batch {
        batch::emit(b, &context, &batch, region, matrices, start, count, output)?;
    }
    b.ins().jump(done, &[]);
    b.switch_to_block(done);
    finish(b, frame, count, complete);
    b.switch_to_block(arithmetic);
    b.ins().jump(slow, &[]);
    b.switch_to_block(slow);
    Ok(())
}
