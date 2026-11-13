//! Native numeric specialization for the compact execution contract.
//! The current canonical proof feeds shared scalar, typed-array and vector
//! instruction emission directly. No managed runtime adapter participates.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::{aggregates, terminators, word_access::SsaWords};
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationPass};
use crate::native_artifact::compiler::{backend, primitive};
use crate::native_artifact::program::prepare::{
    readonly_numeric::Operation,
    tail::{OVERHEAD, Region},
};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeMatchAction as Scalar, NativeScalarOp as Op,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct Report {
    pub header: usize,
    #[serde(skip)]
    pub machine_header: Block,
    #[serde(skip)]
    pub reduction_completions: Vec<Block>,
    pub parent: Option<u32>,
    pub source_sites: Vec<usize>,
    pub typed_loads: Vec<(usize, u8)>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reused_scalar_reads: Vec<(usize, usize, usize)>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reused_scalar_operands: Vec<(usize, usize, usize, usize)>,
    pub read_pack_sites: Vec<(usize, usize, u8)>,
    pub source_guarded_read_packs: Vec<usize>,
    pub fully_lowered_read_packs: Vec<usize>,
    pub hoisted_pack_addresses: Vec<(usize, usize, usize, usize)>,
    pub proven_projection_guards: Vec<(usize, u32, usize, u32)>,
    pub proven_boolean_successors: Vec<(usize, u32)>,
    pub proven_scalar_read_guards: Vec<(usize, usize, u32)>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub proven_scalar_write_guards: Vec<(usize, usize, u32)>,
    pub proven_scalar_arithmetic: Vec<usize>,
    pub proven_changed_writes: Vec<(usize, usize, u32, u32, u8)>,
    pub private_write_sites: Vec<(usize, usize, u8)>,
    pub guarded_loop_windows: Vec<super::numeric_loop_windows::Report>,
    #[serde(skip)]
    pub compact_word_loops: Vec<super::wide_equal::Certificate>,
    pub phase_entry_bounds: Vec<(usize, u64)>,
    pub version_entry_gates: Vec<(usize, usize, Option<usize>)>,
    proven_child_entry_checks: Vec<(usize, super::traversal_facts::Check)>,
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

pub(in crate::native_artifact) fn emit(
    controls: &CompilationControls,
    b: &mut FunctionBuilder<'_>,
    code: CodePlan<'_>,
    region: &Region,
    native: &super::numeric_minimal::Boundary<'_>,
    vectors: Option<&crate::native_artifact::program::compact::VectorContract>,
    private: Option<&crate::native_artifact::program::compact::PrivatePhase>,
    no_growth: Option<&crate::native_artifact::program::compact::NoGrowth>,
    done: Block,
    error: Block,
    countdown: bool,
) -> Result<Report, NativeCompileError> {
    let proof = region
        .numeric
        .as_ref()
        .ok_or_else(|| backend("compact numeric proof missing"))?;
    let CodePlan::Program(program) = code else {
        return Err(backend("compact native code requires canonical Program"));
    };
    let site = region.tail.header() as usize;
    let carry_item =
        no_growth.is_some() && std::env::var_os("MECH_DIRECT_ITEM_NO_GROWTH").is_some();
    let setup_start = b.func.dfg.num_insts();
    let shadow = region
        .shadow
        .ok_or_else(|| backend("numeric region lacks scratch home"))?;
    let slots = super::readonly_numeric::slots(code, proof, region.words, shadow)?;
    let register_words = slots.len();
    let fresh = super::readonly_numeric::fresh_words(code, proof, shadow, &slots)?;
    let words = native.words(b, code, proof, site, slots.clone())?;
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
    // Packing can bypass every original scalar definition in these regions.
    // Source confinement does not account for uses introduced by read/operand
    // reuse. Do not publish fallback-only CLIF values into the snapshot cache:
    // they would not dominate a later use reached through the packed arm.
    // The ordinary source read/check remains when no safe snapshot is cached.
    let pack_snapshot_sites = read_packs
        .iter()
        .flat_map(|pack| pack.sites())
        .collect::<BTreeSet<_>>();
    let boolean_successors =
        super::numeric_branches::derive(&program.metadata().plan.facts(), proof);
    let versions = super::numeric_loop_windows::derive(
        code,
        proof,
        shadow,
        b,
        &read_packs,
        false && controls.enabled(OptimizationPass::ChildFunding),
    );
    let checks = versions
        .iter()
        .map(|v| (v.initialize, v.queries(proof)))
        .collect();
    let initial = super::numeric_traversal::derive(code, proof, shadow, &checks, &[], true);
    let terminal_fallbacks = private.is_some_and(|contract| {
        if std::env::var_os("MECH_PHASE_ASSERTION_FLOORS").is_none() || !contract.permits(proof) {
            return false;
        }
        let handoffs = contract.exports(shadow);
        proof
            .actions
            .iter()
            .all(|(at, op)| !matches!(op, Operation::Fallback) || handoffs.contains_key(at))
    });
    let phase_entry_bounds =
        super::numeric_phase_bounds::derive(code, proof, &versions, &initial, terminal_fallbacks);
    let mut phase_indices = super::numeric_phase_bounds::Indices::new(code);
    let mut traversal = if phase_entry_bounds.is_empty() {
        initial
    } else {
        super::numeric_traversal::derive(code, proof, shadow, &checks, &phase_entry_bounds, true)
    };
    traversal
        .flat_arrays
        .extend(phase_entry_bounds.iter().map(|&(array, _)| array));
    let vector_contract = vectors;
    if let Some(contract) = vector_contract {
        // The same source plan executes only after the private caller's actual
        // current-view vector guard. Indexed writes preserve geometry; append
        // entry requires row1 and refreshes backing through the existing path.
        traversal.flat_arrays.extend(contract.arrays());
    }
    b.ins().jump(target(region.tail.header()), &[]);
    let mut report = Report {
        header: site,
        machine_header: target(region.tail.header()),
        reduction_completions: Vec::new(),
        parent: region.tail.parent(),
        source_sites: proof.sites.clone(),
        typed_loads: Vec::new(),
        reused_scalar_reads: Vec::new(),
        reused_scalar_operands: Vec::new(),
        read_pack_sites: Vec::new(),
        source_guarded_read_packs: Vec::new(),
        fully_lowered_read_packs: Vec::new(),
        hoisted_pack_addresses: Vec::new(),
        proven_boolean_successors: boolean_successors
            .iter()
            .map(|(&site, &next)| (site, next))
            .collect(),
        proven_projection_guards: Vec::new(),
        proven_scalar_read_guards: Vec::new(),
        proven_scalar_write_guards: Vec::new(),
        proven_scalar_arithmetic: Vec::new(),
        proven_changed_writes: Vec::new(),
        private_write_sites: Vec::new(),
        guarded_loop_windows: versions.iter().map(|v| v.report(proof)).collect(),
        compact_word_loops: {
            versions
                .iter()
                .filter_map(|v| {
                    let cert = super::wide_equal::certify(v, proof, &read_packs)?;
                    Some(
                        if std::env::var_os("MECH_REUSE_SHIFTED_EQUALITY").is_some() {
                            cert.with_reuse_scope(code, proof, v, &blocks)
                        } else {
                            cert
                        },
                    )
                })
                .collect()
        },
        phase_entry_bounds,
        version_entry_gates: Vec::new(),
        proven_child_entry_checks: Vec::new(),
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
    let mut read_snapshots = BTreeMap::new();
    for (version, site) in emitted_sites {
        let complete_pack = |pack: &super::numeric_read_packs::Pack| {
            let flat_at_entry = vector_contract.is_some_and(|contract| {
                let view = &proof.arrays[pack.array];
                !view.writable
                    && !view.appends
                    && contract.arrays().any(|array| array == pack.array)
            });
            // The caller checked this immutable view before choosing the
            // vector body. The reached source guard still proves readability.
            pack.source_window
                && version.is_none_or(|v| pack.confined_to(&v.blocks))
                && (flat_at_entry
                    || version.is_some_and(|v| v.facts.flat_arrays.contains(&pack.array)))
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
            if let Some(count) = native.handoff(b, &words, site)? {
                report
                    .boundary_spill_words
                    .push(("private-cold", site, count));
                continue;
            }
            return Err(backend("compact phase contains an unexported continuation"));
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

        if let Some(pack) = read_packs.iter().find(|pack| pack.first == site) {
            let address = version
                .and_then(|v| v.addresses.iter().find(|address| address.site == site))
                .filter(|_| complete_pack(pack))
                .map(|address| {
                    report.hoisted_pack_addresses.push(address.report());
                    address.emit(b, &words)
                });
            let complete = super::numeric_read_packs::emit(
                b,
                controls.options(),
                &words,
                shadow,
                proof,
                pack,
                complete_pack(pack),
                address,
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
        // Snapshot only an operand already read by its original scalar action.
        // Keep each copied version separate; no value crosses a source latch.
        if !matches!(proof.action(site), Some(Operation::Read { .. })) {
            for snapshot in traversal
                .snapshots_at(site)
                .into_iter()
                .filter(|snapshot| !pack_snapshot_sites.contains(&snapshot.site))
            {
                let value = traversal
                    .reused_operand(site, snapshot.word)
                    .and_then(|from| read_snapshots.get(&(version.map(|v| v.header), from)))
                    .copied()
                    .unwrap_or_else(|| words.read(b, snapshot.word));
                read_snapshots.insert((version.map(|v| v.header), snapshot), value);
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
                let store = b.create_block();
                let materialize = b.create_block();
                let done = b.create_block();
                b.set_cold_block(materialize);
                boundaries.insert(materialize);

                native.capacity(b, &words, proof, *array, 1, fail, &mut boundaries)?;
                b.ins().jump(store, &[]);

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
                native.reject(b, super::numeric_minimal::BOUNDS, fail);
                let spilled = 0;
                report.bounds_error_spill_words.push((site, spilled));
                b.switch_to_block(checked);
                // A zero-count append is the existing same-schema identity.
                // It neither confines an owner nor initializes spare cells.
                b.ins().brif(count, changed, &[], done, &[]);
                b.switch_to_block(changed);
                let incoming = words.read(b, *incoming as usize);

                native.capacity(b, &words, proof, *array, 8, fail, &mut boundaries)?;
                b.ins().jump(store, &[]);

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
                    && (traversal.read(site, *array, *index)
                        || (std::env::var_os("MECH_PHASE_ASSERTION_FLOORS").is_some()
                            && report.phase_entry_bounds.iter().any(|&(at, minimum)| {
                                at == *array && phase_indices.proves(*index, minimum)
                            })))
                {
                    report
                        .proven_scalar_read_guards
                        .push((site, *array, *index));
                } else if let Operation::Write { array, index, .. } = operation
                    && std::env::var_os("MECH_TRAVERSAL_WRITE_BOUNDS").is_some()
                    && (traversal.write(site, *array, *index)
                        || (std::env::var_os("MECH_PHASE_ASSERTION_FLOORS").is_some()
                            && report.phase_entry_bounds.iter().any(|&(at, minimum)| {
                                at == *array && phase_indices.proves(*index, minimum)
                            })))
                {
                    report
                        .proven_scalar_write_guards
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
                    native.reject(b, super::numeric_minimal::BOUNDS, fail);
                    let spilled = 0;
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
                let vector_entry = vectors
                    .is_some_and(|contract| contract.arrays().any(|candidate| candidate == *array));
                let (address, ty) = if vector_entry {
                    // The opaque contract is constructed only by the caller
                    // that guards this exact current view. It holds throughout
                    // child reductions; append guards require one row and the
                    // existing capacity path refreshes every backing pointer.
                    super::numeric_words::element_address(b, ordinal, width, |b| {
                        words.read(b, view)
                    })?
                } else {
                    super::numeric_words::linear_address(
                        b,
                        ordinal,
                        rows,
                        columns,
                        width,
                        template.inputs()[0].body(),
                        |b| words.read(b, view),
                    )?
                };
                match operation {
                    Operation::Read { output, .. } => {
                        // The original current-view bounds check and address
                        // validation above remain before any value substitution.
                        let reused = traversal.reused_read(site).and_then(|snapshot| {
                            read_snapshots
                                .get(&(version.map(|v| v.header), snapshot))
                                .copied()
                                .map(|value| (snapshot, value))
                        });
                        let value = if let Some((snapshot, value)) = reused {
                            report
                                .reused_scalar_reads
                                .push((site, snapshot.site, snapshot.word));
                            value
                        } else {
                            let value = super::numeric_words::load_unsigned_word(
                                b,
                                ty,
                                MemFlags::new(),
                                address,
                                0,
                            );
                            report.typed_loads.push((site, width));
                            value
                        };
                        words.write(b, *output as usize, value);

                        for snapshot in traversal
                            .snapshots_at(site)
                            .into_iter()
                            .filter(|snapshot| !pack_snapshot_sites.contains(&snapshot.site))
                        {
                            read_snapshots.insert((version.map(|v| v.header), snapshot), value);
                        }
                    }
                    Operation::Write {
                        array,
                        index: selector,
                        incoming,
                    } => {
                        let always_changed =
                            traversal.changes(site, *array, *selector, *incoming, width);
                        if always_changed {
                            report
                                .proven_changed_writes
                                .push((site, *array, *selector, *incoming, width));
                        }
                        let incoming = words.read(b, *incoming as usize);
                        let done = b.create_block();
                        let store = b.create_block();

                        // The existing compact/private entry admits writable
                        // cells. Equality has no ownership or publication
                        // consequence once this exact view is exclusive.
                        b.ins().jump(store, &[]);

                        b.switch_to_block(store);
                        let narrowed = if width < 8 {
                            b.ins().ireduce(ty, incoming)
                        } else {
                            incoming
                        };
                        b.ins().store(MemFlags::new(), narrowed, address, 0);
                        b.ins().jump(done, &[]);
                        b.switch_to_block(done);
                        report.private_write_sites.push((site, *array, width));
                        report.typed_stores.push((site, width));
                        report.first_change_boundaries.push(site);
                    }
                    _ => unreachable!("selected numeric access"),
                }
            }
            None => match code.blocks()[site].action() {
                Action::Scalar(Scalar::None) => {}
                Action::Scalar(Scalar::Copy { source, output }) => {
                    let word = source.get() as usize;
                    let reused = traversal.reused_operand(site, word).and_then(|from| {
                        read_snapshots
                            .get(&(version.map(|v| v.header), from))
                            .copied()
                            .map(|value| (from, value))
                    });
                    let value = if let Some((from, value)) = reused {
                        report
                            .reused_scalar_operands
                            .push((site, word, from.site, from.word));
                        value
                    } else {
                        words.read(b, word)
                    };
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
                        .map(|v| {
                            let word = v.get() as usize;
                            let reused = traversal.reused_operand(site, word).and_then(|from| {
                                read_snapshots
                                    .get(&(version.map(|v| v.header), from))
                                    .copied()
                                    .map(|value| (from, value))
                            });
                            if let Some((from, value)) = reused {
                                report
                                    .reused_scalar_operands
                                    .push((site, word, from.site, from.word));
                                value
                            } else {
                                words.read(b, word)
                            }
                        })
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
            // Current source facts may establish particular predicates. All
            // remaining checks, view admission and funding still happen here;
            // caller facts never become ownership permission for the child.
            report
                .version_entry_gates
                .push((site, window.header, version.map(|v| v.header)));
            report.proven_child_entry_checks.extend(
                window
                    .queries(proof)
                    .into_iter()
                    .filter(|check| traversal.check(site, *check))
                    .map(|check| (site, check)),
            );
            if let Some(completed) = window.gate(
                b,
                &words,
                shadow,
                proof,
                traversal,
                region.words,
                target(window.header as u32),
                false,
                carry_item,
                countdown,
            ) {
                report.reduction_completions.push(completed);
            }
        } else {
            terminators::emit_source_terminator(
                b,
                &words,
                code,
                site,
                controls.options(),
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

        native.origin(b, site)?;

        b.ins().jump(failed, &[status.into()]);
        report.source_instructions(b.func.dfg.num_insts() - before)?;
    }
    let boundary = b.func.dfg.num_insts();
    b.switch_to_block(bailout);

    native.origin(b, site)?;
    native.reject(b, super::numeric_minimal::UNSUPPORTED, error);

    b.switch_to_block(exit);

    native.finish(b, &words, proof)?;

    b.ins().jump(done, &[]);
    b.switch_to_block(failed);
    let status = b.block_params(failed)[0];
    // The common error arm settles/finalizes and terminates; it cannot resume
    // ordinary source code. Preserve exactly its existing abort consumers.
    let count = 0;
    report
        .boundary_spill_words
        .push(("failure", region.tail.header() as usize, count));
    b.ins().jump(error, &[status.into()]);
    report.fixed_template_instructions = setup_work + b.func.dfg.num_insts() - boundary;
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
    Ok(report)
}
