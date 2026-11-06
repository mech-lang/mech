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
    tail::{Layout, OVERHEAD},
};
use crate::native_artifact::{NativeCompileError, abi::STATUS_ABI};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeMatchAction as Scalar};
use std::collections::{BTreeMap, BTreeSet};
#[path = "numeric_boundary.rs"]
mod boundary;
#[path = "numeric_outlined.rs"]
pub(in crate::native_artifact) mod outlined;

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
    pub scalar_input_loads: usize,
    pub scalar_output_stores: usize,
    pub entry_shadow_argument_copies: usize,
    pub private_boundary_words: usize,
    pub source_home_words: usize,
    pub terminal_scalar_words_omitted: usize,
    pub runtime_scalar_path_walks: usize,
    pub tree_nodes: Vec<usize>,
    pub projected_child_debit_nodes: usize,
    pub fuel_derived_child_debit_nodes: usize,
    pub direct_child_edges: Vec<(usize, usize, usize)>,
    // Compiler-local SSA facts, regenerated from checked typed-view recipes.
    // They are never serialized or accepted from a descriptor/image caller.
    #[serde(skip)]
    pub native_extent_bounds: Vec<(Value, u64)>,
}

/// A terminal error never resumes a source consumer. Mode3 still reconciles
/// every owner/view, active-path flag and independent debit, but does not export
/// private scalar payload. Derive payload homes from actual typed bindings,
/// rather than treating an arbitrary scratch-word range as discardable.
fn terminal_scalars(
    proof: &Proof,
    tree: Option<&crate::native_artifact::program::prepare::numeric_tree::Proof>,
) -> Result<BTreeSet<usize>, NativeCompileError> {
    let mut words = proof
        .frames
        .iter()
        .flat_map(|frame| frame.scalar_slots())
        .collect::<BTreeSet<_>>();
    words.extend(
        proof
            .scalar_inputs
            .iter()
            .chain(&proof.scalar_outputs)
            .chain(&proof.scalar_captures)
            .map(|binding| binding.word),
    );
    words.extend(
        proof
            .cold_scalars
            .iter()
            .flat_map(|(_, bindings)| bindings.iter().map(|binding| binding.word)),
    );
    if let Some(tree) = tree {
        words.extend(
            tree.nodes
                .iter()
                .flat_map(|node| node.scalar_bindings.iter().map(|binding| binding.word)),
        );
        if words
            .iter()
            .any(|word| (tree.counters..=tree.bank()).contains(word))
        {
            return Err(backend(
                "terminal scalar payload overlaps numeric tree accounting",
            ));
        }
    }
    if words.iter().any(|word| *word >= proof.words) {
        return Err(backend(
            "terminal scalar payload exceeds its checked private bank",
        ));
    }
    Ok(words)
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
    user_context: Value,
    outlined_function: Option<cranelift_codegen::ir::FuncRef>,
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
    if region.numeric_tree.is_some() && !controls.enabled(OptimizationPass::NumericTrees) {
        return Ok(());
    }
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
    if let Some(function) = outlined_function {
        outlined::call(
            b,
            function,
            frame,
            locals,
            user_context,
            region,
            ordinary_blocks,
            ordinary,
            error,
        )?;
        b.switch_to_block(ordinary);
        return Ok(());
    }
    let shadow = region
        .shadow
        .ok_or_else(|| backend("numeric region lacks scratch home"))?;
    let slots = slots(code, proof, region.words, shadow)?;
    let register_words = slots.len();
    if register_words > proof.boundary_words {
        return Err(backend(
            "numeric backend homes exceed independent boundary inventory",
        ));
    }
    let bank = if proof.scalar_inputs.is_empty()
        && proof.scalar_captures.is_empty()
        && region.numeric_tree.is_none()
    {
        b.ins().iconst(types::I64, 0)
    } else {
        b.ins()
            .load(types::I64, MemFlags::new(), hooks.arguments, 16)
    };
    let mut words = Words::new_with(b, locals, code, slots, |b, slot| {
        if !(shadow..shadow + proof.words).contains(&slot) {
            return Ok(None);
        }
        let relative = slot - shadow;
        if let Some(tree) = &region.numeric_tree {
            if relative == tree.bank() {
                return Ok(Some(bank));
            }
            if relative == tree.active(0) {
                return Ok(Some(b.ins().iconst(types::I64, 1)));
            }
        }
        if let Some(binding) = proof
            .scalar_inputs
            .iter()
            .chain(&proof.scalar_captures)
            .find(|binding| binding.word == relative)
        {
            let at = binding
                .physical
                .checked_mul(2)
                .ok_or_else(|| backend("numeric scalar import offset overflows"))?;
            return Ok(Some(b.ins().load(
                types::I64,
                MemFlags::new(),
                bank,
                offset(at)?,
            )));
        }
        let metadata_input = proof.arrays.iter().enumerate().any(|(array, _)| {
            (proof.view(array)..proof.view(array) + 4).contains(&relative)
                || proof.capacity(array) == Some(relative)
        });
        if metadata_input {
            Ok(Some(b.ins().load(
                types::I64,
                MemFlags::new(),
                hooks.arguments,
                offset(2 + relative)?,
            )))
        } else {
            Ok(Some(b.ins().iconst(types::I64, 0)))
        }
    })?;
    for (at, array) in proof.arrays.iter().enumerate() {
        if matches!(array.width, 1 | 2 | 4 | 8) {
            // Runtime unsigned_matrix_native_view validates the actual packed
            // slice and retained owner. Rust slices/Vec allocations cannot
            // exceed isize::MAX bytes; appends additionally retain the checked
            // fixed physical capacity. Never infer this for rows/columns:
            // a zero-element matrix can have an arbitrarily large empty axis.
            let upper = i64::MAX as u64 / u64::from(array.width);
            words.bound_native_extent(shadow + proof.view(at) + 1, upper);
            if let Some(capacity) = proof.capacity(at) {
                words.bound_native_extent(shadow + capacity, upper);
            }
        }
    }
    let fuel_debits = region
        .numeric_tree
        .as_ref()
        .filter(|tree| {
            controls.enabled(OptimizationPass::FuelDerivedNumericDebits)
                && controls.enabled(OptimizationPass::ProjectedNumericDebits)
                && tree.projected_child_debits()
        })
        .map(|tree| {
            super::numeric_fuel_debits::Anchors::new(b, &words, region.words, shadow, tree)
        });
    // Every ordinary scalar operand home is retained conservatively. Private
    // virtual-frame intermediates have no cold consumer: reconciliation reads
    // only current carrier/result leaves and exact view/flag/capacity lanes.
    let source_homes = super::numeric_words::source_slots(
        code,
        proof.sites.iter().copied(),
        &proof.actions,
        core::iter::empty(),
    )?;
    let liveness = controls
        .enabled(OptimizationPass::SourceLiveness)
        .then(|| super::source_liveness::Liveness::derive(program, &source_homes))
        .transpose()
        .map_err(backend)?;
    let mut private_live = BTreeSet::new();
    private_live.extend(proof.scalar_inputs.iter().map(|binding| binding.word));
    private_live.extend(proof.scalar_outputs.iter().map(|binding| binding.word));
    private_live.extend(
        proof
            .cold_scalars
            .iter()
            .flat_map(|(_, bindings)| bindings.iter().map(|binding| binding.word)),
    );
    if let Some(tree) = &region.numeric_tree {
        private_live.extend(
            tree.nodes
                .iter()
                .flat_map(|node| node.scalar_bindings.iter().map(|binding| binding.word)),
        );
    }
    private_live.extend(proof.scalar_words..proof.words);
    let exact_private = if controls.enabled(OptimizationPass::PrivateLiveness) {
        proof
            .sites
            .iter()
            .copied()
            .chain([region.tail.exit() as usize])
            .map(|site| {
                Ok((
                    site,
                    super::private_liveness::at(
                        proof,
                        region.numeric_tree.as_ref(),
                        site,
                        region.tail.exit() as usize,
                        controls.enabled(OptimizationPass::ProjectedNumericDebits),
                    )?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, NativeCompileError>>()?
    } else {
        BTreeMap::new()
    };
    let live_at = |site| {
        let source = liveness
            .as_ref()
            .map_or(&source_homes, |live| live.at(site));
        let private = exact_private.get(&site).unwrap_or(&private_live);
        source
            .iter()
            .copied()
            .chain(region.words..region.words + 3)
            .chain(private.iter().map(|word| shadow + word))
            .collect::<BTreeSet<_>>()
    };
    let terminal_scalars = if controls.enabled(OptimizationPass::PrivateLiveness) {
        terminal_scalars(proof, region.numeric_tree.as_ref())?
    } else {
        BTreeSet::new()
    };
    let setup_work = b.func.dfg.num_insts() - setup_start;
    let exit = b.create_block();
    let bailouts = if let Some(tree) = &region.numeric_tree {
        tree.nodes
            .iter()
            .map(|node| (node.tail.header() as usize, b.create_block()))
            .collect::<BTreeMap<_, _>>()
    } else {
        BTreeMap::from([(region.tail.header() as usize, b.create_block())])
    };
    let failed = b.create_block();
    b.append_block_param(failed, types::I64);
    b.set_cold_block(failed);
    let mut boundaries = BTreeSet::from([exit, failed]);
    boundaries.extend(bailouts.values().copied());
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
        native_extent_bounds: Vec::new(),
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
        scalar_input_loads: proof.scalar_inputs.len() + proof.scalar_captures.len(),
        scalar_output_stores: (proof.scalar_inputs.len() + proof.scalar_outputs.len()) * 2,
        entry_shadow_argument_copies: 0,
        private_boundary_words: exact_private
            .values()
            .map(BTreeSet::len)
            .max()
            .unwrap_or(private_live.len()),
        source_home_words: source_homes.len(),
        terminal_scalar_words_omitted: 0,
        runtime_scalar_path_walks: 0,
        tree_nodes: region.numeric_tree.as_ref().map_or_else(Vec::new, |tree| {
            tree.nodes
                .iter()
                .map(|n| n.tail.header() as usize)
                .collect()
        }),
        projected_child_debit_nodes: region.numeric_tree.as_ref().map_or(0, |tree| {
            if controls.enabled(OptimizationPass::ProjectedNumericDebits)
                && tree.projected_child_debits()
            {
                tree.nodes.len() - 1
            } else {
                0
            }
        }),
        fuel_derived_child_debit_nodes: region.numeric_tree.as_ref().map_or(0, |tree| {
            if fuel_debits.is_some() {
                tree.nodes.len() - 1
            } else {
                0
            }
        }),
        direct_child_edges: region.numeric_tree.as_ref().map_or_else(Vec::new, |tree| {
            tree.nodes
                .iter()
                .skip(1)
                .map(|n| {
                    (
                        n.tail.initialize() as usize,
                        n.tail.header() as usize,
                        n.tail.exit() as usize,
                    )
                })
                .collect()
        }),
    };
    for &site in &proof.sites {
        b.switch_to_block(target(site as u32));
        let fail = b.create_block();
        b.append_block_param(fail, types::I64);
        b.set_cold_block(fail);
        boundaries.insert(fail);
        let before = b.func.dfg.num_insts();
        let tree_node = region.numeric_tree.as_ref().and_then(|tree| {
            tree.nodes
                .iter()
                .position(|node| node.tail.header() as usize == site)
        });
        if site == region.tail.header() as usize || tree_node.is_some() {
            let (compute_cost, comparison_cost) =
                tree_node.map_or((region.compute, region.comparison), |node| {
                    let n = &region.numeric_tree.as_ref().unwrap().nodes[node];
                    (n.compute, n.comparison)
                });
            let compute = words.read(b, region.words + 1);
            let comparison = words.read(b, region.words + 2);
            let a = b.ins().icmp_imm(
                IntCC::UnsignedGreaterThanOrEqual,
                compute,
                compute_cost as i64,
            );
            let c = b.ins().icmp_imm(
                IntCC::UnsignedGreaterThanOrEqual,
                comparison,
                comparison_cost as i64,
            );
            let mut enough = b.ins().band(a, c);
            // Capacity is checked before ANY source operation in this visit.
            // A tree frontier ends at its next header check and includes any
            // ancestor continuation after a child returns. Leaves retain their
            // original conservative whole-traversal site count.
            for (array, _) in proof.arrays.iter().enumerate().filter(|(_, a)| a.appends) {
                let count = if region.numeric_tree.is_some() {
                    proof
                        .append_limit(site, array)
                        .ok_or_else(|| backend("numeric header lost its append frontier"))?
                } else {
                    proof
                        .actions
                        .iter()
                        .filter(|(_, op)| {
                            matches!(op, Operation::Append { array: at, .. } if *at == array)
                        })
                        .count()
                };
                // Entry lends a real Vec prefix whose length is within its
                // capacity. Every append-capable frontier still checks its
                // complete advance before executing source operations, and a
                // cold first change reloads a newly validated reserved view.
                // A zero frontier cannot advance this prefix before the next
                // header, so it does not need to re-prove the invariant.
                if count == 0 && controls.enabled(OptimizationPass::ZeroAppendFrontiers) {
                    continue;
                }
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
            b.ins().brif(enough, funded, &[], bailouts[&site], &[]);
            b.switch_to_block(funded);
            let cost = b.ins().iconst(types::I64, compute_cost as i64);
            let compute = b.ins().isub(compute, cost);
            let comparison = b.ins().iadd_imm(comparison, -(comparison_cost as i64));
            words.write(b, region.words + 1, compute);
            words.write(b, region.words + 2, comparison);
            if let Some(node) = tree_node {
                super::numeric_tree::debit(
                    b,
                    &words,
                    shadow,
                    region.numeric_tree.as_ref().unwrap(),
                    node,
                    controls.enabled(OptimizationPass::ProjectedNumericDebits),
                    fuel_debits.is_some(),
                );
            }
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
                    controls,
                    b,
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
                    fuel_debits.as_ref(),
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
                words.spill_live(b, locals, &live_at(site))?;
                // Ordinary source sites retain their deepest independent leaf
                // proof for a declined outer tree. This generated path owns
                // the known root loan, so reconcile that exact region instead
                // of looking up the ordinary site's shadowed leaf region.
                let source_site = b.ins().iconst(types::I64, site as i64);
                tail::leave(
                    controls.options(),
                    b,
                    locals,
                    region,
                    source_site,
                    2,
                    hooks,
                    error,
                )?;
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
                            controls,
                            b,
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
                            fuel_debits.as_ref(),
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
        if let Some(tree) = &region.numeric_tree {
            super::numeric_tree::control(b, &words, shadow, tree, site)?;
        }
        terminators::emit_source_terminator(
            b,
            &words,
            code,
            site,
            controls.options(),
            code.blocks()[site].terminator(),
            fail,
            target,
        );
        b.switch_to_block(fail);
        let status = b.block_params(fail)[0];
        set_origin(b, frame, site as u64);
        let mut live = live_at(site);
        let live_before = live.len();
        for word in &terminal_scalars {
            live.remove(&(shadow + word));
        }
        report.terminal_scalar_words_omitted = report
            .terminal_scalar_words_omitted
            .max(live_before - live.len());
        if let Some(anchors) = &fuel_debits {
            anchors.publish(b, &words);
        }
        words.spill_live(b, locals, &live)?;
        b.ins().jump(failed, &[status.into()]);
        let instructions = b.func.dfg.num_insts() - before;
        report.maximum_source_instructions = report.maximum_source_instructions.max(instructions);
        // Cold synchronization is covered by the fixed boundary envelope.
        if instructions > OVERHEAD as usize + register_words * 2 {
            return Err(backend("numeric leaf exceeds finite source envelope"));
        }
    }
    let boundary = b.func.dfg.num_insts();
    for (&site, &bailout) in &bailouts {
        b.switch_to_block(bailout);
        if let Some(anchors) = &fuel_debits {
            anchors.publish(b, &words);
        }
        words.spill_live(b, locals, &live_at(site))?;
        let header = b.ins().iconst(types::I64, site as i64);
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
        if site == region.tail.header() as usize {
            b.ins().jump(ordinary, &[]);
        } else {
            b.ins().jump(
                ordinary_blocks[site]
                    .ok_or_else(|| backend("numeric tree child header is not emitted"))?,
                &[],
            );
        }
    }
    b.switch_to_block(exit);
    if let Some(anchors) = &fuel_debits {
        anchors.publish(b, &words);
    }
    words.spill_live(b, locals, &live_at(region.tail.exit() as usize))?;
    b.ins()
        .jump(ordinary_blocks[region.tail.exit() as usize].unwrap(), &[]);
    b.switch_to_block(failed);
    let status = b.block_params(failed)[0];
    // Each source-specific failure already synchronized its exact live homes.
    b.ins().jump(error, &[status.into()]);
    report.fixed_template_instructions = setup_work + b.func.dfg.num_insts() - boundary;
    if report.fixed_template_instructions as u64 > proof.boundary_work {
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
    report.native_extent_bounds = words.native_extent_bounds();
    if proof.writes() {
        reports.numeric_write_loops.push(report);
    } else {
        reports.readonly_numeric_loops.push(report);
    }
    b.switch_to_block(ordinary);
    Ok(())
}
