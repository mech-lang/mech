//! One activation-current funding bound; actual visits retain their old debits.
//! This is neither whole-loop prepayment nor authority to speculate operations.
use super::traversal_facts::CapturedWindow;
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::prepare::readonly_numeric::Proof;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, Value, types};
use cranelift_frontend::FunctionBuilder;
#[cfg(all(test, feature = "canonical-native-program-runtime"))]
mod probe;
#[cfg(all(test, feature = "canonical-native-program-runtime"))]
pub(in crate::native_artifact) use probe::{
    begin as begin_test_observation, take as take_test_entries,
};

#[derive(serde::Serialize)]
pub(super) struct Funding {
    pub induction: usize,
    pub limit: usize,
    pub step: u64,
    pub counter: usize,
    pub first_visit: usize,
    pub compute: u64,
    pub comparison: u64,
    #[serde(skip)]
    pub first_body: Block,
}

impl Funding {
    pub fn derive(
        proof: &Proof,
        header: usize,
        captures: &[CapturedWindow],
        b: &mut FunctionBuilder<'_>,
    ) -> Option<Self> {
        let phase = proof.phase.as_ref().filter(|p| p.checked_children)?;
        let debit = phase.loops.iter().skip(1).find(|d| d.header == header)?;
        let current = captures.first()?;
        if current.width == 0
            || captures.iter().any(|w| {
                (w.induction, w.limit, w.width) != (current.induction, current.limit, current.width)
            })
        {
            return None;
        }
        // These coordinates come from the existing selected-edge recurrence
        // proof: every latch advances by step, preserves captures, and requires
        // k + step <= limit. The admitted scope is otherwise acyclic. Outside
        // edges resume checked code, never this activation's selected header.
        Some(Self {
            induction: current.induction,
            limit: current.limit,
            step: current.width,
            counter: debit.counter,
            first_visit: debit.first_visit?,
            compute: debit.compute,
            comparison: debit.comparison,
            first_body: b.create_block(),
        })
    }

    pub fn gate(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        state: usize,
        shadow: usize,
        valid: Value,
        checked: Block,
    ) {
        let start = words.read(b, self.induction);
        let limit = words.read(b, self.limit);
        let distance = b.ins().isub(limit, start);
        let stride = b.ins().iconst(types::I64, self.step as i64);
        let remaining_visits = b.ins().udiv(distance, stride);
        let mut enough = valid;
        // Initialization has already charged visit one. Only these additional
        // headers need funding; no charge is taken merely for this maximum.
        for (slot, price) in [(state + 1, self.compute), (state + 2, self.comparison)] {
            if price == 0 {
                continue;
            }
            let price = b.ins().iconst(types::I64, price as i64);
            let high = b.ins().umulhi(remaining_visits, price);
            let fits = b.ins().icmp_imm(IntCC::Equal, high, 0);
            let cost = b.ins().imul(remaining_visits, price);
            let balance = words.read(b, slot);
            let funded = b
                .ins()
                .icmp(IntCC::UnsignedGreaterThanOrEqual, balance, cost);
            enough = b.ins().band(enough, fits);
            enough = b.ins().band(enough, funded);
        }
        let count = words.read(b, shadow + self.counter);
        let space = b.ins().bnot(count);
        let fits = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, remaining_visits, space);
        enough = b.ins().band(enough, fits);
        let selected = b.create_block();
        b.ins().brif(enough, selected, &[], checked, &[]);
        b.switch_to_block(selected);
        #[cfg(all(test, feature = "canonical-native-program-runtime"))]
        probe::emit(b);
        let zero = b.ins().iconst(types::I64, 0);
        words.write(b, shadow + self.first_visit, zero);
        b.ins().jump(self.first_body, &[]);
    }

    /// Only backedges reach this block; initialization enters after its debit.
    /// All existing fallible operations still execute at their lexical sites.
    pub fn header(&self, b: &mut FunctionBuilder<'_>, words: &Words, state: usize, shadow: usize) {
        for (slot, price) in [(state + 1, self.compute), (state + 2, self.comparison)] {
            if price != 0 {
                let balance = words.read(b, slot);
                let balance = b.ins().iadd_imm(balance, (price as i64).wrapping_neg());
                words.write(b, slot, balance);
            }
        }
        let count = words.read(b, shadow + self.counter);
        let count = b.ins().iadd_imm(count, 1);
        words.write(b, shadow + self.counter, count);
        b.ins().jump(self.first_body, &[]);
        b.switch_to_block(self.first_body);
    }
}
