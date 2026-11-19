//! Physical compilation units using one unchanged source ownership model.
use super::{
    flow::Flow,
    layout::Layout,
    owner_flow::OwnerFlow,
    partition_plan,
    phase_storage::{Geometry, Slots},
    word_boundary::{Boundary, Effects, Expanded},
};
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, offset},
};
use cranelift_codegen::ir::{AbiParam, Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_artifact::NativeProgramArtifactPlan as Plan;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Unit {
    pub entry: usize,
    pub reachable: Vec<bool>,
    pub successors: Vec<Vec<usize>>,
    pub words: BTreeSet<usize>,
    iterations: BTreeSet<usize>,
    collections: BTreeSet<usize>,
}
pub(super) struct Split {
    pub typed: partition_plan::Plan,
    pub boundary: Boundary,
    pub zero_inputs: BTreeSet<usize>,
    pub zero_outputs: BTreeSet<usize>,
    pub root: Unit,
    pub helper: Unit,
    pub function: FuncId,
    pub symbol: String,
}
#[derive(Clone, Copy)]
pub(super) struct View<'a> {
    pub split: &'a Split,
    pub helper: bool,
}
impl<'a> View<'a> {
    pub fn unit(self) -> &'a Unit {
        if self.helper {
            &self.split.helper
        } else {
            &self.split.root
        }
    }
    pub fn owns_source(self, site: usize) -> bool {
        self.split.typed.owners[site] == Some(usize::from(self.helper))
    }
    pub fn owns_iteration(self, id: usize) -> bool {
        self.unit().iterations.contains(&id)
    }
    pub fn owns_collection(self, id: usize) -> bool {
        self.unit().collections.contains(&id)
    }
    pub fn is_call(self, site: usize) -> bool {
        !self.helper && site == self.split.helper.entry
    }
    pub fn is_return(self, site: usize) -> bool {
        self.helper && site == self.split.typed.regions[0].exit
    }
}

impl Split {
    pub fn derive<M: Module>(
        module: &mut M,
        plan: &Plan,
        layout: &Layout,
        flow: &Flow,
        owners: &OwnerFlow,
        transfers: &[Vec<(usize, usize)>],
        retire_before: &[Vec<usize>],
        read_borrows: &[Vec<usize>],
        initial: &BTreeMap<usize, super::constants::Initial>,
        recipes: &[super::recipes::Recipe],
        phases: &[&super::phases::Phase],
        packs: &[crate::native_artifact::program::codegen::numeric_read_packs::Pack],
        reductions: &BTreeMap<usize, super::widening_reduction::Reduction>,
        buffer_results: &[Option<(usize, usize, bool)>],
        windows: &str,
        memory: bool,
    ) -> Result<Self, NativeCompileError> {
        let typed = super::early_regions::plan_partition(plan, layout, flow, windows)?;
        if typed.regions.len() != 1 {
            return Err(backend(
                "early emission currently requires one complete source region",
            ));
        }
        let mut actions = vec![Effects::default(); plan.blocks().len()];
        let mut edges = vec![Vec::new(); plan.blocks().len()];
        for site in 0..plan.blocks().len() {
            if !flow.reachable[site] {
                continue;
            }
            let action = &mut actions[site];
            if memory {
                // Keep old destination owners before the action overwrites them.
                for &value in &flow.defs[site] {
                    for &lane in &layout.ty(value).owners {
                        let slot = layout.slot(value) + lane;
                        if owners.live_at_entry(site, slot) {
                            action.read(slot);
                        }
                    }
                }
            }
            for value in flow.incoming_uses(site) {
                for word in layout.slot(value)..layout.slot(value) + layout.ty(value).words {
                    action.read(word);
                }
            }
            if let mech_engine::native_artifact::NativeIterationAction::CollectionBegin {
                collection,
            } = plan.blocks()[site].action()
                && let mech_engine::ComprehensionKind::MatrixHorizontalBlocks {
                    rows: mech_engine::MatrixBlockRows::InputMatrixElements { input },
                } = plan.collections()[*collection as usize].kind()
            {
                let value = plan
                    .inputs()
                    .get(input as usize)
                    .ok_or_else(|| backend("early collection rows input absent"))?
                    .value()
                    .get();
                action.read(layout.slot(value) + 1);
            }
            if memory {
                // Indexed replacement only retires these aliases on its changed
                // branch. They remain incoming on the unchanged traversal.
                for &word in &retire_before[site] {
                    action.read(word);
                    action.conditional_clear(word);
                }
                if let Some((source, _, true)) = buffer_results[site] {
                    action.read(source);
                }
            }
            for &value in &flow.defs[site] {
                for word in layout.slot(value)..layout.slot(value) + layout.ty(value).words {
                    action.write(word);
                }
            }
            for (destination, source) in owners.copy_lanes(site) {
                action.copy_lane(destination, source);
            }
            if let super::recipes::Recipe::Aggregate(
                mech_engine::native_artifact::NativeScalarAggregateAction::Pack { output, words },
            ) = &recipes[site]
            {
                for &lane in &layout.ty(output.get()).owners {
                    if matches!(
                        words.get(lane),
                        Some(mech_engine::native_artifact::NativeAggregateWord::Constant(
                            0
                        ))
                    ) {
                        action.clear(layout.slot(output.get()) + lane);
                    }
                }
            }
            if memory {
                for &(_, source) in &transfers[site] {
                    action.clear(source);
                }
                if let Some((source, destination, true)) = buffer_results[site] {
                    if source == destination {
                        action.write(source);
                    } else {
                        action.conditional_clear(source);
                    }
                }
                for &value in &flow.defs[site] {
                    for &lane in &layout.ty(value).owners {
                        let word = layout.slot(value) + lane;
                        if read_borrows[site].contains(&word)
                            || (!transfers[site]
                                .iter()
                                .any(|&(destination, _)| destination == word)
                                && owners.unused_copy_owner(site, word))
                        {
                            action.clear(word);
                        }
                    }
                }
                // Retire-after happens after successful action and result-owner
                // settlement. Edge effects happen after source termination.
                for word in owners.retire_after(site) {
                    action.read(word);
                    action.clear(word);
                }
            }
            for &next in &flow.successors[site] {
                let mut edge = Effects::default();
                if memory {
                    for word in owners.retire_on_edge(site, next) {
                        edge.read(word);
                        edge.clear(word);
                    }
                }
                edges[site].push(edge);
            }
        }
        let written = actions
            .iter()
            .flat_map(|a| &a.may_write)
            .chain(edges.iter().flatten().flat_map(|e| &e.may_write))
            .copied()
            .collect::<BTreeSet<_>>();
        let region = &typed.regions[0];
        let mut model = Expanded::new(
            flow.reachable.clone(),
            flow.successors.clone(),
            actions,
            edges,
        );
        let unit_reaches = |header: usize, destination: usize| match typed.owners[header] {
            Some(0) => typed.owners[destination] == Some(0) || destination == region.entry,
            Some(1) => typed.owners[destination] == Some(1) || destination == region.exit,
            _ => false,
        };
        for phase in phases {
            if phase
                .destinations()
                .iter()
                .all(|&exit| unit_reaches(phase.header, exit))
            {
                for (exit, effect) in phase.word_effects(owners) {
                    model
                        .bypass_before(phase.header, exit, effect)
                        .map_err(backend)?;
                }
            }
        }
        for pack in packs {
            if typed.owners[pack.first] == typed.owners[pack.last as usize]
                && unit_reaches(pack.first, pack.next as usize)
            {
                model
                    .bypass_before(
                        pack.first,
                        pack.next as usize,
                        super::word_packs::word_effects(pack, layout, owners, memory),
                    )
                    .map_err(backend)?;
            }
        }
        for reduction in reductions.values() {
            if unit_reaches(reduction.header, reduction.exit) {
                model
                    .bypass_before(
                        reduction.header,
                        reduction.exit,
                        reduction.word_effects(owners),
                    )
                    .map_err(backend)?;
            }
        }
        let graph = model.graph();
        let written = written
            .into_iter()
            .chain(
                graph
                    .actions
                    .iter()
                    .flat_map(|a| a.may_write.iter().copied()),
            )
            .chain(
                graph
                    .edges
                    .iter()
                    .flatten()
                    .flat_map(|e| e.may_write.iter().copied()),
            )
            .collect::<BTreeSet<_>>();
        let constants = plan
            .constants()
            .iter()
            .flat_map(|c| {
                let value = c.value().get();
                layout.slot(value)..layout.slot(value) + layout.ty(value).words
            })
            .filter(|word| !written.contains(word))
            .collect::<BTreeSet<_>>();
        let physical_sites = model.unit_sites(&region.sites);
        let mut boundary = graph
            .boundary(&physical_sites, region.entry, region.exit, &constants)
            .map_err(backend)?;
        let owner_words = layout
            .types
            .iter()
            .enumerate()
            .flat_map(|(value, ty)| ty.owners.iter().map(move |&lane| layout.base[value] + lane))
            .collect::<BTreeSet<_>>();
        let incoming_owners = flow
            .inputs
            .iter()
            .flat_map(|&value| {
                layout
                    .ty(value)
                    .owners
                    .iter()
                    .map(move |&lane| layout.slot(value) + lane)
            })
            .collect::<BTreeSet<_>>();
        let initial_zeros = owner_words
            .iter()
            .copied()
            .filter(|word| {
                if incoming_owners.contains(word) {
                    return memory && !owners.live_at_entry(flow.entry, *word);
                }
                initial
                    .get(word)
                    .is_none_or(|value| matches!(value, super::constants::Initial::Bits(0)))
            })
            .collect();
        let zeros = graph
            .zeros_at(
                flow.entry,
                &owner_words,
                &initial_zeros,
                &[region.entry, region.exit],
            )
            .map_err(backend)?;
        let zero_inputs = boundary
            .inputs
            .iter()
            .copied()
            .filter(|word| zeros[0].contains(word))
            .collect::<BTreeSet<_>>();
        let zero_outputs = boundary
            .outputs
            .iter()
            .copied()
            .filter(|word| zeros[1].contains(word))
            .collect::<BTreeSet<_>>();
        boundary.inputs.retain(|word| !zero_inputs.contains(word));
        boundary.outputs.retain(|word| !zero_outputs.contains(word));
        let mut root_words = graph.root_words(&model.unit_sites(&typed.root_sites), &boundary);
        root_words.extend(&zero_outputs);
        for value in flow.inputs.iter().chain(&flow.outputs) {
            root_words.extend(layout.slot(*value)..layout.slot(*value) + layout.ty(*value).words);
        }
        let helper_scopes = region.scopes.iter().copied().collect::<BTreeSet<_>>();
        let unit = |helper: bool, words| {
            let owner = usize::from(helper);
            let reachable = (0..flow.reachable.len())
                .map(|s| {
                    typed.owners[s] == Some(owner)
                        || s == if helper { region.exit } else { region.entry }
                })
                .collect::<Vec<_>>();
            let successors = (0..flow.reachable.len())
                .map(|site| {
                    if !reachable[site] || helper && site == region.exit {
                        Vec::new()
                    } else if !helper && site == region.entry {
                        vec![region.exit]
                    } else {
                        flow.successors[site].clone()
                    }
                })
                .collect();
            Unit {
                entry: if helper { region.entry } else { flow.entry },
                reachable,
                successors,
                words,
                iterations: (0..plan.iterations().len())
                    .filter(|id| helper_scopes.contains(id) == helper)
                    .collect(),
                collections: (0..plan.collections().len())
                    .filter(|id| helper_scopes.contains(&(plan.iterations().len() + id)) == helper)
                    .collect(),
            }
        };
        let root = unit(false, root_words);
        let helper = unit(true, boundary.local_words.clone());
        let symbol = format!("mech_compact_early_region_{}_{}", region.entry, region.exit);
        let mut signature = module.make_signature();
        signature.params = vec![AbiParam::new(types::I64); if memory { 3 } else { 2 }];
        signature.returns = vec![AbiParam::new(types::I64); 2];
        let function = module
            .declare_function(&symbol, Linkage::Local, &signature)
            .map_err(backend)?;
        Ok(Self {
            typed,
            boundary,
            zero_inputs,
            zero_outputs,
            root,
            helper,
            function,
            symbol,
        })
    }

    pub fn input_map(&self) -> BTreeMap<usize, usize> {
        self.boundary
            .inputs
            .iter()
            .enumerate()
            .map(|(at, &word)| (word, at))
            .collect()
    }
    pub fn reserve(&self, b: &mut FunctionBuilder<'_>) -> Result<Slots, NativeCompileError> {
        Geometry::derive([(self.boundary.inputs.len(), self.boundary.outputs.len())])?
            .expect("one early region call")
            .reserve(b)
    }
    pub fn emit_call<M: Module>(
        &self,
        module: &mut M,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        heap: Option<Value>,
        packet: &Slots,
        exit: Block,
    ) -> Result<(), NativeCompileError> {
        let (input, output) =
            packet.for_call(self.boundary.inputs.len(), self.boundary.outputs.len())?;
        for (at, &word) in self.boundary.inputs.iter().enumerate() {
            let value = words.read(b, word);
            b.ins().stack_store(value, input, offset(at)?);
        }
        let input = b.ins().stack_addr(types::I64, input, 0);
        let output = b.ins().stack_addr(types::I64, output, 0);
        let mut args = Vec::new();
        args.extend(heap);
        args.extend([input, output]);
        let function = module.declare_func_in_func(self.function, b.func);
        let call = b.ins().call(function, &args);
        let &[status, origin] = b.inst_results(call) else {
            unreachable!("early checked status and source origin")
        };
        let failed = b.create_block();
        let success = b.create_block();
        b.set_cold_block(failed);
        b.ins().brif(status, failed, &[], success, &[]);
        b.switch_to_block(failed);
        b.ins().return_(&[status, origin]);
        b.switch_to_block(success);
        for (at, &word) in self.boundary.outputs.iter().enumerate() {
            let value = b
                .ins()
                .load(types::I64, MemFlags::new(), output, offset(at)?);
            words.write(b, word, value);
        }
        if !self.zero_outputs.is_empty() {
            let zero = b.ins().iconst(types::I64, 0);
            for &word in &self.zero_outputs {
                words.write(b, word, zero);
            }
        }
        b.ins().jump(exit, &[]);
        Ok(())
    }
    pub fn emit_return(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        output: Value,
        zero: Value,
    ) -> Result<(), NativeCompileError> {
        for (at, &word) in self.boundary.outputs.iter().enumerate() {
            let value = words.read(b, word);
            b.ins().store(MemFlags::new(), value, output, offset(at)?);
        }
        b.ins().return_(&[zero, zero]);
        Ok(())
    }
    pub fn summary(&self) -> serde_json::Value {
        let r = &self.typed.regions[0];
        serde_json::json!({"source_entry":r.entry,"normal_exit":r.exit,"operations":r.sites.len(),
            "input_word_slots":self.boundary.inputs,"output_word_slots":self.boundary.outputs,
            "constant_zero_input_slots":self.zero_inputs,"constant_zero_output_slots":self.zero_outputs,
            "zero_proof":"all-path physical owner must-zero fixed point including guarded native bypasses; unknown writes kill, joins intersect, copies and clears propagate",
            "root_word_homes":self.root.words.len(),"helper_word_homes":self.helper.words.len(),
            "root_source_blocks":self.root.reachable.iter().filter(|&&v|v).count(),
            "helper_source_blocks":self.helper.reachable.iter().filter(|&&v|v).count(),
            "root_numbered_layout_windows":"excluded until remapped to physical units",
            "source_ownership_model":"shared global facts; no call-boundary retain/release",
            "error_transport":"original checked status and canonical source site; caller returns immediately",
            "runtime_qualification":"required for this changed worker"})
    }
}
