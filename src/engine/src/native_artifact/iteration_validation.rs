//! Current-visit definition proof for structurally owned counted loops.
//! Backedges are validated only through a declared loop. Each lexical body is
//! checked once as an acyclic graph, beginning with ancestors plus its carrier;
//! previous-visit body locals are never part of that initial definition set.
use super::{
    NativeControlLoopRef as Loop, NativeIterationAction as Action, NativeIterationArtifactPlan,
    NativeIterationTerminator as Terminator, NativeIterationValueScope as Scope,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeMatchValue as Value,
    NativeScalarKind, NativeScalarValue,
};
use std::collections::VecDeque;

pub(crate) trait ControlType: Eq {
    fn is_bool(&self) -> bool;
    fn definition(&self) -> Option<&mech_core::Schema> {
        None
    }
}
impl ControlType for super::NativeScalarType {
    fn is_bool(&self) -> bool {
        self.kind() == NativeScalarKind::Bool
    }
}
impl ControlType for super::NativeProgramType {
    fn is_bool(&self) -> bool {
        self.scalar_kind() == Some(NativeScalarKind::Bool)
    }
    fn definition(&self) -> Option<&mech_core::Schema> {
        Some(self.definition())
    }
}

/// One definition/ownership analysis for both legacy scalar and mixed-value
/// compiler views. Neither implementation evaluates values or chooses control.
pub(crate) trait ControlPlan {
    type Type: ControlType;
    fn values(&self) -> &[Self::Type];
    fn blocks(&self) -> &[super::NativeIterationBlock];
    fn iteration_count(&self) -> usize;
    fn iteration(&self, index: usize) -> Loop<'_>;
    fn collections(&self) -> &[super::NativeProgramCollection];
    fn value_scopes(&self) -> &[Scope];
    fn entry(&self) -> u32;
    fn initialize(&self, initial: &mut [bool]) -> Result<(), &'static str>;
    fn output_values(&self) -> Box<dyn Iterator<Item = (Value, &Self::Type)> + '_>;
    fn library(
        &self,
        origin: super::NativeMatchOrigin,
        template: &super::NativeProgramLibraryTemplate,
        operands: &[Value],
        output: Value,
    ) -> Result<(), &'static str>;
}
macro_rules! common_plan_view {
    ($plan:ty, $ty:ty) => {
        type Type = $ty;
        fn values(&self) -> &[Self::Type] {
            <$plan>::values(self)
        }
        fn blocks(&self) -> &[super::NativeIterationBlock] {
            <$plan>::blocks(self)
        }
        fn iteration_count(&self) -> usize {
            <$plan>::iterations(self).len()
        }
        fn iteration(&self, index: usize) -> Loop<'_> {
            (&<$plan>::iterations(self)[index]).into()
        }
        fn value_scopes(&self) -> &[Scope] {
            <$plan>::value_scopes(self)
        }
        fn entry(&self) -> u32 {
            <$plan>::entry(self)
        }
        fn output_values(&self) -> Box<dyn Iterator<Item = (Value, &Self::Type)> + '_> {
            Box::new(
                self.outputs()
                    .iter()
                    .map(|output| (output.value(), output.ty())),
            )
        }
    };
}
fn initialize_value<P: ControlPlan>(
    plan: &P,
    value: Value,
    ty: &P::Type,
    initial: &mut [bool],
) -> Result<(), &'static str> {
    let id = value.get() as usize;
    if plan.values().get(id) != Some(ty)
        || plan.value_scopes().get(id) != Some(&Scope::Immutable { iteration: None })
        || *initial.get(id).ok_or(BAD)?
    {
        return Err(BAD);
    }
    initial[id] = true;
    Ok(())
}
impl ControlPlan for NativeIterationArtifactPlan {
    common_plan_view!(NativeIterationArtifactPlan, super::NativeScalarType);
    fn collections(&self) -> &[super::NativeProgramCollection] {
        &[]
    }
    fn initialize(&self, initial: &mut [bool]) -> Result<(), &'static str> {
        for input in self.inputs() {
            initialize_value(self, input.value(), input.declaration().ty(), initial)?;
        }
        for constant in self.constants() {
            let ty = self
                .values()
                .get(constant.value().get() as usize)
                .ok_or(BAD)?;
            let kind = match constant.scalar() {
                NativeScalarValue::Bool(_) => NativeScalarKind::Bool,
                NativeScalarValue::U64(_) => NativeScalarKind::U64,
                NativeScalarValue::I64(_) => NativeScalarKind::I64,
                NativeScalarValue::F64(_) => NativeScalarKind::F64,
            };
            if ty.kind() != kind {
                return Err(BAD);
            }
            initialize_value(self, constant.value(), ty, initial)?;
        }
        Ok(())
    }
    fn library(
        &self,
        _: super::NativeMatchOrigin,
        _: &super::NativeProgramLibraryTemplate,
        _: &[Value],
        _: Value,
    ) -> Result<(), &'static str> {
        Err(BAD) // Frozen scalar plans never acquire managed-call authority.
    }
}
impl ControlPlan for super::NativeProgramArtifactPlan {
    common_plan_view!(super::NativeProgramArtifactPlan, super::NativeProgramType);
    fn collections(&self) -> &[super::NativeProgramCollection] {
        self.collections()
    }
    fn initialize(&self, initial: &mut [bool]) -> Result<(), &'static str> {
        for input in self.inputs() {
            initialize_value(self, input.value(), input.ty(), initial)?;
        }
        for constant in self.constants() {
            let ty = self
                .values()
                .get(constant.value().get() as usize)
                .ok_or(BAD)?;
            if ty.schema() != constant.constant().schema()
                || ty.schema_key() != constant.constant().schema_key()
            {
                return Err(BAD);
            }
            initialize_value(self, constant.value(), ty, initial)?;
        }
        Ok(())
    }
    fn library(
        &self,
        origin: super::NativeMatchOrigin,
        template: &super::NativeProgramLibraryTemplate,
        operands: &[Value],
        output: Value,
    ) -> Result<(), &'static str> {
        if template.origin() != origin || template.program_revision() != self.program_revision() {
            return Err(BAD);
        }
        crate::resident::general::native_match_lowering::validate_library_types(
            template,
            operands,
            output,
            self.values(),
        )
    }
}

const BAD: &str = "invalid counted-loop ownership or current-visit definitions";
const ANALYSIS_CELLS: usize = 16 * 1024 * 1024;
fn scalar_successors(term: Term) -> [Option<u32>; 2] {
    match term {
        Term::Jump(next) => [Some(next), None],
        Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. } => [Some(yes), Some(no)],
        Term::Complete | Term::Reject => [None, None],
    }
}
fn scope_ancestor(plan: &impl ControlPlan, ancestor: Option<u32>, mut scope: Option<u32>) -> bool {
    // Loop ids are pre-order; every parent must be earlier. The caller first
    // validates this property, so the walk cannot cycle.
    loop {
        if scope == ancestor {
            return true;
        }
        let Some(id) = scope else {
            return false;
        };
        scope = plan.iteration(id as usize).parent();
    }
}
fn readable(plan: &impl ControlPlan, value: Value, scope: Option<u32>, block: usize) -> bool {
    match plan.value_scopes().get(value.get() as usize) {
        Some(Scope::Immutable { iteration }) => scope_ancestor(plan, *iteration, scope),
        Some(Scope::Carrier { iteration }) => {
            scope_ancestor(plan, Some(*iteration), scope)
                || plan.iteration(*iteration as usize).exit() as usize == block
        }
        None => false,
    }
}

/// Called before the opaque engine plan is returned, including runtime-only
/// same-Object rederivation. No caller can construct or mutate a certified plan.
pub(crate) fn validate(plan: &impl ControlPlan) -> Result<(), &'static str> {
    let blocks = plan.blocks();
    let count = plan.values().len();
    let cells = count.checked_add(plan.collections().len()).ok_or(BAD)?;
    if plan.entry() as usize >= blocks.len()
        || plan.value_scopes().len() != count
        || blocks
            .len()
            .checked_mul(cells)
            .is_none_or(|n| n > ANALYSIS_CELLS)
    {
        return Err(BAD);
    }
    let ty = |v: Value| plan.values().get(v.get() as usize).ok_or(BAD);
    let mut initial = vec![false; cells];
    plan.initialize(&mut initial)?;
    let mut roles = vec![None; blocks.len()];
    for id in 0..plan.iteration_count() {
        let iteration = plan.iteration(id);
        if iteration
            .parent()
            .is_some_and(|parent| parent as usize >= id)
        {
            return Err(BAD);
        }
        let mut depth = 0;
        let mut parent = iteration.parent();
        while let Some(p) = parent {
            depth += 1;
            if depth > crate::MAX_CONTROL_DEPTH {
                return Err(BAD);
            }
            parent = plan.iteration(p as usize).parent();
        }
        if let Loop::Iteration(fold) = iteration {
            if ty(fold.seed())? != ty(fold.carrier())?
                || ty(fold.next())? != ty(fold.carrier())?
                || ty(fold.result())? != ty(fold.carrier())?
                || plan.value_scopes()[fold.carrier().get() as usize]
                    != (Scope::Carrier {
                        iteration: id as u32,
                    })
            {
                return Err(BAD);
            }
        }
        for (block, role) in [
            (iteration.initialize(), 0),
            (iteration.header(), 1),
            (iteration.latch(), 2),
            (iteration.exit(), 3),
        ] {
            let entry = roles.get_mut(block as usize).ok_or(BAD)?;
            if entry.replace((id as u32, role)).is_some() {
                return Err(BAD);
            }
        }
        let init = &blocks[iteration.initialize() as usize];
        let header = &blocks[iteration.header() as usize];
        let latch = &blocks[iteration.latch() as usize];
        let exit = &blocks[iteration.exit() as usize];
        if init.iteration() != iteration.parent()
            || !matches!(init.action(),Action::Initialize { iteration: n } if *n == id as u32)
            || init.terminator() != Terminator::Scalar(Term::Jump(iteration.header()))
            || header.iteration() != Some(id as u32)
            || !matches!(header.action(), Action::Scalar(Scalar::None))
            || header.terminator()
                != (Terminator::Header {
                    iteration: id as u32,
                    body: iteration.body(),
                    exit: iteration.exit(),
                })
            || latch.iteration() != Some(id as u32)
            || !matches!(latch.action(),Action::Latch { iteration:n } if *n == id as u32)
            || latch.terminator() != Terminator::Scalar(Term::Jump(iteration.header()))
            || exit.iteration() != iteration.parent()
            || !match iteration {
                Loop::Iteration(fold) => {
                    matches!(exit.action(), Action::Scalar(Scalar::Copy { source,output })
                    if *source==fold.carrier() && *output==fold.result())
                }
                Loop::Collection(collection) => {
                    matches!(exit.action(), Action::Scalar(Scalar::None))
                        && matches!(blocks.get(collection.body() as usize).map(|block| block.action()),
                        Some(Action::CollectionElement { iteration }) if *iteration == id as u32)
                }
            }
            || blocks
                .get(iteration.body() as usize)
                .ok_or(BAD)?
                .iteration()
                != Some(id as u32)
        {
            return Err(BAD);
        }
    }
    for (id, scope) in plan.value_scopes().iter().enumerate() {
        let owner = match scope {
            Scope::Immutable { iteration } => *iteration,
            Scope::Carrier { iteration } => Some(*iteration),
        };
        if owner.is_some_and(|n| n as usize >= plan.iteration_count()) {
            return Err(BAD);
        }
        if let Scope::Carrier { iteration } = scope {
            if !matches!(plan.iteration(*iteration as usize), Loop::Iteration(fold) if fold.carrier().get() as usize == id)
            {
                return Err(BAD);
            }
        }
    }
    // Check every structural edge, including dormant literal arms. Entry into
    // a child scope is exclusively its initializer; exit is exclusively header.
    for (id, block) in blocks.iter().enumerate() {
        if block
            .iteration()
            .is_some_and(|n| n as usize >= plan.iteration_count())
        {
            return Err(BAD);
        }
        let successors = match block.terminator() {
            Terminator::Scalar(term) => scalar_successors(term),
            Terminator::Header { body, exit, .. } => [Some(body), Some(exit)],
        };
        for next in successors.into_iter().flatten() {
            let next_block = blocks.get(next as usize).ok_or(BAD)?;
            let special = match roles.get(next as usize).copied().flatten() {
                Some((loop_id, 1)) => {
                    id == plan.iteration(loop_id as usize).initialize() as usize
                        || id == plan.iteration(loop_id as usize).latch() as usize
                }
                Some((loop_id, 3)) => id == plan.iteration(loop_id as usize).header() as usize,
                _ => block.iteration() == next_block.iteration(),
            };
            if !special {
                return Err(BAD);
            }
        }
        if let Terminator::Header { iteration, .. } = block.terminator() {
            if roles[id] != Some((iteration, 1)) {
                return Err(BAD);
            }
        }
        match block.action() {
            Action::CollectionBegin { collection }
            | Action::CollectionFinish { collection }
            | Action::CollectionPush { collection, .. } => {
                let collection = plan.collections().get(*collection as usize).ok_or(BAD)?;
                let expected = match block.action() {
                    Action::CollectionBegin { .. } => collection.begin(),
                    Action::CollectionFinish { .. } => collection.finish(),
                    _ => collection.push(),
                };
                if expected as usize != id {
                    return Err(BAD);
                }
            }
            Action::CollectionElement { iteration } => {
                if *iteration as usize >= plan.iteration_count()
                    || !matches!(plan.iteration(*iteration as usize), Loop::Collection(data) if data.body() as usize == id)
                {
                    return Err(BAD);
                }
            }
            Action::LibraryCall {
                template,
                operands,
                output,
            } => {
                plan.library(block.origin(), template, operands, *output)?;
            }
            Action::Initialize { iteration } if roles[id] != Some((*iteration, 0)) => {
                return Err(BAD);
            }
            Action::Latch { iteration } if roles[id] != Some((*iteration, 2)) => return Err(BAD),
            Action::Scalar(Scalar::Primitive {
                operands, output, ..
            }) => {
                ty(*output)?;
                if operands.iter().any(|v| ty(*v).is_err()) {
                    return Err(BAD);
                }
            }
            Action::Scalar(Scalar::Copy { source, output }) if ty(*source)? != ty(*output)? => {
                return Err(BAD);
            }
            _ => {}
        }
        match block.terminator() {
            Terminator::Scalar(Term::Branch { condition, .. }) if !ty(condition)?.is_bool() => {
                return Err(BAD);
            }
            Terminator::Scalar(Term::LiteralBranch { left, right, .. })
                if ty(left)? != ty(right)? =>
            {
                return Err(BAD);
            }
            Terminator::Scalar(Term::Complete) if block.iteration().is_some() => return Err(BAD),
            _ => {}
        }
    }
    super::collection_validation::validate(plan)?;
    let mut visited = vec![false; blocks.len()];
    check_scope(plan, None, plan.entry(), initial, &roles, &mut visited)?;
    // Unreachable fail-closed Reject blocks still have checked structure/types;
    // no operation with a read or write is silently omitted from the proof.
    for (id, block) in blocks.iter().enumerate() {
        if !visited[id]
            && !(matches!(block.action(), Action::Scalar(Scalar::None))
                && matches!(block.terminator(), Terminator::Scalar(Term::Reject)))
        {
            return Err(BAD);
        }
    }
    Ok(())
}
fn check_scope(
    plan: &impl ControlPlan,
    scope: Option<u32>,
    entry: u32,
    initial: Vec<bool>,
    roles: &[Option<(u32, u8)>],
    visited: &mut [bool],
) -> Result<(), &'static str> {
    let mut incoming = vec![Vec::new(); plan.blocks().len()];
    let mut degrees = vec![0usize; plan.blocks().len()];
    let mut reachable = vec![false; plan.blocks().len()];
    let mut pending = vec![entry];
    while let Some(id) = pending.pop() {
        if std::mem::replace(&mut reachable[id as usize], true) {
            continue;
        }
        if plan.blocks()[id as usize].iteration() != scope {
            return Err(BAD);
        }
        for next in collapsed_successors(plan, id as usize)
            .into_iter()
            .flatten()
        {
            incoming[next as usize].push(id as usize);
            degrees[next as usize] = degrees[next as usize].checked_add(1).ok_or(BAD)?;
            pending.push(next);
        }
    }
    let mut ends = vec![Vec::new(); plan.blocks().len()];
    let mut ready = (0..degrees.len())
        .filter(|&i| reachable[i] && degrees[i] == 0)
        .collect::<VecDeque<_>>();
    let mut seen = 0;
    while let Some(id) = ready.pop_front() {
        seen += 1;
        visited[id] = true;
        let block = &plan.blocks()[id];
        let mut known = if let Some(&first) = incoming[id].first() {
            ends[first].clone()
        } else {
            initial.clone()
        };
        for &previous in incoming[id].iter().skip(1) {
            for (bit, &other) in known.iter_mut().zip(&ends[previous]) {
                *bit &= other;
            }
        }
        let read = |v: Value, known: &[bool]| -> Result<(), &'static str> {
            if known.get(v.get() as usize).copied().unwrap_or(false) && readable(plan, v, scope, id)
            {
                Ok(())
            } else {
                Err(BAD)
            }
        };
        match block.action() {
            Action::LibraryCall {
                operands, output, ..
            } => {
                for &value in operands {
                    read(value, &known)?;
                }
                let ordinal = output.get() as usize;
                if known[ordinal]
                    || plan.value_scopes()[ordinal] != (Scope::Immutable { iteration: scope })
                {
                    return Err(BAD);
                }
                known[ordinal] = true;
            }
            Action::Initialize { iteration } => {
                let child = plan.iteration(*iteration as usize);
                let mut body_initial = known.clone();
                match child {
                    Loop::Iteration(fold) => {
                        read(fold.seed(), &known)?;
                        body_initial[fold.carrier().get() as usize] = true;
                    }
                    Loop::Collection(collection) => read(collection.source(), &known)?,
                }
                // The header is not part of the body DAG; it owns the only
                // backedge and always dispatches before a fresh body visit.
                visited[child.header() as usize] = true;
                check_scope(
                    plan,
                    Some(*iteration),
                    child.body(),
                    body_initial,
                    roles,
                    visited,
                )?;
                if let Loop::Iteration(fold) = child {
                    known[fold.carrier().get() as usize] = true;
                }
            }
            Action::Latch { iteration } => {
                if let Loop::Iteration(fold) = plan.iteration(*iteration as usize) {
                    read(fold.next(), &known)?;
                }
                if known[plan.values().len()..] != initial[plan.values().len()..] {
                    return Err(BAD);
                }
            }
            Action::CollectionElement { iteration } => {
                let Loop::Collection(collection) = plan.iteration(*iteration as usize) else {
                    return Err(BAD);
                };
                read(collection.source(), &known)?;
                if let Some(element) = collection.element() {
                    let ordinal = element.get() as usize;
                    if known[ordinal]
                        || plan.value_scopes()[ordinal] != (Scope::Immutable { iteration: scope })
                    {
                        return Err(BAD);
                    }
                    known[ordinal] = true;
                }
            }
            Action::CollectionBegin { collection } => {
                let data = &plan.collections()[*collection as usize];
                let offset = plan.values().len();
                let active = known[offset..]
                    .iter()
                    .rposition(|live| *live)
                    .map(|id| id as u32);
                if active != data.parent_collection() || known[offset + *collection as usize] {
                    return Err(BAD);
                }
                known[offset + *collection as usize] = true;
            }
            Action::CollectionPush { collection, value } => {
                if known[plan.values().len()..].iter().rposition(|live| *live)
                    != Some(*collection as usize)
                {
                    return Err(BAD);
                }
                read(*value, &known)?;
            }
            Action::CollectionFinish { collection } => {
                let offset = plan.values().len();
                if known[offset..].iter().rposition(|live| *live) != Some(*collection as usize) {
                    return Err(BAD);
                }
                known[offset + *collection as usize] = false;
                let output = plan.collections()[*collection as usize].output().get() as usize;
                if known[output]
                    || plan.value_scopes()[output] != (Scope::Immutable { iteration: scope })
                {
                    return Err(BAD);
                }
                known[output] = true;
            }
            Action::Scalar(action) => {
                let output = match action {
                    Scalar::Primitive {
                        operands, output, ..
                    } => {
                        for &value in operands {
                            read(value, &known)?;
                        }
                        Some(*output)
                    }
                    Scalar::Copy { source, output } => {
                        read(*source, &known)?;
                        Some(*output)
                    }
                    Scalar::None => None,
                };
                if let Some(output) = output {
                    let ordinal = output.get() as usize;
                    if known[ordinal]
                        || plan.value_scopes()[ordinal] != (Scope::Immutable { iteration: scope })
                    {
                        return Err(BAD);
                    }
                    known[ordinal] = true;
                }
            }
        }
        if let Terminator::Scalar(term) = block.terminator() {
            match term {
                Term::Branch { condition, .. } => read(condition, &known)?,
                Term::LiteralBranch { left, right, .. } => {
                    read(left, &known)?;
                    read(right, &known)?;
                }
                Term::Complete => {
                    if known[plan.values().len()..].iter().any(|live| *live) {
                        return Err(BAD);
                    }
                    for (output, ty) in plan.output_values() {
                        read(output, &known)?;
                        if plan.values()[output.get() as usize] != *ty {
                            return Err(BAD);
                        }
                    }
                }
                _ => {}
            }
        }
        ends[id] = known;
        for next in collapsed_successors(plan, id).into_iter().flatten() {
            degrees[next as usize] -= 1;
            if degrees[next as usize] == 0 {
                ready.push_back(next as usize);
            }
        }
    }
    if seen != reachable.into_iter().filter(|v| *v).count() {
        return Err(BAD);
    }
    Ok(())
}
fn collapsed_successors(plan: &impl ControlPlan, id: usize) -> [Option<u32>; 2] {
    match plan.blocks()[id].action() {
        Action::Initialize { iteration } => {
            [Some(plan.iteration(*iteration as usize).exit()), None]
        }
        Action::Latch { .. } => [None, None],
        _ => match plan.blocks()[id].terminator() {
            Terminator::Scalar(term) => scalar_successors(term),
            Terminator::Header { .. } => [None, None],
        },
    }
}
