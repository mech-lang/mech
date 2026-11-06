//! Numeric adapter to the shared current-traversal facts, not a second analysis.
use super::traversal_facts::{self, Proof, Scope, Step};
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::{aggregates, word_access};
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::prepare::readonly_numeric::{Operation, Proof as Numeric};
use cranelift_codegen::ir::Block;
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeControlLoop, NativeIterationAction as Action, NativeMatchValue,
};
use std::collections::BTreeSet;

pub(super) fn derive(code: CodePlan<'_>, numeric: &Numeric, shadow: usize) -> Proof {
    let CodePlan::Program(program) = code else {
        return Proof::default();
    };
    let mut facts = Proof::default();
    for (iteration, control) in program.metadata().plan.iterations().iter().enumerate() {
        let NativeControlLoop::Tail(tail) = control else {
            continue;
        };
        // Use exact canonical iteration ownership. A nested loop has a fresh
        // scope, even when emitted as part of its parent's larger native phase.
        let sites = numeric
            .sites
            .iter()
            .copied()
            .filter(|&site| code.blocks()[site].iteration() == Some(iteration as u32))
            .collect::<Vec<_>>();
        if !sites.contains(&(tail.header() as usize)) {
            continue;
        }
        let scope = Scope {
            header: tail.header() as usize,
            latch: tail.latch() as usize,
            exit: tail.exit(),
            sites,
            outside: BTreeSet::new(),
        };
        if let Some(proof) = local(code, numeric, shadow, &scope, &[]) {
            facts.extend(proof);
        }
    }
    facts
}

pub(super) fn local(
    code: CodePlan<'_>,
    numeric: &Numeric,
    shadow: usize,
    scope: &Scope,
    windows: &[traversal_facts::Window],
) -> Option<Proof> {
    analyze(code, numeric, shadow, scope, windows, &[], None)
}

pub(super) fn captured_version(
    code: CodePlan<'_>,
    numeric: &Numeric,
    shadow: usize,
    scope: &Scope,
    windows: &[traversal_facts::CapturedWindow],
) -> Option<Proof> {
    analyze(code, numeric, shadow, scope, &[], windows, None)
}

pub(super) fn complete(
    code: CodePlan<'_>,
    numeric: &Numeric,
    shadow: usize,
    scope: &Scope,
    windows: &[traversal_facts::Window],
    candidate: traversal_facts::Candidate,
) -> Option<Proof> {
    analyze(code, numeric, shadow, scope, windows, &[], Some(candidate))
}

fn analyze(
    code: CodePlan<'_>,
    numeric: &Numeric,
    shadow: usize,
    scope: &Scope,
    windows: &[traversal_facts::Window],
    captures: &[traversal_facts::CapturedWindow],
    candidate: Option<traversal_facts::Candidate>,
) -> Option<Proof> {
    let CodePlan::Program(program) = code else {
        return None;
    };
    let sites = BTreeSet::from_iter(scope.sites.iter().copied());
    // An outside edge may only enter at the fresh header. In particular no
    // skipped case selection or cold resume can inherit a predecessor fact.
    for (site, visit) in program.metadata().visits.iter().enumerate() {
        if sites.contains(&site) || !captures.is_empty() {
            continue;
        }
        if visit
            .successors()
            .into_iter()
            .flatten()
            .any(|next| sites.contains(&(next as usize)) && next as usize != scope.header)
        {
            return None;
        }
    }
    let mut slots = super::numeric_words::source_slots(
        code,
        scope.sites.iter().copied(),
        &numeric.actions,
        std::iter::empty(),
    )
    .ok()?;
    for &site in &scope.sites {
        if matches!(numeric.action(site), Some(Operation::Frame)) {
            let moves =
                super::numeric_words::frame_word_moves(shadow, code, site, &numeric.frames).ok()?;
            for (to, from) in moves.assignments {
                slots.extend([to, from]);
            }
        }
    }
    let step = |site| {
        match numeric.action(site) {
            Some(Operation::Aggregate) => Some(Step::Aggregate(
                &program.metadata().scalar_aggregates,
                program.metadata().scalar_aggregates.action(site)?,
            )),
            Some(Operation::Frame) => {
                let moves =
                    super::numeric_words::frame_word_moves(shadow, code, site, &numeric.frames)
                        .ok()?;
                Some(Step::Assign {
                    moves: moves.assignments,
                    parallel: moves.parallel,
                })
            }
            Some(Operation::ArrayCopy) => Some(Step::Keep),
            Some(Operation::Read {
                array,
                index,
                output,
            }) => Some(Step::Read {
                array: *array,
                index: *index,
                output: *output as usize,
            }),
            Some(Operation::Length { array, output }) => Some(Step::Length {
                array: *array,
                output: *output as usize,
            }),
            // Mutable arrays, unsupported actions and cold boundaries decline
            // this whole local analysis. The original guarded emitter remains.
            Some(_) => None,
            None => match code.blocks()[site].action() {
                Action::Scalar(action) => Some(Step::Scalar(action)),
                _ => None,
            },
        }
    };
    if !captures.is_empty() {
        traversal_facts::with_captured_windows(code, scope, &slots, captures, step)
    } else if let Some(candidate) = candidate {
        traversal_facts::with_completion(code, scope, &slots, windows, candidate, step)
    } else {
        traversal_facts::with_windows(code, scope, &slots, None, windows, step)
    }
}

/// Strategy-owned use of an exact, rederived current-site fact. All other
/// projections use the single maintained checked aggregate emitter.
pub(super) struct Guards<'a> {
    pub site: usize,
    pub facts: &'a Proof,
    pub emitted: &'a mut Vec<(usize, u32, usize, u32)>,
}
impl aggregates::GuardPolicy for Guards<'_> {
    fn projection<W: word_access::WordAccess>(
        &mut self,
        b: &mut FunctionBuilder<'_>,
        words: &mut W,
        word: word_access::LocalWord,
        source: NativeMatchValue,
        index: usize,
        ordinal: u32,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        if self.facts.guard(self.site, source, index, ordinal) {
            self.emitted.push((self.site, source.get(), index, ordinal));
            Ok(())
        } else {
            aggregates::GuardPolicy::projection(
                &mut aggregates::CheckedGuards,
                b,
                words,
                word,
                source,
                index,
                ordinal,
                error,
            )
        }
    }
}
