//! Use a checked full extent to prove that a smaller current byte offset
//! fits. The comparisons are about scalar SSA values and do not move reads.
use super::super::window_bounds::comparison;
use super::super::word_ranges::literal;
use super::incoming;
use super::relations::replace_proved;
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{
    Block, Function, Inst, InstructionData as Data, Opcode, Value, ValueDef, types,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
struct Order {
    lower: Value,
    upper: Value,
    strict: bool,
}

fn order(f: &Function, inst: Inst, selected: bool) -> Option<Order> {
    let Data::IntCompare {
        cond,
        args: [left, right],
        ..
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    if f.dfg.value_type(left) != types::I64 || f.dfg.value_type(right) != types::I64 {
        return None;
    }
    let cond = if selected { cond } else { cond.complement() };
    let (lower, upper, strict) = match cond {
        IntCC::UnsignedLessThan => (left, right, true),
        IntCC::UnsignedLessThanOrEqual => (left, right, false),
        IntCC::UnsignedGreaterThan => (right, left, true),
        IntCC::UnsignedGreaterThanOrEqual => (right, left, false),
        _ => return None,
    };
    Some(Order {
        lower,
        upper,
        strict,
    })
}

struct Proof<'a> {
    f: &'a Function,
    cfg: &'a ControlFlowGraph,
    dom: &'a DominatorTree,
}

impl Proof<'_> {
    fn before(&self, value: Value, block: Block) -> bool {
        match self.f.dfg.value_def(value) {
            ValueDef::Param(owner, _) => owner != block && self.dom.block_dominates(owner, block),
            ValueDef::Result(inst, _) => self.dom.dominates(inst, block, &self.f.layout),
            ValueDef::Union(..) => false,
        }
    }

    fn incoming_values(&self, block: Block, slot: usize) -> Option<Vec<Value>> {
        let f = self.f;
        let mut values = Vec::new();
        for predecessor in self.cfg.pred_iter(block) {
            for call in f.dfg.insts[predecessor.inst]
                .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
            {
                if call.block(&f.dfg.value_lists) == block {
                    let value = call.args(&f.dfg.value_lists).nth(slot)?.as_value()?;
                    values.push(f.dfg.resolve_aliases(value));
                }
            }
        }
        (!values.is_empty()).then_some(values)
    }

    fn current(&self, mut value: Value) -> Value {
        value = self.f.dfg.resolve_aliases(value);
        // Only peel a join whose inputs are one identical earlier word. A
        // different latch value or a dynamic join remains its own SSA value.
        while let ValueDef::Param(block, slot) = self.f.dfg.value_def(value) {
            let Some(values) = self.incoming_values(block, slot) else {
                break;
            };
            let first = values[0];
            if !values.iter().all(|&arg| arg == first) || !self.before(first, block) {
                break;
            }
            value = first;
        }
        value
    }

    fn definition(&self, value: Value) -> Option<(Inst, Data)> {
        let ValueDef::Result(inst, 0) = self.f.dfg.value_def(self.current(value)) else {
            return None;
        };
        Some((inst, self.f.dfg.insts[inst]))
    }

    fn sum(&self, value: Value) -> Option<[Value; 2]> {
        let (
            _,
            Data::Binary {
                opcode: Opcode::Iadd,
                args,
            },
        ) = self.definition(value)?
        else {
            return None;
        };
        Some(args.map(|word| self.current(word)))
    }

    fn selected_orders(&self, mut block: Block) -> Vec<Order> {
        let mut facts = Vec::new();
        loop {
            if let Some((condition, selected)) = incoming(self.f, self.cfg, block)
                && let Some(inst) = comparison(self.f, condition)
                && let Some(fact) = order(self.f, inst, selected)
            {
                facts.push(Order {
                    lower: self.current(fact.lower),
                    upper: self.current(fact.upper),
                    strict: fact.strict,
                });
            }
            let Some(parent) = self.dom.idom(block) else {
                break;
            };
            block = parent;
        }
        facts
    }

    fn ordered_base(&self, lower: Value, upper: Value, orders: &[Order]) -> bool {
        if lower == upper
            || literal(self.f, lower) == Some(0)
            || contains(orders, lower, upper, false)
        {
            return true;
        }
        let ValueDef::Param(block, slot) = self.f.dfg.value_def(lower) else {
            return false;
        };
        if !self.before(upper, block) {
            return false;
        }
        self.incoming_values(block, slot).is_some_and(|values| {
            values.into_iter().all(|arg| {
                if literal(self.f, arg) == Some(0) || self.current(arg) == upper {
                    return true;
                }
                let Some((
                    inst,
                    Data::Binary {
                        opcode: Opcode::Isub,
                        args: [base, delta],
                    },
                )) = self.definition(arg)
                else {
                    return false;
                };
                self.current(base) == upper
                    && self.f.layout.inst_block(inst).is_some_and(|at| {
                        contains(&self.selected_orders(at), self.current(delta), upper, false)
                    })
            })
        })
    }

    fn increment_in_span(
        &self,
        index: Value,
        next: Value,
        mut block: Block,
        readable: &BTreeSet<Inst>,
    ) -> bool {
        let index = self.current(index);
        let Some([a, b]) = self.sum(next) else {
            return false;
        };
        if ![(a, b), (b, a)]
            .into_iter()
            .any(|(value, step)| value == index && literal(self.f, step) == Some(1))
        {
            return false;
        }
        loop {
            if let Some((condition, selected)) = incoming(self.f, self.cfg, block)
                && let Some(guard) = comparison(self.f, condition)
                && readable.contains(&guard)
                && let Some(bound) = order(self.f, guard, selected)
                && bound.strict
                && self.current(bound.lower) == index
            {
                return true;
            }
            let Some(parent) = self.dom.idom(block) else {
                return false;
            };
            block = parent;
        }
    }

    fn fits(&self, value: Value, length: Value, block: Block) -> bool {
        let Some([left, right]) = self.sum(value) else {
            return false;
        };
        let length = self.current(length);
        let orders = self.selected_orders(block);
        for &Order {
            lower: end, upper, ..
        } in &orders
        {
            if upper != length {
                continue;
            }
            let Some([a, b]) = self.sum(end) else {
                continue;
            };
            for (base, maximum) in [(a, b), (b, a)] {
                if !contains(&orders, base, end, false) && !contains(&orders, maximum, end, false) {
                    continue;
                }
                if [(left, right), (right, left)]
                    .into_iter()
                    .any(|(candidate, offset)| {
                        contains(&orders, offset, maximum, true)
                            && self.ordered_base(candidate, base, &orders)
                    })
                {
                    return true;
                }
            }
        }
        false
    }
}

fn contains(orders: &[Order], lower: Value, upper: Value, strict: bool) -> bool {
    orders
        .iter()
        .any(|fact| fact.lower == lower && fact.upper == upper && (!strict || fact.strict))
}

pub(in super::super) fn fold(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> usize {
    let proof = Proof { f, cfg, dom };
    let mut edits = Vec::new();
    for block in f.layout.blocks().filter(|&block| dom.is_reachable(block)) {
        for inst in f.layout.block_insts(block) {
            if f.dfg.insts[inst].exception_table().is_some() {
                return 0;
            }
            if let Some(test) = order(f, inst, true)
                && test.strict
                && proof.fits(test.lower, test.upper, block)
            {
                edits.push((inst, true));
            }
        }
    }
    let readable = edits.iter().map(|&(inst, _)| inst).collect::<BTreeSet<_>>();
    if readable.is_empty() {
        return 0;
    }
    for block in f.layout.blocks().filter(|&block| dom.is_reachable(block)) {
        for inst in f.layout.block_insts(block) {
            if let Some(test) = order(f, inst, true)
                && !test.strict
                && proof.increment_in_span(test.lower, test.upper, block, &readable)
            {
                edits.push((inst, true));
            }
        }
    }
    replace_proved(f, &edits)
}

#[cfg(test)]
mod tests;
