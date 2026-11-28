//! A checked scalar DAG inside a native immutable-source collection loop.
use super::hooks::Hooks;
#[path = "numeric_map_selection.rs"]
pub(super) mod selection;
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::compiler::{backend, f64_arithmetic, offset, primitive};
use crate::native_artifact::program::prepare::numeric_maps::{RectangleBounds, Region};
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

/// An emission-only choice, never a runtime value or new source proof. All
/// admitted map reductions are ordered F64 sums. Keep that sum in its native
/// register class until the existing raw-word frame boundary. Source terms
/// and all their checked scalar operations retain their original word DAG.
#[derive(Clone, Copy)]
struct Fold {
    variable: Variable,
    typed: bool,
}
impl Fold {
    fn new(b: &mut FunctionBuilder<'_>, zero: Value, typed: bool) -> Self {
        let variable = b.declare_var(if typed { types::F64 } else { types::I64 });
        let initial = if typed {
            b.ins()
                .f64const(cranelift_codegen::ir::immediates::Ieee64::with_bits(0))
        } else {
            zero
        };
        b.def_var(variable, initial);
        Self { variable, typed }
    }
    fn push(self, b: &mut FunctionBuilder<'_>, term: Value, miss: Block) {
        use mech_engine::native_artifact::NativeScalarOp::F64Add;
        let prior = b.use_var(self.variable);
        let next = if self.typed {
            let term = b.ins().bitcast(types::F64, MemFlags::new(), term);
            f64_arithmetic(b, F64Add, prior, term)
        } else {
            primitive(b, F64Add, &[prior, term], miss)
        };
        b.def_var(self.variable, next);
    }
    fn word(self, b: &mut FunctionBuilder<'_>) -> Value {
        let value = b.use_var(self.variable);
        if self.typed {
            b.ins().bitcast(types::I64, MemFlags::new(), value)
        } else {
            value
        }
    }
}

pub(in crate::native_artifact) fn emit_entry(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    site: usize,
    hooks: &Hooks,
    region: &Region,
    plan: &Plan,
    counter_slot: usize,
    typed_folds: bool,
    ordered_dots: bool,
    affine_cells: bool,
    exit: Block,
    error: Block,
) -> Result<(), NativeCompileError> {
    let zero = b.ins().iconst(types::I64, 0);
    let args = if let Some(rectangle) = &region.rectangle {
        if let RectangleBounds::Current { operands } = rectangle.bounds {
            let mech_engine::native_artifact::NativeControlLoop::Collection(inner) =
                &plan.iterations()[rectangle.inner_iteration as usize]
            else {
                return Err(backend("current rectangle lost inner collection"));
            };
            let mech_engine::native_artifact::NativeCollectionSource::U64Range(cursor) =
                inner.source()
            else {
                return Err(backend("current rectangle lost inner range"));
            };
            if cursor.operands().map(|value| value.get()) != operands {
                return Err(backend("current rectangle endpoint inventory differs"));
            }
            let [first, last] = cursor.operands();
            [
                crate::native_artifact::control_compiler::load(b, locals, *first)?,
                crate::native_artifact::control_compiler::load(b, locals, *last)?,
                zero,
            ]
        } else {
            [zero; 3]
        }
    } else {
        [zero; 3]
    };
    hooks.call(b, site, GlobalHelper::NumericMapPrepare, args, error);
    let frame = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let selected = b.create_block();
    let ordinary = b.create_block();
    b.ins().brif(frame, selected, &[], ordinary, &[]);
    b.switch_to_block(selected);
    // External scalar captures are immutable and already current at this
    // header. Snapshot each native word once, never inside the element loop.
    let mut captures = BTreeMap::new();
    for &site in &region.body {
        for value in crate::native_artifact::program::prepare::numeric_maps::nested::reads(
            plan,
            site,
            &region.nested,
        ) {
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
    emit(
        b,
        frame,
        region,
        plan,
        &captures,
        typed_folds,
        ordered_dots,
        affine_cells,
        complete,
        miss,
        malformed,
    )?;
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
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    region: &Region,
    plan: &Plan,
    captures: &BTreeMap<u32, Value>,
    typed_folds: bool,
    ordered_dots: bool,
    affine_cells: bool,
    complete: Block,
    miss: Block,
    malformed: Block,
) -> Result<(), NativeCompileError> {
    emit_with_selection(
        b,
        frame,
        region,
        plan,
        captures,
        typed_folds,
        ordered_dots,
        affine_cells,
        None,
        None,
        complete,
        miss,
        malformed,
    )
}
pub(super) fn emit_with_selection(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    region: &Region,
    plan: &Plan,
    captures: &BTreeMap<u32, Value>,
    typed_folds: bool,
    ordered_dots: bool,
    affine_cells: bool,
    selection: Option<&selection::Selection>,
    whole_output_initialized: Option<Value>,
    complete: Block,
    miss: Block,
    malformed: Block,
) -> Result<(), NativeCompileError> {
    if whole_output_initialized.is_some() && selection.is_none() {
        return Err(backend("partial scratch requires an exact selected map"));
    }
    // Arithmetic failures carry the common emitter's diagnostic word. A
    // bounds miss needs no synthetic source error: ordinary replay owns it.
    let index_miss = b.create_block();
    let count = get(b, frame, offset_of!(Frame, count));
    let source_count = get(b, frame, offset_of!(Frame, source_count));
    let (inner_start, inner_count) = match &region.rectangle {
        Some(rectangle) => match rectangle.bounds {
            RectangleBounds::Fixed { start, rows, .. } => (
                b.ins().iconst(types::I64, start as i64),
                b.ins().iconst(types::I64, rows as i64),
            ),
            RectangleBounds::Current { .. } => {
                let start = get(b, frame, offset_of!(Frame, rectangle_start));
                let rows = get(b, frame, offset_of!(Frame, rectangle_rows));
                let positive = b.ins().icmp_imm(IntCC::NotEqual, rows, 0);
                require(b, positive, malformed);
                let last = b.ins().iadd_imm(rows, -1);
                let (_, overflow) = b.ins().uadd_overflow(start, last);
                let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
                require(b, good, malformed);
                (start, rows)
            }
        },
        None => (b.ins().iconst(types::I64, 0), b.ins().iconst(types::I64, 1)),
    };
    let (expected, overflow) = b.ins().umul_overflow(source_count, inner_count);
    let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    require(b, good, malformed);
    let good = b.ins().icmp(IntCC::Equal, expected, count);
    require(b, good, malformed);
    if let Some(selection) = selection {
        let valid = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, selection.count(), count);
        require(b, valid, malformed);
    }
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
        // Keep literal source geometry visible to the native optimizer after
        // checking the actual current view. These are scalar facts, never a
        // cached owner or pointer. Unsupported shapes retain dynamic geometry.
        let (cells, rows, columns) = guard_fixed_geometry(
            b,
            plan.values()[matrix.value as usize].definition().body(),
            cells,
            rows,
            columns,
            malformed,
        );
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
    if ordered_dots && selection.is_none() {
        super::numeric_map_dots::emit(
            b, frame, region, plan, captures, &matrices, start, count, output, complete,
        )?;
    }
    if affine_cells {
        super::numeric_map_dots::cells::emit(
            b,
            frame,
            region,
            plan,
            captures,
            &matrices,
            start,
            count,
            source_count,
            inner_start,
            inner_count,
            output,
            selection,
            whole_output_initialized,
            complete,
        )?;
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
        let output = if let Some(child) = region.nested.iter().find(|r| r.source.entry == site) {
            child.output(plan).get()
        } else {
            match plan.blocks()[site as usize].action() {
                Action::Scalar(Scalar::Copy { output, .. } | Scalar::Primitive { output, .. })
                | Action::LibraryCall { output, .. } => output.get(),
                _ => return Err(backend("numeric map source step changed")),
            }
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
    let fold = region
        .reduction
        .as_ref()
        .map(|_| Fold::new(b, zero, typed_folds));
    let head = b.create_block();
    let body = b.create_block();
    let done = b.create_block();
    b.ins().jump(head, &[]);
    b.switch_to_block(head);
    let at = b.use_var(cursor);
    let iterations = selection.map_or(count, |selection| selection.count());
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, iterations);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    let ordinal = b.use_var(cursor);
    let at = ordinal;
    let (source_at, rectangle_row) = if let Some(selection) = selection {
        // Keep the actual source axes in SSA rather than flattening and then
        // rediscovering them through an unnecessary quotient/remainder pair.
        selection.cursors(b, ordinal)
    } else if region.rectangle.is_some() {
        let column = b.ins().udiv(at, inner_count);
        // One quotient serves the source cursor and final row-major store.
        // 0 <= at < rows * source_count, so column * rows <= at.
        let consumed = b.ins().imul(column, inner_count);
        let row = b.ins().isub(at, consumed);
        (column, Some(row))
    } else {
        (at, None)
    };
    if let Some(rectangle) = &region.rectangle {
        let row = rectangle_row
            .ok_or_else(|| backend("numeric map lost its current inner coordinate"))?;
        let inner = b.ins().iadd(row, inner_start);
        b.def_var(words[&rectangle.inner_element], inner);
    }
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
        if let Some(child) = region.nested.iter().find(|r| r.source.entry == site) {
            let first = word_at(b, &words, child.source.first)?;
            let last = word_at(b, &words, child.source.last)?;
            let captures = child
                .source
                .region
                .captures
                .iter()
                .map(|&v| Ok((v, word_at(b, &words, v)?)))
                .collect::<Result<BTreeMap<_, _>, NativeCompileError>>()?;
            let views = child
                .matrices
                .iter()
                .map(|&at| {
                    let (input, count, rows, columns) = matrices[at];
                    [input, count, rows, columns]
                })
                .collect::<Vec<_>>();
            let value = super::numeric_phase_reductions::emit_from_views(
                b,
                &child.source,
                plan,
                first,
                last,
                &views,
                &captures,
                typed_folds,
                ordered_dots,
                affine_cells,
                index_miss,
            )?;
            b.def_var(words[&child.output(plan).get()], value);
            let target = if child.source.after == source_latch {
                latch
            } else {
                *source_blocks
                    .get(&child.source.after)
                    .ok_or_else(|| backend("nested fold leaves map"))?
            };
            b.ins().jump(target, &[]);
            continue;
        }
        if let Some(read) = region.reads.iter().find(|r| r.site == site) {
            let value = emit_read(b, read, region, plan, &matrices, &words, index_miss)?;
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
                        fold.push(b, result, miss);
                    } else {
                        let value = if region.output_width == 8 {
                            result
                        } else {
                            b.ins().ireduce(integer(region.output_width), result)
                        };
                        // Preserve source evaluation order (one complete column at a time),
                        // but initialize each cell at its final row-major location.
                        // count = rows * source_count and 0 <= at < count were
                        // checked above, so this permutation is in-bounds and
                        // visits each selected output cell exactly once (all
                        // cells on the dense path). Complement initialization
                        // and the complete-prefix certificate are separate.
                        let output_at = if let Some(row) = rectangle_row {
                            let base = b.ins().imul(row, source_count);
                            b.ins().iadd(base, source_at)
                        } else {
                            source_at
                        };
                        let displacement =
                            b.ins().imul_imm(output_at, i64::from(region.output_width));
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
        if let Some(to) = selection.and_then(|selection| selection.selected_edge(site)) {
            // Interval selection proves this source edge for every reached
            // coordinate. Prefix work is total; active checked work is unchanged.
            b.ins().jump(target(to)?, &[]);
            continue;
        }
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
    let next = b.ins().iadd_imm(ordinal, 1);
    b.def_var(cursor, next);
    b.ins().jump(head, &[]);
    b.switch_to_block(done);
    if let Some(fold) = fold {
        let value = fold.word(b);
        b.ins().store(
            MemFlags::new(),
            value,
            frame,
            offset_of!(Frame, fold) as i32,
        );
    }
    // `next` certifies a complete initialized prefix, never logical source
    // visits. Selected-only scratch may have uninitialized holes: its caller
    // publishes only the exact cells written above and must not use ordinary
    // NumericMapFinish. Conservatively grant no complete-prefix certificate.
    let initialized = whole_output_initialized
        .map(|whole| b.ins().select(whole, count, zero))
        .unwrap_or(count);
    b.ins().store(
        MemFlags::new(),
        initialized,
        frame,
        offset_of!(Frame, next) as i32,
    );
    b.ins().jump(complete, &[]);
    b.switch_to_block(index_miss);
    let no_diagnostic = b.ins().iconst(types::I64, 0);
    b.ins().jump(miss, &[no_diagnostic.into()]);
    Ok(())
}

/// Literal source extents only. Current entry guards retain their authority;
/// no runtime parameter or compound dimension is speculatively evaluated here.
fn fixed_geometry(schema: &mech_core::SchemaBody) -> Option<[u64; 3]> {
    let mech_core::SchemaBody::Matrix { dimensions, .. } = schema else {
        return None;
    };
    let [
        mech_core::DimensionExpr::Constant(rows),
        mech_core::DimensionExpr::Constant(columns),
    ] = &dimensions[..]
    else {
        return None;
    };
    if *rows == 0 || *columns == 0 {
        return None;
    }
    Some([rows.checked_mul(*columns)?, *rows, *columns])
}

fn guard_fixed_geometry(
    b: &mut FunctionBuilder<'_>,
    schema: &mech_core::SchemaBody,
    cells: Value,
    rows: Value,
    columns: Value,
    malformed: Block,
) -> (Value, Value, Value) {
    let Some([cells, fixed_rows, fixed_columns]) = fixed_geometry(schema) else {
        return (cells, rows, columns);
    };
    for (actual, expected) in [(rows, fixed_rows), (columns, fixed_columns)] {
        let good = b.ins().icmp_imm(IntCC::Equal, actual, expected as i64);
        require(b, good, malformed);
    }
    (
        b.ins().iconst(types::I64, cells as i64),
        b.ins().iconst(types::I64, fixed_rows as i64),
        b.ins().iconst(types::I64, fixed_columns as i64),
    )
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
    plan: &Plan,
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
    let width = region.matrices[read.matrix].width;
    let (address, ty) = match &indices[..] {
        [linear] => {
            // Unsigned (index - 1) < length checks zero and the upper bound
            // together, including zero-length and maximum-word selectors.
            let ordinal = b.ins().iadd_imm(*linear, -1);
            let valid = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, cells);
            require(b, valid, miss);
            super::numeric_words::linear_address(
                b,
                ordinal,
                rows,
                columns,
                width,
                plan.values()[region.matrices[read.matrix].value as usize]
                    .definition()
                    .body(),
                |_| pointer,
            )?
        }
        [row, column] => super::numeric_words::cell_address(
            b, *row, *column, rows, columns, width, pointer, miss,
        )?,
        _ => return Err(backend("numeric map read selector count")),
    };
    Ok(super::numeric_words::load_unsigned_word(
        b,
        ty,
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
    emit_projection_finish(
        b,
        locals,
        site,
        hooks,
        GlobalHelper::NumericReductionFinish,
        reduction.scalar,
        continuation,
        error,
    )
}

pub(in crate::native_artifact) fn emit_projection_finish(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    site: usize,
    hooks: &Hooks,
    helper: GlobalHelper,
    output: u32,
    continuation: Block,
    error: Block,
) -> Result<(), NativeCompileError> {
    hooks.require_argument_words(b, 1, error);
    let zero = b.ins().iconst(types::I64, 0);
    hooks.call(b, site, helper, [zero; 3], error);
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let done = b.create_block();
    let ordinary = b.create_block();
    b.ins().brif(ready, done, &[], ordinary, &[]);
    b.switch_to_block(done);
    let word = b
        .ins()
        .load(types::I64, MemFlags::new(), hooks.arguments, 0);
    b.ins()
        .store(MemFlags::new(), word, locals, offset(output as usize)?);
    b.ins().jump(continuation, &[locals.into()]);
    b.switch_to_block(ordinary);
    Ok(())
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "numeric_map_geometry_tests.rs"]
mod geometry_tests;
