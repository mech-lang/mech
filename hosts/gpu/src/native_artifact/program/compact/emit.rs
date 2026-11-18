//! Native typed CFG using the shared scalar, aggregate and branch emitters.
use super::{
    arrays::{self, Arrays},
    checked_traversals,
    collections::Collections,
    constants::{self, Initial},
    flow::Flow,
    guards::Guards,
    layout::Layout,
    owner_flow::OwnerFlow,
    packed, phase_storage, phases,
    recipes::{self, Recipe},
    terminal_appends::TerminalAppends,
    vector_guard_facts, widening_reduction, word_collections, word_packs,
};
use crate::native_artifact::codegen::{
    common::{
        aggregates::{self, CheckedGuards},
        ssa_words::Words,
        terminators::emit_source_terminator,
        word_access::SsaWords,
    },
    control::plan::CodePlan,
    options::{CompilationControls, OptimizationOptions},
};
use crate::native_artifact::program::{codegen::cfg_cleanup, prepare::PreparedNativeProgram};
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, compiled_stack_bytes, guard, offset, primitive},
};
use cranelift_codegen::ir::{AbiParam, Block, InstBuilder, MemFlags, types};
use cranelift_codegen::{Context, ir::condcodes::IntCC};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module, ModuleResult};
use mech_engine::native_artifact::{
    NativeControlLoop as Loop, NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeScalarAggregateAction as Aggregate,
};
use std::collections::{BTreeMap, BTreeSet};

const BOUNDS: u64 = arrays::BOUNDS;
// Seal a real join when all incoming edges are known. Delaying this across the
// entire function creates incomplete phis through every intervening block.
fn seal_auxiliary(
    b: &mut FunctionBuilder<'_>,
    source: &BTreeSet<Block>,
    entry: Block,
    sealed_error: Option<Block>,
    last: &mut Option<Block>,
) {
    let mut next = match *last {
        Some(block) => b.func.layout.next_block(block),
        None => b.func.layout.blocks().next(),
    };
    while let Some(block) = next {
        if block != entry && Some(block) != sealed_error && !source.contains(&block) {
            b.seal_block(block);
        }
        *last = Some(block);
        next = b.func.layout.next_block(block);
    }
}
pub(super) struct Shared {
    pub layout: Layout,
    pub actions: Vec<Recipe>,
    pub phases: Vec<phases::Phase>,
    pub kernels: Vec<word_collections::Kernel>,
    pub initial: BTreeMap<usize, Initial>,
    pub histogram: Option<cranelift_module::FuncId>,
    pub classified_histogram: Option<cranelift_module::FuncId>,
    pub matrix_word: Option<FuncId>,
    pub classes: Vec<super::utf8_classes::Proof>,
}

type CompileRoot<'a, M> = &'a dyn Fn(&mut M, FuncId, &Context) -> ModuleResult<(u32, u64)>;

pub(in crate::native_artifact) fn define<M: Module>(
    module: &mut M,
    prepared: &PreparedNativeProgram,
    observe: bool,
    share_phase_bodies: bool,
) -> Result<serde_json::Value, NativeCompileError> {
    define_with_root(module, prepared, observe, share_phase_bodies, None)
}

pub(in crate::native_artifact::program) fn define_with_root<M: Module>(
    module: &mut M,
    prepared: &PreparedNativeProgram,
    observe: bool,
    share_phase_bodies: bool,
    compile_root: Option<CompileRoot<'_, M>>,
) -> Result<serde_json::Value, NativeCompileError> {
    define_with_pool(
        module,
        prepared,
        observe,
        share_phase_bodies,
        None,
        compile_root,
        false,
    )
}

pub(in crate::native_artifact::program) fn define_with_isa<M: Module>(
    module: &mut M,
    prepared: &PreparedNativeProgram,
    observe: bool,
    share_phase_bodies: bool,
    isa: cranelift_codegen::isa::OwnedTargetIsa,
    threads: usize,
    direct_runtime_calls: super::object::DirectRuntimeCalls,
    compile_root: Option<CompileRoot<'_, M>>,
    direct_literals: bool,
) -> Result<serde_json::Value, NativeCompileError> {
    let pool = (threads != 0)
        .then(|| super::compile_jobs::Pool::new(isa, threads, direct_runtime_calls))
        .transpose()?;
    define_with_pool(
        module,
        prepared,
        observe,
        share_phase_bodies,
        pool.as_ref(),
        compile_root,
        direct_literals,
    )
}

fn define_with_pool<M: Module>(
    module: &mut M,
    prepared: &PreparedNativeProgram,
    observe: bool,
    share_phase_bodies: bool,
    pool: Option<&super::compile_jobs::Pool>,
    compile_root: Option<CompileRoot<'_, M>>,
    direct_literals: bool,
) -> Result<serde_json::Value, NativeCompileError> {
    let plan = prepared.plan();
    let layout = Layout::derive(plan)?;
    let actions = (0..plan.blocks().len())
        .map(|site| recipes::derive(plan, &layout, site))
        .collect::<Result<Vec<_>, _>>()?;
    let literals = if std::env::var_os("MECH_COMPOSE_READONLY_LOOKUPS").is_some() {
        let root_flow = Flow::derive(plan)?;
        super::immutable_arrays::LiteralViews::derive(plan, &layout, &root_flow, &actions)
    } else {
        super::immutable_arrays::LiteralViews::default()
    };
    let phases = phases::define(
        module,
        prepared,
        &layout,
        &literals,
        observe,
        share_phase_bodies,
        pool,
    )?;
    let kernels = word_collections::define(module, prepared, &layout, observe)?;
    let initial = constants::prepare(module, plan, &layout)?;
    let histogram = super::histogram::define(module, &actions)?;
    let classes = super::utf8_classes::Proof::derive(prepared, &layout);
    let classified_histogram = if classes.is_empty() {
        None
    } else {
        Some(
            crate::native_artifact::program::codegen::utf8_histogram::define(
                module,
                "mech_compact_classified_histogram",
                true,
                crate::native_artifact::program::codegen::utf8_histogram::Funding::Unmetered,
                true,
                true,
            )?
            .0,
        )
    };
    let matrix_word = (!observe && std::env::var_os("MECH_COMPACT_SHARE_MATRIX_WORDS").is_some())
        .then(|| word_packs::define_matrix_word(module))
        .transpose()?;
    let shared = Shared {
        layout,
        actions,
        phases,
        kernels,
        initial,
        histogram,
        classified_histogram,
        matrix_word,
        classes,
    };
    let mut workers = Vec::new();
    for region in &prepared.metadata().parallel.regions {
        let entry = super::parallel::Entry::derive(plan, region)?;
        let report = define_entry(
            module,
            prepared,
            observe,
            &shared,
            Some(&entry),
            &[],
            None,
            false,
        )?;
        workers.push(super::parallel::Worker::new(
            module, plan, region, entry, report,
        )?);
    }
    let mut report = define_entry(
        module,
        prepared,
        observe,
        &shared,
        None,
        &workers,
        compile_root,
        direct_literals,
    )?;
    report["native_worker_entries"] =
        serde_json::json!(workers.iter().map(|w| &w.report).collect::<Vec<_>>());
    Ok(report)
}

fn scalar_literal_slots(
    plan: &mech_engine::native_artifact::NativeProgramArtifactPlan,
    layout: &Layout,
    flow: &Flow,
    inputs: &BTreeMap<usize, usize>,
    initial: &BTreeMap<usize, Initial>,
) -> BTreeSet<usize> {
    let mut written = vec![false; plan.values().len()];
    for &value in flow.defs.iter().flatten() {
        written[value as usize] = true;
    }
    plan.constants()
        .iter()
        .filter_map(|constant| {
            let value = constant.value().get();
            let ty = layout.ty(value);
            let slot = layout.slot(value);
            (!written[value as usize]
                && matches!(ty.kind, super::layout::LayoutKind::Scalar { .. })
                && ty.owners.is_empty()
                && !inputs.contains_key(&slot)
                && matches!(initial.get(&slot), Some(Initial::Bits(_))))
            .then_some(slot)
        })
        .collect()
}

fn single_definition_scalar_homes(
    layout: &Layout,
    flow: &Flow,
    inputs: &BTreeMap<usize, usize>,
    initial: &BTreeMap<usize, Initial>,
) -> BTreeSet<usize> {
    let mut definitions = vec![0u8; layout.types.len()];
    for (site, written) in flow.defs.iter().enumerate() {
        if flow.reachable[site] {
            for &value in written {
                let count = &mut definitions[value as usize];
                *count = count.saturating_add(1);
            }
        }
    }
    layout
        .types
        .iter()
        .enumerate()
        .filter_map(|(value, ty)| {
            let slot = layout.base[value];
            (definitions[value] == 1
                && ty.owners.is_empty()
                && matches!(ty.kind, super::layout::LayoutKind::Scalar { .. })
                && !inputs.contains_key(&slot)
                && !initial.contains_key(&slot))
            .then_some(slot)
        })
        .collect()
}

fn nonowner_field_homes(
    layout: &Layout,
    flow: &Flow,
    inputs: &BTreeMap<usize, usize>,
    initial: &BTreeMap<usize, Initial>,
) -> BTreeSet<usize> {
    use super::layout::LayoutKind;

    let mut definitions = vec![0u8; layout.types.len()];
    for (site, written) in flow.defs.iter().enumerate() {
        if flow.reachable[site] {
            for &value in written {
                let count = &mut definitions[value as usize];
                *count = count.saturating_add(1);
            }
        }
    }
    let mut eligible = BTreeSet::new();
    for (value, ty) in layout.types.iter().enumerate() {
        if definitions[value] != 1
            || !matches!(ty.kind, LayoutKind::Product(_) | LayoutKind::Array { .. })
        {
            continue;
        }
        for (offset, leaf) in ty.leaves() {
            // Owners and tagged payloads continue through the ordinary word
            // homes. A geometry word still needs the finalized-CFG proof.
            if !matches!(
                leaf.kind,
                LayoutKind::Scalar { .. } | LayoutKind::Array { .. }
            ) {
                continue;
            }
            let base = layout.base[value] + offset;
            eligible.extend((0..leaf.words).filter_map(|lane| {
                let slot = base + lane;
                (!leaf.owners.contains(&lane)
                    && !inputs.contains_key(&slot)
                    && !initial.contains_key(&slot))
                .then_some(slot)
            }));
        }
    }
    eligible
}

fn define_entry<M: Module>(
    module: &mut M,
    prepared: &PreparedNativeProgram,
    observe: bool,
    shared: &Shared,
    worker: Option<&super::parallel::Entry>,
    workers: &[super::parallel::Worker],
    compile_root: Option<CompileRoot<'_, M>>,
    direct_literals: bool,
) -> Result<serde_json::Value, NativeCompileError> {
    let plan = prepared.plan();
    let endian = module.isa().endianness();
    let route_sites = std::env::var("MECH_COMPACT_ROUTE_SITES")
        .ok()
        .map(|text| {
            text.split(',')
                .filter(|v| !v.is_empty())
                .map(|v| v.parse::<usize>().map_err(backend))
                .collect::<Result<BTreeSet<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    if route_sites.iter().any(|&site| site >= plan.blocks().len()) {
        return Err(backend(
            "diagnostic route site is outside current source plan",
        ));
    }
    let route_locations = std::env::var_os("MECH_COMPACT_ROUTE_SOURCELOCS").is_some();
    eprintln!("compact-stage: typed layout");
    let layout = &shared.layout;
    eprintln!("compact-stage: flow proof");
    let flow = if let Some(worker) = worker {
        worker.flow(plan)?
    } else {
        Flow::derive(plan)?
    };
    eprintln!("compact-stage: operation recipes");
    if !plan.observations().is_empty() || !plan.effects().is_empty() {
        return Err(backend(
            "compact closed entry does not admit provider/effect boundaries",
        ));
    }
    let actions = &shared.actions;
    let memory = actions
        .iter()
        .enumerate()
        .any(|(site, a)| flow.reachable[site] && matches!(a, Recipe::Buffer { .. }))
        || !plan.collections().is_empty();
    for (site, recipe) in actions.iter().enumerate() {
        if flow.reachable[site] {
            if let Recipe::Pending(op) = recipe {
                return Err(backend(format!(
                    "compact source operation has no executable implementation at site {site}: {op:?}"
                )));
            }
        }
    }
    let phases = shared
        .phases
        .iter()
        .filter(|p| {
            flow.reachable[p.header] && p.destinations().iter().all(|&site| flow.reachable[site])
        })
        .collect::<Vec<_>>();
    let packs = word_packs::derive(prepared)?
        .into_iter()
        .filter(|p| {
            flow.reachable[p.first as usize]
                && flow.reachable[p.last as usize]
                && flow.reachable[p.next as usize]
        })
        .collect::<Vec<_>>();
    let phase_geometry = phase_storage::Geometry::derive(phases.iter().map(|p| p.packet_words()))?;
    let word_kernels = &shared.kernels;
    let owner_flow = OwnerFlow::derive(plan, &layout, &flow, &actions)?;
    let reuse_pointwise_sources = worker.is_none()
        && !observe
        && std::env::var_os("MECH_COMPACT_REUSE_POINTWISE_SOURCE").is_some();
    let guards = Guards::derive(plan, &flow);
    let transfers = owner_flow.transfers(&flow, &guards);
    let initial = &shared.initial;
    let reductions = if memory {
        widening_reduction::derive(prepared, &layout, &flow, &actions, &initial)
    } else {
        BTreeMap::new()
    };
    let read_borrows = if memory && std::env::var_os("MECH_BORROW_COMPACT_READ_COPIES").is_some() {
        // A specialized native route may read or settle different owner
        // lanes. Confine this proof to the ordinary checked source actions.
        let boundaries = phases
            .iter()
            .flat_map(|phase| std::iter::once(phase.header).chain(phase.destinations()))
            .chain(
                packs
                    .iter()
                    .flat_map(|pack| pack.sites().chain([pack.next as usize])),
            )
            .chain(reductions.values().flat_map(|r| [r.header, r.exit]))
            .collect::<BTreeSet<_>>();
        owner_flow.borrow_read_copies(plan, layout, &flow, actions, &transfers, &boundaries)
    } else {
        vec![vec![]; actions.len()]
    };
    let retire_before_change = actions
        .iter()
        .enumerate()
        .map(|(site, action)| {
            if let Recipe::Buffer {
                operation:
                    mech_engine::native_artifact::NativeCollectionPrimitive::MatrixIndexedReplace,
                operands,
                ..
            } = action
            {
                let source = layout.slot(operands[0].get()) + super::layout::ARRAY_OWNER;
                owner_flow.retire_before_indexed_change(&flow, &guards, site, source)
            } else {
                Vec::new()
            }
        })
        .collect::<Vec<_>>();
    let consuming = actions
        .iter()
        .enumerate()
        .map(|(site, action)| {
            let Recipe::Buffer { operands, .. } = action else {
                return false;
            };
            let Some(source) = operands.first() else {
                return false;
            };
            if !layout
                .ty(source.get())
                .owners
                .contains(&super::layout::ARRAY_OWNER)
            {
                return false;
            }
            let slot = layout.slot(source.get()) + super::layout::ARRAY_OWNER;
            !owner_flow.live_after(site, slot)
                || owner_flow.last_use_with_guards(&flow, &guards, site, slot)
        })
        .collect::<Vec<_>>();
    let buffer_results = actions
        .iter()
        .enumerate()
        .map(|(site, action)| {
            let Recipe::Buffer {
                operands, output, ..
            } = action
            else {
                return None;
            };
            let source = operands.first()?;
            let same_backing_family = matches!(
                (&layout.ty(source.get()).kind, &layout.ty(output.get()).kind),
                (
                    super::layout::LayoutKind::Array { .. },
                    super::layout::LayoutKind::Array { .. }
                ) | (
                    super::layout::LayoutKind::SortedMap { .. },
                    super::layout::LayoutKind::SortedMap { .. }
                )
            );
            if !same_backing_family {
                return None;
            }
            let from = layout.slot(source.get()) + super::layout::ARRAY_OWNER;
            let to = layout.slot(output.get()) + super::layout::ARRAY_OWNER;
            Some((
                from,
                to,
                consuming[site] && owner_flow.can_finish_consumed_source(site, from),
            ))
        })
        .collect::<Vec<_>>();
    let chains = if std::env::var_os("MECH_PRIVATE_INDEXED_CHAINS").is_some() {
        let boundaries = phases
            .iter()
            .map(|phase| phase.header)
            .chain(packs.iter().map(|pack| pack.first))
            .collect::<BTreeSet<_>>();
        super::private_chains::derive(
            plan,
            &layout,
            &flow,
            &actions,
            &consuming,
            &buffer_results,
            &retire_before_change,
            &boundaries,
        )
    } else {
        Vec::new()
    };
    let overwrite_reuse = if worker.is_none()
        && memory
        && std::env::var_os("MECH_COMPACT_REUSE_OVERWRITTEN_RESULT").is_some()
    {
        super::overwrite_reuse::derive(
            plan,
            layout,
            &flow,
            actions,
            &guards,
            &owner_flow,
            &consuming,
        )
    } else {
        Vec::new()
    };
    let reduction_function = (!reductions.is_empty())
        .then(|| widening_reduction::declare(module))
        .transpose()?;
    let mut inputs = BTreeMap::new();
    let mut input_words = 0;
    let mut ports = vec![];
    for &id in &flow.inputs {
        let ty = layout.ty(id);
        ports.push(serde_json::json!({"name":plan.inputs().iter().find(|p|p.value().get()==id).map(|p|p.name()),"value":id,"input_offset":input_words,"type":ty.summary()}));
        for index in 0..ty.words {
            inputs.insert(layout.slot(id) + index, input_words + index);
        }
        input_words += ty.words;
    }
    let symbol = worker.map(|w| w.symbol.as_str()).unwrap_or(if memory {
        "mech_compact_closed_v2"
    } else {
        "mech_compact_closed_v1"
    });
    let terminal_appends = (worker.is_none()
        && memory
        && !observe
        && route_sites.is_empty()
        && std::env::var_os("MECH_TERMINAL_BYTE_APPENDS").is_some())
    .then(|| {
        TerminalAppends::derive(
            plan,
            layout,
            &flow,
            actions,
            &owner_flow,
            initial,
            &buffer_results,
        )
    })
    .flatten();
    let checked_symbol = if terminal_appends.is_some() {
        "mech_compact_closed_checked"
    } else {
        symbol
    };
    let mut signature = module.make_signature();
    signature.params = vec![AbiParam::new(types::I64); if memory { 3 } else { 2 }];
    signature.returns = vec![AbiParam::new(types::I64); 2];
    // Give the independent small-input entry a stable position while other
    // functions change size. Its checked fallback keeps the same signature.
    let early_entry = if let Some(appends) = terminal_appends.as_ref()
        && std::env::var_os("MECH_COMPACT_DEFINE_TERMINAL_ENTRY_FIRST").is_some()
    {
        let checked = module
            .declare_function(checked_symbol, Linkage::Local, &signature)
            .map_err(backend)?;
        let report = appends.define(module, symbol, &signature, checked)?;
        Some((checked, report))
    } else {
        None
    };
    let mut context = module.make_context();
    context.func.signature = signature.clone();
    context.func.share_cold_epilogues = worker.is_none()
        && !route_locations
        && std::env::var_os("MECH_COMPACT_SHARE_COLD_EPILOGUES").is_some();
    let mut fb = FunctionBuilderContext::new();
    let traversal_stage = std::env::var_os("MECH_COMPACT_SHARE_CHECKED_TRAVERSALS");
    let outline_allowed =
        worker.is_none() && !observe && route_sites.is_empty() && !route_locations;
    let census_homes = outline_allowed
        .then(|| std::env::var_os("MECH_COMPACT_DIRECT_SCALAR_HOME_CENSUS"))
        .flatten();
    let direct_homes = outline_allowed
        .then(|| std::env::var("MECH_COMPACT_DIRECT_SCALAR_HOMES").ok())
        .flatten();
    if census_homes.is_some() && direct_homes.is_some() {
        return Err(backend("use one direct scalar home mode at a time"));
    }
    let forward_empty_source =
        outline_allowed && std::env::var_os("MECH_COMPACT_EMPTY_SOURCE_ROUTES").is_some();
    context.func.forward_spill_reloads =
        outline_allowed && std::env::var_os("MECH_COMPACT_FORWARD_SPILL_RELOADS").is_some();
    context.func.aarch64_share_cold_tails =
        outline_allowed && std::env::var_os("MECH_COMPACT_SHARE_COLD_TAILS").is_some();
    context.func.aarch64_share_frame_copies =
        outline_allowed && std::env::var_os("MECH_COMPACT_SHARE_FRAME_COPIES").is_some();
    context.func.aarch64_branch_on_zero = outline_allowed
        && module.isa().name() == "aarch64"
        && std::env::var_os("MECH_COMPACT_BRANCH_ON_ZERO").is_some();
    let vector_facts = outline_allowed
        .then(|| std::env::var_os("MECH_COMPACT_VECTOR_FACTS"))
        .flatten();
    let mut vector_guards = vector_facts.as_ref().map(|_| Vec::new());
    let outline_traversals = outline_allowed && traversal_stage.is_some();
    let controller_windows = (outline_allowed && memory)
        .then(|| std::env::var("MECH_COMPACT_SHARE_CONTROLLER_WINDOWS").ok())
        .flatten();
    let controllers;
    let capacity_growth;
    let mut traversals = BTreeMap::<_, Vec<_>>::new();
    let mut output_words = 0;
    let mut omitted_copy_owners = Vec::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        b.seal_block(entry);
        let input = b.block_params(entry)[usize::from(memory)];
        let output = b.block_params(entry)[1 + usize::from(memory)];
        let zero = b.ins().iconst(types::I64, 0);
        let arrays = if memory {
            let heap = b.block_params(entry)[0];
            let mut arrays = Arrays::new(module, &mut b, heap, observe)?;
            if outline_allowed
                && let Some(strategy) = std::env::var_os("MECH_COMPACT_PACK_LE_SCALAR_STORES")
            {
                use super::le_scalar_stores::Packing;
                arrays.pack_le_stores = Some(if strategy == "ends" {
                    Packing::Ends
                } else {
                    Packing::Widths
                });
            }
            if outline_allowed
                && let Some(mode) = std::env::var_os("MECH_COMPACT_SHARE_CAPACITY_GROWTH")
            {
                use super::array_growth::BackingResults;
                let combined =
                    std::env::var("MECH_COMPACT_FUSE_WRITABLE_GROWTH")
                        .ok()
                        .map(|mode| match mode.as_str() {
                            "preserve" => BackingResults::Preserve,
                            "registers" => BackingResults::Registers,
                            "byte-registers" => BackingResults::ByteRegisters,
                            _ => BackingResults::Packet,
                        });
                arrays.share_capacity_growth(module, &mut b, mode == "preserve", combined)?;
            }
            if outline_allowed
                && let Some(mode) = std::env::var_os("MECH_COMPACT_SHARE_INDEXED_STORES")
            {
                arrays.share_indexed_stores(
                    module,
                    &mut b,
                    layout,
                    actions,
                    &consuming,
                    mode == "complete",
                )?;
            }
            Some(arrays)
        } else {
            None
        };
        let histogram = shared
            .histogram
            .map(|id| module.declare_func_in_func(id, b.func));
        let classified_histogram = shared
            .classified_histogram
            .map(|id| module.declare_func_in_func(id, b.func));
        let classes = super::utf8_classes::State::reserve(&mut b, &shared.classes, &flow);
        let collections = Collections::new(&mut b, plan, zero)?;
        let reuse_state = overwrite_reuse
            .iter()
            .map(|proof| super::overwrite_reuse::State::declare(module, &mut b, proof, zero))
            .collect::<Result<Vec<_>, _>>()?;
        let phase_packets = phase_geometry
            .map(|geometry| geometry.reserve(&mut b))
            .transpose()?;
        let transfer_observer = phases::observer(module, &mut b, observe)?;
        let route_observer = phases::observer(module, &mut b, !route_sites.is_empty())?;
        let overwrite_reuse_observer = phases::observer(
            module,
            &mut b,
            !overwrite_reuse.is_empty() && std::env::var_os("MECH_COMPACT_REUSE_CENSUS").is_some(),
        )?;
        let mut slots = BTreeSet::new();
        for (value, ty) in layout.types.iter().enumerate() {
            slots.extend(layout.base[value]..layout.base[value] + ty.words);
        }
        let initialize = |b: &mut FunctionBuilder<'_>, slot| {
            let value = if let Some(&at) = inputs.get(&slot) {
                b.ins()
                    .load(types::I64, MemFlags::new(), input, offset(at)?)
            } else if let Some(initial) = initial.get(&slot) {
                match initial {
                    Initial::Bits(n) => b.ins().iconst(types::I64, *n as i64),
                    Initial::Table(id) => {
                        let global = module.declare_data_in_func(*id, b.func);
                        b.ins().global_value(types::I64, global)
                    }
                }
            } else {
                zero
            };
            // Zero is only a backend SSA seed. Flow independently proves that
            // every reached source read has an actual input/constant/definition.
            Ok(Some(value))
        };
        let literals = (direct_literals && outline_allowed)
            .then(|| scalar_literal_slots(plan, layout, &flow, &inputs, initial))
            .filter(|literals| !literals.is_empty());
        let mut words = if let Some(literals) = &literals {
            eprintln!("compact-stage: direct scalar literals={}", literals.len());
            Words::new_with_entry_literals(
                &mut b,
                zero,
                CodePlan::Program(prepared),
                slots,
                initialize,
                literals,
            )?
        } else {
            Words::new_with_regions_and_initial(
                &mut b,
                zero,
                CodePlan::Program(prepared),
                slots,
                initialize,
                |_| MemFlags::new(),
            )?
        };
        if census_homes.is_some() || direct_homes.is_some() {
            let mut eligible = single_definition_scalar_homes(layout, &flow, &inputs, initial);
            if std::env::var_os("MECH_COMPACT_PROVE_NONOWNER_FIELDS").is_some() {
                eligible.extend(nonowner_field_homes(layout, &flow, &inputs, initial));
            }
            let selected = if let Some(request) = &direct_homes {
                let selected = request
                    .split(',')
                    .map(str::parse::<usize>)
                    .collect::<Result<BTreeSet<_>, _>>()
                    .map_err(backend)?;
                if !selected.is_subset(&eligible) {
                    return Err(backend(
                        "direct scalar home request exceeds typed source eligibility",
                    ));
                }
                selected
            } else {
                eligible
            };
            words.track_direct_homes(&selected, direct_homes.is_some());
        }
        if let Some(arrays) = &arrays {
            for &value in &flow.inputs {
                for &lane in &layout.ty(value).owners {
                    let slot = layout.slot(value) + lane;
                    if owner_flow.live_at_entry(flow.entry, slot) {
                        let owner = words.read(&mut b, slot);
                        arrays.retain(&mut b, owner);
                    } else {
                        // Metadata readers do not acquire backing ownership.
                        // Retirement uses this same source-home liveness; an
                        // unused token would otherwise survive final transfer.
                        words.write(&mut b, slot, zero);
                    }
                }
            }
        }
        let blocks = flow
            .reachable
            .iter()
            .map(|&yes| yes.then(|| b.create_block()))
            .collect::<Vec<_>>();
        controllers = controller_windows
            .as_deref()
            .map(|spec| checked_traversals::controller_candidates(spec, &blocks))
            .transpose()?
            .unwrap_or_default();
        let mut source_routes = (0..blocks.len()).collect::<Vec<_>>();
        if forward_empty_source {
            // These entries are used by generated routes or are proof
            // boundaries. Leave each of them as a distinct native block.
            let mut required = BTreeSet::from([flow.entry]);
            for phase in &phases {
                required.insert(phase.header);
                required.extend(phase.destinations());
            }
            for pack in &packs {
                required.extend([pack.first, pack.next as usize]);
            }
            for (&header, reduction) in &reductions {
                required.extend([header, reduction.exit]);
            }
            for chain in &chains {
                required.extend([chain.first(), chain.exit()]);
            }
            for proof in &overwrite_reuse {
                required.extend([proof.initialize, proof.clear, proof.exit]);
            }
            for controller in &controllers {
                required.extend([controller.header, controller.header + controller.span]);
            }

            // Restrict forwarding to ascending jumps. Reverse enumeration
            // resolves complete chains without traversing loops or changing
            // any source edge that carries an owner retirement.
            let mut forwarded = 0usize;
            for site in (0..blocks.len()).rev() {
                if blocks[site].is_none() || flow.complete[site] || required.contains(&site) {
                    continue;
                }
                let block = &plan.blocks()[site];
                let Term::Scalar(mech_engine::native_artifact::NativeMatchTerminator::Jump(next)) =
                    block.terminator()
                else {
                    continue;
                };
                let next = next as usize;
                if next > site
                    && matches!(block.action(), Action::Scalar(Scalar::None))
                    && matches!(actions[site], Recipe::None)
                    && flow.uses[site].is_empty()
                    && flow.defs[site].is_empty()
                    && buffer_results[site].is_none()
                    && transfers[site].is_empty()
                    && owner_flow.retire_after(site).is_empty()
                    && owner_flow.retire_on_edge(site, next).is_empty()
                {
                    source_routes[site] = source_routes[next];
                    forwarded += 1;
                }
            }
            eprintln!("compact source empty jumps forwarded: {forwarded}");
        }
        let source_blocks = blocks.iter().flatten().copied().collect::<BTreeSet<_>>();
        let mut remaining = vec![0usize; blocks.len()];
        for (site, successors) in flow.successors.iter().enumerate() {
            if flow.reachable[site] && source_routes[site] == site {
                for next in successors.iter().copied().collect::<BTreeSet<_>>() {
                    remaining[source_routes[next]] += 1;
                }
            }
        }
        for phase in &phases {
            for destination in phase.destinations() {
                remaining[destination] += 1;
            }
        }
        for pack in &packs {
            remaining[pack.next as usize] += 1;
        }
        for reduction in reductions.values() {
            remaining[reduction.exit] += 1;
        }
        for chain in &chains {
            remaining[chain.exit()] += 1;
        }

        for proof in &overwrite_reuse {
            remaining[proof.exit] += 1;
        }
        let mut last_layout = None;
        b.ins().jump(blocks[flow.entry].unwrap(), &[]);
        if remaining[flow.entry] == 0 {
            b.seal_block(blocks[flow.entry].unwrap());
        }
        seal_auxiliary(&mut b, &source_blocks, entry, None, &mut last_layout);
        // One shared action/settlement implementation serves both the
        // checked source CFG and the short guarded private interval.
        let mut emit_action = |b: &mut FunctionBuilder<'_>,
                               site: usize,
                               error: Block,
                               private: Option<&super::private_chains::Admitted<'_>>|
         -> Result<(), NativeCompileError> {
            let block = &plan.blocks()[site];
            if flow.skip_action[site] {
                return Ok(());
            }
            let mut previous = vec![];
            if memory {
                for &value in &flow.defs[site] {
                    for &lane in &layout.ty(value).owners {
                        // A live scalar field does not keep the old allocation
                        // field live. Dead owners are already zeroed on entry.
                        if !owner_flow.live_at_entry(site, layout.slot(value) + lane) {
                            continue;
                        }
                        previous.push(words.read(b, layout.slot(value) + lane));
                    }
                }
            }
            let observed_source = if observe {
                buffer_results[site].map(|(source, destination, allowed)| {
                    (words.read(b, source), destination, allowed)
                })
            } else {
                None
            };
            let buffer_source = buffer_results[site]
                .filter(|&(_, _, allowed)| allowed)
                .map(|(source, destination, _)| (source, destination, words.read(b, source)));
            match &actions[site] {
                Recipe::StringEqual {
                    source,
                    offset,
                    peer,
                    guards,
                    negate,
                    output,
                } => {
                    let base = layout.slot(source.get());
                    for &(at, ordinal) in guards {
                        let tag = words.read(b, base + at);
                        let valid = b.ins().icmp_imm(IntCC::Equal, tag, i64::from(ordinal));
                        guard(b, valid, error, crate::native_artifact::abi::STATUS_ABI);
                    }
                    let left = [
                        words.read(b, base + offset),
                        words.read(b, base + offset + 1),
                    ];
                    let peer = layout.slot(peer.get());
                    let right = [words.read(b, peer), words.read(b, peer + 1)];
                    let mut value = super::strings::equal(b, left[0], left[1], right[0], right[1]);
                    if *negate {
                        value = b.ins().bxor_imm(value, 1);
                    }
                    words.write(b, layout.slot(output.get()), value);
                }
                Recipe::Aggregate(action) => aggregates::emit(
                    b,
                    &mut SsaWords(&words),
                    &layout,
                    action,
                    error,
                    &mut CheckedGuards,
                )?,
                Recipe::Length { source, output } => {
                    let value = words.read(b, layout.slot(source.get()) + 1);
                    words.write(b, layout.slot(output.get()), value);
                }
                Recipe::Read {
                    source,
                    index,
                    output,
                } => {
                    let base = layout.slot(source.get());
                    let index = words.read(b, layout.slot(index.get()));
                    let index = b.ins().iadd_imm(index, -1);
                    let length = words.read(b, base + 1);
                    let valid = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
                    guard(b, valid, error, BOUNDS);
                    let element = layout.ty(source.get()).element()?;
                    let stride = element
                        .packed_bytes
                        .ok_or_else(|| backend("compact read lacks packed stride"))?;
                    let rows = words.read(b, base + 3);
                    let columns = words.read(b, base + 4);
                    let physical = super::super::codegen::numeric_words::linear_ordinal(
                        b,
                        index,
                        rows,
                        columns,
                        plan.values()[source.get() as usize].definition().body(),
                    );
                    let bytes = b.ins().imul_imm(physical, stride as i64);
                    let pointer = words.read(b, base);
                    let pointer = b.ins().iadd(pointer, bytes);
                    let values = packed::load(b, pointer, element)?;
                    arrays::write(&words, b, layout.slot(output.get()), &values);
                }
                Recipe::Buffer {
                    operation,
                    operands,
                    output,
                } if super::maps::supports(*operation) => super::maps::emit(
                    arrays.as_ref().expect("concrete allocation entry"),
                    b,
                    &words,
                    &layout,
                    consuming[site],
                    *operation,
                    operands,
                    *output,
                    error,
                    endian,
                    histogram,
                    classes
                        .iter()
                        .find(|state| state.proof.producer == site)
                        .map(|state| {
                            (
                                classified_histogram.expect("proved native classifier"),
                                state,
                            )
                        }),
                )?,
                Recipe::Buffer {
                    operation,
                    operands,
                    output,
                } => arrays.as_ref().expect("concrete allocation entry").emit(
                    b,
                    &words,
                    &layout,
                    plan.values()[operands[0].get() as usize]
                        .definition()
                        .body(),
                    consuming[site],
                    &retire_before_change[site],
                    site,
                    private,
                    *operation,
                    operands,
                    *output,
                    error,
                )?,
                Recipe::Pending(_) => {
                    return Err(backend("compact unresolved operation passed preflight"));
                }
                Recipe::None => {
                    let handled = if let Some(arrays) = &arrays {
                        collections.action(
                            b,
                            &words,
                            &layout,
                            arrays,
                            plan,
                            block.action(),
                            error,
                        )?
                    } else {
                        false
                    };
                    if !handled {
                        match block.action() {
                            Action::Scalar(Scalar::None) => {}
                            Action::Scalar(Scalar::Primitive {
                                op,
                                operands,
                                output,
                            }) => {
                                let args = operands
                                    .iter()
                                    .map(|v| words.read(b, layout.slot(v.get())))
                                    .collect::<Vec<_>>();
                                let value = primitive(b, *op, &args, error);
                                words.write(b, layout.slot(output.get()), value);
                            }
                            Action::Initialize { iteration } | Action::Latch { iteration } => {
                                let Loop::Tail(tail) = &plan.iterations()[*iteration as usize]
                                else {
                                    return Err(backend(
                                        "compact loop source lost checked tail kind",
                                    ));
                                };
                                let source = if matches!(block.action(), Action::Initialize { .. })
                                {
                                    tail.seed()
                                } else {
                                    tail.next()
                                };
                                aggregates::emit(
                                    b,
                                    &mut SsaWords(&words),
                                    &layout,
                                    &Aggregate::Copy {
                                        source,
                                        output: tail.carrier(),
                                    },
                                    error,
                                    &mut CheckedGuards,
                                )?;
                            }
                            _ => return Err(backend("compact action has no native recipe")),
                        }
                    }
                }
            }
            if let (Some(observer), Some((source, destination, allowed))) =
                (transfer_observer, observed_source)
            {
                let result = words.read(b, destination);
                let present = b.ins().icmp_imm(IntCC::NotEqual, source, 0);
                let same = b.ins().icmp(IntCC::Equal, source, result);
                let same = b.ins().band(same, present);
                let owned_result = b.ins().icmp_imm(IntCC::NotEqual, result, 0);
                let inspect = b.create_block();
                let join = b.create_block();
                b.append_block_param(join, types::I64);
                b.ins().brif(present, inspect, &[], join, &[zero.into()]);
                b.switch_to_block(inspect);
                let refs = b.ins().load(types::I64, MemFlags::new(), source, 0);
                b.ins().jump(join, &[refs.into()]);
                b.switch_to_block(join);
                let refs = b.block_params(join)[0];
                let unique = b.ins().icmp_imm(IntCC::Equal, refs, 1);
                let mut event = b.ins().iconst(
                    types::I64,
                    (site as u64
                        | (1 << 55)
                        | (u64::from(consuming[site]) << 54)
                        | (u64::from(allowed) << 53)) as i64,
                );
                for (bit, value) in [(40, same), (41, present), (42, owned_result), (43, unique)] {
                    let value = b.ins().uextend(types::I64, value);
                    let value = b.ins().ishl_imm(value, bit);
                    event = b.ins().bor(event, value);
                }
                let selected = b.ins().iconst(types::I64, 1);
                b.ins()
                    .call(observer, &[arrays.as_ref().unwrap().heap, event, selected]);
            }
            if let Some(arrays) = &arrays {
                // Shared aggregate emission has completed every guard and read
                // before overwriting destinations. Transfer only now, so source
                // failures preserve the original held references and outputs.
                for &(destination, source) in &transfers[site] {
                    if let Some(observer) = transfer_observer {
                        let owner = words.read(b, destination);
                        let nonnull = b.ins().icmp_imm(IntCC::NotEqual, owner, 0);
                        let selected = b.ins().uextend(types::I64, nonnull);
                        let at = b.ins().iconst(types::I64, (site as u64 | (1 << 61)) as i64);
                        b.ins().call(observer, &[arrays.heap, at, selected]);
                    }
                    words.write(b, source, zero);
                }
                for &value in &flow.defs[site] {
                    for &lane in &layout.ty(value).owners {
                        let slot = layout.slot(value) + lane;
                        if read_borrows[site].contains(&slot) {
                            // Keep the aggregate words and its guards. The
                            // exact source home owns this checked-read span.
                            words.write(b, slot, zero);
                            continue;
                        }
                        if !transfers[site]
                            .iter()
                            .any(|&(destination, _)| destination == slot)
                        {
                            if owner_flow.unused_copy_owner(site, slot) {
                                // The checked aggregate has completed. Its
                                // scalar metadata is retained, but no reached
                                // continuation or current output reads this
                                // allocation. Do not acquire then immediately
                                // retire an otherwise unobserved reference.
                                omitted_copy_owners.push((site, slot));
                                if let Some(observer) = transfer_observer {
                                    let owner = words.read(b, slot);
                                    let nonnull = b.ins().icmp_imm(IntCC::NotEqual, owner, 0);
                                    let selected = b.ins().uextend(types::I64, nonnull);
                                    let at = b
                                        .ins()
                                        .iconst(types::I64, (site as u64 | (1 << 56)) as i64);
                                    b.ins().call(observer, &[arrays.heap, at, selected]);
                                }
                                words.write(b, slot, zero);
                                continue;
                            }
                            let owner = words.read(b, slot);
                            if let Some((source_slot, destination, source_owner)) =
                                buffer_source.filter(|&(_, destination, _)| destination == slot)
                            {
                                debug_assert_eq!(destination, slot);
                                // All original buffer checks and result writes
                                // have succeeded. The SAME owner can receive
                                // this held, now-dead source reference. Shared
                                // backing needs no extra uniqueness condition.
                                let same = b.ins().icmp(IntCC::Equal, source_owner, owner);
                                let acquire = b.ins().select(same, zero, owner);
                                arrays.retain(b, acquire);
                                let remaining = b.ins().select(same, zero, source_owner);
                                words.write(b, source_slot, remaining);
                                if let Some(observer) = transfer_observer {
                                    let present =
                                        b.ins().icmp_imm(IntCC::NotEqual, source_owner, 0);
                                    let moved = b.ins().band(same, present);
                                    let moved = b.ins().uextend(types::I64, moved);
                                    let at = b
                                        .ins()
                                        .iconst(types::I64, (site as u64 | (1 << 51)) as i64);
                                    b.ins().call(observer, &[arrays.heap, at, moved]);
                                }
                            } else {
                                arrays.retain(b, owner);
                            }
                        }
                    }
                }
                for owner in previous {
                    arrays.release(b, owner, error);
                }
            }
            if let Some(arrays) = &arrays {
                collections.after(b, arrays, block.action(), error);
            }
            Ok(())
        };
        eprintln!("compact-stage: source CFG");
        for (site, block) in plan.blocks().iter().enumerate() {
            if site % 4096 == 0 {
                eprintln!("compact-stage: source site {site}/{}", plan.blocks().len());
            }
            let Some(native) = blocks[site] else { continue };
            if source_routes[site] != site {
                // Reserve the original unused error-block ID so that later
                // layout and checked-continuation diagnostics can remap it.
                b.create_block();
                continue;
            }
            b.switch_to_block(native);
            if route_locations {
                b.set_srcloc(cranelift_codegen::ir::SourceLoc::new(site as u32));
            }
            if route_sites.contains(&site) {
                let observer = route_observer.expect("diagnostic route observer is declared");
                let at = b.ins().iconst(types::I64, (site as u64 | (1 << 49)) as i64);
                let selected = b.ins().iconst(types::I64, 1);
                let heap = arrays.as_ref().map(|a| a.heap).unwrap_or(zero);
                b.ins().call(observer, &[heap, at, selected]);
            }
            // Source location is needed only after a reached operation fails.
            // A shared mutable origin home creates hot moves/spills through
            // every successful traversal of this otherwise independent state.
            let error = b.create_block();
            b.append_block_param(error, types::I64);
            b.set_cold_block(error);
            for state in &reuse_state {
                if state.proof.initialize == site {
                    state.reset(&mut b);
                }
                if state.proof.exit == site {
                    state.record_success(&mut b, arrays.as_ref().unwrap().heap);
                }
                if state.proof.clear == site {
                    let exit = state.proof.exit;
                    state.emit(
                        &mut b,
                        &words,
                        layout,
                        arrays.as_ref().unwrap(),
                        &owner_flow,
                        blocks[exit].unwrap(),
                        error,
                        overwrite_reuse_observer,
                    );
                    remaining[exit] -= 1;
                    if remaining[exit] == 0 {
                        b.seal_block(blocks[exit].unwrap());
                    }
                }
            }
            if let Some(phase) = phases.iter().find(|phase| phase.header == site) {
                phase.emit(
                    module,
                    &mut b,
                    &words,
                    arrays
                        .as_ref()
                        .ok_or_else(|| backend("compact numeric phase needs concrete storage"))?,
                    phase_packets.as_ref().ok_or_else(|| {
                        backend("compact numeric phase lacks shared call storage")
                    })?,
                    &owner_flow,
                    &layout,
                    blocks[phase.exit].unwrap(),
                    &blocks,
                    error,
                    worker.is_none(),
                    vector_guards.as_mut(),
                )?;
                if outline_traversals && let Some(key) = phase.checked_traversal_key() {
                    traversals
                        .entry(key)
                        .or_default()
                        .push(checked_traversals::Candidate {
                            resume: Some(native),
                            start: b.current_block().expect("checked fallback entry"),
                            exit: blocks[phase.exit].expect("checked continuation"),
                            header: site,
                            origin_base: None,
                            span: phase.exit.saturating_sub(site),
                        });
                }
                for destination in phase.destinations() {
                    remaining[destination] -= 1;
                    if remaining[destination] == 0 {
                        b.seal_block(blocks[destination].unwrap());
                    }
                }
            }
            if let Some(reduction) = reductions.get(&site) {
                reduction.emit(
                    module,
                    &mut b,
                    reduction_function.expect("declared reduction"),
                    &words,
                    arrays.as_ref().expect("reduction retains current views"),
                    &owner_flow,
                    transfer_observer,
                    blocks[reduction.exit].expect("reachable reduction exit"),
                    error,
                );
                remaining[reduction.exit] -= 1;
                if remaining[reduction.exit] == 0 {
                    b.seal_block(blocks[reduction.exit].unwrap());
                }
            }
            if let Some(pack) = packs.iter().find(|pack| pack.first == site) {
                word_packs::emit(
                    module,
                    &mut b,
                    &words,
                    &layout,
                    &owner_flow,
                    arrays.as_ref(),
                    observe,
                    shared.matrix_word.filter(|_| worker.is_none() && memory),
                    pack,
                    blocks[pack.next as usize].unwrap(),
                    error,
                )?;
                remaining[pack.next as usize] -= 1;
                if remaining[pack.next as usize] == 0 {
                    b.seal_block(blocks[pack.next as usize].unwrap());
                }
            }
            if let Some(chain) = chains.iter().find(|chain| chain.first() == site) {
                let ordinary = b.create_block();
                let private = chain.admit(&mut b, &words, &layout, ordinary);
                if let Some(observer) = transfer_observer {
                    let at = b.ins().iconst(types::I64, (site as u64 | (1 << 50)) as i64);
                    let selected = b.ins().iconst(types::I64, 1);
                    b.ins()
                        .call(observer, &[arrays.as_ref().unwrap().heap, at, selected]);
                }
                let mut failures = Vec::new();
                for (position, &current) in chain.sites().iter().enumerate() {
                    let failure = b.create_block();
                    b.append_block_param(failure, types::I64);
                    b.set_cold_block(failure);
                    if route_locations {
                        b.set_srcloc(cranelift_codegen::ir::SourceLoc::new(current as u32));
                    }
                    emit_action(&mut b, current, failure, Some(&private))?;
                    let arrays = arrays.as_ref().expect("private interval requires storage");
                    arrays.retire(&mut b, &words, &owner_flow.retire_after(current), failure);
                    let next = chain
                        .sites()
                        .get(position + 1)
                        .copied()
                        .unwrap_or(chain.exit());
                    arrays.retire(
                        &mut b,
                        &words,
                        &owner_flow.retire_on_edge(current, next),
                        failure,
                    );
                    failures.push((failure, current));
                }
                b.ins().jump(blocks[chain.exit()].unwrap(), &[]);
                remaining[chain.exit()] -= 1;
                if remaining[chain.exit()] == 0 {
                    b.seal_block(blocks[chain.exit()].unwrap());
                }
                for (failure, current) in failures {
                    b.switch_to_block(failure);
                    let status = b.block_params(failure)[0];
                    let origin = b.ins().iconst(types::I64, current as i64);
                    b.ins().return_(&[status, origin]);
                }
                b.switch_to_block(ordinary);
                if let Some(observer) = transfer_observer {
                    let at = b.ins().iconst(types::I64, (site as u64 | (1 << 50)) as i64);
                    b.ins()
                        .call(observer, &[arrays.as_ref().unwrap().heap, at, zero]);
                }
            }
            if route_locations {
                b.set_srcloc(cranelift_codegen::ir::SourceLoc::new(site as u32));
            }
            emit_action(&mut b, site, error, None)?;
            if flow.complete[site] {
                let mut results = vec![];
                for &value in &flow.outputs {
                    let base = layout.slot(value);
                    for field in 0..layout.ty(value).words {
                        results.push(words.read(&mut b, base + field));
                    }
                }
                if let Some(arrays) = &arrays {
                    for &value in &flow.outputs {
                        for &lane in &layout.ty(value).owners {
                            let owner = words.read(&mut b, layout.slot(value) + lane);
                            arrays.retain(&mut b, owner);
                        }
                    }
                    arrays.retire(&mut b, &words, &owner_flow.retire_after(site), error);
                }
                for (at, &value) in results.iter().enumerate() {
                    b.ins().store(MemFlags::new(), value, output, offset(at)?);
                }
                output_words = results.len();
                if let Some(arrays) = &arrays {
                    for state in &classes {
                        state.finish(&mut b, arrays, error);
                    }
                }
                b.ins().return_(&[zero, zero]);
            } else {
                let mut edges = BTreeMap::new();
                if let Some(arrays) = &arrays {
                    arrays.retire(&mut b, &words, &owner_flow.retire_after(site), error);
                    for &next in &flow.successors[site] {
                        if !owner_flow.retire_on_edge(site, next).is_empty() {
                            edges.entry(next).or_insert_with(|| b.create_block());
                        }
                    }
                }
                let target = |site: u32| {
                    edges.get(&(site as usize)).copied().unwrap_or_else(|| {
                        blocks[source_routes[site as usize]]
                            .expect("proved reachable source successor")
                    })
                };
                if let Term::Header {
                    iteration,
                    body,
                    exit,
                } = block.terminator()
                {
                    let reuse_pointwise_source = reuse_pointwise_sources
                        && word_kernels
                            .iter()
                            .find(|kernel| kernel.proof.iteration == iteration)
                            .is_some_and(|kernel| {
                                let owner =
                                    layout.slot(kernel.proof.source) + super::layout::ARRAY_OWNER;
                                // The checked collection header is also a
                                // boundary in the existing read-borrow proof.
                                // Any other current alias has a counted owner.
                                owner_flow.live_at_entry(site, owner)
                                    && !owner_flow.live_at_entry(exit as usize, owner)
                            });
                    for state in &classes {
                        if let Some(region) = state.at_header(site) {
                            state.emit_lookup(
                                &mut b,
                                &words,
                                layout,
                                arrays.as_ref().unwrap(),
                                &collections,
                                region,
                                target(exit),
                                error,
                                endian,
                            )?;
                        }
                    }
                    if let Some(worker) = workers.iter().find(|w| w.iteration == iteration) {
                        worker.emit(
                            module,
                            &mut b,
                            &words,
                            layout,
                            arrays.as_ref().unwrap(),
                            &collections,
                            plan,
                            target(exit),
                            error,
                        )?;
                    }
                    collections.header(
                        module,
                        &mut b,
                        &words,
                        &layout,
                        arrays
                            .as_ref()
                            .ok_or_else(|| backend("compact counted header remains unsupported"))?,
                        prepared,
                        &initial,
                        &word_kernels,
                        reuse_pointwise_source,
                        iteration,
                        target(body),
                        target(exit),
                        error,
                    )?;
                } else {
                    emit_source_terminator(
                        &mut b,
                        &words,
                        CodePlan::Program(prepared),
                        site,
                        Default::default(),
                        block.terminator(),
                        error,
                        target,
                    );
                }
                if let Some(arrays) = &arrays {
                    for (next, edge) in edges {
                        b.switch_to_block(edge);
                        arrays.retire(
                            &mut b,
                            &words,
                            &owner_flow.retire_on_edge(site, next),
                            error,
                        );
                        b.ins().jump(blocks[source_routes[next]].unwrap(), &[]);
                    }
                }
            }
            b.switch_to_block(error);
            let sealed_error = if outline_allowed {
                // All branches to this local failure block are known after its
                // source site has emitted ordinary and specialized edges.
                b.seal_block(error);
                Some(error)
            } else {
                None
            };
            if sealed_error.is_none() || !b.is_unreachable() {
                let status = b.block_params(error)[0];
                let at = b.ins().iconst(types::I64, site as i64);
                b.ins().return_(&[status, at]);
            }
            for next in flow.successors[site]
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
            {
                let next = source_routes[next];
                remaining[next] -= 1;
                if remaining[next] == 0 {
                    b.seal_block(blocks[next].expect("reachable source successor"));
                }
            }
            seal_auxiliary(
                &mut b,
                &source_blocks,
                entry,
                sealed_error,
                &mut last_layout,
            );
        }
        assert!(remaining.iter().all(|&n| n == 0));
        if let Some(folder) = std::env::var_os("MECH_COMPACT_ROUTE_DUMP") {
            let folder = std::path::PathBuf::from(folder);
            std::fs::create_dir_all(&folder).map_err(backend)?;
            let sites = plan.blocks().iter().enumerate().map(|(site, source)| {
                serde_json::json!({"site":site,"origin":source.origin(),
                    "block":blocks[site].map(|block|block.to_string()),
                    "action":format!("{:?}",source.action()),
                    "recipe":match &actions[site] {
                        Recipe::StringEqual { .. } => "StringEqual".to_owned(),
                        Recipe::Aggregate(a) => format!("{a:?}"),
                        Recipe::Read{source,index,output} => format!("Read {source:?} {index:?} {output:?}"),
                        Recipe::Length{source,output} => format!("Length {source:?} {output:?}"),
                        Recipe::Buffer{operation,operands,output} => format!("Buffer {operation:?} {operands:?} {output:?}"),
                        Recipe::Pending(op) => format!("Pending {op:?}"),
                        Recipe::None => "Scalar/control".to_owned(),
                    },
                    "terminator":format!("{:?}",source.terminator()),
                    "uses":flow.uses[site],"defs":flow.defs[site],
                    "retire_after":owner_flow.retire_after(site),
                    "transfer":transfers[site]})
            }).collect::<Vec<_>>();
            let values = layout
                .types
                .iter()
                .enumerate()
                .map(|(value, ty)| {
                    serde_json::json!({"value":value,"base":layout.base[value],
                    "type":ty.summary(),"words":ty.words,"owners":ty.owners})
                })
                .collect::<Vec<_>>();
            let data = serde_json::json!({"sites":sites,"values":values,
                "iterations":format!("{:#?}",plan.iterations())});
            std::fs::write(
                folder.join("source-cfg.json"),
                serde_json::to_vec_pretty(&data).map_err(backend)?,
            )
            .map_err(backend)?;
        }
        if words.attempted_literal_write() {
            return Err(backend("native lowering wrote an admitted entry literal"));
        }
        eprintln!("compact-stage: finalize SSA");
        capacity_growth = arrays.and_then(|arrays| arrays.capacity_growth);
        b.finalize();
        if census_homes.is_some() || direct_homes.is_some() {
            let proof = words.prove_tracked_homes(&context.func);
            let admitted = proof.iter().filter(|home| home.admitted).count();
            eprintln!("compact direct scalar homes: {admitted}/{}", proof.len());
            if let Some(path) = &census_homes {
                let bytes = serde_json::to_vec(&proof).map_err(backend)?;
                std::fs::write(path, bytes).map_err(backend)?;
            }
            if direct_homes.is_some() && admitted != proof.len() {
                let rejected = proof
                    .iter()
                    .filter(|home| !home.admitted)
                    .map(|home| home.slot);
                return Err(backend(format!(
                    "direct scalar home lowering did not prove homes {:?}",
                    rejected.collect::<Vec<_>>()
                )));
            }
        }
    }
    if let Some(growth) = capacity_growth {
        let (standalone, combined) = growth.replace(&mut context.func);
        if standalone + combined != 0 {
            context.flowgraph();
            context
                .eliminate_unreachable_code(module.isa())
                .map_err(backend)?;
        }
        eprintln!("compact scalar array growth sites shared: {standalone}, backing: {combined}");
    }
    if let Some(folder) = std::env::var_os("MECH_COMPACT_ROUTE_DUMP") {
        let folder = std::path::PathBuf::from(folder);
        std::fs::write(folder.join("before-cleanup.clif"), context.func.to_string())
            .map_err(backend)?;
        context.set_disasm(true);
    }
    context.verify(module.isa()).map_err(backend)?;
    // The builder is finished and no producer Value handles survive below.
    // Drop retired SSA IDs before value-indexed cleanup/backend allocations.
    eprintln!(
        "compact-stage: compact values before={}",
        context.func.dfg.num_values()
    );
    context.func.dfg.compact_values();
    eprintln!("compact-values: retained={}", context.func.dfg.num_values());
    eprintln!("compact-stage: CFG cleanup");
    let controls = CompilationControls::new(OptimizationOptions::default());
    // Match pure numeric idioms only after SSA is sealed. A later pass also
    // sees aliases exposed by CFG/identity forwarding; packet loads stay opaque.
    let mut bit_counts =
        crate::native_artifact::codegen::common::bit_counts::optimize(&mut context.func);
    let cleanup = cfg_cleanup::optimize(&controls, &mut context, module.isa())?;
    let current_predicates = std::env::var_os("MECH_CURRENT_PREDICATES")
        .is_some()
        .then(|| cfg_cleanup::refine_current_predicates(&controls, &mut context, module.isa()))
        .transpose()?;
    let identity_routes = std::env::var_os("MECH_IDENTITY_ROUTE_FORWARDING")
        .is_some()
        .then(|| cfg_cleanup::forward_identity_routes(&controls, &mut context, module.isa()))
        .transpose()?;
    bit_counts += crate::native_artifact::codegen::common::bit_counts::optimize(&mut context.func);
    if let (Some(mode), Some(guards)) = (vector_facts, vector_guards)
        && !guards.is_empty()
    {
        let apply = mode != std::ffi::OsStr::new("trace");
        let proved = vector_guard_facts::fold(&mut context, &guards, apply);
        if apply && proved != 0 {
            context
                .eliminate_unreachable_code(module.isa())
                .map_err(backend)?;
            context.verify(module.isa()).map_err(backend)?;
        }
    }
    if outline_allowed && let Some(mode) = std::env::var_os("MECH_COMPACT_FOLD_UNIT_BRANCHES") {
        let apply = mode != std::ffi::OsStr::new("trace");
        let proved = vector_guard_facts::fold_unit_branches(&mut context, apply);
        eprintln!("compact unit branches proved: {proved}");
        if apply && proved != 0 {
            context
                .eliminate_unreachable_code(module.isa())
                .map_err(backend)?;
            context.verify(module.isa()).map_err(backend)?;
        }
    }
    let checked_traversals = if outline_traversals {
        let all = traversal_stage.as_deref() == Some(std::ffi::OsStr::new("all"));
        let shared = checked_traversals::share(module, &mut context.func, traversals, all)?;
        if shared.is_some() {
            context.flowgraph();
            context.verify(module.isa()).map_err(backend)?;
        }
        shared
    } else {
        None
    };
    let checked_controllers = checked_traversals::share_controllers(
        module,
        &mut context.func,
        vec![controllers],
        checked_traversals::CaptureBindings::default(),
    )?;
    if checked_controllers.is_some() {
        context.flowgraph();
        context.verify(module.isa()).map_err(backend)?;
    }
    let discovered_controllers = if outline_allowed
        && let Ok(windows) = std::env::var("MECH_COMPACT_LAYOUT_CONTROLLER_WINDOWS")
    {
        let families = super::checked_region_discovery::families(
            &context.func,
            &windows,
            plan.blocks().len(),
        )?;
        let shared = checked_traversals::share_controllers(
            module,
            &mut context.func,
            families,
            checked_traversals::CaptureBindings::from_environment(),
        )?;
        if shared.is_some() {
            context.flowgraph();
            context.verify(module.isa()).map_err(backend)?;
        }
        shared
    } else {
        None
    };
    if outline_allowed
        && std::env::var_os("MECH_COMPACT_SHARE_OWNER_RETIREMENT")
            .is_some_and(|scope| scope != "helpers")
    {
        let shared = super::owner_retirement::share(module, &mut context.func)?;
        if shared != 0 {
            context.flowgraph();
            context.verify(module.isa()).map_err(backend)?;
        }
        eprintln!("compact owner retirements shared: {shared}");
    }
    // Close repeated callers after their leaf callees have selected shared
    // targets. Each region re-proves its boundaries on the resulting graph.
    if outline_allowed
        && let Some(path) = std::env::var_os("MECH_COMPACT_CONTINUATION_REGION_REPORT")
    {
        super::checked_region_discovery::report(&context.func, path.as_ref()).map_err(backend)?;
    }
    let checked_continuations = if outline_allowed
        && let Ok(windows) = std::env::var("MECH_COMPACT_CONTINUATION_WINDOWS")
    {
        let families = super::checked_region_discovery::families(
            &context.func,
            &windows,
            plan.blocks().len(),
        )?;
        let shared = checked_traversals::share_continuations(module, &mut context.func, families)?;
        if shared.is_some() {
            context.flowgraph();
            context.verify(module.isa()).map_err(backend)?;
        }
        shared
    } else {
        None
    };
    if outline_allowed && std::env::var_os("MECH_COMPACT_ROOT_SELECTS").is_some() {
        let selected =
            crate::native_artifact::codegen::common::scalar_selects::optimize(&mut context.func);
        if selected != 0 {
            context.flowgraph();
            context.verify(module.isa()).map_err(backend)?;
        }
        eprintln!("compact root scalar choices folded: {selected}");
    }
    if outline_allowed && std::env::var_os("MECH_COMPACT_FOLD_CHECKED_ADDS").is_some() {
        context.compute_cfg();
        let folded = cfg_cleanup::checked_adds::fold(&mut context.func, &context.cfg);
        if folded != 0 {
            context.verify(module.isa()).map_err(backend)?;
        }
        eprintln!("compact root proved carries: {folded}");
    }
    if outline_allowed && let Ok(proofs) = std::env::var("MECH_COMPACT_ROOT_PROOFS") {
        let isa = module.isa();
        if proofs == "ranges" || proofs == "all" {
            let extrema = std::env::var_os("MECH_COMPACT_PROVE_UNSIGNED_EXTREMA").is_some();
            let unsigned =
                crate::native_artifact::codegen::common::unsigned_guard_facts::optimize_with_ranges(
                    &mut context,
                    &[],
                    true,
                    extrema,
                );
            if unsigned != 0 {
                context
                    .prune_unreachable_and_remove_constant_phis(isa)
                    .map_err(backend)?;
                context.verify(isa).map_err(backend)?;
            }
            let repeated = cfg_cleanup::share_repeated_bounds(&controls, &mut context, isa)?;
            let windows = cfg_cleanup::reuse_dominating_window_facts(&controls, &mut context, isa)?;
            let read = cfg_cleanup::reuse_selected_read_bounds(&controls, &mut context, isa)?;
            let spans = cfg_cleanup::prove_scalar_spans(&controls, &mut context, isa)?;
            let packed = cfg_cleanup::prove_packed_bases(&controls, &mut context, isa, false)?;
            eprintln!(
                "compact root range proofs: {unsigned} unsigned, {repeated} repeated, {windows} windows, {read} reads, {spans} spans, {packed} packed"
            );
        }
        if proofs == "joins" || proofs == "all" {
            let selected = std::env::var("MECH_COMPACT_ROOT_FLOW_PASSES").ok();
            let enabled = |name| {
                selected
                    .as_deref()
                    .is_none_or(|set| set.split(',').any(|pass| pass == name))
            };
            let equal = if enabled("equal") {
                cfg_cleanup::coalesce_equal_join_arguments(&controls, &mut context, isa)?
            } else {
                0
            };
            let loop_edges = if enabled("thread") {
                cfg_cleanup::thread_loop_edges(&controls, &mut context, isa)?.threaded_edges
            } else {
                0
            };
            let identity = if enabled("identity") {
                cfg_cleanup::forward_identity_routes(&controls, &mut context, isa)?.threaded_edges
            } else {
                0
            };
            let predicates = if enabled("predicates") {
                cfg_cleanup::refine_current_predicates(&controls, &mut context, isa)?
                    .removed_branches
            } else {
                0
            };
            eprintln!(
                "compact root flow proofs: {equal} equal arguments, {loop_edges} loop edges, {identity} identity routes, {predicates} current predicates"
            );
        }
    }
    if outline_allowed && let Some(mode) = std::env::var_os("MECH_COMPACT_LITERAL_EDGE_ARGUMENTS") {
        let (literals, params) = cfg_cleanup::coalesce_literal_join_arguments(
            &controls,
            &mut context,
            module.isa(),
            mode == "fold",
        )?;
        eprintln!(
            "compact root literal edge arguments: {literals} definitions, {params} parameters removed"
        );
    }
    if outline_allowed && std::env::var_os("MECH_COMPACT_COALESCE_PARALLEL_PHIS").is_some() {
        let removed =
            cfg_cleanup::coalesce_parallel_join_arguments(&controls, &mut context, module.isa())?;
        eprintln!("compact parallel join arguments coalesced: {removed}");
    }
    if outline_allowed && std::env::var_os("MECH_COMPACT_FOLD_FRESH_OWNER_COUNTS").is_some() {
        let (backings, loads) = super::owner_counts::fold(module, &mut context);
        context.verify(module.isa()).map_err(backend)?;
        eprintln!("compact fresh owner counts: {backings} backings, {loads} loads folded");
    }
    if outline_allowed && let Some(scope) = std::env::var_os("MECH_COMPACT_PROVE_BACKING_OWNERS") {
        use super::backing_owners::Scope;
        let scope = match scope.to_str() {
            Some("direct") => Scope::Direct,
            Some("ranges") => Scope::DirectRanges,
            Some("joins") => Scope::Joins,
            _ => Scope::All,
        };
        let (backings, guards) = super::backing_owners::fold(module, &mut context, scope);
        if guards != 0 {
            let isa = module.isa();
            context.flowgraph();
            context
                .eliminate_unreachable_code(isa)
                .map_err(|error| backend(format!("{error:?}")))?;
            context.flowgraph();
            context
                .remove_constant_phis(isa)
                .map_err(|error| backend(format!("{error:?}")))?;
            context.verify(isa).map_err(backend)?;
        }
        eprintln!("compact positive backings: {backings}, owner checks proved: {guards}");
    }
    if outline_allowed && std::env::var_os("MECH_COMPACT_TRIM_RETIREMENTS").is_some() {
        let trimmed = super::owner_retirement::trim_empty_owners(module, &mut context.func)?;
        if trimmed != 0 {
            context.verify(module.isa()).map_err(backend)?;
        }
        eprintln!("compact root owner retirement calls trimmed: {trimmed}");
    }
    if outline_allowed
        && std::env::var_os("MECH_COMPACT_SKIP_EMPTY_JUMPS")
            .is_some_and(|scope| scope == "root" || scope == "all")
    {
        let forwarded = cfg_cleanup::empty_jumps::forward(&mut context.func);
        if forwarded != 0 {
            let isa = module.isa();
            context.flowgraph();
            context.eliminate_unreachable_code(isa).map_err(backend)?;
            context.verify(isa).map_err(backend)?;
        }
        eprintln!("compact root forwarded empty jumps: {forwarded}");
    }
    if outline_allowed && std::env::var_os("MECH_COMPACT_FINISH_CFG_CLEANUP").is_some() {
        let cleanup = cfg_cleanup::optimize(&controls, &mut context, module.isa())?;
        context.verify(module.isa()).map_err(backend)?;
        eprintln!(
            "compact final CFG cleanup: {} to {} instructions, {} to {} blocks",
            cleanup.before_instructions,
            cleanup.after_instructions,
            cleanup.before_blocks,
            cleanup.after_blocks,
        );
    }
    let stack_pairs = std::env::var_os("MECH_COMPACT_PAIR_STACK_WORDS").is_some();
    if outline_allowed
        && super::word_pairs::available(module.isa())
        && (stack_pairs
            || std::env::var_os("MECH_COMPACT_PAIR_WORDS")
                .is_some_and(|scope| scope == "root" || scope == "all"))
    {
        let paired = super::word_pairs::fold(&mut context.func, None, stack_pairs);
        if paired != 0 {
            context.verify(module.isa()).map_err(backend)?;
        }
        eprintln!("compact root paired words: {paired}");
    }
    if outline_allowed && let Some(path) = std::env::var_os("MECH_COMPACT_LAYOUT_REGION_REPORT") {
        super::checked_region_discovery::report(&context.func, path.as_ref()).map_err(backend)?;
    }
    if outline_allowed && std::env::var_os("MECH_COMPACT_SHARE_STATIC_ERROR_STATUS").is_some() {
        context.flowgraph();
        let shared = super::cold_status::share(&mut context.func, &context.cfg);
        if shared != 0 {
            context.flowgraph();
            context.verify(module.isa()).map_err(backend)?;
            eprintln!("compact static error returns shared: {shared}");
        }
    }
    let calls = context
        .func
        .layout
        .blocks()
        .flat_map(|block| context.func.layout.block_insts(block))
        .filter(|&inst| context.func.dfg.insts[inst].opcode().is_call())
        .count();
    if !memory && calls != 0 {
        return Err(backend(
            "compact pure computation contains an unresolved native call",
        ));
    }
    if outline_allowed
        && memory
        && super::word_pairs::available(module.isa())
        && std::env::var_os("MECH_COMPACT_SELECT_DEFAULTED_LOADS").is_some()
    {
        let folded = cfg_cleanup::defaulted_loads::fold(module, &mut context)?;
        eprintln!("compact root defaulted loads folded: {folded}");
    }
    if let Some(folder) = std::env::var_os("MECH_COMPACT_ROUTE_DUMP") {
        std::fs::write(
            std::path::PathBuf::from(folder).join("after-cleanup.clif"),
            context.func.to_string(),
        )
        .map_err(backend)?;
    }
    let release_stubs = if outline_allowed
        && memory
        && compile_root.is_none()
        && terminal_appends.is_some()
        && std::env::var_os("MECH_COMPACT_SHARE_RELEASE_SETUP").is_some()
    {
        let stubs = super::release_stubs::ReleaseStubs::prepare(module, &mut context.func)?;
        if stubs.is_some() {
            context.verify(module.isa()).map_err(backend)?;
        }
        stubs
    } else {
        None
    };
    eprintln!("compact-stage: machine code");
    let function = if let Some((checked, _)) = &early_entry {
        *checked
    } else {
        module
            .declare_function(
                checked_symbol,
                if worker.is_some() || terminal_appends.is_some() {
                    Linkage::Local
                } else {
                    Linkage::Export
                },
                &signature,
            )
            .map_err(backend)?
    };
    let root_machine = compile_root
        .map(|compile| compile(module, function, &context))
        .transpose()
        .map_err(backend)?;
    let (code_bytes, stack_bytes) = if let Some(machine) = root_machine {
        machine
    } else {
        module
            .define_function(function, &mut context)
            .map_err(backend)?;
        let compiled = context
            .compiled_code()
            .ok_or_else(|| backend("compact function lacks native code"))?;
        if let Some(folder) = std::env::var_os("MECH_COMPACT_ROUTE_DUMP") {
            let folder = std::path::PathBuf::from(folder);
            if let Some(vcode) = &compiled.vcode {
                std::fs::write(folder.join("native-vcode.asm"), vcode).map_err(backend)?;
            }
            let locations = compiled
                .buffer
                .get_srclocs_sorted()
                .iter()
                .map(|v| serde_json::json!({"start":v.start,"end":v.end,"site":v.loc.bits()}))
                .collect::<Vec<_>>();
            std::fs::write(
                folder.join("machine-source-ranges.json"),
                serde_json::to_vec_pretty(&locations).map_err(backend)?,
            )
            .map_err(backend)?;
        }
        if let Some(stubs) = release_stubs {
            let frame = compiled
                .buffer
                .frame_layout()
                .ok_or_else(|| backend("release setup needs a checked frame"))?;
            stubs.define(module, frame)?;
        }
        (
            compiled.code_info().total_size,
            compiled_stack_bytes(&compiled.buffer)?,
        )
    };
    eprintln!("compact-stage: machine code complete");
    if let Some(function) = reduction_function {
        widening_reduction::define(module, function)?;
    }
    let terminal_report = if let Some((_, report)) = early_entry {
        Some(report)
    } else {
        terminal_appends
            .map(|appends| appends.define(module, symbol, &signature, function))
            .transpose()?
    };
    Ok({
        let mut report = serde_json::json!({"private_indexed_chains":chains.iter().map(|chain|chain.summary()).collect::<Vec<_>>(),"phase_call_storage":phase_geometry.map(|g|g.summary()),"symbol":symbol,"buffer_result_sites":buffer_results.iter().enumerate().filter_map(|(site, value)|value.map(|(source, destination, allowed)|serde_json::json!({"site":site,"source_owner_slot":source,"result_owner_slot":destination,"consume":consuming[site],"can_finish_source":allowed}))).collect::<Vec<_>>(),"omitted_copy_owner_lanes":omitted_copy_owners,"prechange_retirement_sites":retire_before_change.iter().enumerate().filter_map(|(site, slots)|(!slots.is_empty()).then_some(serde_json::json!({"site":site,"slots":slots}))).collect::<Vec<_>>(),"source_error_origins":"cold-return constants","owner_transfer_lanes":transfers.iter().map(Vec::len).sum::<usize>(),"owner_transfer_sites":transfers.iter().enumerate().filter_map(|(site, lanes)|(!lanes.is_empty()).then_some(serde_json::json!({"site":site,"lanes":lanes}))).collect::<Vec<_>>(),"phase_observer":observe,"word_pack_sites":packs.iter().map(|p|serde_json::json!({"first":p.first,"last":p.last,"width":p.width,"next":p.next})).collect::<Vec<_>>(),"word_collections":word_kernels.iter().map(|k|&k.summary).collect::<Vec<_>>(),"numeric_phases":phases.iter().map(|p|p.summary(&owner_flow)).collect::<Vec<_>>(),"concrete_heap_context":memory,"inputs":ports,"input_words":input_words,"output_words":output_words,"output_types":flow.outputs.iter().map(|&v|layout.ty(v).summary()).collect::<Vec<_>>(),"code_bytes":code_bytes,"stack_bytes":stack_bytes,"source_blocks":plan.blocks().len(),"reachable_blocks":flow.reachable.iter().filter(|&&v|v).count(),"all_path_initialization_proved":true,"calls":calls,"guarded_consuming_sites":consuming.iter().enumerate().filter_map(|(site, &yes)| yes.then_some(site)).collect::<Vec<_>>(),"cleanup":cleanup,"origins":plan.blocks().iter().map(|b|b.origin()).collect::<Vec<_>>(),"typed_ir_version":crate::native_artifact::program::image::identity::NATIVE_PROGRAM_TYPED_IR_VERSION,"contract":"Diagnostic typed closed computation; ordinary exact resource/admission/recovery contract omitted. Not NoLimits or qualified ordinary entry."});
        if root_machine.is_some() {
            report["checked_root_backend"] = serde_json::json!("llvm-size");
        }
        if let Some(checked_traversals) = checked_traversals {
            report["checked_traversals"] = checked_traversals;
        }
        if let Some(checked_controllers) = checked_controllers {
            report["checked_controllers"] = checked_controllers;
        }
        if let Some(discovered_controllers) = discovered_controllers {
            report["layout_checked_controllers"] = discovered_controllers;
        }
        if let Some(checked_continuations) = checked_continuations {
            report["checked_continuations"] = checked_continuations;
        }
        if !overwrite_reuse.is_empty() {
            report["overwrite_result_reuse"] = serde_json::json!(
                overwrite_reuse
                    .iter()
                    .map(|proof| proof.summary())
                    .collect::<Vec<_>>()
            );
        }
        if let Some(terminal) = terminal_report {
            report["terminal_byte_appends"] = terminal;
        }
        if read_borrows.iter().any(|slots| !slots.is_empty()) {
            report["borrowed_read_copy_sites"] = serde_json::json!(
                read_borrows
                    .iter()
                    .enumerate()
                    .filter(|(_, slots)| !slots.is_empty())
                    .collect::<Vec<_>>()
            );
        }
        report["bit_counts"] = serde_json::json!(bit_counts);
        report["native_string_classes"] = serde_json::json!(
            shared
                .classes
                .iter()
                .filter(|p| flow.reachable[p.producer])
                .map(|p| p.summary())
                .collect::<Vec<_>>()
        );
        if let Some(current_predicates) = current_predicates {
            report["current_predicates"] = serde_json::json!(current_predicates);
        }
        if let Some(identity_routes) = identity_routes {
            report["identity_route_forwarding"] = serde_json::json!(identity_routes);
        }
        if !reductions.is_empty() {
            report["widening_reductions"] =
                serde_json::json!(reductions.keys().collect::<Vec<_>>());
        }
        report
    })
}
