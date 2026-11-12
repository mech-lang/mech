//! All-path initialized-value proof for compact native SSA entry.
use crate::native_artifact::{NativeCompileError, compiler::backend};
use mech_engine::native_artifact::{
    NativeCollectionSource as Source, NativeControlLoop as Loop, NativeIterationAction as Action,
    NativeIterationTerminator as Term, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch, NativeMatchValue as Value, NativeProgramArtifactPlan as Plan,
};

pub(super) struct Flow {
    pub entry: usize,
    pub inputs: Vec<u32>,
    pub outputs: Vec<u32>,
    pub complete: Vec<bool>,
    pub skip_action: Vec<bool>,
    pub reachable: Vec<bool>,
    pub uses: Vec<Vec<u32>>,
    action_uses: Vec<Vec<u32>>,
    pub defs: Vec<Vec<u32>>,
    pub successors: Vec<Vec<usize>>,
}
fn successors(term: Term) -> Vec<usize> {
    match term {
        Term::Scalar(Branch::Jump(n)) | Term::TailHeader { body: n, .. } => vec![n as usize],
        Term::Scalar(Branch::Branch { yes, no, .. } | Branch::LiteralBranch { yes, no, .. }) => {
            vec![yes as usize, no as usize]
        }
        Term::Header { body, exit, .. } => vec![body as usize, exit as usize],
        _ => vec![],
    }
}
fn add(out: &mut Vec<u32>, value: Value) {
    out.push(value.get())
}
impl Flow {
    pub fn derive(plan: &Plan) -> Result<Self, NativeCompileError> {
        Self::derive_entry(
            plan,
            plan.entry() as usize,
            plan.inputs().iter().map(|p| p.value().get()).collect(),
            plan.outputs().iter().map(|p| p.value().get()).collect(),
            None,
            None,
        )
    }
    pub fn derive_entry(
        plan: &Plan,
        entry: usize,
        inputs: Vec<u32>,
        outputs: Vec<u32>,
        stop: Option<usize>,
        element: Option<usize>,
    ) -> Result<Self, NativeCompileError> {
        let complete = plan
            .blocks()
            .iter()
            .enumerate()
            .map(|(site, b)| {
                stop.map_or(b.terminator() == Term::Scalar(Branch::Complete), |end| {
                    site == end
                })
            })
            .collect::<Vec<_>>();
        let skip_action = (0..plan.blocks().len())
            .map(|site| Some(site) == stop || Some(site) == element)
            .collect::<Vec<_>>();
        let mut uses = vec![];
        let mut defs = vec![];
        let mut action_uses = vec![];
        for (site, block) in plan.blocks().iter().enumerate() {
            let mut reads = vec![];
            let mut writes = vec![];
            if !skip_action[site] {
                match block.action() {
                    Action::Scalar(Scalar::Primitive {
                        operands, output, ..
                    })
                    | Action::LibraryCall {
                        operands, output, ..
                    } => {
                        reads.extend(operands.iter().map(|v| v.get()));
                        add(&mut writes, *output)
                    }
                    Action::Scalar(Scalar::Copy { source, output }) => {
                        add(&mut reads, *source);
                        add(&mut writes, *output)
                    }
                    Action::Structural {
                        source,
                        observation,
                        output,
                        ..
                    } => {
                        add(&mut reads, *source);
                        if let mech_engine::native_artifact::NativeStructuralObservation::StringEqual { peer } = observation {
                        add(&mut reads, *peer);
                    }
                        add(&mut writes, *output)
                    }
                    Action::Initialize { iteration } | Action::Latch { iteration } => {
                        match &plan.iterations()[*iteration as usize] {
                            Loop::Tail(tail) => {
                                let source = if matches!(block.action(), Action::Initialize { .. })
                                {
                                    tail.seed()
                                } else {
                                    tail.next()
                                };
                                add(&mut reads, source);
                                add(&mut writes, tail.carrier());
                            }
                            Loop::Collection(generator) => {
                                if matches!(block.action(), Action::Initialize { .. }) {
                                    if let Source::Value(source) = generator.source() {
                                        add(&mut reads, *source)
                                    }
                                }
                            }
                            Loop::Iteration(_) => {
                                return Err(backend(
                                    "compact counted source loop remains unsupported",
                                ));
                            }
                        }
                    }
                    Action::PrepareU64Range { iteration } => {
                        let Loop::Collection(generator) = &plan.iterations()[*iteration as usize]
                        else {
                            return Err(backend("compact range lacks collection generator"));
                        };
                        let Source::U64Range(cursor) = generator.source() else {
                            return Err(backend("compact range lacks checked source cursor"));
                        };
                        reads.extend(cursor.operands().iter().map(|v| v.get()));
                    }
                    Action::CollectionBegin { .. } => {}
                    Action::CollectionElement { iteration } => {
                        let Loop::Collection(generator) = &plan.iterations()[*iteration as usize]
                        else {
                            return Err(backend("compact element lacks collection generator"));
                        };
                        if let Source::Value(source) = generator.source() {
                            add(&mut reads, *source)
                        }
                        if let Some(element) = generator.element() {
                            add(&mut writes, element)
                        }
                    }
                    Action::CollectionPush { value, .. } => add(&mut reads, *value),
                    Action::CollectionFinish { collection } => {
                        let definition = &plan.collections()[*collection as usize];
                        add(&mut writes, definition.output());
                        if definition.kind() == mech_engine::ComprehensionKind::MatrixPreserveShape
                        {
                            let Loop::Collection(generator) =
                                &plan.iterations()[definition.generators()[0] as usize]
                            else {
                                return Err(backend(
                                    "compact shape-preserving source lacks generator",
                                ));
                            };
                            if let Source::Value(source) = generator.source() {
                                add(&mut reads, *source)
                            }
                        }
                    }
                    Action::Scalar(Scalar::None) => {}
                    _ => {
                        return Err(backend(format!(
                            "compact emitter has an unimplemented source action at {:?}: {:?}",
                            block.origin(),
                            block.action()
                        )));
                    }
                }
            }
            action_uses.push(reads.clone());
            if complete[site] {
                reads.extend(&outputs);
            } else {
                match block.terminator() {
                    Term::Scalar(Branch::Branch { condition, .. }) => add(&mut reads, condition),
                    Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                        add(&mut reads, left);
                        add(&mut reads, right)
                    }
                    Term::Scalar(Branch::Complete) => {
                        reads.extend(plan.outputs().iter().map(|o| o.value().get()))
                    }
                    _ => {}
                }
            }
            reads.sort_unstable();
            reads.dedup();
            writes.sort_unstable();
            writes.dedup();
            uses.push(reads);
            defs.push(writes);
        }
        let successors = plan
            .blocks()
            .iter()
            .enumerate()
            .map(|(site, b)| {
                if complete[site] {
                    vec![]
                } else {
                    successors(b.terminator())
                }
            })
            .collect::<Vec<_>>();
        let mut reachable = vec![false; plan.blocks().len()];
        let mut todo = vec![entry];
        while let Some(site) = todo.pop() {
            if !reachable[site] {
                reachable[site] = true;
                todo.extend(&successors[site]);
            }
        }
        let flow = Self {
            entry,
            inputs,
            outputs,
            complete,
            skip_action,
            reachable,
            uses,
            action_uses,
            defs,
            successors,
        };
        flow.prove_initialized(plan)?;
        Ok(flow)
    }
    fn prove_initialized(&self, plan: &Plan) -> Result<(), NativeCompileError> {
        let n = plan.values().len();
        let width = n.div_ceil(64);
        let mut incoming = vec![vec![u64::MAX; width]; plan.blocks().len()];
        let mut predecessors = vec![vec![]; plan.blocks().len()];
        for (site, edges) in self.successors.iter().enumerate() {
            if self.reachable[site] {
                for &next in edges {
                    predecessors[next].push(site)
                }
            }
        }
        let mut entry = vec![0; width];
        for value in self
            .inputs
            .iter()
            .copied()
            .chain(plan.constants().iter().map(|v| v.value().get()))
        {
            let v = value as usize;
            entry[v / 64] |= 1u64 << (v % 64);
        }
        let mut todo = (0..plan.blocks().len())
            .rev()
            .filter(|&n| self.reachable[n])
            .collect::<Vec<_>>();
        let mut queued = self.reachable.clone();
        while let Some(site) = todo.pop() {
            queued[site] = false;
            let mut words = if site == self.entry {
                entry.clone()
            } else {
                vec![u64::MAX; width]
            };
            for &parent in &predecessors[site] {
                for (i, word) in words.iter_mut().enumerate() {
                    let mut after = incoming[parent][i];
                    for &defined in &self.defs[parent] {
                        let defined = defined as usize;
                        if defined / 64 == i {
                            after |= 1u64 << (defined % 64)
                        }
                    }
                    *word &= after;
                }
            }
            if words != incoming[site] {
                incoming[site] = words;
                for &next in &self.successors[site] {
                    if !queued[next] {
                        queued[next] = true;
                        todo.push(next)
                    }
                }
            }
        }
        for (site, reads) in self.uses.iter().enumerate() {
            if !self.reachable[site] {
                continue;
            }
            for &value in reads {
                let v = value as usize;
                let available = incoming[site][v / 64] & (1u64 << (v % 64)) != 0
                    || (!self.action_uses[site].contains(&value)
                        && self.defs[site].contains(&value));
                // Same-site defs are available only to the terminator, never to
                // the action that is creating that result.
                if !available {
                    return Err(backend(format!(
                        "compact native value {value} lacks an all-path initialization at source site {site}"
                    )));
                }
            }
        }
        Ok(())
    }
}
