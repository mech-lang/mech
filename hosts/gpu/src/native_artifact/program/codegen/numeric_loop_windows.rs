//! Version an immutable numeric child for its complete current read windows.
use super::traversal_facts::{Proof as Facts, Scope, Window};
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::prepare::readonly_numeric::{Operation, Proof};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeControlLoop, NativeIterationTerminator as Term, NativeMatchTerminator as Branch,
};
use std::collections::BTreeMap;

pub(super) struct Version {
    pub initialize: usize,
    pub header: usize,
    pub sites: Vec<usize>,
    pub windows: Vec<Window>,
    pub facts: Facts,
    pub blocks: BTreeMap<u32, Block>,
    pub prepaid: Option<super::numeric_loop_prepayment::Prepayment>,
}

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct Report {
    pub initialize: usize,
    pub header: usize,
    pub views: Vec<usize>,
    pub source_successors: Vec<(usize, u32)>,
    pub read_sites: Vec<usize>,
    pub count: usize,
    pub prepaid: Option<serde_json::Value>,
}

pub(super) fn derive(
    code: CodePlan<'_>,
    proof: &Proof,
    shadow: usize,
    b: &mut FunctionBuilder<'_>,
) -> Vec<Version> {
    let CodePlan::Program(program) = code else {
        return Vec::new();
    };
    let mut versions = Vec::new();
    for (id, control) in program.metadata().plan.iterations().iter().enumerate() {
        let NativeControlLoop::Tail(tail) = control else {
            continue;
        };
        if tail.parent().is_none() || !proof.sites.contains(&(tail.initialize() as usize)) {
            continue;
        }
        if !matches!(code.blocks()[tail.initialize() as usize].terminator(),
            Term::TailHeader { body, .. } | Term::Scalar(Branch::Jump(body)) if body == tail.header())
        {
            continue;
        }
        let scope = Scope {
            header: tail.header() as usize,
            latch: tail.latch() as usize,
            exit: tail.exit(),
            sites: proof
                .sites
                .iter()
                .copied()
                .filter(|&site| code.blocks()[site].iteration() == Some(id as u32))
                .collect(),
        };
        if !scope.sites.contains(&scope.header) {
            continue;
        }
        let Some(facts) = super::numeric_traversal::local(code, proof, shadow, &scope, &[]) else {
            continue;
        };
        let mut groups = BTreeMap::<(usize, usize), Vec<Window>>::new();
        for &window in &facts.windows {
            groups
                .entry((window.induction, window.count))
                .or_default()
                .push(window);
        }
        // Begin with a paired forward/reverse immutable window. This confines
        // code duplication to a complete leaf computation with both benefits.
        let Some(windows) = groups.into_values().find(|windows| {
            windows
                .iter()
                .any(|a| !a.reverse && windows.iter().any(|b| b.reverse && b.array != a.array))
        }) else {
            continue;
        };
        let Some(mut facts) =
            super::numeric_traversal::local(code, proof, shadow, &scope, &windows)
        else {
            continue;
        };
        if facts.predicates.len() < 2 || facts.successors.len() < 2 {
            continue;
        }
        let mut prepaid = None;
        if let Some(candidate) =
            super::numeric_loop_prepayment::candidate(code, proof, tail, &windows)
            && let Some(complete) =
                super::numeric_traversal::complete(code, proof, shadow, &scope, &windows, candidate)
            && let Some(completion) = complete.completed.clone()
        {
            prepaid =
                super::numeric_loop_prepayment::Prepayment::new(proof, scope.header, completion);
            if prepaid.is_some() {
                facts = complete;
            }
        }
        versions.push(Version {
            initialize: tail.initialize() as usize,
            header: scope.header,
            blocks: scope
                .sites
                .iter()
                .map(|&s| (s as u32, b.create_block()))
                .collect(),
            sites: scope.sites,
            windows,
            facts,
            prepaid,
        });
    }
    versions
}

impl Version {
    pub fn report(&self, proof: &Proof) -> Report {
        Report {
            initialize: self.initialize,
            header: self.header,
            views: self.windows.iter().map(|w| w.array).collect(),
            source_successors: self
                .facts
                .successors
                .iter()
                .map(|(&s, &n)| (s, n))
                .collect(),
            read_sites: self
                .sites
                .iter()
                .filter_map(|&site| {
                    let Operation::Read { array, index, .. } = proof.action(site)? else {
                        return None;
                    };
                    self.facts.read(site, *array, *index).then_some(site)
                })
                .collect(),
            count: self.windows[0].count,
            prepaid: self
                .prepaid
                .as_ref()
                .map(|p| serde_json::to_value(p).unwrap()),
        }
    }

    /// No source error is raised at admission: refusal enters the unchanged
    /// checked child, including its original zero-trip and overflow behavior.
    pub fn gate(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        shadow: usize,
        proof: &Proof,
        state: usize,
        checked: Block,
    ) {
        let mut enough = b.ins().iconst(cranelift_codegen::ir::types::I8, 1);
        for window in &self.windows {
            let count = words.read(b, window.count);
            let safe = b.ins().icmp_imm(
                IntCC::UnsignedLessThanOrEqual,
                count,
                u64::MAX.wrapping_sub(window.delta) as i64,
            );
            let required = b.ins().iadd_imm(count, window.adjustment() as i64);
            let length = words.read(b, shadow + proof.view(window.array) + 1);
            let fits = b
                .ins()
                .icmp(IntCC::UnsignedGreaterThanOrEqual, length, required);
            let valid = b.ins().band(safe, fits);
            enough = b.ins().band(enough, valid);
        }
        if let Some(prepaid) = &self.prepaid {
            prepaid.gate(
                b,
                words,
                state,
                shadow,
                enough,
                self.blocks[&(self.header as u32)],
                checked,
            );
            return;
        }
        b.ins().brif(
            enough,
            self.blocks[&(self.header as u32)],
            &[],
            checked,
            &[],
        );
    }
}
