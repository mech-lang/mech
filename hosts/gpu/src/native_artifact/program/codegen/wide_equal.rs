//! Widen an admitted pure equality recurrence through its own loop header.
//! A mismatch re-enters the scalar header with the exact current position.
//! This experiment applies only to compact phases, never ordinary work debits.
use super::{numeric_loop_windows::Version, numeric_read_packs::Pack};
use crate::native_artifact::program::prepare::readonly_numeric::Proof;
use cranelift_codegen::Context;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{
    Block, BlockArg, Endianness, Function, Inst, InstBuilder, InstructionData as Data, MemFlags,
    Opcode, Value, ValueDef, types,
};
use std::collections::BTreeSet;
#[path = "wide_equal_prefix.rs"]
mod prefix;
#[path = "wide_equal_reuse.rs"]
mod reuse;

pub(in crate::native_artifact) struct Certificate {
    header: Block,
    blocks: BTreeSet<Block>,
    source_header: usize,
    reuse_scope: Option<reuse::Scope>,
}

pub(super) fn certify(version: &Version, proof: &Proof, packs: &[Pack]) -> Option<Certificate> {
    let [a, b] = version.captures.as_slice() else {
        return None;
    };
    if a.width != 8
        || b.width != 8
        || a.induction != b.induction
        || a.limit != b.limit
        || version.addresses.len() != 2
        || version.funding.is_some()
    {
        return None;
    }
    for window in [a, b] {
        let array = proof.arrays.get(window.array)?;
        if array.width != 1
            || array.writable
            || array.appends
            || !version.facts.flat_arrays.contains(&window.array)
        {
            return None;
        }
    }
    for address in &version.addresses {
        let pack = packs.iter().find(|p| p.first == address.site)?;
        if pack.width != 8
            || pack.offset != 1
            || !pack.source_window
            || !pack.confined_to(&version.blocks)
        {
            return None;
        }
    }
    Some(Certificate {
        header: *version.blocks.get(&(version.header as u32))?,
        blocks: version.blocks.values().copied().collect(),
        source_header: version.header,
        reuse_scope: None,
    })
}

impl Certificate {
    pub(super) fn with_reuse_scope(
        mut self,
        code: crate::native_artifact::codegen::control::plan::CodePlan<'_>,
        proof: &Proof,
        version: &Version,
        blocks: &std::collections::BTreeMap<u32, Block>,
    ) -> Self {
        self.reuse_scope = reuse::scope(code, proof, version, blocks);
        self
    }
}

struct Resolve<'a> {
    context: &'a Context,
    header: Block,
}
impl Resolve<'_> {
    fn value(&self, mut value: Value) -> Option<Value> {
        let f = &self.context.func;
        let mut seen = BTreeSet::new();
        loop {
            value = f.dfg.resolve_aliases(value);
            if !seen.insert(value) {
                return None;
            }
            let ValueDef::Param(block, ordinal) = f.dfg.value_def(value) else {
                return Some(value);
            };
            if block == self.header {
                return Some(value);
            }
            let mut incoming = None;
            for pred in self.context.cfg.pred_iter(block) {
                for edge in f.dfg.insts[pred.inst]
                    .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
                {
                    if edge.block(&f.dfg.value_lists) != block {
                        continue;
                    }
                    let current = edge.args(&f.dfg.value_lists).nth(ordinal)?.as_value()?;
                    if incoming.replace(current).is_some() {
                        return Some(value);
                    }
                }
            }
            let Some(next) = incoming else {
                return Some(value);
            };
            value = next;
        }
    }
    fn inst(&self, value: Value) -> Option<Inst> {
        let ValueDef::Result(inst, 0) = self.context.func.dfg.value_def(self.value(value)?) else {
            return None;
        };
        Some(inst)
    }
    fn constant(&self, value: Value) -> Option<u64> {
        let Data::UnaryImm {
            opcode: Opcode::Iconst,
            imm,
        } = self.context.func.dfg.insts[self.inst(value)?]
        else {
            return None;
        };
        Some(imm.bits() as u64)
    }
    fn invariant(&self, value: Value) -> Option<Value> {
        let value = self.value(value)?;
        let f = &self.context.func;
        let before = match f.dfg.value_def(value) {
            ValueDef::Result(inst, _) => {
                self.context.domtree.dominates(inst, self.header, &f.layout)
            }
            ValueDef::Param(block, _) => {
                block != self.header && self.context.domtree.block_dominates(block, self.header)
            }
            _ => false,
        };
        before.then_some(value)
    }
    fn add(&self, value: Value, at: Value, amount: u64) -> bool {
        let Some(inst) = self.inst(value) else {
            return false;
        };
        match self.context.func.dfg.insts[inst] {
            Data::Binary {
                opcode: Opcode::Iadd,
                args,
            } => {
                (self.value(args[0]) == Some(at) && self.constant(args[1]) == Some(amount))
                    || (self.value(args[1]) == Some(at) && self.constant(args[0]) == Some(amount))
            }
            Data::BinaryImm64 {
                opcode: Opcode::IaddImm,
                arg,
                imm,
            } => self.value(arg) == Some(at) && imm.bits() as u64 == amount,
            _ => false,
        }
    }
    fn predicate(&self, mut value: Value, mut yes: bool) -> Option<(IntCC, Value, Value)> {
        let f = &self.context.func;
        let mut seen = BTreeSet::new();
        loop {
            value = self.value(value)?;
            if !seen.insert(value) {
                return None;
            }
            match f.dfg.insts[self.inst(value)?] {
                Data::Unary {
                    opcode: Opcode::Uextend,
                    arg,
                } => value = arg,
                Data::IntCompare { cond, args, .. } => {
                    if matches!(cond, IntCC::Equal | IntCC::NotEqual) {
                        let (inner, constant) = if let Some(v) = self.constant(args[1]) {
                            (args[0], v)
                        } else if let Some(v) = self.constant(args[0]) {
                            (args[1], v)
                        } else {
                            return Some((
                                if yes { cond } else { cond.complement() },
                                self.value(args[0])?,
                                self.value(args[1])?,
                            ));
                        };
                        let inner_inst = self.inst(inner)?;
                        if constant <= 1
                            && matches!(
                                f.dfg.insts[inner_inst],
                                Data::Unary {
                                    opcode: Opcode::Uextend,
                                    ..
                                } | Data::IntCompare { .. }
                                    | Data::IntCompareImm { .. }
                            )
                        {
                            yes ^= (constant == 0) == (cond == IntCC::Equal);
                            value = inner;
                            continue;
                        }
                    }
                    return Some((
                        if yes { cond } else { cond.complement() },
                        self.value(args[0])?,
                        self.value(args[1])?,
                    ));
                }
                _ => return None,
            }
        }
    }
}

struct Trace {
    instructions: Vec<Inst>,
    end: Inst,
}
fn trace(f: &Function, cert: &Certificate, start: Block, back: bool) -> Option<Trace> {
    let mut block = start;
    let mut seen = BTreeSet::new();
    let mut instructions = Vec::new();
    loop {
        if !cert.blocks.contains(&block) || !seen.insert(block) {
            return None;
        }
        let end = f.layout.last_inst(block)?;
        instructions.extend(f.layout.block_insts(block).filter(|&i| i != end));
        match f.dfg.insts[end] {
            Data::Jump { destination, .. } => {
                block = destination.block(&f.dfg.value_lists);
                if back && block == cert.header {
                    return Some(Trace { instructions, end });
                }
            }
            Data::Brif { .. } if !back => return Some(Trace { instructions, end }),
            _ => return None,
        }
    }
}

#[derive(Clone, Copy)]
struct Change {
    entry: Inst,
    limit: Value,
    bases: [Value; 2],
    limit_check: Inst,
    limit_exit: Block,
}
impl Change {
    fn initial(self, f: &Function, cert: &Certificate) -> Option<Value> {
        f.dfg.insts[self.entry]
            .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
            .iter()
            .find(|edge| edge.block(&f.dfg.value_lists) == cert.header)?
            .args(&f.dfg.value_lists)
            .next()?
            .as_value()
    }
}

fn candidate(context: &Context, cert: &Certificate) -> Option<Change> {
    let f = &context.func;
    let &[at] = f.dfg.block_params(cert.header) else {
        return None;
    };
    if f.dfg.value_type(at) != types::I64 {
        return None;
    }
    let r = Resolve {
        context,
        header: cert.header,
    };
    let header = trace(f, cert, cert.header, false)?;
    let Data::Brif { arg, blocks, .. } = f.dfg.insts[header.end] else {
        return None;
    };
    if blocks[0].block(&f.dfg.value_lists) == blocks[1].block(&f.dfg.value_lists) {
        return None;
    }
    let (cc, sum, limit) = r.predicate(arg, true)?;
    let body_index = match cc {
        IntCC::UnsignedLessThanOrEqual => 0,
        IntCC::UnsignedGreaterThan => 1,
        _ => return None,
    };
    if !r.add(sum, at, 8) {
        return None;
    }
    let limit = r.invariant(limit)?;
    let body = trace(f, cert, blocks[body_index].block(&f.dfg.value_lists), false)?;
    let Data::Brif {
        arg,
        blocks: compared,
        ..
    } = f.dfg.insts[body.end]
    else {
        return None;
    };
    let (cc, a, b) = r.predicate(arg, true)?;
    let equal_index = match cc {
        IntCC::Equal => 0,
        IntCC::NotEqual => 1,
        _ => return None,
    };
    let difference = if r.constant(b) == Some(0) {
        a
    } else if r.constant(a) == Some(0) {
        b
    } else {
        return None;
    };
    let Data::Binary {
        opcode: Opcode::Bxor,
        args,
    } = f.dfg.insts[r.inst(difference)?]
    else {
        return None;
    };
    let mut bases = Vec::new();
    for value in args {
        let inst = r.inst(value)?;
        if f.dfg.value_type(f.dfg.inst_results(inst)[0]) != types::I64 {
            return None;
        }
        let Data::Load {
            arg, offset, flags, ..
        } = f.dfg.insts[inst]
        else {
            return None;
        };
        if i32::from(offset) != 0 || flags.endianness(Endianness::Little) != Endianness::Little {
            return None;
        }
        let Data::Binary {
            opcode: Opcode::Iadd,
            args,
        } = f.dfg.insts[r.inst(arg)?]
        else {
            return None;
        };
        let base = if r.value(args[0])? == at {
            r.invariant(args[1])?
        } else if r.value(args[1])? == at {
            r.invariant(args[0])?
        } else {
            return None;
        };
        bases.push(base);
    }
    let back = trace(
        f,
        cert,
        compared[equal_index].block(&f.dfg.value_lists),
        true,
    )?;
    let Data::Jump { destination, .. } = f.dfg.insts[back.end] else {
        return None;
    };
    let args = destination.args(&f.dfg.value_lists).collect::<Vec<_>>();
    if args.len() != 1 || !r.add(args[0].as_value()?, at, 8) {
        return None;
    }
    // The cycle must contain only exactly these pure scalar predicates,
    // addresses, equality and immutable loads. No debit/store/call/check can be
    // bypassed. A sole induction phi excludes an observed extra carried value.
    let loads = body
        .instructions
        .iter()
        .filter(|&&i| f.dfg.insts[i].opcode() == Opcode::Load)
        .count();
    if loads != 2 {
        return None;
    }
    for inst in header
        .instructions
        .iter()
        .chain(&body.instructions)
        .chain(&back.instructions)
    {
        if !matches!(
            f.dfg.insts[*inst].opcode(),
            Opcode::Iconst
                | Opcode::Iadd
                | Opcode::IaddImm
                | Opcode::Icmp
                | Opcode::Uextend
                | Opcode::Bxor
                | Opcode::Load
        ) {
            return None;
        }
        if f.dfg.insts[*inst].opcode() == Opcode::Load && !body.instructions.contains(inst) {
            return None;
        }
    }
    // Exactly one initial edge and the proved equal backedge. Do not redirect
    // an additional traversal whose entry facts have not been established.
    let incoming = context
        .cfg
        .pred_iter(cert.header)
        .map(|pred| pred.inst)
        .collect::<BTreeSet<_>>();
    if incoming.len() != 2
        || !incoming.contains(&back.end)
        || incoming.iter().any(|&inst| {
            inst != back.end && context.domtree.dominates(cert.header, inst, &f.layout)
        })
    {
        return None;
    }
    Some(Change {
        entry: incoming.into_iter().find(|&inst| inst != back.end)?,
        limit,
        bases: bases.try_into().ok()?,
        limit_check: header.end,
        limit_exit: blocks[1 - body_index].block(&f.dfg.value_lists),
    })
}

// Keep the equality result in vector form until its one Boolean observation.
// This reads exactly the same admitted words and retains the scalar mismatch
// continuation; only the pure reduction of already-loaded bits changes.
fn any_bits(c: &mut FuncCursor<'_>, difference: Value) -> Value {
    if std::env::var_os("MECH_WIDE_ANY_BITS").is_some() {
        c.ins().vany_true(difference)
    } else {
        let low = c.ins().extractlane(difference, 0);
        let high = c.ins().extractlane(difference, 1);
        c.ins().bor(low, high)
    }
}

enum SavedInterval {
    Long(reuse::State, Option<Value>),
    Prefix(reuse::State, u64, prefix::Handoff),
}

fn apply(
    f: &mut Function,
    cert: &Certificate,
    change: Change,
    state: Option<SavedInterval>,
) -> Option<(usize, reuse::State, Value)> {
    let check = f.dfg.make_block();
    let initial = f.dfg.append_block_param(check, types::I64);
    let wide = f.dfg.make_block();
    let wide_at = f.dfg.append_block_param(wide, types::I64);
    let advance = f.dfg.make_block();
    let short_check = f.dfg.make_block();
    let short_initial = f.dfg.append_block_param(short_check, types::I64);
    let short_wide = f.dfg.make_block();
    let short_at = f.dfg.append_block_param(short_wide, types::I64);
    let short_advance = f.dfg.make_block();
    for block in [check, wide, advance, short_check, short_wide, short_advance] {
        f.layout.append_block(block);
    }
    // The capture gate establishes initial <= limit. Once an immutable group
    // differs, its first differing word is reachable by the unchanged scalar
    // loop. A sub-vector remainder likewise stays in that scalar loop.
    f.rewrite_branch_destination(change.entry, cert.header, check);
    let mut c = FuncCursor::new(f).at_bottom(check);
    let mut forwarded = None;
    let initial = match state {
        Some(SavedInterval::Long(state, Some(end))) => {
            state.advance_forwarded(&mut c, &change, initial, end)
        }
        Some(SavedInterval::Long(state, None)) => state.advance(&mut c, &change, initial, false),
        Some(SavedInterval::Prefix(state, width, handoff)) => {
            let end = state.proved_prefix_end(&mut c, &change, initial);
            let compare = c.func.dfg.make_block();
            c.func.layout.append_block(compare);
            c.ins().brif(
                end,
                cert.header,
                &[BlockArg::Value(change.limit)],
                compare,
                &[],
            );
            state.record_prefix(c.func, &change, width, end);
            let header = handoff.header;
            let parameter = handoff.emit(c.func, end);
            forwarded = Some((header, state, parameter));
            c.goto_bottom(compare);
            initial
        }
        None => initial,
    };
    let remaining = c.ins().isub(change.limit, initial);
    let last_wide = c.ins().iadd_imm(change.limit, -64);
    let enough = c
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 64);
    c.ins().brif(
        enough,
        wide,
        &[BlockArg::Value(initial)],
        short_check,
        &[BlockArg::Value(initial)],
    );
    c.goto_bottom(wide);
    let flags = MemFlags::new().with_endianness(Endianness::Little);
    let left = c.ins().iadd(change.bases[0], wide_at);
    let right = c.ins().iadd(change.bases[1], wide_at);
    // A failed group restarts at the same scalar word. The certificate
    // excludes effects and observable intermediate state from the cycle.
    let mut combined = None;
    for offset in [0, 16, 32, 48] {
        let a = c.ins().load(types::I64X2, flags, left, offset);
        let b = c.ins().load(types::I64X2, flags, right, offset);
        let difference = c.ins().bxor(a, b);
        combined = Some(match combined {
            Some(previous) => c.ins().bor(previous, difference),
            None => difference,
        });
    }
    let differs = any_bits(&mut c, combined.unwrap());
    c.ins().brif(
        differs,
        cert.header,
        &[BlockArg::Value(wide_at)],
        advance,
        &[],
    );
    c.goto_bottom(advance);
    // Entry proves limit >= 64 and wide_at <= limit - 64. The same
    // comparison at the latch proves the next complete group readable.
    let next = c.ins().iadd_imm(wide_at, 64);
    let enough = c
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, next, last_wide);
    c.ins().brif(
        enough,
        wide,
        &[BlockArg::Value(next)],
        short_check,
        &[BlockArg::Value(next)],
    );

    c.goto_bottom(short_check);
    let remaining = c.ins().isub(change.limit, short_initial);
    let last_short = c.ins().iadd_imm(change.limit, -16);
    let enough = c
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 16);
    c.ins().brif(
        enough,
        short_wide,
        &[BlockArg::Value(short_initial)],
        cert.header,
        &[BlockArg::Value(short_initial)],
    );
    c.goto_bottom(short_wide);
    let left = c.ins().iadd(change.bases[0], short_at);
    let right = c.ins().iadd(change.bases[1], short_at);
    let a = c.ins().load(types::I64X2, flags, left, 0);
    let b = c.ins().load(types::I64X2, flags, right, 0);
    let difference = c.ins().bxor(a, b);
    let differs = any_bits(&mut c, difference);
    c.ins().brif(
        differs,
        cert.header,
        &[BlockArg::Value(short_at)],
        short_advance,
        &[],
    );
    c.goto_bottom(short_advance);
    // Remaining length only decreases, so an exhausted short tier can
    // proceed straight to the scalar remainder at the current position.
    let next = c.ins().iadd_imm(short_at, 16);
    let enough = c
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, next, last_short);
    c.ins().brif(
        enough,
        short_wide,
        &[BlockArg::Value(next)],
        cert.header,
        &[BlockArg::Value(next)],
    );
    forwarded
}

#[derive(Default)]
pub(in crate::native_artifact) struct Outcome {
    pub wider: Vec<usize>,
    pub reused: Vec<(usize, usize)>,
    pub prefixes: Vec<(usize, usize)>,
}

pub(in crate::native_artifact) fn optimize(
    context: &mut Context,
    certificates: &[Certificate],
    reuse_prefix: bool,
) -> Outcome {
    let mut outcome = Outcome::default();
    let mut continuations = std::collections::BTreeMap::new();
    for (index, cert) in certificates.iter().enumerate() {
        context.compute_cfg();
        context.compute_domtree();
        let Some(change) = candidate(context, cert) else {
            continue;
        };
        let mut saved = None;
        if reuse_prefix && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            let extent = reuse::extent(context, cert, &change);
            eprintln!(
                "equality entry {} {:?}: {extent:?}, limit exit {}",
                cert.source_header, cert.reuse_scope, change.limit_exit
            );
        }
        if let Some(scope) = cert.reuse_scope {
            if let Some((state, end)) = continuations.remove(&cert.source_header) {
                saved = Some(SavedInterval::Long(state, Some(end)));
                outcome.reused.push((cert.source_header, scope.header));
            } else {
                match reuse::extent(context, cert, &change) {
                    Some(reuse::Extent::Fixed(width))
                        if reuse_prefix && width >= 8 && width % 8 == 0 =>
                    {
                        if let Some(handoff) = prefix::continuation(
                            context,
                            cert,
                            &change,
                            scope,
                            &certificates[index + 1..],
                            &continuations,
                        ) && let Some(state) = reuse::begin(context, scope, &change)
                        {
                            saved = Some(SavedInterval::Prefix(state, width, handoff));
                        }
                    }
                    Some(reuse::Extent::Variable) => {
                        if let Some(state) = reuse::begin(context, scope, &change) {
                            saved = Some(SavedInterval::Long(state, None));
                            outcome.reused.push((cert.source_header, scope.header));
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(SavedInterval::Long(state, _)) = &saved {
            state.record_words(&mut context.func, cert, &change);
        }
        if let Some((header, state, end)) = apply(&mut context.func, cert, change, saved) {
            continuations.insert(header, (state, end));
            outcome.prefixes.push((cert.source_header, header));
        }
        outcome.wider.push(cert.source_header);
    }
    if !outcome.wider.is_empty() {
        context.compute_cfg();
        context.compute_domtree();
    }
    outcome
}

#[cfg(test)]
#[path = "wide_equal_tests.rs"]
mod tests;
