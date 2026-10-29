//! Register-backed numeric leaf. The funded successful body contains only
//! native words, typed loads/stores, arithmetic and source branches. Boundary and
//! original bounds-error calls are cold and reconcile the existing account.
use crate::native_artifact::codegen::common::{
    aggregates, ssa_words::Words, terminators, word_access::SsaWords,
};
use crate::native_artifact::codegen::control::{CodegenReport, plan::CodePlan};
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationPass};
use crate::native_artifact::compiler::{backend, offset, primitive, set_origin};
use crate::native_artifact::program::codegen::{hooks::Hooks, tail};
use crate::native_artifact::program::prepare::{
    readonly_numeric::{Operation, Proof},
    tail::{FIXED_OVERHEAD, Layout, OVERHEAD},
};
use crate::native_artifact::{NativeCompileError, abi::STATUS_ABI};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeBufferFrameOperation as Frame, NativeIterationAction as Action,
    NativeIterationTerminator as Term, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch,
};
use std::collections::{BTreeMap, BTreeSet};
#[path = "numeric_boundary.rs"]
mod boundary;

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct Report {
    pub header: usize,
    pub parent: Option<u32>,
    pub source_sites: Vec<usize>,
    pub typed_loads: Vec<(usize, u8)>,
    pub typed_stores: Vec<(usize, u8)>,
    pub first_change_boundaries: Vec<usize>,
    pub length_sites: Vec<usize>,
    pub append_sites: Vec<(usize, u8)>,
    pub capacity_guards: Vec<(usize, usize)>,
    pub scalar_operations: usize,
    pub register_words: usize,
    pub successful_backedge_helpers: usize,
    pub maximum_source_instructions: usize,
    pub fixed_template_instructions: usize,
}

fn slots(
    code: CodePlan<'_>,
    proof: &Proof,
    base: usize,
) -> Result<BTreeSet<usize>, NativeCompileError> {
    let mut slots = BTreeSet::from_iter(base..base + 3 + proof.words);
    for &site in &proof.sites {
        match code.blocks()[site].action() {
            Action::Scalar(Scalar::Primitive {
                operands, output, ..
            }) => {
                slots.extend(operands.iter().map(|v| v.get() as usize));
                slots.insert(output.get() as usize);
            }
            Action::Scalar(Scalar::Copy { source, output }) if code.kind(*output).is_some() => {
                slots.extend([source.get() as usize, output.get() as usize]);
            }
            Action::Structural { output, .. } | Action::LibraryCall { output, .. }
                if code.kind(*output).is_some() =>
            {
                slots.insert(output.get() as usize);
            }
            _ => {}
        }
        match proof.action(site) {
            Some(Operation::Read { index, .. }) => {
                slots.insert(*index as usize);
            }
            Some(Operation::Write {
                index, incoming, ..
            }) => {
                slots.extend([*index as usize, *incoming as usize]);
            }
            Some(Operation::Append { incoming, .. }) => {
                slots.insert(*incoming as usize);
            }
            _ => {}
        }
        if let CodePlan::Program(program) = code {
            if let Some(action) = program.metadata().scalar_aggregates.action(site) {
                aggregates::aggregate_words(
                    &program.metadata().scalar_aggregates,
                    action,
                    &mut slots,
                )
                .ok_or_else(|| backend("numeric aggregate lost exact word lanes"))?;
            }
            if let Some(frame) = program.metadata().buffer_frames.action(site) {
                match frame.operation() {
                    Frame::Pack { .. } => {
                        if let Action::LibraryCall { operands, .. } = code.blocks()[site].action() {
                            slots.extend(
                                operands
                                    .iter()
                                    .filter(|v| code.kind(**v).is_some())
                                    .map(|v| v.get() as usize),
                            );
                        }
                    }
                    Frame::Project { output, .. } if code.kind(*output).is_some() => {
                        slots.insert(output.get() as usize);
                    }
                    _ => {}
                }
            }
        }
        match code.blocks()[site].terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => {
                slots.insert(condition.get() as usize);
            }
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                slots.extend([left.get() as usize, right.get() as usize]);
            }
            _ => {}
        }
    }
    Ok(slots)
}

pub(in crate::native_artifact) fn emit_entry(
    controls: &CompilationControls,
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    locals: Value,
    code: CodePlan<'_>,
    closed: &Layout,
    site: usize,
    hooks: &Hooks,
    ordinary_blocks: &[Option<Block>],
    error: Block,
    reports: &mut CodegenReport,
) -> Result<(), NativeCompileError> {
    if !controls.enabled(OptimizationPass::ScalarLoops) {
        return Ok(());
    }
    let Some(id) = closed.sites.get(site).copied().flatten() else {
        return Ok(());
    };
    let region = &closed.regions[id];
    let Some(proof) = &region.numeric else {
        return Ok(());
    };
    if region.tail.header() as usize != site {
        return Ok(());
    }
    let CodePlan::Program(program) = code else {
        return Err(backend("numeric leaf lacks managed program"));
    };
    let setup_start = b.func.dfg.num_insts();
    let ordinary = b.create_block();
    let selected = b.create_block();
    let ready = b
        .ins()
        .load(types::I64, MemFlags::new(), locals, offset(region.words)?);
    let ready = b.ins().icmp_imm(IntCC::NotEqual, ready, 0);
    b.ins().brif(ready, selected, &[], ordinary, &[]);
    b.switch_to_block(selected);
    let slots = slots(code, proof, region.words)?;
    let register_words = slots.len();
    let words = Words::new(b, locals, code, slots)?;
    let setup_work = b.func.dfg.num_insts() - setup_start;
    let exit = b.create_block();
    let bailout = b.create_block();
    let failed = b.create_block();
    b.append_block_param(failed, types::I64);
    b.set_cold_block(failed);
    let mut boundaries = BTreeSet::from([exit, bailout, failed]);
    let blocks = proof
        .sites
        .iter()
        .map(|&s| (s as u32, b.create_block()))
        .collect::<BTreeMap<_, _>>();
    let target = |site| {
        if site == region.tail.exit() {
            exit
        } else {
            blocks[&site]
        }
    };
    b.ins().jump(target(region.tail.header()), &[]);
    let mut report = Report {
        header: site,
        parent: region.tail.parent(),
        source_sites: proof.sites.clone(),
        typed_loads: Vec::new(),
        typed_stores: Vec::new(),
        first_change_boundaries: Vec::new(),
        length_sites: Vec::new(),
        append_sites: Vec::new(),
        capacity_guards: Vec::new(),
        scalar_operations: 0,
        register_words,
        successful_backedge_helpers: 0,
        maximum_source_instructions: 0,
        fixed_template_instructions: 0,
    };
    let shadow = region.words + 3;
    for &site in &proof.sites {
        b.switch_to_block(target(site as u32));
        let fail = b.create_block();
        b.append_block_param(fail, types::I64);
        b.set_cold_block(fail);
        boundaries.insert(fail);
        let before = b.func.dfg.num_insts();
        if site == region.tail.header() as usize {
            let compute = words.read(b, region.words + 1);
            let comparison = words.read(b, region.words + 2);
            let a = b.ins().icmp_imm(
                IntCC::UnsignedGreaterThanOrEqual,
                compute,
                region.compute as i64,
            );
            let c = b.ins().icmp_imm(
                IntCC::UnsignedGreaterThanOrEqual,
                comparison,
                region.comparison as i64,
            );
            let mut enough = b.ins().band(a, c);
            // Capacity is checked before ANY source operation in this visit.
            // Count all append sites: a conservative DAG traversal bound, even
            // if a branch would skip some. A miss uses the existing header exit.
            for (array, _) in proof.arrays.iter().enumerate().filter(|(_, a)| a.appends) {
                let count = proof
                    .actions
                    .iter()
                    .filter(|(_, op)| {
                        matches!(op,
                    Operation::Append { array: at, .. } if *at == array)
                    })
                    .count();
                let length = words.read(b, shadow + proof.view(array) + 1);
                let capacity = words.read(b, shadow + proof.capacity(array).unwrap());
                let valid = b
                    .ins()
                    .icmp(IntCC::UnsignedLessThanOrEqual, length, capacity);
                let spare = b.ins().isub(capacity, length);
                let fits = b
                    .ins()
                    .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, spare, count as i64);
                let fits = b.ins().band(valid, fits);
                enough = b.ins().band(enough, fits);
                report.capacity_guards.push((array, count));
            }
            let funded = b.create_block();
            b.ins().brif(enough, funded, &[], bailout, &[]);
            b.switch_to_block(funded);
            let cost = b.ins().iconst(types::I64, region.compute as i64);
            let compute = b.ins().isub(compute, cost);
            let comparison = b.ins().iadd_imm(comparison, -(region.comparison as i64));
            words.write(b, region.words + 1, compute);
            words.write(b, region.words + 2, comparison);
        }
        match proof.action(site) {
            Some(Operation::Aggregate) => {
                let layout = &program.metadata().scalar_aggregates;
                let action = layout
                    .action(site)
                    .ok_or_else(|| backend("numeric aggregate lost its shared action"))?;
                aggregates::emit(
                    b,
                    &mut SsaWords(&words),
                    layout,
                    action,
                    fail,
                    &mut aggregates::CheckedGuards,
                )?;
            }
            Some(Operation::Frame) => {
                let operation = program
                    .metadata()
                    .buffer_frames
                    .action(site)
                    .ok_or_else(|| backend("numeric frame lost fields"))?
                    .operation();
                match operation {
                    Frame::Pack { output } => {
                        let Action::LibraryCall { operands, .. } = code.blocks()[site].action()
                        else {
                            return Err(backend("numeric pack lost operands"));
                        };
                        for (field, operand) in
                            proof.fields(output.get()).unwrap().iter().zip(operands)
                        {
                            if let Some(field) = field {
                                let value = words.read(b, operand.get() as usize);
                                words.write(b, shadow + field, value);
                            }
                        }
                    }
                    Frame::Copy { source, output } => {
                        // Read every old scalar before writing any next-state lane.
                        let copied = proof
                            .fields(source.get())
                            .unwrap()
                            .iter()
                            .zip(proof.fields(output.get()).unwrap())
                            .filter_map(|(from, to)| from.zip(*to))
                            .map(|(from, to)| (to, words.read(b, shadow + from)))
                            .collect::<Vec<_>>();
                        for (to, value) in copied {
                            words.write(b, shadow + to, value);
                        }
                    }
                    Frame::Project {
                        source,
                        ordinal,
                        output,
                    } => {
                        if let Some(field) = proof.fields(source.get()).unwrap()[*ordinal] {
                            let value = words.read(b, shadow + field);
                            words.write(b, output.get() as usize, value);
                        }
                    }
                }
            }
            Some(Operation::ArrayCopy) => {}
            Some(Operation::Length { array, output }) => {
                let length = words.read(b, shadow + proof.view(*array) + 1);
                words.write(b, *output as usize, length);
                report.length_sites.push(site);
            }
            Some(Operation::Append { array, incoming }) => {
                let view = shadow + proof.view(*array);
                let length = words.read(b, view + 1);
                let incoming = words.read(b, *incoming as usize);
                let private = words.read(b, shadow + proof.private_flag(*array));
                let store = b.create_block();
                let materialize = b.create_block();
                let done = b.create_block();
                b.set_cold_block(materialize);
                boundaries.insert(materialize);
                b.ins().brif(private, store, &[], materialize, &[]);
                b.switch_to_block(materialize);
                let index = b.ins().iadd_imm(length, 1);
                boundary::materialize(
                    b, &words, region, proof, *array, site, frame, locals, index, incoming, hooks,
                    error,
                )?;
                b.ins().jump(done, &[]);
                b.switch_to_block(store);
                let width = proof.arrays[*array].width;
                let base = words.read(b, view);
                let at = b.ins().imul_imm(length, i64::from(width));
                let address = b.ins().iadd(base, at);
                let ty = match width {
                    1 => types::I8,
                    2 => types::I16,
                    4 => types::I32,
                    8 => types::I64,
                    _ => return Err(backend("numeric append width differs")),
                };
                let value = if width < 8 {
                    b.ins().ireduce(ty, incoming)
                } else {
                    incoming
                };
                // The admitted header proves spare cells for this complete
                // traversal. Advance logical length only after initialization.
                b.ins().store(MemFlags::new(), value, address, 0);
                let length = b.ins().iadd_imm(length, 1);
                words.write(b, view + 1, length);
                words.write(b, view + 3, length);
                b.ins().jump(done, &[]);
                b.switch_to_block(done);
                report.typed_stores.push((site, width));
                report.append_sites.push((site, width));
                report.first_change_boundaries.push(site);
            }
            Some(
                operation @ (Operation::Read { array, index, .. }
                | Operation::Write { array, index, .. }),
            ) => {
                let view = shadow + proof.view(*array);
                let index = words.read(b, *index as usize);
                let length = words.read(b, view + 1);
                let ordinal = b.ins().iadd_imm(index, -1);
                let valid = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, length);
                let good = b.create_block();
                let bad = b.create_block();
                b.set_cold_block(bad);
                boundaries.insert(bad);
                b.ins().brif(valid, good, &[], bad, &[]);
                b.switch_to_block(bad);
                set_origin(b, frame, site as u64);
                words.spill_all(b, locals)?;
                tail::failure(controls.options(), b, locals, closed, site, hooks, error)?;
                let Action::LibraryCall { operands, .. } = code.blocks()[site].action() else {
                    return Err(backend("numeric read lost selected call"));
                };
                hooks.library(b, site, operands, locals, |v| code.kind(v).is_some(), error)?;
                let status = b.ins().iconst(types::I64, STATUS_ABI as i64);
                b.ins().jump(error, &[status.into()]);
                b.switch_to_block(good);
                // Canonical storage is row-major; source linear selection is
                // one-based column-major. A valid ordinal proves rows > 0.
                let rows = words.read(b, view + 2);
                let columns = words.read(b, view + 3);
                let row = b.ins().urem(ordinal, rows);
                let column = b.ins().udiv(ordinal, rows);
                let row = b.ins().imul(row, columns);
                let physical = b.ins().iadd(row, column);
                let width = proof.arrays[*array].width;
                let at = b.ins().imul_imm(physical, i64::from(width));
                let base = words.read(b, view);
                let address = b.ins().iadd(base, at);
                let ty = match width {
                    1 => types::I8,
                    2 => types::I16,
                    4 => types::I32,
                    8 => types::I64,
                    _ => return Err(backend("numeric load width differs")),
                };
                let value = b.ins().load(ty, MemFlags::new(), address, 0);
                let value = if width < 8 {
                    b.ins().uextend(types::I64, value)
                } else {
                    value
                };
                report.typed_loads.push((site, width));
                match operation {
                    Operation::Read { output, .. } => words.write(b, *output as usize, value),
                    Operation::Write { incoming, .. } => {
                        let incoming = words.read(b, *incoming as usize);
                        let changed = b.ins().icmp(IntCC::NotEqual, value, incoming);
                        let update = b.create_block();
                        let done = b.create_block();
                        b.ins().brif(changed, update, &[], done, &[]);
                        b.switch_to_block(update);
                        let private = words.read(b, shadow + proof.private_flag(*array));
                        let store = b.create_block();
                        let materialize = b.create_block();
                        b.set_cold_block(materialize);
                        boundaries.insert(materialize);
                        b.ins().brif(private, store, &[], materialize, &[]);
                        b.switch_to_block(materialize);
                        boundary::materialize(
                            b, &words, region, proof, *array, site, frame, locals, index, incoming,
                            hooks, error,
                        )?;
                        b.ins().jump(done, &[]);
                        b.switch_to_block(store);
                        let narrowed = if width < 8 {
                            b.ins().ireduce(ty, incoming)
                        } else {
                            incoming
                        };
                        b.ins().store(MemFlags::new(), narrowed, address, 0);
                        b.ins().jump(done, &[]);
                        b.switch_to_block(done);
                        report.typed_stores.push((site, width));
                        report.first_change_boundaries.push(site);
                    }
                    _ => unreachable!("selected numeric access"),
                }
            }
            None => match code.blocks()[site].action() {
                Action::Scalar(Scalar::None) => {}
                Action::Scalar(Scalar::Copy { source, output }) => {
                    let value = words.read(b, source.get() as usize);
                    words.write(b, output.get() as usize, value);
                }
                Action::Scalar(Scalar::Primitive {
                    op,
                    operands,
                    output,
                    ..
                }) => {
                    let args = operands
                        .iter()
                        .map(|v| words.read(b, v.get() as usize))
                        .collect::<Vec<_>>();
                    let value = primitive(b, *op, &args, fail);
                    words.write(b, output.get() as usize, value);
                    report.scalar_operations += 1;
                }
                _ => return Err(backend("numeric proof contains unsupported action")),
            },
        }
        terminators::emit_source_terminator(
            b,
            &words,
            code,
            code.blocks()[site].terminator(),
            fail,
            target,
        );
        b.switch_to_block(fail);
        let status = b.block_params(fail)[0];
        set_origin(b, frame, site as u64);
        b.ins().jump(failed, &[status.into()]);
        let instructions = b.func.dfg.num_insts() - before;
        report.maximum_source_instructions = report.maximum_source_instructions.max(instructions);
        // Cold synchronization is covered by the fixed boundary envelope.
        if instructions > OVERHEAD as usize + register_words * 2 {
            return Err(backend("numeric leaf exceeds finite source envelope"));
        }
    }
    let boundary = b.func.dfg.num_insts();
    b.switch_to_block(bailout);
    words.spill_all(b, locals)?;
    let header = b.ins().iconst(types::I64, region.tail.header() as i64);
    tail::leave(
        controls.options(),
        b,
        locals,
        region,
        header,
        1,
        hooks,
        error,
    )?;
    b.ins().jump(ordinary, &[]);
    b.switch_to_block(exit);
    words.spill_all(b, locals)?;
    b.ins()
        .jump(ordinary_blocks[region.tail.exit() as usize].unwrap(), &[]);
    b.switch_to_block(failed);
    let status = b.block_params(failed)[0];
    words.spill_all(b, locals)?;
    b.ins().jump(error, &[status.into()]);
    report.fixed_template_instructions = setup_work + b.func.dfg.num_insts() - boundary;
    if report.fixed_template_instructions > FIXED_OVERHEAD as usize {
        return Err(backend("numeric leaf exceeds finite boundary envelope"));
    }
    // Inspect the actual emitted graph, stopping at the admitted cold/exit
    // boundaries. This is a structural check, not a fabricated runtime counter.
    let cfg = cranelift_codegen::flowgraph::ControlFlowGraph::with_function(b.func);
    let mut seen = BTreeSet::new();
    let mut pending = vec![target(region.tail.header())];
    while let Some(block) = pending.pop() {
        if boundaries.contains(&block) || !seen.insert(block) {
            continue;
        }
        report.successful_backedge_helpers += b
            .func
            .layout
            .block_insts(block)
            .filter(|inst| b.func.dfg.insts[*inst].opcode().is_call())
            .count();
        pending.extend(cfg.succ_iter(block));
    }
    if report.successful_backedge_helpers != 0 {
        return Err(backend("numeric funded body contains a general call"));
    }
    if proof.writes() {
        reports.numeric_write_loops.push(report);
    } else {
        reports.readonly_numeric_loops.push(report);
    }
    b.switch_to_block(ordinary);
    Ok(())
}
