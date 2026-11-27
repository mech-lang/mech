//! Move an array bound to phase entry when the loop proves every index smaller
//! than a captured end word. Decreasing loop indices need a nonwrapping step.
use super::super::{incoming_block_arguments, word_ranges::constant_offset};
use super::{Admission, comparison, fixed_word, sole_predecessor};
use cranelift_codegen::Context;
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{
    Block, Function, Inst, InstBuilder, InstructionData as Data, Opcode, Value, ValueDef, types,
};
use std::collections::{BTreeMap, BTreeSet};

struct Order {
    lower: Value,
    upper: Value,
    strict: bool,
    guard: Block,
}

fn current(f: &Function, value: Value) -> Value {
    let (base, offset) = constant_offset(f, value);
    if offset == 0 {
        base
    } else {
        f.dfg.resolve_aliases(value)
    }
}

fn selected_orders(ctx: &Context) -> Vec<Order> {
    let f = &ctx.func;
    let mut orders = Vec::new();
    for block in f.layout.blocks() {
        let Some(branch) = f.layout.last_inst(block) else {
            continue;
        };
        let Data::Brif { arg, blocks, .. } = f.dfg.insts[branch] else {
            continue;
        };
        let Some(test) = comparison(f, arg) else {
            continue;
        };
        let Data::IntCompare { cond, args, .. } = f.dfg.insts[test] else {
            continue;
        };
        if args
            .iter()
            .any(|&word| f.dfg.value_type(word) != types::I64)
        {
            continue;
        }
        let [yes, no] = blocks.map(|call| call.block(&f.dfg.value_lists));
        if yes == no {
            continue;
        }
        let [a, b] = args.map(|value| current(f, value));
        for (guard, cond) in [(yes, cond), (no, cond.complement())] {
            if Some(guard) == f.layout.entry_block()
                || sole_predecessor(&ctx.cfg, guard).map(|(_, inst)| inst) != Some(branch)
            {
                continue;
            }
            let (lower, upper, strict) = match cond {
                IntCC::UnsignedLessThan => (a, b, true),
                IntCC::UnsignedLessThanOrEqual => (a, b, false),
                IntCC::UnsignedGreaterThan => (b, a, true),
                IntCC::UnsignedGreaterThanOrEqual => (b, a, false),
                _ => continue,
            };
            orders.push(Order {
                lower,
                upper,
                strict,
                guard,
            });
        }
    }
    orders
}

struct Proof<'a> {
    ctx: &'a Context,
    end: Value,
    orders: &'a [Order],
    cursors: BTreeSet<Value>,
}

impl Proof<'_> {
    fn ordered(&self, lower: Value, upper: Value, strict: bool, at: Block) -> bool {
        self.orders.iter().any(|order| {
            order.lower == lower
                && order.upper == upper
                && (!strict || order.strict)
                && self.ctx.domtree.block_dominates(order.guard, at)
        })
    }

    fn below(&self, value: Value, at: Block, hypothesis: Option<(Value, Block)>) -> bool {
        self.prove(value, at, hypothesis, 24, &mut 512)
    }

    fn prove(
        &self,
        value: Value,
        at: Block,
        hypothesis: Option<(Value, Block)>,
        depth: usize,
        fuel: &mut usize,
    ) -> bool {
        if depth == 0 || *fuel == 0 {
            return false;
        }
        *fuel -= 1;
        let f = &self.ctx.func;
        if f.dfg.value_type(value) != types::I64 {
            return false;
        }
        let value = current(f, value);
        if self.cursors.contains(&value)
            || self.ordered(value, self.end, true, at)
            || hypothesis.is_some_and(|(cursor, header)| {
                cursor == value && self.ctx.domtree.block_dominates(header, at)
            })
        {
            return true;
        }
        match f.dfg.value_def(value) {
            ValueDef::Param(block, slot) => {
                let mut inputs = incoming_block_arguments(f, &self.ctx.cfg, block, slot)
                    .filter(|(pred, _)| self.ctx.domtree.is_reachable(*pred))
                    .peekable();
                inputs.peek().is_some()
                    && inputs.all(|(pred, value)| {
                        value.is_some_and(|value| {
                            self.prove(value, pred, hypothesis, depth - 1, fuel)
                        })
                    })
            }
            ValueDef::Result(inst, 0) => {
                let Data::Binary {
                    opcode: Opcode::Isub,
                    args,
                } = f.dfg.insts[inst]
                else {
                    return false;
                };
                let [start, step] = args.map(|value| current(f, value));
                f.layout.inst_block(inst).is_some_and(|block| {
                    self.ordered(step, start, false, block)
                        && self.prove(start, block, hypothesis, depth - 1, fuel)
                })
            }
            _ => false,
        }
    }

    fn inductive(&self, cursor: Value, header: Block, slot: usize) -> bool {
        let mut initialized = false;
        let mut recurs = false;
        for (pred, value) in incoming_block_arguments(&self.ctx.func, &self.ctx.cfg, header, slot)
            .filter(|(pred, _)| self.ctx.domtree.is_reachable(*pred))
        {
            let latch = self.ctx.domtree.block_dominates(header, pred);
            initialized |= !latch;
            recurs |= latch;
            // A first entry establishes the bound without assuming it. On
            // each latch the hypothesis refers to that activation's cursor.
            let hypothesis = latch.then_some((cursor, header));
            if !value.is_some_and(|value| self.below(value, pred, hypothesis)) {
                return false;
            }
        }
        initialized && recurs
    }
}

struct Check {
    inst: Inst,
    block: Block,
    index: Value,
    length_word: usize,
    outcome: i64,
}

/// The returned admission must guard the exact current packet words before
/// selecting this body. A failed admission resumes the original checked CFG.
pub(in crate::native_artifact) fn fold(
    ctx: &mut Context,
    loads: &BTreeMap<Value, (Inst, usize)>,
    lengths: &BTreeSet<usize>,
) -> Option<Admission> {
    if lengths.is_empty() || ctx.func.layout.blocks().count() > 4096 {
        return None;
    }
    ctx.compute_cfg();
    ctx.compute_domtree();
    let f = &ctx.func;
    let orders = selected_orders(ctx);
    let ends: BTreeMap<_, _> = orders
        .iter()
        .filter(|order| order.strict)
        .filter_map(|order| {
            fixed_word(f, &ctx.cfg, loads, order.upper).map(|word| (order.upper, word))
        })
        .collect();
    let mut checks = Vec::new();
    for block in f
        .layout
        .blocks()
        .filter(|&block| ctx.domtree.is_reachable(block))
    {
        for inst in f.layout.block_insts(block) {
            let Data::IntCompare {
                cond, args: [a, b], ..
            } = f.dfg.insts[inst]
            else {
                continue;
            };
            let (index, length, outcome) = match cond {
                IntCC::UnsignedLessThan => (a, b, 1),
                IntCC::UnsignedGreaterThan => (b, a, 1),
                IntCC::UnsignedGreaterThanOrEqual => (a, b, 0),
                IntCC::UnsignedLessThanOrEqual => (b, a, 0),
                _ => continue,
            };
            if f.dfg.value_type(index) != types::I64 {
                continue;
            }
            if let Some(length_word) = fixed_word(f, &ctx.cfg, loads, current(f, length))
                .filter(|word| lengths.contains(word))
            {
                checks.push(Check {
                    inst,
                    block,
                    index,
                    length_word,
                    outcome,
                });
            }
        }
    }
    // Select a single guard with the most proved checks. This avoids replacing
    // an isolated dynamic comparison with an extra check of another extent.
    let mut best = Vec::new();
    let mut admission = None;
    for (end, end_word) in ends {
        let mut proof = Proof {
            ctx,
            end,
            orders: &orders,
            cursors: BTreeSet::new(),
        };
        for header in f
            .layout
            .blocks()
            .filter(|&block| ctx.domtree.is_reachable(block))
        {
            for (slot, &value) in f.dfg.block_params(header).iter().enumerate() {
                if f.dfg.value_type(value) == types::I64 && proof.inductive(value, header, slot) {
                    proof.cursors.insert(value);
                }
            }
        }
        if proof.cursors.is_empty() {
            continue;
        }
        let mut matches = BTreeMap::<_, Vec<_>>::new();
        for check in &checks {
            if check.length_word != end_word && proof.below(check.index, check.block, None) {
                matches
                    .entry(check.length_word)
                    .or_default()
                    .push((check.inst, check.outcome));
            }
        }
        for (length_word, proved) in matches {
            if proved.len() >= 2 && proved.len() > best.len() {
                admission = Some(Admission {
                    begin_word: None,
                    end_word,
                    length_word,
                    minimum_length: 0,
                    checks_removed: proved.len(),
                    window_checks_removed: 0,
                });
                best = proved;
            }
        }
    }
    let admission = admission?;
    for (inst, outcome) in best {
        let ty = ctx.func.dfg.value_type(ctx.func.dfg.first_result(inst));
        ctx.func.dfg.replace(inst).iconst(ty, outcome);
    }
    super::super::fold(&mut ctx.func);
    Some(admission)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::{
        ir::{AbiParam, MemFlags},
        settings,
        verifier::verify_function,
    };
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

    fn decreasing_loop(
        step: Opcode,
        direction: IntCC,
    ) -> (Context, BTreeMap<Value, (Inst, usize)>) {
        let mut ctx = Context::new();
        ctx.func.signature.params = vec![AbiParam::new(types::I64); 3];
        ctx.func.signature.returns.push(AbiParam::new(types::I64));
        let mut state = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut state);
        let entry = b.create_block();
        let initial = b.create_block();
        let header = b.create_block();
        let test = b.create_block();
        let latch = b.create_block();
        let done = b.create_block();
        let fail = b.create_block();
        b.append_block_params_for_function_params(entry);
        let cursor = b.append_block_param(header, types::I64);
        b.switch_to_block(entry);
        let [packet, first, amount] = b.block_params(entry).try_into().unwrap();
        let end = b.ins().load(types::I64, MemFlags::new(), packet, 0);
        let length = b.ins().load(types::I64, MemFlags::new(), packet, 8);
        let in_span = b.ins().icmp(IntCC::UnsignedLessThan, first, end);
        b.ins().brif(in_span, initial, &[], fail, &[]);
        b.switch_to_block(initial);
        let valid = b.ins().icmp(IntCC::UnsignedLessThan, first, length);
        b.ins().brif(valid, header, &[first.into()], fail, &[]);
        b.switch_to_block(header);
        let valid = b.ins().icmp(IntCC::UnsignedLessThan, cursor, length);
        b.ins().brif(valid, test, &[], fail, &[]);
        b.switch_to_block(test);
        let safe = b.ins().icmp(direction, cursor, amount);
        b.ins().brif(safe, latch, &[], done, &[]);
        b.switch_to_block(latch);
        let next = if step == Opcode::Isub {
            b.ins().isub(cursor, amount)
        } else {
            b.ins().iadd(cursor, amount)
        };
        let valid = b.ins().icmp(IntCC::UnsignedLessThan, next, length);
        b.ins().brif(valid, header, &[next.into()], fail, &[]);
        b.switch_to_block(done);
        b.ins().return_(&[cursor]);
        b.switch_to_block(fail);
        let status = b.ins().iconst(types::I64, -1);
        b.ins().return_(&[status]);
        b.seal_all_blocks();
        b.finalize();
        let load = |value| match ctx.func.dfg.value_def(value) {
            ValueDef::Result(inst, 0) => inst,
            _ => unreachable!("fixture packet load"),
        };
        let loads = BTreeMap::from([(end, (load(end), 0)), (length, (load(length), 1))]);
        (ctx, loads)
    }

    #[test]
    fn one_extent_guard_covers_the_seed_and_nonwrapping_latches() {
        let (mut ctx, loads) = decreasing_loop(Opcode::Isub, IntCC::UnsignedGreaterThanOrEqual);
        let guarded = fold(&mut ctx, &loads, &BTreeSet::from([1])).unwrap();
        assert_eq!(
            (guarded.begin_word, guarded.end_word, guarded.length_word),
            (None, 0, 1)
        );
        assert_eq!(guarded.checks_removed, 3);
        verify_function(&ctx.func, &settings::Flags::new(settings::builder())).unwrap();
    }

    #[test]
    fn a_wrapping_or_ascending_recurrence_cannot_admit_the_loop() {
        for (step, direction) in [
            (Opcode::Iadd, IntCC::UnsignedGreaterThanOrEqual),
            (Opcode::Isub, IntCC::UnsignedLessThan),
        ] {
            let (mut ctx, loads) = decreasing_loop(step, direction);
            let original = ctx.func.to_string();
            assert!(fold(&mut ctx, &loads, &BTreeSet::from([1])).is_none());
            assert_eq!(ctx.func.to_string(), original);
        }
    }
}
