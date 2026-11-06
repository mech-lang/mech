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
    NativeIterationAction as Action, NativeMatchAction as Scalar, NativeScalarOp as Op,
};
use std::collections::{BTreeMap, BTreeSet};
#[path = "numeric_boundary.rs"]
mod boundary;
#[path = "numeric_child_fuel.rs"]
mod child_fuel;
#[path = "numeric_snapshots.rs"]
mod snapshots;

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct Report {
    pub header: usize,
    pub parent: Option<u32>,
    pub source_sites: Vec<usize>,
    pub typed_loads: Vec<(usize, u8)>,
    pub read_pack_sites: Vec<(usize, usize, u8)>,
    pub source_guarded_read_packs: Vec<usize>,
    pub fully_lowered_read_packs: Vec<usize>,
    pub proven_projection_guards: Vec<(usize, u32, usize, u32)>,
    pub proven_boolean_successors: Vec<(usize, u32)>,
    pub proven_scalar_read_guards: Vec<(usize, usize, u32)>,
    pub proven_scalar_arithmetic: Vec<usize>,
    pub guarded_loop_windows: Vec<super::numeric_loop_windows::Report>,
    pub version_entry_gates: Vec<(usize, usize, Option<usize>)>,
    pub typed_stores: Vec<(usize, u8)>,
    pub first_change_boundaries: Vec<usize>,
    pub cold_exits: Vec<(usize, usize)>,
    pub length_sites: Vec<usize>,
    pub append_sites: Vec<(usize, u8)>,
    pub bulk_append_sites: Vec<usize>,
    pub capacity_guards: Vec<(usize, usize)>,
    pub scalar_operations: usize,
    pub register_words: usize,
    pub fresh_traversal_words: usize,
    pub bounds_error_spill_words: Vec<(usize, usize)>,
    pub boundary_spill_words: Vec<(&'static str, usize, usize)>,
    pub successful_backedge_helpers: usize,
    pub maximum_source_instructions: usize,
    pub fixed_template_instructions: usize,
}

impl Report {
    fn source_instructions(&mut self, instructions: usize) -> Result<(), NativeCompileError> {
        self.maximum_source_instructions = self.maximum_source_instructions.max(instructions);
        // Cold synchronization is covered by the fixed boundary envelope.
        if instructions > OVERHEAD as usize + self.register_words * 2 {
            return Err(backend("numeric leaf exceeds finite source envelope"));
        }
        Ok(())
    }
}

fn slots(
    code: CodePlan<'_>,
    proof: &Proof,
    state: usize,
    shadow: usize,
) -> Result<BTreeSet<usize>, NativeCompileError> {
    super::numeric_words::source_slots(
        code,
        proof.sites.iter().copied(),
        &proof.actions,
        (state..state + 3).chain(shadow..shadow + proof.words),
    )
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
    let Some(id) = closed.header(site) else {
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
    tail::resume_cold(controls.options(), b, locals, region, site, hooks, error)?;
    let ordinary = b.create_block();
    let selected = b.create_block();
    let ready = b.ins().load(
        types::I64,
        tail::flags(controls.options(), region),
        locals,
        offset(region.words)?,
    );
    let ready = b.ins().icmp_imm(IntCC::NotEqual, ready, 0);
    b.ins().brif(ready, selected, &[], ordinary, &[]);
    b.switch_to_block(selected);
    let shadow = region
        .shadow
        .ok_or_else(|| backend("numeric region lacks scratch home"))?;
    let slots = slots(code, proof, region.words, shadow)?;
    let candidates =
        snapshots::source_words(code, proof, shadow, &proof.boundary_values.candidates)?;
    if slots
        .iter()
        .copied()
        .filter(|slot| {
            !(region.words..region.words + 3).contains(slot)
                && !(shadow..shadow + proof.words).contains(slot)
        })
        .chain(shadow..shadow + proof.scalar_words)
        .any(|slot| !candidates.contains(&slot))
    {
        return Err(backend("numeric exit query omits a registered source word"));
    }
    let register_words = slots.len();
    let mut fresh = BTreeSet::new();
    for &value in &proof.fresh_values {
        if program.metadata().plan.values()[value as usize]
            .scalar_kind()
            .is_some()
        {
            fresh.insert(value as usize);
        }
        if let Some(fields) = proof.fields(value) {
            fresh.extend(fields.iter().flatten().map(|field| shadow + field));
        }
        if let Some((base, count)) = program.metadata().scalar_aggregates.slot_at(value as usize) {
            fresh.extend(base..base + count);
        }
    }
    fresh.retain(|slot| slots.contains(slot));
    let words = Words::new_with_regions(b, locals, code, slots.clone(), |slot| {
        if (region.words..region.words + 3).contains(&slot)
            || (shadow..shadow + proof.words).contains(&slot)
        {
            tail::flags(controls.options(), region)
        } else {
            MemFlags::new()
        }
    })?;
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
    let read_packs = super::numeric_read_packs::derive(&program.metadata().plan.facts(), proof);
    let boolean_successors =
        super::numeric_branches::derive(&program.metadata().plan.facts(), proof);
    let traversal = super::numeric_traversal::derive(code, proof, shadow);
    let versions = super::numeric_loop_windows::derive(code, proof, shadow, b, &read_packs);
    b.ins().jump(target(region.tail.header()), &[]);
    let mut report = Report {
        header: site,
        parent: region.tail.parent(),
        source_sites: proof.sites.clone(),
        typed_loads: Vec::new(),
        read_pack_sites: Vec::new(),
        source_guarded_read_packs: Vec::new(),
        fully_lowered_read_packs: Vec::new(),
        proven_boolean_successors: boolean_successors
            .iter()
            .map(|(&site, &next)| (site, next))
            .collect(),
        proven_projection_guards: Vec::new(),
        proven_scalar_read_guards: Vec::new(),
        proven_scalar_arithmetic: Vec::new(),
        guarded_loop_windows: versions.iter().map(|v| v.report(proof)).collect(),
        version_entry_gates: Vec::new(),
        typed_stores: Vec::new(),
        first_change_boundaries: Vec::new(),
        cold_exits: proof
            .exits
            .iter()
            .map(|exit| (exit.from, exit.to))
            .collect(),
        length_sites: Vec::new(),
        append_sites: Vec::new(),
        bulk_append_sites: Vec::new(),
        capacity_guards: Vec::new(),
        scalar_operations: 0,
        register_words,
        fresh_traversal_words: fresh.len(),
        bounds_error_spill_words: Vec::new(),
        boundary_spill_words: Vec::new(),
        successful_backedge_helpers: 0,
        maximum_source_instructions: 0,
        fixed_template_instructions: 0,
    };
    // Both bodies share the same admitted SSA homes and loan. A guard is
    // evaluated afresh only at the canonical initialize edge. Each version's
    // latch returns to its own header; exits rejoin the existing parent.
    let emitted_sites = proof.sites.iter().map(|&site| (None, site)).chain(
        versions
            .iter()
            .flat_map(|version| version.sites.iter().map(move |&site| (Some(version), site))),
    );
    for (version, site) in emitted_sites {
        let complete_pack = |pack: &super::numeric_read_packs::Pack| {
            pack.source_window
                && version.is_some_and(|v| {
                    v.facts.flat_arrays.contains(&pack.array) && pack.confined_to(&v.blocks)
                })
        };
        // A completed packed read replaces its entire confined expression.
        // Do not manufacture unreachable byte definitions and their SSA joins
        // in the selected loop. Original checked code is emitted separately.
        if read_packs
            .iter()
            .any(|pack| pack.first != site && pack.contains(site) && complete_pack(pack))
        {
            continue;
        }
        let target = |site| {
            if let Some(block) = version.and_then(|v| v.blocks.get(&site)) {
                *block
            } else if site == region.tail.exit() {
                exit
            } else {
                blocks[&site]
            }
        };
        let traversal = version.map_or(&traversal, |v| &v.facts);
        b.switch_to_block(target(site as u32));
        if matches!(proof.action(site), Some(Operation::Fallback)) {
            boundaries.insert(target(site as u32));
            let continuation = proof
                .exits
                .iter()
                .find(|exit| exit.to == site)
                .ok_or_else(|| backend("ordinary numeric snapshot lacks its exact exit"))?;
            let count = snapshots::spill(
                b,
                &words,
                locals,
                code,
                region,
                6,
                Some(site),
                &continuation.values,
                &slots,
            )?;
            report.boundary_spill_words.push(("ordinary", site, count));
            set_origin(b, frame, site as u64);
            let origin = b.ins().iconst(types::I64, site as i64);
            tail::leave_at(
                controls.options(),
                b,
                locals,
                region,
                origin,
                6,
                Some(site),
                hooks,
                error,
            )?;
            let one = b.ins().iconst(types::I64, 1);
            b.ins().store(
                tail::flags(controls.options(), region),
                one,
                locals,
                offset(region.cold_resume().unwrap())?,
            );
            b.ins().jump(
                ordinary_blocks[site]
                    .ok_or_else(|| backend("cold numeric continuation is absent"))?,
                &[],
            );
            continue;
        }
        let fail = b.create_block();
        b.append_block_param(fail, types::I64);
        b.set_cold_block(fail);
        boundaries.insert(fail);
        let before = b.func.dfg.num_insts();
        if site == region.tail.header() as usize {
            // Canonical liveness proves no source read before redefinition in
            // this new traversal. These private SSA definitions add no source
            // operations or canonical writes and break historical loop-carried
            // dependencies created solely by conservative exit snapshots.
            let zero = b.ins().iconst(types::I64, 0);
            for &slot in &fresh {
                words.write(b, slot, zero);
            }
        }
        if let Some(count) = child_fuel::emit(
            b,
            controls.options(),
            &words,
            code,
            &slots,
            region,
            proof,
            site,
            frame,
            locals,
            hooks,
            ordinary_blocks,
            error,
            &mut boundaries,
        )? {
            report.boundary_spill_words.push(("child", site, count));
        }
        if site == region.tail.header() as usize {
            let compute = words.read(b, region.words + 1);
            let comparison = words.read(b, region.words + 2);
            let (compute_bound, comparison_bound, valid) = if let Some(phase) = &proof.phase {
                let (work, a) = super::numeric_fuel::envelope(
                    b,
                    &words,
                    code,
                    proof,
                    shadow,
                    region.tail.carrier().get(),
                    &phase.compute_bound,
                )?;
                let (compare, c) = super::numeric_fuel::envelope(
                    b,
                    &words,
                    code,
                    proof,
                    shadow,
                    region.tail.carrier().get(),
                    &phase.comparison_bound,
                )?;
                (work, compare, b.ins().band(a, c))
            } else {
                (
                    b.ins().iconst(types::I64, region.compute as i64),
                    b.ins().iconst(types::I64, region.comparison as i64),
                    b.ins().iconst(types::I8, 1),
                )
            };
            let a = b
                .ins()
                .icmp(IntCC::UnsignedGreaterThanOrEqual, compute, compute_bound);
            let c = b.ins().icmp(
                IntCC::UnsignedGreaterThanOrEqual,
                comparison,
                comparison_bound,
            );
            let mut enough = b.ins().band(a, c);
            enough = b.ins().band(enough, valid);
            // Capacity is checked before ANY source operation in this visit.
            // Count all append sites: a conservative DAG traversal bound, even
            // if a branch would skip some. A miss uses the existing header exit.
            for (array, _) in proof.arrays.iter().enumerate().filter(|(_, a)| a.appends) {
                let count = proof
                    .actions
                    .iter()
                    .try_fold(0usize, |count, (_, op)| {
                        count.checked_add(op.maximum_append(array))
                    })
                    .ok_or_else(|| backend("numeric append capacity bound overflows"))?;
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
        if let Some(debit) = proof.phase.as_ref().and_then(|phase| {
            phase.loops.iter().find(|loop_| {
                loop_.header == site
                    && (!phase.checked_children || loop_.header == region.tail.header() as usize)
                    && !version.is_some_and(|v| v.header == site && v.prepaid.is_some())
            })
        }) {
            if site != region.tail.header() as usize {
                let compute = words.read(b, region.words + 1);
                let comparison = words.read(b, region.words + 2);
                let compute = b.ins().iadd_imm(compute, -(debit.compute as i64));
                let comparison = b.ins().iadd_imm(comparison, -(debit.comparison as i64));
                words.write(b, region.words + 1, compute);
                words.write(b, region.words + 2, comparison);
            }
            let count = words.read(b, shadow + debit.counter);
            let count = b.ins().iadd_imm(count, 1);
            words.write(b, shadow + debit.counter, count);
        }
        if let Some(pack) = read_packs.iter().find(|pack| pack.first == site) {
            let complete = super::numeric_read_packs::emit(
                b,
                &words,
                shadow,
                proof,
                pack,
                complete_pack(pack),
                target(pack.next),
            );
            if pack.source_window {
                report.source_guarded_read_packs.push(pack.first);
            }
            report
                .read_pack_sites
                .push((pack.first, pack.last, pack.width));
            if complete {
                report.fully_lowered_read_packs.push(pack.first);
                report.source_instructions(b.func.dfg.num_insts() - before)?;
                continue;
            }
        }
        match proof.action(site) {
            Some(Operation::Fallback) => unreachable!("cold edge emitted above"),
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
                    &mut super::numeric_traversal::Guards {
                        site,
                        facts: traversal,
                        emitted: &mut report.proven_projection_guards,
                    },
                )?;
            }
            Some(Operation::Frame) => {
                super::numeric_words::frame_words(b, &words, shadow, code, site, &proof.frames)?;
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
                    b,
                    controls.options(),
                    &words,
                    region,
                    proof,
                    *array,
                    site,
                    frame,
                    locals,
                    index,
                    incoming,
                    hooks,
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
            Some(Operation::AppendLeBytes {
                array,
                incoming,
                count,
            }) => {
                let count = words.read(b, *count as usize);
                let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, count, 8);
                let checked = b.create_block();
                let bad = b.create_block();
                let changed = b.create_block();
                let store = b.create_block();
                let materialize = b.create_block();
                let done = b.create_block();
                b.set_cold_block(bad);
                b.set_cold_block(materialize);
                boundaries.extend([bad, materialize]);
                b.ins().brif(valid, checked, &[], bad, &[]);
                b.switch_to_block(bad);
                let spilled = boundary::source_failure(
                    b, controls, &words, frame, locals, code, closed, id, site, hooks, error,
                )?;
                report.bounds_error_spill_words.push((site, spilled));
                b.switch_to_block(checked);
                // A zero-count append is the existing same-schema identity.
                // It neither confines an owner nor initializes spare cells.
                b.ins().brif(count, changed, &[], done, &[]);
                b.switch_to_block(changed);
                let incoming = words.read(b, *incoming as usize);
                let private = words.read(b, shadow + proof.private_flag(*array));
                b.ins().brif(private, store, &[], materialize, &[]);
                b.switch_to_block(materialize);
                boundary::materialize(
                    b,
                    controls.options(),
                    &words,
                    region,
                    proof,
                    *array,
                    site,
                    frame,
                    locals,
                    count,
                    incoming,
                    hooks,
                    error,
                )?;
                b.ins().jump(done, &[]);
                b.switch_to_block(store);
                let view = shadow + proof.view(*array);
                let base = words.read(b, view);
                let length = words.read(b, view + 1);
                let address = b.ins().iadd(base, length);
                // The current traversal admits eight spare U8 cells per site.
                // Match the selected leaf's whole-word store: only count bytes
                // become visible, and the exclusive owner hides spare cells.
                b.ins().store(
                    MemFlags::new().with_endianness(cranelift_codegen::ir::Endianness::Little),
                    incoming,
                    address,
                    0,
                );
                let length = b.ins().iadd(length, count);
                words.write(b, view + 1, length);
                words.write(b, view + 3, length);
                b.ins().jump(done, &[]);
                b.switch_to_block(done);
                report.typed_stores.push((site, 8));
                report.append_sites.push((site, 1));
                report.bulk_append_sites.push(site);
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
                if let Operation::Read { array, index, .. } = operation
                    && traversal.read(site, *array, *index)
                {
                    report
                        .proven_scalar_read_guards
                        .push((site, *array, *index));
                } else {
                    let valid = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, length);
                    let good = b.create_block();
                    let bad = b.create_block();
                    b.set_cold_block(bad);
                    boundaries.insert(bad);
                    b.ins().brif(valid, good, &[], bad, &[]);
                    b.switch_to_block(bad);
                    // Preserve the exact original source-failure path whenever
                    // no current selected-view proof justifies this read.
                    let spilled = boundary::source_failure(
                        b, controls, &words, frame, locals, code, closed, id, site, hooks, error,
                    )?;
                    report.bounds_error_spill_words.push((site, spilled));
                    b.switch_to_block(good);
                }
                // Canonical storage is row-major; source linear selection is
                // one-based column-major. A valid ordinal proves rows > 0.
                let rows = words.read(b, view + 2);
                let columns = words.read(b, view + 3);
                let width = proof.arrays[*array].width;
                let Action::LibraryCall { template, .. } =
                    code.blocks()[proof.arrays[*array].witness].action()
                else {
                    return Err(backend("numeric address lost its entry witness"));
                };
                let (address, ty) = super::numeric_words::linear_address(
                    b,
                    ordinal,
                    rows,
                    columns,
                    width,
                    template.inputs()[0].body(),
                    |b| words.read(b, view),
                )?;
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
                            b,
                            controls.options(),
                            &words,
                            region,
                            proof,
                            *array,
                            site,
                            frame,
                            locals,
                            index,
                            incoming,
                            hooks,
                            error,
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
                    let value = if let Some(&known) = traversal.predicates.get(&site) {
                        b.ins().iconst(types::I64, i64::from(known))
                    } else if version.is_some_and(|v| !v.captures.is_empty())
                        && traversal.arithmetic(site, *op, operands)
                    {
                        report.proven_scalar_arithmetic.push(site);
                        primitive(
                            b,
                            match op {
                                Op::U64Add => Op::U64WrappingAdd,
                                Op::U64Sub => Op::U64WrappingSub,
                                _ => return Err(backend("proved numeric arithmetic differs")),
                            },
                            &args,
                            fail,
                        )
                    } else {
                        primitive(b, *op, &args, fail)
                    };
                    words.write(b, output.get() as usize, value);
                    report.scalar_operations += 1;
                }
                _ => return Err(backend("numeric proof contains unsupported action")),
            },
        }
        if let Some(window) = versions.iter().find(|v| v.initialize == site) {
            // Initialize just copied this child's current source arguments.
            // Re-admit it even when its caller is another selected version;
            // caller facts never become permissions for the child.
            report
                .version_entry_gates
                .push((site, window.header, version.map(|v| v.header)));
            window.gate(
                b,
                &words,
                shadow,
                proof,
                region.words,
                target(window.header as u32),
            );
        } else {
            terminators::emit_source_terminator(
                b,
                &words,
                code,
                traversal
                    .successors
                    .get(&site)
                    .or_else(|| boolean_successors.get(&site))
                    .map_or(code.blocks()[site].terminator(), |&next| {
                        mech_engine::native_artifact::NativeIterationTerminator::Scalar(
                            mech_engine::native_artifact::NativeMatchTerminator::Jump(next),
                        )
                    }),
                fail,
                target,
            );
        }
        b.switch_to_block(fail);
        let status = b.block_params(fail)[0];
        set_origin(b, frame, site as u64);
        b.ins().jump(failed, &[status.into()]);
        report.source_instructions(b.func.dfg.num_insts() - before)?;
    }
    let boundary = b.func.dfg.num_insts();
    b.switch_to_block(bailout);
    let count = snapshots::spill(
        b,
        &words,
        locals,
        code,
        region,
        1,
        None,
        &proof.boundary_values.header,
        &slots,
    )?;
    report
        .boundary_spill_words
        .push(("header", region.tail.header() as usize, count));
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
    let count = snapshots::spill(
        b,
        &words,
        locals,
        code,
        region,
        0,
        None,
        &proof.boundary_values.exit,
        &slots,
    )?;
    report
        .boundary_spill_words
        .push(("exit", region.tail.exit() as usize, count));
    b.ins()
        .jump(ordinary_blocks[region.tail.exit() as usize].unwrap(), &[]);
    b.switch_to_block(failed);
    let status = b.block_params(failed)[0];
    // The common error arm settles/finalizes and terminates; it cannot resume
    // ordinary source code. Preserve exactly its existing abort consumers.
    let count = snapshots::spill(b, &words, locals, code, region, 3, None, &[], &slots)?;
    report
        .boundary_spill_words
        .push(("failure", region.tail.header() as usize, count));
    b.ins().jump(error, &[status.into()]);
    report.fixed_template_instructions = setup_work + b.func.dfg.num_insts() - boundary;
    if report.fixed_template_instructions
        > proof
            .phase
            .as_ref()
            .map_or(FIXED_OVERHEAD, |p| p.boundary_work) as usize
    {
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
