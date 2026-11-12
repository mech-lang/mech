//! Word-level live input proof. Aggregate read/write effects are obtained by
//! running the SAME shared emitter against a compile-time recording adapter.
use super::*;
use crate::native_artifact::codegen::common::{
    aggregates,
    word_access::{LocalWord, WordAccess},
};
use crate::native_artifact::program::prepare::readonly_numeric::Operation;
use cranelift_codegen::ir::{Function, Signature};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::FunctionBuilderContext;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeProgramFacts,
};
#[derive(Default)]
struct Effects {
    reads: BTreeSet<usize>,
    writes: BTreeSet<usize>,
}
impl Effects {
    fn used(&mut self, word: usize) {
        if !self.writes.contains(&word) {
            self.reads.insert(word);
        }
    }
}
impl WordAccess for Effects {
    fn read(
        &mut self,
        b: &mut FunctionBuilder<'_>,
        word: LocalWord,
    ) -> Result<Value, NativeCompileError> {
        self.used(word.0);
        Ok(b.ins().iconst(types::I64, 0))
    }
    fn write(
        &mut self,
        _: &mut FunctionBuilder<'_>,
        word: LocalWord,
        _: Value,
    ) -> Result<(), NativeCompileError> {
        self.writes.insert(word.0);
        Ok(())
    }
}
pub(super) fn required(
    code: CodePlan<'_>,
    proof: &Proof,
    shadow: usize,
    header: usize,
    exit: usize,
    result: &[usize],
    handoffs: &BTreeMap<usize, Vec<(usize, usize)>>,
) -> Result<BTreeSet<usize>, NativeCompileError> {
    let CodePlan::Program(program) = code else {
        return Err(backend("native scalar flow requires source plan"));
    };
    let mut context = FunctionBuilderContext::new();
    let mut f = Function::new();
    f.signature = Signature::new(CallConv::SystemV);
    let mut b = FunctionBuilder::new(&mut f, &mut context);
    let entry = b.create_block();
    b.switch_to_block(entry);
    let error = b.create_block();
    b.append_block_param(error, types::I64);
    let mut effects = BTreeMap::new();
    let mut edges = BTreeMap::new();
    let mut exits = BTreeSet::new();
    let terminal = result
        .iter()
        .map(|word| shadow + word)
        .collect::<BTreeSet<_>>();
    for &site in &proof.sites {
        let mut e = Effects::default();
        match proof.action(site) {
            Some(Operation::Frame) => {
                let moves = super::super::numeric_words::frame_word_moves(
                    shadow,
                    code,
                    site,
                    &proof.frames,
                )?;
                for &(_, from) in &moves.assignments {
                    e.used(from)
                }
                for (to, _) in moves.assignments {
                    e.writes.insert(to);
                }
            }
            Some(Operation::Aggregate) => {
                let layout = &program.metadata().scalar_aggregates;
                aggregates::emit(
                    &mut b,
                    &mut e,
                    layout,
                    layout
                        .action(site)
                        .ok_or_else(|| backend("native flow lacks shared aggregate action"))?,
                    error,
                    &mut aggregates::CheckedGuards,
                )?;
            }
            Some(Operation::ArrayCopy) => {}
            Some(Operation::Length { array, output }) => {
                e.used(shadow + proof.view(*array) + 1);
                e.writes.insert(*output as usize);
            }
            Some(Operation::Read {
                array,
                index,
                output,
            }) => {
                e.used(*index as usize);
                for at in 0..4 {
                    e.used(shadow + proof.view(*array) + at)
                }
                e.writes.insert(*output as usize);
            }
            Some(Operation::Write {
                array,
                index,
                incoming,
            }) => {
                e.used(*index as usize);
                e.used(*incoming as usize);
                for at in 0..4 {
                    e.used(shadow + proof.view(*array) + at)
                }
            }
            Some(Operation::Append { array, incoming })
            | Some(Operation::AppendLeBytes {
                array, incoming, ..
            }) => {
                e.used(*incoming as usize);
                let view = shadow + proof.view(*array);
                e.used(view);
                e.used(view + 1);
                e.used(
                    shadow
                        + proof
                            .capacity(*array)
                            .ok_or_else(|| backend("native flow append lacks capacity"))?,
                );
                if let Some(Operation::AppendLeBytes { count, .. }) = proof.action(site) {
                    e.used(*count as usize)
                }
                e.writes.extend([view + 1, view + 3]);
            }
            Some(Operation::Fallback) => {
                let exports = handoffs
                    .get(&site)
                    .ok_or_else(|| backend("native flow reaches unsupported source computation"))?;
                for &(_, source) in exports {
                    e.used(source);
                }
                effects.insert(site, e);
                edges.insert(site, Vec::new());
                continue;
            }
            None => match code.blocks()[site].action() {
                Action::Scalar(Scalar::None) => {}
                Action::Scalar(Scalar::Copy { source, output }) => {
                    e.used(source.get() as usize);
                    e.writes.insert(output.get() as usize);
                }
                Action::Scalar(Scalar::Primitive {
                    operands, output, ..
                }) => {
                    for v in operands {
                        e.used(v.get() as usize)
                    }
                    e.writes.insert(output.get() as usize);
                }
                _ => return Err(backend("native flow lacks scalar action")),
            },
        }
        match code.blocks()[site].terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => e.used(condition.get() as usize),
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                e.used(left.get() as usize);
                e.used(right.get() as usize);
            }
            _ => {}
        }
        let successors = NativeProgramFacts::control_successors(code.blocks()[site].terminator());
        if successors.into_iter().flatten().any(|n| n as usize == exit) {
            exits.insert(site);
        }
        let next = successors
            .into_iter()
            .flatten()
            .map(|n| n as usize)
            .filter(|n| proof.sites.binary_search(n).is_ok())
            .collect::<Vec<_>>();
        edges.insert(site, next);
        effects.insert(site, e);
    }
    b.ins().return_(&[]);
    b.switch_to_block(error);
    b.ins().return_(&[]);
    b.seal_all_blocks();
    b.finalize();
    let mut live = effects
        .iter()
        .map(|(&s, e)| (s, e.reads.clone()))
        .collect::<BTreeMap<_, _>>();
    loop {
        let mut changed = false;
        for &site in proof.sites.iter().rev() {
            let mut next = effects[&site].reads.clone();
            if exits.contains(&site) {
                next.extend(terminal.difference(&effects[&site].writes).copied());
            }
            for target in &edges[&site] {
                next.extend(live[target].difference(&effects[&site].writes).copied());
            }
            if live[&site] != next {
                live.insert(site, next);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    live.remove(&header)
        .ok_or_else(|| backend("native flow lacks root header"))
}
