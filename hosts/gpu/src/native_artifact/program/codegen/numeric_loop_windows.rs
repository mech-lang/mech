//! Version an immutable numeric child for its complete current read windows.
use super::traversal_facts::{Check, Operand, Proof as Facts, Scope, Window};
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
#[path = "numeric_capture_windows.rs"]
mod captured;

pub(super) struct Version {
    pub initialize: usize,
    pub header: usize,
    pub sites: Vec<usize>,
    pub windows: Vec<Window>,
    pub captures: Vec<super::traversal_facts::CapturedWindow>,
    pub facts: Facts,
    pub blocks: BTreeMap<u32, Block>,
    pub prepaid: Option<super::numeric_loop_prepayment::Prepayment>,
    pub reduction: Option<super::ordered_reduction::Reduction>,
    pub funding: Option<super::numeric_child_funding::Funding>,
}

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct Report {
    pub initialize: usize,
    pub header: usize,
    pub views: Vec<usize>,
    pub source_successors: Vec<(usize, u32)>,
    pub read_sites: Vec<usize>,
    pub count: usize,
    captured_windows: Vec<super::traversal_facts::CapturedWindow>,
    pub prepaid: Option<serde_json::Value>,
    pub reduction: Option<serde_json::Value>,
    pub funding: Option<serde_json::Value>,
}

pub(super) fn derive(
    code: CodePlan<'_>,
    proof: &Proof,
    shadow: usize,
    b: &mut FunctionBuilder<'_>,
    packs: &[super::numeric_read_packs::Pack],
    child_funding: bool,
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
            outside: Default::default(),
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
        // Preserve paired-window selection, then consider an ordinary forward
        // traversal of immutable views. A current entry gate proves the whole
        // interval; each visit must still select its original induction bound.
        let paired = groups.values().find(|windows| {
            windows
                .iter()
                .any(|a| !a.reverse && windows.iter().any(|b| b.reverse && b.array != a.array))
        });
        let forward = paired.is_none();
        let Some(windows) = paired
            .or_else(|| {
                groups.values().find(|windows| {
                    windows.iter().all(|window| {
                        !window.reverse
                            && !proof.arrays[window.array].writable
                            && !proof.arrays[window.array].appends
                    })
                })
            })
            .cloned()
        else {
            continue;
        };
        let Some(mut selected) =
            super::numeric_traversal::local(code, proof, shadow, &scope, &windows)
        else {
            continue;
        };
        if selected.predicates.len() < 2 || selected.successors.len() < 2 {
            continue;
        }
        // Do not duplicate a forward loop unless admission removes an actual
        // source branch or read check that ordinary current facts cannot prove.
        if forward
            && !selected
                .successors
                .iter()
                .any(|(site, target)| facts.successors.get(site) != Some(target))
            && !scope.sites.iter().any(|&site| {
                let Some(Operation::Read { array, index, .. }) = proof.action(site) else {
                    return false;
                };
                selected.read(site, *array, *index) && !facts.read(site, *array, *index)
            })
        {
            continue;
        }
        let mut prepaid = None;
        let mut reduction = None;
        if let Some(candidate) =
            super::numeric_loop_prepayment::candidate(code, proof, tail, &windows)
            && let Some(complete) =
                super::numeric_traversal::complete(code, proof, shadow, &scope, &windows, candidate)
            && let Some(completion) = complete.completed.clone()
        {
            prepaid = super::numeric_loop_prepayment::Prepayment::new(
                proof,
                scope.header,
                completion.clone(),
            );
            if prepaid.is_some() {
                if let Some(graph) = &complete.expressions {
                    reduction = super::ordered_reduction::derive(
                        code,
                        proof,
                        tail,
                        &completion,
                        graph,
                        &windows,
                    );
                }
                selected = complete;
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
            captures: Vec::new(),
            facts: selected,
            prepaid,
            reduction,
            funding: None,
        });
    }
    let captured = captured::derive(code, proof, shadow, b, packs, &versions, child_funding);
    versions.extend(captured);
    versions
}

impl Version {
    pub fn queries(&self, proof: &Proof) -> std::collections::BTreeSet<Check> {
        let mut checks = std::collections::BTreeSet::new();
        for window in &self.windows {
            checks.insert(Check::Addition {
                word: window.count,
                extra: window.delta,
            });
            if !proof.arrays[window.array].writable && !proof.arrays[window.array].appends {
                checks.insert(Check::Fits {
                    array: window.array,
                    base: None,
                    count: window.count,
                    extra: window.adjustment(),
                });
            }
        }
        for window in &self.captures {
            checks.extend([
                Check::Le(Operand::Word(window.induction), Operand::Word(window.limit)),
                Check::Le(Operand::Constant(window.width), Operand::Word(window.limit)),
                Check::Addition {
                    word: window.limit,
                    extra: window.width,
                },
                Check::Le(Operand::Word(window.base), Operand::Length(window.array)),
                Check::Fits {
                    array: window.array,
                    base: Some(window.base),
                    count: window.limit,
                    extra: 0,
                },
            ]);
        }
        checks
    }

    pub fn report(&self, proof: &Proof) -> Report {
        Report {
            initialize: self.initialize,
            header: self.header,
            views: self
                .windows
                .iter()
                .map(|w| w.array)
                .chain(self.captures.iter().map(|w| w.array))
                .collect(),
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
            count: self
                .windows
                .first()
                .map_or_else(|| self.captures[0].limit, |w| w.count),
            captured_windows: self.captures.clone(),
            prepaid: self
                .prepaid
                .as_ref()
                .map(|p| serde_json::to_value(p).unwrap()),
            reduction: self
                .reduction
                .as_ref()
                .map(|r| serde_json::to_value(r).unwrap()),
            funding: self
                .funding
                .as_ref()
                .map(|r| serde_json::to_value(r).unwrap()),
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
        facts: &Facts,
        state: usize,
        checked: Block,
    ) {
        if !self.captures.is_empty() {
            captured::gate(self, b, words, shadow, proof, facts, state, checked);
            return;
        }
        let mut enough = b.ins().iconst(cranelift_codegen::ir::types::I8, 1);
        for window in &self.windows {
            let count = words.read(b, window.count);
            let safe = if facts.check(
                self.initialize,
                Check::Addition {
                    word: window.count,
                    extra: window.delta,
                },
            ) {
                b.ins().iconst(cranelift_codegen::ir::types::I8, 1)
            } else {
                b.ins().icmp_imm(
                    IntCC::UnsignedLessThanOrEqual,
                    count,
                    u64::MAX.wrapping_sub(window.delta) as i64,
                )
            };
            let required = b.ins().iadd_imm(count, window.adjustment() as i64);
            let length = words.read(b, shadow + proof.view(window.array) + 1);
            let fits = if facts.check(
                self.initialize,
                Check::Fits {
                    array: window.array,
                    base: None,
                    count: window.count,
                    extra: window.adjustment(),
                },
            ) {
                b.ins().iconst(cranelift_codegen::ir::types::I8, 1)
            } else {
                b.ins()
                    .icmp(IntCC::UnsignedGreaterThanOrEqual, length, required)
            };
            let valid = b.ins().band(safe, fits);
            enough = b.ins().band(enough, valid);
        }
        if let Some(prepaid) = &self.prepaid {
            let scalar = self.blocks[&(self.header as u32)];
            let selected = if self.reduction.is_some() {
                b.create_block()
            } else {
                scalar
            };
            prepaid.gate(b, words, state, shadow, enough, selected, checked);
            if let Some(reduction) = &self.reduction {
                b.switch_to_block(selected);
                reduction.emit(b, words, proof, shadow, scalar);
            }
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
