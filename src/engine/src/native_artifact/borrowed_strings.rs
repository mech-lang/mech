//! Compiler-only escape proof for String views inside a root synchronous FSM.
//! A backend must still bind genuine immutable owners and check representation
//! before using these actions. This metadata grants no runtime permission.
use super::{
    NativeCollectionPrimitive as Primitive, NativeControlLoop, NativeIterationAction as Action,
    NativeIterationValueScope as Scope, NativeMatchAction as Scalar, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeScalarKind,
    NativeStructuralObservation as Observation,
};
use mech_core::{SchemaBody, ValueData};

#[derive(Clone, Debug)]
pub enum NativeBorrowedStringAction {
    MatrixElement {
        source: Value,
        index: Value,
        output: Value,
    },
    Copy {
        source: Value,
        output: Value,
    },
    Length {
        source: Value,
        output: Value,
    },
    ScalarSelect {
        source: Value,
        index: Value,
        output: Value,
    },
    FixedRange {
        source: Value,
        first: u64,
        last: u64,
        output: Value,
    },
    Equal {
        source: Value,
        literal: Value,
        output: Value,
        negate: bool,
    },
}
impl NativeBorrowedStringAction {
    pub fn output(&self) -> Value {
        match *self {
            Self::MatrixElement { output, .. }
            | Self::Copy { output, .. }
            | Self::Length { output, .. }
            | Self::ScalarSelect { output, .. }
            | Self::FixedRange { output, .. }
            | Self::Equal { output, .. } => output,
        }
    }
    fn view_inputs(&self) -> Vec<Value> {
        match *self {
            Self::MatrixElement { .. } => Vec::new(),
            Self::Equal {
                source, literal, ..
            } => vec![source, literal],
            Self::Copy { source, .. }
            | Self::Length { source, .. }
            | Self::ScalarSelect { source, .. }
            | Self::FixedRange { source, .. } => vec![source],
        }
    }
    fn produces_view(&self) -> bool {
        !matches!(self, Self::Length { .. } | Self::Equal { .. })
    }
}
#[derive(Clone, Debug)]
pub struct NativeBorrowedStringRegion {
    iteration: u32,
    matrices: Box<[Value]>,
    literals: Box<[Value]>,
    values: Box<[Value]>,
    actions: Box<[(u32, NativeBorrowedStringAction)]>,
}
impl NativeBorrowedStringRegion {
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }
    pub fn matrices(&self) -> &[Value] {
        &self.matrices
    }
    pub fn literals(&self) -> &[Value] {
        &self.literals
    }
    pub fn values(&self) -> &[Value] {
        &self.values
    }
    pub fn actions(&self) -> &[(u32, NativeBorrowedStringAction)] {
        &self.actions
    }
}
fn string(plan: &Plan, value: Value) -> bool {
    let schema = plan.values()[value.get() as usize].definition();
    schema.body() == &SchemaBody::String && schema.dimension_parameters().is_empty()
}
fn literal(plan: &Plan, value: Value) -> bool {
    string(plan, value)
        && plan.constants().iter().any(|entry| {
            entry.value() == value
                && matches!(entry.constant().data(),ValueData::String(s)
            if s.len() <= 32 && s.is_ascii() && !s.as_bytes().windows(2).any(|p| p == b"\r\n"))
        })
}
fn u64_constant(plan: &Plan, value: Value) -> Option<u64> {
    plan.constants().iter().find_map(|entry| {
        if entry.value() == value {
            match entry.constant().data() {
                ValueData::U64(n) => Some(*n),
                _ => None,
            }
        } else {
            None
        }
    })
}
fn fixed_range(plan: &Plan, selector: Value) -> Option<(u64, u64)> {
    // The original selected range call is NOT elided by this proof. Its shape
    // and failures still precede String selection. Only its constant geometry
    // is used to emit a borrowed slice rather than a copied String result.
    let (first, last) = plan.blocks().iter().find_map(|block| {
        let Action::LibraryCall {
            template,
            operands,
            output,
        } = block.action()
        else {
            return None;
        };
        if *output != selector
            || template.primitive() != Primitive::RangeInclusive
            || operands.len() != 2
        {
            return None;
        }
        Some((
            u64_constant(plan, operands[0])?,
            u64_constant(plan, operands[1])?,
        ))
    })?;
    (first > 0 && last >= first && last - first < 32).then_some((first, last))
}
fn selected(plan: &Plan, site: usize) -> Option<NativeBorrowedStringAction> {
    use NativeBorrowedStringAction as View;
    let scalar = |v: Value, kind| plan.values()[v.get() as usize].scalar_kind() == Some(kind);
    match plan.blocks()[site].action() {
        Action::Scalar(Scalar::Copy { source, output })
            if string(plan, *source) && string(plan, *output) =>
        {
            Some(View::Copy {
                source: *source,
                output: *output,
            })
        }
        Action::Structural {
            source,
            path,
            observation: Observation::StringEqual { peer },
            output,
        } if path.is_empty() && string(plan, *source) && literal(plan, *peer) => {
            Some(View::Equal {
                source: *source,
                literal: *peer,
                output: *output,
                negate: false,
            })
        }
        Action::LibraryCall {
            template,
            operands,
            output,
        } => match (template.primitive(), operands.as_ref()) {
            (Primitive::MatrixScalarAccess, [source, index])
                if string(plan, *output)
                    && scalar(*index, NativeScalarKind::U64)
                    && matches!(plan.values()[source.get() as usize].definition().body(),
                        SchemaBody::Matrix {element,dimensions} if element.as_ref()==&SchemaBody::String && dimensions.len()==2)
                    && plan.value_scopes()[source.get() as usize]
                        == (Scope::Immutable { iteration: None }) =>
            {
                Some(View::MatrixElement {
                    source: *source,
                    index: *index,
                    output: *output,
                })
            }
            #[cfg(feature = "pure_text")]
            (Primitive::SequenceLength, [source])
                if string(plan, *source) && scalar(*output, NativeScalarKind::U64) =>
            {
                Some(View::Length {
                    source: *source,
                    output: *output,
                })
            }
            (Primitive::StringScalarAccess, [source, index])
                if string(plan, *source)
                    && string(plan, *output)
                    && scalar(*index, NativeScalarKind::U64) =>
            {
                Some(View::ScalarSelect {
                    source: *source,
                    index: *index,
                    output: *output,
                })
            }
            (Primitive::StringRangeAccess, [source, selector])
                if string(plan, *source) && string(plan, *output) =>
            {
                let (first, last) = fixed_range(plan, *selector)?;
                Some(View::FixedRange {
                    source: *source,
                    first,
                    last,
                    output: *output,
                })
            }
            (op @ (Primitive::StringEqual | Primitive::StringNotEqual), [a, b]) => {
                let (source, constant) = if literal(plan, *b) {
                    (*a, *b)
                } else if literal(plan, *a) {
                    (*b, *a)
                } else {
                    return None;
                };
                (string(plan, source) && scalar(*output, NativeScalarKind::Bool)).then_some(
                    View::Equal {
                        source,
                        literal: constant,
                        output: *output,
                        negate: op == Primitive::StringNotEqual,
                    },
                )
            }
            _ => None,
        },
        _ => None,
    }
}
fn region(
    plan: &Plan,
    iteration: u32,
    reads: &[Vec<super::fusion_semantics::Read>],
) -> Option<NativeBorrowedStringRegion> {
    use super::fusion_semantics::Read;
    let NativeControlLoop::Tail(tail) = plan.iterations().get(iteration as usize)? else {
        return None;
    };
    if tail.parent().is_some() {
        return None;
    }
    let size = plan.values().len();
    let candidate = (0..size)
        .map(|i| {
            string(plan, Value(i as u32))
                && plan.value_scopes()[i]
                    == (Scope::Immutable {
                        iteration: Some(iteration),
                    })
        })
        .collect::<Vec<_>>();
    let mut bad = candidate.iter().map(|c| !c).collect::<Vec<_>>();
    let mut defined = vec![false; size];
    let actions = plan
        .blocks()
        .iter()
        .enumerate()
        .map(|(site, block)| {
            (block.iteration() == Some(iteration))
                .then(|| selected(plan, site))
                .flatten()
        })
        .collect::<Vec<_>>();
    let mut edges = vec![Vec::new(); size];
    for action in actions.iter().flatten() {
        let output = action.output().get() as usize;
        if action.produces_view() {
            defined[output] = true;
        }
        for source in action.view_inputs() {
            let input = source.get() as usize;
            if action.produces_view() && candidate[output] {
                if candidate[input] {
                    edges[output].push(input);
                    edges[input].push(output);
                } else if !literal(plan, source) {
                    bad[output] = true;
                }
            }
        }
    }
    for i in 0..size {
        if !candidate[i] {
            continue;
        }
        if !defined[i] {
            bad[i] = true;
        }
        for usage in &reads[i] {
            let Read::Action(site) = usage else {
                bad[i] = true;
                continue;
            };
            let allowed = actions[*site as usize].as_ref().is_some_and(|action| {
                action.view_inputs().contains(&Value(i as u32))
                    && (!action.produces_view() || candidate[action.output().get() as usize])
            });
            if !allowed {
                bad[i] = true;
            }
        }
    }
    let mut pending = bad
        .iter()
        .enumerate()
        .filter_map(|(i, b)| b.then_some(i))
        .collect::<Vec<_>>();
    while let Some(i) = pending.pop() {
        for &other in &edges[i] {
            if !bad[other] {
                bad[other] = true;
                pending.push(other);
            }
        }
    }
    let mut matrices = Vec::new();
    let mut literals = Vec::new();
    let mut kept = Vec::new();
    let add = |items: &mut Vec<Value>, value| {
        if !items.contains(&value) {
            items.push(value);
        }
    };
    for (site, action) in actions.into_iter().enumerate() {
        let Some(action) = action else {
            continue;
        };
        let inputs = action.view_inputs();
        if action.produces_view() && bad[action.output().get() as usize]
            || inputs
                .iter()
                .any(|v| bad[v.get() as usize] && !literal(plan, *v))
        {
            continue;
        }
        if let NativeBorrowedStringAction::MatrixElement { source, .. } = &action {
            add(&mut matrices, *source);
        }
        for value in inputs {
            if literal(plan, value) {
                add(&mut literals, value);
            }
        }
        kept.push((site as u32, action));
    }
    // This pass is rooted in genuine matrix owners, not detached String words.
    if matrices.is_empty() {
        return None;
    }
    Some(NativeBorrowedStringRegion {
        iteration,
        matrices: matrices.into(),
        literals: literals.into(),
        values: (0..size)
            .filter(|&i| !bad[i])
            .map(|i| Value(i as u32))
            .collect(),
        actions: kept.into(),
    })
}
impl Plan {
    pub fn borrowed_string_regions(&self) -> Vec<NativeBorrowedStringRegion> {
        let Ok(actions) = super::fusion_semantics::semantic_actions(self) else {
            return Vec::new();
        };
        let Ok(reads) = super::fusion_semantics::reads(self, &actions) else {
            return Vec::new();
        };
        (0..self.iterations().len())
            .filter_map(|i| region(self, i as u32, &reads))
            .collect()
    }
}
