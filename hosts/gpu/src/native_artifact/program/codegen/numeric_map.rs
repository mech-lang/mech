//! A checked scalar DAG inside a native immutable-source collection loop.
use super::hooks::Hooks;
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::compiler::{backend, offset, primitive};
use crate::native_artifact::program::prepare::numeric_maps::Region;
use crate::native_artifact::program::protocol::GlobalHelper;
use crate::native_artifact::program::runtime::NativeNumericMapFrame as Frame;
use crate::native_artifact::program::runtime::NativeNumericMapMatrixView as MatrixView;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, Variable};
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeProgramArtifactPlan as Plan,
};
use std::{collections::BTreeMap, mem::offset_of};

pub(in crate::native_artifact) fn emit_entry(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    site: usize,
    hooks: &Hooks,
    region: &Region,
    plan: &Plan,
    counter_slot: usize,
    exit: Block,
    error: Block,
) -> Result<(), NativeCompileError> {
    let zero = b.ins().iconst(types::I64, 0);
    hooks.call(b, site, GlobalHelper::NumericMapPrepare, [zero; 3], error);
    let frame = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let selected = b.create_block();
    let ordinary = b.create_block();
    b.ins().brif(frame, selected, &[], ordinary, &[]);
    b.switch_to_block(selected);
    // External scalar captures are immutable and already current at this
    // header. Snapshot each native word once, never inside the element loop.
    let mut captures = BTreeMap::new();
    for &site in &region.body {
        for value in
            crate::native_artifact::program::prepare::numeric_maps::scalar_reads(plan, site)
        {
            if region.captures.contains(&value.get()) && !captures.contains_key(&value.get()) {
                captures.insert(
                    value.get(),
                    crate::native_artifact::control_compiler::load(b, locals, value)?,
                );
            }
        }
    }
    if captures.len() != region.captures.len() {
        return Err(backend("numeric map capture inventory differs"));
    }
    let complete = b.create_block();
    let miss = b.create_block();
    let malformed = b.create_block();
    let settle = b.create_block();
    b.append_block_param(miss, types::I64);
    b.append_block_param(settle, types::I64);
    emit(b, frame, region, plan, &captures, complete, miss, malformed)?;
    b.switch_to_block(complete);
    b.ins().jump(settle, &[zero.into()]);
    b.switch_to_block(miss);
    let declined = b.ins().iconst(types::I64, 1);
    b.ins().jump(settle, &[declined.into()]);
    b.switch_to_block(malformed);
    let invalid = b.ins().iconst(types::I64, 2);
    b.ins().jump(settle, &[invalid.into()]);
    b.switch_to_block(settle);
    let status = b.block_params(settle)[0];
    hooks.call(
        b,
        site,
        GlobalHelper::NumericMapFinish,
        [frame, status, zero],
        error,
    );
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let done = b.create_block();
    b.ins().brif(ready, done, &[], ordinary, &[]);
    b.switch_to_block(done);
    b.ins()
        .store(MemFlags::new(), zero, locals, offset(counter_slot)?);
    b.ins().jump(exit, &[]);
    b.switch_to_block(ordinary);
    Ok(())
}
fn get(b: &mut FunctionBuilder<'_>, frame: Value, at: usize) -> Value {
    b.ins().load(types::I64, MemFlags::new(), frame, at as i32)
}
fn require(b: &mut FunctionBuilder<'_>, good: Value, bad: Block) {
    let next = b.create_block();
    b.ins().brif(good, next, &[], bad, &[]);
    b.switch_to_block(next);
}
fn integer(width: u8) -> cranelift_codegen::ir::Type {
    match width {
        1 => types::I8,
        2 => types::I16,
        4 => types::I32,
        8 => types::I64,
        _ => unreachable!(),
    }
}
/// All dynamic loop work is prepaid by the loan. Arithmetic uses the common
/// checked native emitter; no source visit, helper or per-operation dispatch
/// occurs between the exact prepare and finish boundaries.
fn emit(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    region: &Region,
    plan: &Plan,
    captures: &BTreeMap<u32, Value>,
    complete: Block,
    miss: Block,
    malformed: Block,
) -> Result<(), NativeCompileError> {
    // Arithmetic failures carry the common emitter's diagnostic word. A
    // bounds miss needs no synthetic source error: ordinary replay owns it.
    let index_miss = b.create_block();
    let count = get(b, frame, offset_of!(Frame, count));
    let source_count = get(b, frame, offset_of!(Frame, source_count));
    let inner_count = b.ins().iconst(
        types::I64,
        region.rectangle.as_ref().map_or(1, |r| r.rows) as i64,
    );
    let (expected, overflow) = b.ins().umul_overflow(source_count, inner_count);
    let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    require(b, good, malformed);
    let good = b.ins().icmp(IntCC::Equal, expected, count);
    require(b, good, malformed);
    let input = get(b, frame, offset_of!(Frame, input));
    let output = get(b, frame, offset_of!(Frame, output));
    for (field, expected) in [
        (offset_of!(Frame, input_width), region.input_width),
        (offset_of!(Frame, output_width), region.output_width),
    ] {
        let width = get(b, frame, field);
        let good = b.ins().icmp_imm(IntCC::Equal, width, i64::from(expected));
        require(b, good, malformed);
    }
    let epoch = get(b, frame, offset_of!(Frame, epoch));
    let issued = get(b, frame, offset_of!(Frame, issued));
    let next = get(b, frame, offset_of!(Frame, next));
    let good = b.ins().icmp(IntCC::Equal, epoch, issued);
    require(b, good, malformed);
    let good = b.ins().icmp_imm(IntCC::NotEqual, epoch, 0);
    require(b, good, malformed);
    let good = b.ins().icmp_imm(IntCC::Equal, next, 0);
    require(b, good, malformed);
    let output_count = get(b, frame, offset_of!(Frame, output_count));
    let expected_output_count = if region.reduction.is_some() {
        b.ins().iconst(types::I64, 0)
    } else {
        count
    };
    let good = b
        .ins()
        .icmp(IntCC::Equal, output_count, expected_output_count);
    require(b, good, malformed);
    let initial_fold = get(b, frame, offset_of!(Frame, fold));
    let good = b.ins().icmp_imm(IntCC::Equal, initial_fold, 0);
    require(b, good, malformed);
    let mut banks = vec![(
        region.output_width,
        output_count,
        output,
        offset_of!(Frame, capacity_bytes),
    )];
    if !region.range {
        banks.push((
            region.input_width,
            source_count,
            input,
            offset_of!(Frame, input_bytes),
        ));
    } else {
        let bytes = get(b, frame, offset_of!(Frame, input_bytes));
        let good = b.ins().icmp_imm(IntCC::Equal, bytes, 0);
        require(b, good, malformed);
        let good = b.ins().icmp_imm(IntCC::Equal, input, 0);
        require(b, good, malformed);
    }
    for (width, count, pointer, field) in banks {
        let good = b.ins().icmp_imm(
            IntCC::UnsignedLessThanOrEqual,
            count,
            (isize::MAX as u64 / u64::from(width)) as i64,
        );
        require(b, good, malformed);
        let bytes = b.ins().imul_imm(count, i64::from(width));
        let bound = get(b, frame, field);
        let good = b.ins().icmp(IntCC::Equal, bytes, bound);
        require(b, good, malformed);
        let empty = b.ins().icmp_imm(IntCC::Equal, count, 0);
        let nonnull = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
        let good = b.ins().bor(empty, nonnull);
        require(b, good, malformed);
        let (_, overflow) = b.ins().uadd_overflow(pointer, bytes);
        let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        require(b, good, malformed);
    }
    // Hoist immutable matrix geometry and complete pointer bounds once. The
    // source is genuinely pinned through the exact same synchronous loan.
    let mut matrices = Vec::new();
    for (at, matrix) in region.matrices.iter().enumerate() {
        let base = offset_of!(Frame, matrices) + at * std::mem::size_of::<MatrixView>();
        let pointer = get(b, frame, base + offset_of!(MatrixView, input));
        let cells = get(b, frame, base + offset_of!(MatrixView, count));
        let rows = get(b, frame, base + offset_of!(MatrixView, rows));
        let columns = get(b, frame, base + offset_of!(MatrixView, columns));
        let (product, overflow) = b.ins().umul_overflow(rows, columns);
        let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        require(b, good, malformed);
        let good = b.ins().icmp(IntCC::Equal, product, cells);
        require(b, good, malformed);
        let good = b.ins().icmp_imm(
            IntCC::UnsignedLessThanOrEqual,
            cells,
            (isize::MAX as u64 / u64::from(matrix.width)) as i64,
        );
        require(b, good, malformed);
        let bytes = b.ins().imul_imm(cells, i64::from(matrix.width));
        let empty = b.ins().icmp_imm(IntCC::Equal, cells, 0);
        let nonnull = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
        let good = b.ins().bor(empty, nonnull);
        require(b, good, malformed);
        let (_, overflow) = b.ins().uadd_overflow(pointer, bytes);
        let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        require(b, good, malformed);
        matrices.push((pointer, cells, rows, columns));
    }
    let start = get(b, frame, offset_of!(Frame, range_start));
    if region.range {
        // count==0 has no generated element; otherwise the last value must fit.
        let last = b.ins().iadd_imm(source_count, -1);
        let (_, overflow) = b.ins().uadd_overflow(start, last);
        let empty = b.ins().icmp_imm(IntCC::Equal, source_count, 0);
        let no_overflow = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        let good = b.ins().bor(empty, no_overflow);
        require(b, good, malformed);
    }
    // Source phi copies share a native SSA variable. Complete current-visits
    // and body inventories establish definite assignment before any use;
    // private body words need neither canonical slots nor runtime dispatch.
    let mut words = BTreeMap::<u32, Variable>::new();
    for (&id, &value) in captures {
        let var = b.declare_var(types::I64);
        b.def_var(var, value);
        words.insert(id, var);
    }
    words.insert(region.element, b.declare_var(types::I64));
    if let Some(rectangle) = &region.rectangle {
        words.insert(rectangle.inner_element, b.declare_var(types::I64));
    }
    for &site in &region.steps {
        let output = match plan.blocks()[site as usize].action() {
            Action::Scalar(Scalar::Copy { output, .. } | Scalar::Primitive { output, .. })
            | Action::LibraryCall { output, .. } => output.get(),
            _ => return Err(backend("numeric map source step changed")),
        };
        words
            .entry(output)
            .or_insert_with(|| b.declare_var(types::I64));
    }
    let source_blocks: BTreeMap<_, _> = region
        .body
        .iter()
        .map(|&at| (at, b.create_block()))
        .collect();
    let latch = b.create_block();
    let cursor = b.declare_var(types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.def_var(cursor, zero);
    let fold = if region.reduction.is_some() {
        let var = b.declare_var(types::I64);
        b.def_var(var, zero);
        Some(var)
    } else {
        None
    };
    let head = b.create_block();
    let body = b.create_block();
    let done = b.create_block();
    b.ins().jump(head, &[]);
    b.switch_to_block(head);
    let at = b.use_var(cursor);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    let at = b.use_var(cursor);
    let source_at = if let Some(rectangle) = &region.rectangle {
        let rows = b.ins().iconst(types::I64, rectangle.rows as i64);
        let offset = b.ins().urem(at, rows);
        let inner = b.ins().iadd_imm(offset, rectangle.start as i64);
        b.def_var(words[&rectangle.inner_element], inner);
        b.ins().udiv(at, rows)
    } else {
        at
    };
    let word = if region.range {
        b.ins().iadd(start, source_at)
    } else {
        let displacement = b.ins().imul_imm(source_at, i64::from(region.input_width));
        let address = b.ins().iadd(input, displacement);
        super::numeric_words::load_unsigned_word(
            b,
            integer(region.input_width),
            MemFlags::new(),
            address,
            0,
        )
    };
    b.def_var(words[&region.element], word);
    b.ins().jump(source_blocks[&region.body[0]], &[]);
    let source_latch = plan.iterations()[region
        .rectangle
        .as_ref()
        .map_or(region.iteration, |r| r.inner_iteration)
        as usize]
        .as_ref()
        .latch();
    for &site in &region.body {
        b.switch_to_block(source_blocks[&site]);
        if let Some(read) = region.reads.iter().find(|r| r.site == site) {
            let value = emit_read(b, read, region, &matrices, &words, index_miss)?;
            b.def_var(words[&read.output], value);
        } else {
            match plan.blocks()[site as usize].action() {
                Action::Scalar(Scalar::Copy { source, output }) => {
                    let value = word_at(b, &words, source.get())?;
                    b.def_var(words[&output.get()], value);
                }
                Action::Scalar(Scalar::Primitive {
                    op,
                    operands,
                    output,
                }) => {
                    let values = operands
                        .iter()
                        .map(|v| word_at(b, &words, v.get()))
                        .collect::<Result<Vec<_>, _>>()?;
                    let value = primitive(b, *op, &values, miss);
                    b.def_var(words[&output.get()], value);
                }
                Action::CollectionPush { value, .. } => {
                    let result = word_at(b, &words, value.get())?;
                    if plan.values()[value.get() as usize].scalar_kind()
                        == Some(mech_engine::native_artifact::NativeScalarKind::Bool)
                    {
                        let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, result, 1);
                        require(b, valid, malformed);
                    }
                    if let Some(fold) = fold {
                        let prior = b.use_var(fold);
                        let next = primitive(
                            b,
                            mech_engine::native_artifact::NativeScalarOp::F64Add,
                            &[prior, result],
                            miss,
                        );
                        b.def_var(fold, next);
                    } else {
                        let value = if region.output_width == 8 {
                            result
                        } else {
                            b.ins().ireduce(integer(region.output_width), result)
                        };
                        let displacement = b.ins().imul_imm(at, i64::from(region.output_width));
                        let address = b.ins().iadd(output, displacement);
                        b.ins().store(MemFlags::new(), value, address, 0);
                    }
                }
                Action::Scalar(Scalar::None) | Action::CollectionElement { .. } => {}
                _ => return Err(backend("numeric map source action changed")),
            }
        }
        let target = |to| {
            if to == source_latch {
                Ok(latch)
            } else {
                source_blocks
                    .get(&to)
                    .copied()
                    .ok_or_else(|| backend("numeric map branch leaves its exact body"))
            }
        };
        match plan.blocks()[site as usize].terminator() {
            Terminator::Scalar(Term::Jump(to)) => {
                b.ins().jump(target(to)?, &[]);
            }
            Terminator::Scalar(Term::Branch { condition, yes, no }) => {
                let value = word_at(b, &words, condition.get())?;
                let condition = b.ins().icmp_imm(IntCC::NotEqual, value, 0);
                b.ins().brif(condition, target(yes)?, &[], target(no)?, &[]);
            }
            Terminator::Scalar(Term::LiteralBranch {
                left,
                right,
                yes,
                no,
            }) => {
                let kind = plan.values()[left.get() as usize]
                    .scalar_kind()
                    .ok_or_else(|| backend("numeric map literal branch kind"))?;
                let left = word_at(b, &words, left.get())?;
                let right = word_at(b, &words, right.get())?;
                let condition =
                    crate::native_artifact::control_compiler::literal_equal(b, kind, left, right);
                b.ins().brif(condition, target(yes)?, &[], target(no)?, &[]);
            }
            Terminator::Scalar(Term::Reject | Term::IntegrityFailure) => {
                b.ins().jump(index_miss, &[]);
            }
            _ => return Err(backend("numeric map source terminator changed")),
        }
    }
    b.switch_to_block(latch);
    let next = b.ins().iadd_imm(at, 1);
    b.def_var(cursor, next);
    b.ins().jump(head, &[]);
    b.switch_to_block(done);
    if let Some(fold) = fold {
        let value = b.use_var(fold);
        b.ins().store(
            MemFlags::new(),
            value,
            frame,
            offset_of!(Frame, fold) as i32,
        );
    }
    b.ins().store(
        MemFlags::new(),
        count,
        frame,
        offset_of!(Frame, next) as i32,
    );
    b.ins().jump(complete, &[]);
    b.switch_to_block(index_miss);
    let no_diagnostic = b.ins().iconst(types::I64, 0);
    b.ins().jump(miss, &[no_diagnostic.into()]);
    Ok(())
}

fn word_at(
    b: &mut FunctionBuilder<'_>,
    words: &BTreeMap<u32, Variable>,
    at: u32,
) -> Result<Value, NativeCompileError> {
    Ok(b.use_var(
        *words
            .get(&at)
            .ok_or_else(|| backend("numeric map missing source word"))?,
    ))
}
fn emit_read(
    b: &mut FunctionBuilder<'_>,
    read: &crate::native_artifact::program::prepare::numeric_maps::MatrixRead,
    region: &Region,
    matrices: &[(Value, Value, Value, Value)],
    words: &BTreeMap<u32, Variable>,
    miss: Block,
) -> Result<Value, NativeCompileError> {
    let (pointer, cells, rows, columns) = matrices[read.matrix];
    let indices = read
        .indices
        .iter()
        .map(|&v| word_at(b, words, v))
        .collect::<Result<Vec<_>, _>>()?;
    let mut index = |value, bound| {
        let nonzero = b.ins().icmp_imm(IntCC::NotEqual, value, 0);
        require(b, nonzero, miss);
        let in_bounds = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, value, bound);
        require(b, in_bounds, miss);
        b.ins().iadd_imm(value, -1)
    };
    let ordinal = match &indices[..] {
        [linear] => {
            let linear = index(*linear, cells);
            let row = b.ins().urem(linear, rows);
            let column = b.ins().udiv(linear, rows);
            let offset = b.ins().imul(row, columns);
            b.ins().iadd(offset, column)
        }
        [row, column] => {
            let row = index(*row, rows);
            let column = index(*column, columns);
            let offset = b.ins().imul(row, columns);
            b.ins().iadd(offset, column)
        }
        _ => return Err(backend("numeric map read selector count")),
    };
    let width = region.matrices[read.matrix].width;
    let bytes = b.ins().imul_imm(ordinal, i64::from(width));
    let address = b.ins().iadd(pointer, bytes);
    Ok(super::numeric_words::load_unsigned_word(
        b,
        integer(width),
        MemFlags::new(),
        address,
        0,
    ))
}

/// Complete the exact elided producer/sum/extraction chain after the original
/// collection exit visit. The ordinary builder remains live on a declined map.
pub(in crate::native_artifact) fn emit_reduction_finish(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    site: usize,
    hooks: &Hooks,
    reduction: &crate::native_artifact::program::prepare::numeric_maps::Reduction,
    continuation: Block,
    error: Block,
) -> Result<(), NativeCompileError> {
    hooks.require_argument_words(b, 1, error);
    let zero = b.ins().iconst(types::I64, 0);
    hooks.call(
        b,
        site,
        GlobalHelper::NumericReductionFinish,
        [zero; 3],
        error,
    );
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let done = b.create_block();
    let ordinary = b.create_block();
    b.ins().brif(ready, done, &[], ordinary, &[]);
    b.switch_to_block(done);
    let word = b
        .ins()
        .load(types::I64, MemFlags::new(), hooks.arguments, 0);
    b.ins().store(
        MemFlags::new(),
        word,
        locals,
        offset(reduction.scalar as usize)?,
    );
    b.ins().jump(continuation, &[locals.into()]);
    b.switch_to_block(ordinary);
    Ok(())
}
