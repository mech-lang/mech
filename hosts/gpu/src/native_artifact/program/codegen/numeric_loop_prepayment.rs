//! Exact complete-child settlement through the existing current phase loan.
use super::traversal_facts::{Candidate, Completed, Window};
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::prepare::readonly_numeric::Proof;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, Value, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{NativeScalarAggregateAction as Aggregate, NativeTailLoop};

#[derive(serde::Serialize)]
pub(super) struct Prepayment {
    completion: Completed,
    pub counter: usize,
    pub compute: u64,
    pub comparison: u64,
}

pub(super) fn candidate(
    code: CodePlan<'_>,
    proof: &Proof,
    tail: &NativeTailLoop,
    windows: &[Window],
) -> Option<Candidate> {
    let phase = proof.phase.as_ref()?;
    if phase.checked_children {
        return None;
    }
    let debit = phase
        .loops
        .iter()
        .find(|d| d.header == tail.header() as usize)?;
    let active = debit.trips.as_ref()?.tag?;
    let CodePlan::Program(program) = code else {
        return None;
    };
    let layout = &program.metadata().scalar_aggregates;
    // The candidate uses the same exact carrier assignments as emission.
    for (site, source) in [
        (tail.initialize(), tail.seed()),
        (tail.latch(), tail.next()),
    ] {
        if !matches!(layout.action(site as usize), Some(Aggregate::Copy { source: from, output })
            if *from == source && *output == tail.carrier())
        {
            return None;
        }
    }
    let (tag, width) = layout.slot(tail.carrier())?;
    let (next_tag, next_width) = layout.slot(tail.next())?;
    if width != next_width {
        return None;
    }
    let window = windows.first()?;
    let offset = window.induction.checked_sub(tag)?;
    if offset == 0 || offset >= width {
        return None;
    }
    let mech_core::SchemaBody::Enum { variants, .. } = program.plan().values()
        [tail.carrier().get() as usize]
        .definition()
        .body()
    else {
        return None;
    };
    if variants.len() != 2 || active > 1 {
        return None;
    }
    Some(Candidate {
        tag,
        active,
        terminal: 1 - active,
        induction: window.induction,
        count: window.count,
        next_tag,
        next_induction: next_tag.checked_add(offset)?,
    })
}

impl Prepayment {
    pub fn new(proof: &Proof, header: usize, completion: Completed) -> Option<Self> {
        let phase = proof.phase.as_ref()?;
        if phase.checked_children {
            return None;
        }
        let debit = phase.loops.iter().find(|d| d.header == header)?;
        Some(Self {
            completion,
            counter: debit.counter,
            compute: debit.compute,
            comparison: debit.comparison,
        })
    }

    /// Diagnostic computation contract: retain actual enum/window safety and
    /// complete-operation proof, without introducing visit accounting state.
    pub fn native_gate(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        valid: Value,
        selected: Block,
        checked: Block,
    ) {
        let c = self.completion.candidate();
        let tag = words.read(b, c.tag);
        let active = b.ins().icmp_imm(IntCC::Equal, tag, c.active as i64);
        let terminal = b.ins().icmp_imm(IntCC::Equal, tag, c.terminal as i64);
        let known = b.ins().bor(active, terminal);
        let enough = b.ins().band(valid, known);
        b.ins().brif(enough, selected, &[], checked, &[]);
    }

    /// Overflow or insufficient current funding declines to the original header.
    /// All fallible source operations were proved total for this activation.
    pub fn gate(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        state: usize,
        shadow: usize,
        valid: Value,
        selected: Block,
        checked: Block,
    ) {
        let c = self.completion.candidate();
        let tag = words.read(b, c.tag);
        let active = b.ins().icmp_imm(IntCC::Equal, tag, c.active as i64);
        let terminal = b.ins().icmp_imm(IntCC::Equal, tag, c.terminal as i64);
        let known = b.ins().bor(active, terminal);
        let start = words.read(b, c.induction);
        let stop = words.read(b, c.count);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, start, stop);
        let difference = b.ins().isub(stop, start);
        let zero = b.ins().iconst(types::I64, 0);
        let remaining = b.ins().select(more, difference, zero);
        let safe = b
            .ins()
            .icmp_imm(IntCC::UnsignedLessThanOrEqual, remaining, -3);
        let visits = b.ins().iadd_imm(remaining, 2);
        let one = b.ins().iconst(types::I64, 1);
        let visits = b.ins().select(active, visits, one);
        let mut enough = b.ins().band(valid, known);
        enough = b.ins().band(enough, safe);
        let mut balances = Vec::new();
        for (word, price) in [(state + 1, self.compute), (state + 2, self.comparison)] {
            let remaining = words.read(b, word);
            let price = b.ins().iconst(types::I64, price as i64);
            let high = b.ins().umulhi(visits, price);
            let fits = b.ins().icmp_imm(IntCC::Equal, high, 0);
            let cost = b.ins().imul(visits, price);
            let funded = b
                .ins()
                .icmp(IntCC::UnsignedGreaterThanOrEqual, remaining, cost);
            enough = b.ins().band(enough, fits);
            enough = b.ins().band(enough, funded);
            balances.push((word, b.ins().isub(remaining, cost)));
        }
        let count = words.read(b, shadow + self.counter);
        let space = b.ins().bnot(count);
        let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, visits, space);
        enough = b.ins().band(enough, fits);
        let count = b.ins().iadd(count, visits);
        let funded = b.create_block();
        b.ins().brif(enough, funded, &[], checked, &[]);
        b.switch_to_block(funded);
        for (word, balance) in balances {
            words.write(b, word, balance);
        }
        words.write(b, shadow + self.counter, count);
        b.ins().jump(selected, &[]);
    }
}
