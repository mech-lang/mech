//! Forward the last initialized cell of an exclusively appended U64 vector.
//! The enclosing append-run proof fixes the read's (length - 1) geometry.
//! The compact caller additionally admits only this one append mutation; every
//! ordinary, literal-run and paired-run store is recorded by its own emitter.
use cranelift_codegen::ir::{
    Block, Function, Inst, InstBuilder, InstructionData as Data, MemFlags, Opcode, Value,
    condcodes::IntCC, types,
};
use cranelift_codegen::{
    Context,
    cursor::{Cursor, FuncCursor},
    flowgraph::ControlFlowGraph,
};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(all(test, feature = "jit", target_pointer_width = "64"))]
#[path = "append_previous_tests.rs"]
mod tests;

#[derive(Clone)]
pub(in crate::native_artifact) struct Hint {
    read: Inst,
    stores: Vec<Inst>,
    base: Value,
}
impl Hint {
    pub(super) fn new(read: Inst, stores: Vec<Inst>, base: Value) -> Self {
        Self { read, stores, base }
    }
    pub(in crate::native_artifact) fn apply(
        &self,
        context: &mut Context,
        loads: &BTreeMap<Value, (Inst, usize)>,
        output_word: usize,
        length_word: usize,
    ) -> Result<(usize, usize), &'static str> {
        // Address/CFG rewrites can duplicate a source store after recording.
        // Rebuild the complete set from the admitted current packet identity.
        let f = &context.func;
        let entry = f.layout.entry_block().ok_or("missing entry")?;
        let packet = f.dfg.block_params(entry);
        if packet.len() != 3 {
            return Err("unexpected private packet signature");
        }
        let mut stores = Vec::new();
        for block in f.layout.blocks() {
            for inst in f.layout.block_insts(block) {
                if let Data::Store { .. } = f.dfg.insts[inst] {
                    let [value, address] = *f.dfg.inst_args(inst) else {
                        return Err("unsupported store");
                    };
                    let root = origin(
                        f,
                        &context.cfg,
                        loads,
                        output_word,
                        [packet[1], packet[2]],
                        address,
                        0,
                    );
                    match root {
                        Some(Origin::Array) if f.dfg.value_type(value) == types::I64 => {
                            stores.push(inst)
                        }
                        Some(Origin::Packet | Origin::Stack) => {}
                        _ => return Err("unclassified current write"),
                    }
                }
            }
        }
        if self.stores.is_empty() {
            return Err("no original append lineage");
        }
        if super::super::cfg_cleanup::composed_reads::packet_word(f, &context.cfg, loads, self.base)
            != Some(output_word)
        {
            return Err("append lineage does not name the admitted current view");
        }
        let seed = if std::env::var_os("MECH_FORWARD_PREVIOUS_APPEND").is_some_and(|v| v == "seed")
        {
            let word = |at| {
                loads.iter().find_map(|(&value, &(inst, word))| {
                    (word == at && f.layout.inst_block(inst) == Some(entry)).then_some(value)
                })
            };
            Some((
                word(output_word).ok_or("append pointer is not loaded at entry")?,
                word(length_word).ok_or("append length is not loaded at entry")?,
            ))
        } else {
            None
        };
        forward(
            context,
            self.read,
            &stores,
            seed,
            std::env::var_os("MECH_FORWARD_PREVIOUS_NARROW").is_some(),
        )
    }
}

#[derive(Clone, Copy)]
enum Origin {
    Array,
    Packet,
    Stack,
}
fn origin(
    f: &Function,
    cfg: &ControlFlowGraph,
    loads: &BTreeMap<Value, (Inst, usize)>,
    word: usize,
    packets: [Value; 2],
    value: Value,
    depth: usize,
) -> Option<Origin> {
    if depth > 16 {
        return None;
    }
    let value = f.dfg.resolve_aliases(value);
    if super::super::cfg_cleanup::composed_reads::packet_word(f, cfg, loads, value) == Some(word) {
        return Some(Origin::Array);
    }
    if packets.contains(&value) {
        return Some(Origin::Packet);
    }
    let cranelift_codegen::ir::ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return None;
    };
    match f.dfg.insts[inst] {
        Data::StackLoad {
            opcode: Opcode::StackAddr,
            ..
        } => Some(Origin::Stack),
        Data::StackLoad { .. } => None,
        Data::Binary {
            opcode: Opcode::Iadd,
            args: [a, b],
        } => {
            let left = origin(f, cfg, loads, word, packets, a, depth + 1);
            let right = origin(f, cfg, loads, word, packets, b, depth + 1);
            if matches!(left, Some(Origin::Array)) || matches!(right, Some(Origin::Array)) {
                Some(Origin::Array)
            } else if literal(f, b) {
                left
            } else if literal(f, a) {
                right
            } else {
                None
            }
        }
        Data::Binary {
            opcode: Opcode::Isub,
            args: [a, b],
        } => {
            let root = origin(f, cfg, loads, word, packets, a, depth + 1);
            if matches!(root, Some(Origin::Array)) || literal(f, b) {
                root
            } else {
                None
            }
        }
        Data::BinaryImm64 {
            opcode: Opcode::IaddImm,
            arg: a,
            ..
        } => origin(f, cfg, loads, word, packets, a, depth + 1),
        _ => None,
    }
}

fn literal(f: &Function, value: Value) -> bool {
    let value = f.dfg.resolve_aliases(value);
    matches!(f.dfg.value_def(value),cranelift_codegen::ir::ValueDef::Result(inst,0)
        if matches!(f.dfg.insts[inst],Data::UnaryImm {opcode:Opcode::Iconst,..}))
}

struct State<'a> {
    cfg: &'a ControlFlowGraph,
    definitions: BTreeMap<Block, Value>,
    incoming: BTreeMap<Block, (Value, Value)>,
    entry: Block,
    zero: Value,
    one: Value,
    initial: Value,
    value_zero: Value,
    seeded: bool,
    reachable: BTreeSet<Block>,
}
impl State<'_> {
    fn end(&mut self, f: &mut Function, block: Block) -> (Value, Value) {
        if let Some(&value) = self.definitions.get(&block) {
            (value, self.one)
        } else {
            self.start(f, block)
        }
    }
    fn start(&mut self, f: &mut Function, block: Block) -> (Value, Value) {
        if block == self.entry {
            return (self.initial, if self.seeded { self.one } else { self.zero });
        }
        if !self.reachable.contains(&block) {
            return (self.value_zero, self.zero);
        }
        if let Some(&values) = self.incoming.get(&block) {
            return values;
        }
        let edges = self
            .cfg
            .pred_iter(block)
            .map(|edge| (edge.block, edge.inst))
            .collect::<BTreeSet<_>>();
        if edges.len() == 1
            && let Some(&(predecessor, _)) = edges.first()
            && predecessor != block
        {
            let values = self.end(f, predecessor);
            self.incoming.insert(block, values);
            return values;
        }
        let ty = f.dfg.value_type(self.initial);
        let value = f.dfg.append_block_param(block, ty);
        let valid = if self.seeded {
            self.one
        } else {
            f.dfg.append_block_param(block, types::I64)
        };
        self.incoming.insert(block, (value, valid));
        for (predecessor, inst) in edges {
            let (value, valid) = self.end(f, predecessor);
            let dfg = &mut f.dfg;
            for call in dfg.insts[inst]
                .branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
            {
                if call.block(&dfg.value_lists) == block {
                    call.append_argument(value, &mut dfg.value_lists);
                    if !self.seeded {
                        call.append_argument(valid, &mut dfg.value_lists);
                    }
                }
            }
        }
        (value, valid)
    }
}

pub(super) fn forward(
    context: &mut Context,
    read: Inst,
    stores: &[Inst],
    seed: Option<(Value, Value)>,
    narrow: bool,
) -> Result<(usize, usize), &'static str> {
    let f = &mut context.func;
    let mut blocks = f.layout.blocks().collect::<Vec<_>>();
    if blocks.len() > 1024 || stores.is_empty() {
        return Err("append forwarding analysis limit");
    }
    let Data::Load {
        opcode: opcode @ (Opcode::Load | Opcode::Uload32),
        flags: read_flags,
        offset: read_offset,
        ..
    } = f.dfg.insts[read]
    else {
        return Err("previous word is not a plain load");
    };
    let old = f.dfg.first_result(read);
    let ty = f.dfg.value_type(old);
    if (ty == types::I32 || opcode == Opcode::Uload32)
        && read_flags.endianness(cranelift_codegen::ir::Endianness::Little)
            != cranelift_codegen::ir::Endianness::Little
    {
        return Err("narrow forwarding requires little-endian low words");
    }
    if !matches!(ty, types::I32 | types::I64) || i32::from(read_offset) != 0 {
        return Err("previous word width or displacement differs");
    }
    let known = stores.iter().copied().collect::<BTreeSet<_>>();
    for &store in stores {
        let Data::Store { flags, .. } = f.dfg.insts[store] else {
            return Err("append definition is not a store");
        };
        if f.dfg.value_type(f.dfg.inst_args(store)[0]) != types::I64
            || flags.endianness(cranelift_codegen::ir::Endianness::Little)
                != read_flags.endianness(cranelift_codegen::ir::Endianness::Little)
        {
            return Err("append word representation differs");
        }
    }
    let at = f
        .layout
        .inst_block(read)
        .ok_or("previous load has no block")?;
    let after = f
        .layout
        .next_inst(read)
        .ok_or("previous load has no continuation")?;
    let mut entry = f.layout.entry_block().ok_or("missing entry")?;
    let first = f.layout.first_inst(entry).ok_or("empty entry")?;
    if let Some((pointer, length)) = seed {
        if at == entry
            || stores
                .iter()
                .any(|&inst| f.layout.inst_block(inst) == Some(entry))
            || [pointer, length].iter().any(|&value| {
                f.dfg.value_type(value) != types::I64
                    || match f.dfg.value_def(value) {
                        cranelift_codegen::ir::ValueDef::Result(inst, _) => {
                            f.layout.inst_block(inst) != Some(entry)
                        }
                        cranelift_codegen::ir::ValueDef::Param(block, _) => block != entry,
                        _ => true,
                    }
            })
        {
            return Err("entry seed does not precede the append recurrence");
        }
    }
    let mut definitions = BTreeMap::new();
    let mut preceding = None;
    let carry_ty = if narrow && (ty == types::I32 || opcode == Opcode::Uload32) {
        types::I32
    } else {
        types::I64
    };
    for &block in &blocks {
        let insts = f.layout.block_insts(block).collect::<Vec<_>>();
        for inst in insts {
            if block == at && inst == read {
                preceding = definitions.get(&block).copied();
            }
            if known.contains(&inst) {
                let mut stored = f.dfg.inst_args(inst)[0];
                if carry_ty == types::I32 {
                    let next = f
                        .layout
                        .next_inst(inst)
                        .expect("append store has a continuation");
                    let mut c = FuncCursor::new(f).at_inst(next);
                    stored = c.ins().ireduce(types::I32, stored);
                }
                definitions.insert(block, stored);
            }
        }
    }
    let mut c = FuncCursor::new(f).at_inst(first);
    let zero = c.ins().iconst(types::I64, 0);
    let one = c.ins().iconst(types::I64, 1);
    let value_zero = if carry_ty == types::I32 {
        c.ins().iconst(carry_ty, 0)
    } else {
        zero
    };
    let initial = if let Some((pointer, length)) = seed {
        // The sealed NoGrowth caller owns an initialized U64 view and admits
        // its unchanged geometry. A nonempty entry view therefore permits one
        // pure read here, even when the source loop has zero iterations. Empty
        // views take no read; source nonempty/error checks remain in place.
        let terminator = c.func.layout.last_inst(entry).expect("nonempty entry");
        let resume = c.func.dfg.make_block();
        c.func.layout.split_block(resume, terminator);
        let load = c.func.dfg.make_block();
        c.func.layout.append_block(load);
        let join = c.func.dfg.make_block();
        c.func.layout.append_block(join);
        let value = c.func.dfg.append_block_param(join, carry_ty);
        c.goto_bottom(entry);
        let nonempty = c.ins().icmp_imm(IntCC::NotEqual, length, 0);
        c.ins()
            .brif(nonempty, load, &[], join, &[value_zero.into()]);
        c.goto_bottom(load);
        let last = c.ins().iadd_imm(length, -1);
        let displacement = c.ins().ishl_imm(last, 3);
        let address = c.ins().iadd(pointer, displacement);
        // Mutable element memory: no readonly, can_move or alignment claim.
        let flags = MemFlags::new()
            .with_notrap()
            .with_endianness(read_flags.endianness(cranelift_codegen::ir::Endianness::Little));
        let loaded = c.ins().load(carry_ty, flags, address, 0);
        c.ins().jump(join, &[loaded.into()]);
        c.goto_bottom(join);
        c.ins().jump(resume, &[]);
        entry = resume;
        blocks = c.func.layout.blocks().collect();
        value
    } else {
        value_zero
    };
    context.compute_cfg();
    context.compute_domtree();
    let mut c = FuncCursor::new(&mut context.func);
    let mut state = State {
        cfg: &context.cfg,
        definitions,
        incoming: BTreeMap::new(),
        entry,
        zero,
        one,
        initial,
        value_zero,
        seeded: seed.is_some(),
        reachable: blocks
            .iter()
            .copied()
            .filter(|&b| context.domtree.is_reachable(b))
            .collect(),
    };
    let (cached, valid) = if let Some(value) = preceding {
        (value, one)
    } else {
        state.start(c.func, at)
    };
    let count = state.incoming.len();
    if seed.is_some() {
        c.goto_inst(read);
        let cached = if carry_ty == types::I32 {
            if ty == types::I32 {
                cached
            } else {
                c.ins().uextend(types::I64, cached)
            }
        } else if ty == types::I32 {
            c.ins().ireduce(types::I32, cached)
        } else if opcode == Opcode::Uload32 {
            let low = c.ins().ireduce(types::I32, cached);
            c.ins().uextend(types::I64, low)
        } else {
            cached
        };
        c.func.layout.remove_inst(read);
        c.func.dfg.clear_results(read);
        c.func.dfg.change_to_alias(old, cached);
        context.compute_cfg();
        context.compute_domtree();
        return Ok((count, stores.len()));
    }
    let join = c.func.dfg.make_block();
    c.func.layout.split_block(join, after);
    let loaded = c.func.dfg.append_block_param(join, ty);
    let miss = c.func.dfg.make_block();
    c.func.layout.append_block(miss);
    c.func.layout.remove_inst(read);
    c.func.layout.append_inst(read, miss);
    c.goto_bottom(at);
    let cached = if carry_ty == types::I32 {
        if ty == types::I32 {
            cached
        } else {
            c.ins().uextend(types::I64, cached)
        }
    } else if ty == types::I32 {
        c.ins().ireduce(types::I32, cached)
    } else if opcode == Opcode::Uload32 {
        let low = c.ins().ireduce(types::I32, cached);
        c.ins().uextend(types::I64, low)
    } else {
        cached
    };
    let present = c.ins().icmp_imm(IntCC::NotEqual, valid, 0);
    c.ins().brif(present, join, &[cached.into()], miss, &[]);
    c.goto_bottom(miss);
    let miss_jump = c.ins().jump(join, &[old.into()]);
    let insts = c
        .func
        .layout
        .blocks()
        .flat_map(|b| c.func.layout.block_insts(b))
        .collect::<Vec<_>>();
    for inst in insts {
        if inst != miss_jump {
            c.func
                .dfg
                .map_inst_values(inst, |v| if v == old { loaded } else { v });
        }
    }
    context.compute_cfg();
    context.compute_domtree();
    Ok((count, stores.len()))
}
