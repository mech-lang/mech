//! Prepare an outlined region without copying the enclosing function's value arena.
use super::{Candidate, Traversal, inputs};
use cranelift_codegen::{
    dominator_tree::DominatorTree,
    entity::SecondaryMap,
    ir::{
        Block, BlockCall, Constant, DynamicStackSlot, ExceptionTable, FuncRef, Function,
        GlobalValue, Immediate, JumpTable, SigRef, SourceLoc, StackSlot, Value, ValueDef,
        ValueList, ValueListPool, instructions::InstructionMapper,
    },
};
use std::collections::BTreeMap;

struct RegionInstructions<'a> {
    source: &'a Function,
    values: &'a BTreeMap<Value, Value>,
    blocks: &'a BTreeMap<Block, Block>,
    lists: &'a mut ValueListPool,
}

impl RegionInstructions<'_> {
    fn value(&self, old: Value) -> Value {
        self.values[&self.source.dfg.resolve_aliases(old)]
    }
}

impl InstructionMapper for RegionInstructions<'_> {
    fn map_value(&mut self, value: Value) -> Value {
        self.value(value)
    }

    fn map_value_list(&mut self, list: ValueList) -> ValueList {
        let mut mapped = ValueList::new();
        for &value in list.as_slice(&self.source.dfg.value_lists) {
            let value = self.value(value);
            mapped.push(value, self.lists);
        }
        mapped
    }

    fn map_block_call(&mut self, call: BlockCall) -> BlockCall {
        let old = &self.source.dfg.value_lists;
        let block = self.blocks[&call.block(old)];
        let args = call
            .args(old)
            .map(|arg| arg.map_value(|v| self.value(v)))
            .collect::<Vec<_>>();
        BlockCall::new(block, args, self.lists)
    }

    fn map_block(&mut self, block: Block) -> Block {
        self.blocks[&block]
    }

    // These references keep their original local indexes in the region seed.
    fn map_func_ref(&mut self, value: FuncRef) -> FuncRef {
        value
    }

    fn map_sig_ref(&mut self, value: SigRef) -> SigRef {
        value
    }

    fn map_global_value(&mut self, value: GlobalValue) -> GlobalValue {
        value
    }

    fn map_stack_slot(&mut self, value: StackSlot) -> StackSlot {
        value
    }

    fn map_constant(&mut self, value: Constant) -> Constant {
        value
    }

    fn map_immediate(&mut self, value: Immediate) -> Immediate {
        value
    }

    fn map_dynamic_stack_slot(&mut self, _: DynamicStackSlot) -> DynamicStackSlot {
        unreachable!("dynamic stack operations are excluded from checked regions")
    }

    fn map_jump_table(&mut self, _: JumpTable) -> JumpTable {
        unreachable!("branch tables are excluded from checked regions")
    }

    fn map_exception_table(&mut self, _: ExceptionTable) -> ExceptionTable {
        unreachable!("try calls are excluded from checked regions")
    }
}

pub(super) struct BlockOrder {
    layout: SecondaryMap<Block, usize>,
    reverse_postorder: SecondaryMap<Block, usize>,
}

impl BlockOrder {
    pub(super) fn new(source: &Function, dominance: &DominatorTree) -> Self {
        fn ranks(blocks: impl Iterator<Item = Block>) -> SecondaryMap<Block, usize> {
            let mut ranks = SecondaryMap::new();
            for (rank, block) in blocks.enumerate() {
                ranks[block] = rank;
            }
            ranks
        }
        Self {
            layout: ranks(source.layout.blocks()),
            reverse_postorder: ranks(dominance.cfg_postorder().iter().rev().copied()),
        }
    }
}

pub(super) fn prepare(
    source: &Function,
    traversal: &Traversal,
    dominance: &DominatorTree,
    order: &BlockOrder,
) -> Option<(Function, Traversal)> {
    if source.stack_limit.is_some()
        || source.preferred_fallthrough.is_some()
        || source.dfg.values_labels.is_some()
        || !source.debug_tags.is_empty()
        || traversal.blocks.iter().any(|&b| !dominance.is_reachable(b))
    {
        return None;
    }

    let mut seed = Function::with_name_signature(source.name.clone(), source.signature.clone());
    seed.params = source.params.clone();
    seed.sized_stack_slots = source.sized_stack_slots.clone();
    seed.dynamic_stack_slots = source.dynamic_stack_slots.clone();
    seed.global_values = source.global_values.clone();
    seed.dfg.dynamic_types = source.dfg.dynamic_types.clone();
    seed.dfg.ext_funcs = source.dfg.ext_funcs.clone();
    seed.dfg.signatures = source.dfg.signatures.clone();
    seed.dfg.constants = source.dfg.constants.clone();
    seed.dfg.immediates = source.dfg.immediates.clone();
    seed.share_cold_epilogues = source.share_cold_epilogues;

    let entry_sources = seed.dfg.make_block();
    seed.layout.append_block(entry_sources);
    let mut blocks = BTreeMap::new();
    let mut values = BTreeMap::new();
    let exits = traversal.candidate.exits();
    let mut region_blocks = traversal.blocks.iter().copied().collect::<Vec<_>>();
    let mut with_exits = region_blocks.clone();
    with_exits.extend(
        exits
            .into_iter()
            .flatten()
            .filter(|&b| source.layout.is_block_inserted(b) && !traversal.blocks.contains(&b)),
    );
    with_exits.sort_unstable_by_key(|&block| order.layout[block]);
    with_exits.dedup();
    for block in with_exits {
        let local = seed.dfg.make_block();
        blocks.insert(block, local);
        seed.layout.append_block(local);
        if source.layout.is_cold(block) {
            seed.layout.set_cold(local);
        }
        for &parameter in source.dfg.block_params(block) {
            let ty = source.dfg.value_type(parameter);
            values.insert(parameter, seed.dfg.append_block_param(local, ty));
        }
    }
    for &value in traversal
        .inputs
        .iter()
        .chain(traversal.passthrough.values())
    {
        let value = source.dfg.resolve_aliases(value);
        values.entry(value).or_insert_with(|| {
            seed.dfg
                .append_block_param(entry_sources, source.dfg.value_type(value))
        });
    }
    for &value in &traversal.constants {
        let ValueDef::Result(inst, _) = source.dfg.value_def(value) else {
            return None;
        };
        let local = seed.dfg.make_inst(source.dfg.insts[inst]);
        seed.dfg
            .make_inst_results(local, source.dfg.ctrl_typevar(inst));
        seed.layout.append_inst(local, entry_sources);
        values.insert(value, seed.dfg.first_result(local));
    }

    let mut input_roles = BTreeMap::new();

    // Definitions precede uses in reverse postorder. Keep the source layout
    // order in the seed, independently of when each instruction is copied.
    region_blocks.sort_unstable_by_key(|&block| order.reverse_postorder[block]);
    for block in region_blocks {
        let local_block = blocks[&block];
        for inst in source.layout.block_insts(block) {
            if source.dfg.user_stack_map_entries(inst).is_some()
                || inputs(source, inst)
                    .any(|v| !values.contains_key(&source.dfg.resolve_aliases(v)))
            {
                return None;
            }
            let mapped = source.dfg.insts[inst].map(RegionInstructions {
                source,
                values: &values,
                blocks: &blocks,
                lists: &mut seed.dfg.value_lists,
            });
            let local = seed.dfg.make_inst(mapped);
            if let Some(roles) = traversal.input_roles.get(&inst) {
                input_roles.insert(local, roles.clone());
            }
            seed.dfg
                .make_inst_results(local, source.dfg.ctrl_typevar(inst));
            seed.layout.append_inst(local, local_block);
            let location = source.srcloc(inst);
            if location != SourceLoc::default() {
                seed.set_srcloc(local, location);
            }
            for (&from, &to) in source
                .dfg
                .inst_results(inst)
                .iter()
                .zip(seed.dfg.inst_results(local))
            {
                values.insert(from, to);
            }
        }
    }
    let value = |v: Value| values[&source.dfg.resolve_aliases(v)];
    let candidate = traversal.candidate;
    let local = Traversal {
        candidate: Candidate {
            start: blocks[&candidate.start],
            exit: blocks[&candidate.exit],
            resume: candidate.resume.map(|b| blocks[&b]),
            ..candidate
        },
        blocks: traversal.blocks.iter().map(|b| blocks[b]).collect(),
        retained_returns: Default::default(),
        inputs: traversal.inputs.iter().copied().map(value).collect(),
        input_roles,
        outputs: traversal.outputs.iter().copied().map(value).collect(),
        aligned_outputs: traversal
            .aligned_outputs
            .iter()
            .copied()
            .map(value)
            .collect(),
        finish: traversal.finish.iter().copied().map(value).collect(),
        passthrough: traversal
            .passthrough
            .iter()
            .map(|(&from, &to)| (value(from), value(to)))
            .collect(),
        live_captures: Vec::new(),
        packed_inputs: traversal.packed_inputs,
        register_inputs: false,
        register_outputs: traversal.register_outputs,
        return_error_origin: traversal.return_error_origin,
        constants: traversal.constants.iter().copied().map(value).collect(),
        slots: traversal.slots.clone(),
        literals: traversal
            .literals
            .iter()
            .map(|capture| super::region_literals::Capture {
                values: capture.values.iter().copied().map(value).collect(),
                ty: capture.ty,
                bits: capture.bits,
            })
            .collect(),
    };
    Some((seed, local))
}
