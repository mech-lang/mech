//! Strict selected String histogram/keys/identity-set/lazy-count certificate.
use super::super::fusion_semantics::{Read, SemanticActions};
use super::super::iteration_validation::{ControlPlan, ControlType};
use super::super::{
    NativeGuardedHistogramQuery as Sink, NativeHistogramQueryPhase as Phase,
    NativeHistogramQuerySetShape as SetShape, NativeHistogramQueryStep as Step,
    NativeHistogramQueryStepKind as Kind, NativeIterationAction as Action,
    NativeIterationTerminator as Terminator, NativeMatchAction as Scalar,
    NativeMatchTerminator as Term, NativeMatchValue as Value, NativeScalarValue,
};
pub(super) fn schema(plan: &impl ControlPlan, value: Value) -> Option<&mech_core::Schema> {
    plan.values().get(value.get() as usize)?.definition()
}
fn plain(plan: &impl ControlPlan, value: Value, body: &mech_core::SchemaBody) -> bool {
    schema(plan, value).is_some_and(|s| s.dimension_parameters().is_empty() && s.body() == body)
}
fn aliases(
    plan: &impl ControlPlan,
    actions: &SemanticActions<'_>,
    uses: &[Vec<Read>],
    mut value: Value,
    phase: Phase,
) -> Option<(Value, Vec<Step>)> {
    let mut steps = Vec::new();
    for _ in 0..=actions.len() {
        let [Read::Action(site)] = uses.get(value.get() as usize)?.as_slice() else {
            return Some((value, steps));
        };
        let Action::Scalar(Scalar::Copy { source, output }) = actions.get(*site as usize)? else {
            return Some((value, steps));
        };
        if *source != value || schema(plan, *source)? != schema(plan, *output)? {
            return None;
        }
        steps.push(Step {
            site: *site,
            kind: Kind::Alias { phase },
            template: None,
            operands: vec![value].into(),
            output: Some(*output),
        });
        value = *output;
    }
    None
}
fn call(actions: &SemanticActions<'_>, site: u32, kind: Kind) -> Option<Step> {
    let Action::LibraryCall {
        template,
        operands,
        output,
    } = actions.get(site as usize)?
    else {
        return None;
    };
    Some(Step {
        site,
        kind,
        template: Some(template.clone()),
        operands: operands.clone(),
        output: Some(*output),
    })
}
fn one_call(uses: &[Vec<Read>], value: Value) -> Option<u32> {
    let [Read::Action(site)] = uses.get(value.get() as usize)?.as_slice() else {
        return None;
    };
    Some(*site)
}
fn next(plan: &impl ControlPlan, site: u32) -> Option<u32> {
    let Terminator::Scalar(Term::Jump(next)) = plan.blocks().get(site as usize)?.terminator()
    else {
        return None;
    };
    Some(next)
}
fn same_scope(plan: &impl ControlPlan, site: u32, parent: Option<u32>) -> bool {
    plan.blocks()
        .get(site as usize)
        .is_some_and(|block| block.iteration() == parent && !plan.lifecycle_barrier(site as usize))
}
fn skip_none(
    plan: &impl ControlPlan,
    actions: &SemanticActions<'_>,
    mut site: u32,
    parent: Option<u32>,
) -> Option<u32> {
    for _ in 0..actions.len() {
        if !same_scope(plan, site, parent) {
            return None;
        }
        if !matches!(actions.get(site as usize)?, Action::Scalar(Scalar::None)) {
            return Some(site);
        }
        match plan.blocks()[site as usize].terminator() {
            Terminator::Scalar(Term::Jump(target)) => site = target,
            _ => return Some(site),
        }
    }
    None
}
fn bool_test(plan: &impl ControlPlan, site: u32, condition: Value) -> Option<(bool, u32, u32)> {
    let Terminator::Scalar(Term::LiteralBranch {
        left,
        right,
        yes,
        no,
    }) = plan.blocks().get(site as usize)?.terminator()
    else {
        return None;
    };
    let constant = if left == condition {
        right
    } else if right == condition {
        left
    } else {
        return None;
    };
    let NativeScalarValue::Bool(literal) = plan.scalar_constant(constant)? else {
        return None;
    };
    Some((literal, yes, no))
}
struct Guard {
    tests: Box<[u32]>,
    true_body: u32,
    false_body: u32,
}
fn guard(
    plan: &impl ControlPlan,
    actions: &SemanticActions<'_>,
    start: u32,
    condition: Value,
    parent: Option<u32>,
) -> Option<Guard> {
    let first = skip_none(plan, actions, start, parent)?;
    if !matches!(actions[first as usize], Action::Scalar(Scalar::None)) {
        return None;
    }
    if let Terminator::Scalar(Term::Branch {
        condition: actual,
        yes,
        no,
    }) = plan.blocks()[first as usize].terminator()
    {
        return (actual == condition).then_some(Guard {
            tests: vec![first].into(),
            true_body: yes,
            false_body: no,
        });
    }
    let (literal, selected, no) = bool_test(plan, first, condition)?;
    let second = skip_none(plan, actions, no, parent)?;
    if !matches!(actions[second as usize], Action::Scalar(Scalar::None)) {
        return None;
    }
    let (other, selected_other, rejected) = bool_test(plan, second, condition)?;
    if other == literal {
        return None;
    }
    let rejected = skip_none(plan, actions, rejected, parent)?;
    if !matches!(actions[rejected as usize], Action::Scalar(Scalar::None))
        || plan.blocks()[rejected as usize].terminator() != Terminator::Scalar(Term::Reject)
    {
        return None;
    }
    Some(Guard {
        tests: vec![first, second].into(),
        true_body: if literal { selected } else { selected_other },
        false_body: if literal { selected_other } else { selected },
    })
}
fn arm_copy(
    plan: &impl ControlPlan,
    actions: &SemanticActions<'_>,
    start: u32,
    source: Value,
    parent: Option<u32>,
) -> Option<(u32, Value, u32)> {
    let site = skip_none(plan, actions, start, parent)?;
    let Action::Scalar(Scalar::Copy {
        source: actual,
        output,
    }) = actions.get(site as usize)?
    else {
        return None;
    };
    (*actual == source).then_some((site, *output, next(plan, site)?))
}

fn free_parameter(schema: &mech_core::Schema) -> bool {
    use mech_core::{DimensionExpr, DimensionLifetime};
    let [parameter] = schema.dimension_parameters() else {
        return false;
    };
    parameter.lifetime() == DimensionLifetime::Turn
        && parameter.lower_bound() == &DimensionExpr::Constant(0)
        && parameter.upper_bound().is_none()
}
pub(super) fn schema_certificate(
    plan: &impl ControlPlan,
    histogram: &Step,
    keys: &Step,
    set: &Step,
) -> Option<SetShape> {
    use mech_core::{
        CardinalitySpec as C, ConversionStep, DimensionExpr as D, DimensionParameterDeclaration,
        DimensionParameterId, DimensionParameterOrigin, IntegerWidth, ResolvedType,
        SchemaBody as B,
    };
    let h = schema(plan, histogram.output()?)?;
    if !h.dimension_parameters().is_empty()
        || !matches!(h.body(), B::Map { key, value, cardinality: C::Dynamic { upper_bound: None } }
        if key.as_ref() == &B::String && value.as_ref() == &B::UnsignedInteger(IntegerWidth::W64))
    {
        return None;
    }
    let k = schema(plan, keys.output()?)?;
    if !free_parameter(k)
        || !matches!(k.body(), B::Matrix { element, dimensions }
        if element.as_ref() == &B::String && dimensions.as_ref() == [D::Constant(1),D::Parameter(DimensionParameterId::new(0))])
    {
        return None;
    }
    let s = schema(plan, set.output()?)?;
    let B::Set {
        element,
        cardinality,
    } = s.body()
    else {
        return None;
    };
    if element.as_ref() != &B::String {
        return None;
    }
    let shape = match cardinality {
        C::Dynamic { upper_bound: None } if s.dimension_parameters().is_empty() => {
            SetShape::Dynamic
        }
        C::Exact(D::Parameter(id)) if id.get() == 0 && free_parameter(s) => SetShape::FreeExact,
        _ => return None,
    };
    let resolved = |schema: &mech_core::Schema| {
        let declarations = schema
            .dimension_parameters()
            .iter()
            .enumerate()
            .map(|(id, p)| DimensionParameterDeclaration {
                id: DimensionParameterId::new(id as u32),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: p.lifetime(),
                lower_bound: p.lower_bound().clone(),
                upper_bound: p.upper_bound().cloned(),
            })
            .collect::<Vec<_>>();
        ResolvedType::from_schema_body(schema.body(), &declarations).ok()
    };
    let conversion = mech_core::plan_explicit_cast(&resolved(k)?, &resolved(s)?).ok()?;
    match conversion.step {
        ConversionStep::MatrixToSet(inner) if matches!(inner.step, ConversionStep::Identity) => {
            Some(shape)
        }
        _ => None,
    }
}

pub(super) fn candidate(
    plan: &impl ControlPlan,
    actions: &SemanticActions<'_>,
    uses: &[Vec<Read>],
    id: usize,
) -> Option<Sink> {
    use super::super::NativeCollectionPrimitive as P;
    use mech_core::{ImplementationMemoryClass as M, IntegerWidth, SchemaBody as B};
    let data = plan.collections().get(id)?;
    if data.kind() != crate::ComprehensionKind::Matrix
        || !matches!(schema(plan,data.output())?.body(), B::Matrix { element, dimensions }
        if element.as_ref() == &B::String && dimensions.len()==2 && dimensions[0]==mech_core::DimensionExpr::Constant(1))
    {
        return None;
    }
    let (w, mut steps) = aliases(plan, actions, uses, data.output(), Phase::ProducerReady)?;
    let h = call(actions, one_call(uses, w)?, Kind::Histogram)?;
    if h.operands() != [w]
        || h.template()?.primitive() != P::StringHistogram
        || h.template()?.implementation_memory() != M::CanonicalSortUnique
    {
        return None;
    }
    steps.push(h.clone());
    let (hv, aliases_h) = aliases(plan, actions, uses, h.output()?, Phase::HistogramReady)?;
    steps.extend(aliases_h);
    let hs = uses.get(hv.get() as usize)?;
    if hs.len() != 2 {
        return None;
    }
    let mut key = None;
    let mut lookup = None;
    for usage in hs {
        let Read::Action(site) = usage else {
            return None;
        };
        let c = call(actions, *site, Kind::Keys)?;
        match c.template()?.primitive() {
            P::StringMapKeys if c.operands() == [hv] && key.is_none() => key = Some(c),
            P::MapKeyAccess
                if c.operands().len() == 2 && c.operands()[0] == hv && lookup.is_none() =>
            {
                let mut c = c;
                c.kind = Kind::Lookup;
                lookup = Some(c);
            }
            _ => return None,
        }
    }
    let k = key?;
    let lookup = lookup?;
    steps.push(k.clone());
    let (kv, aliases_k) = aliases(plan, actions, uses, k.output()?, Phase::KeysReady)?;
    steps.extend(aliases_k);
    let s = call(actions, one_call(uses, kv)?, Kind::Set)?;
    if s.operands() != [kv] || s.template()?.primitive() != P::MatrixToSetConversion {
        return None;
    }
    steps.push(s.clone());
    let (sv, aliases_s) = aliases(plan, actions, uses, s.output()?, Phase::SetReady)?;
    steps.extend(aliases_s);
    let b = call(actions, one_call(uses, sv)?, Kind::Membership)?;
    if b.template()?.primitive() != P::SetContains
        || b.operands().len() != 2
        || b.operands()[1] != sv
    {
        return None;
    }
    let query = b.operands()[0];
    if lookup.operands()[1] != query
        || !plain(plan, query, &B::String)
        || !plain(plan, b.output()?, &B::Bool)
        || !plain(
            plan,
            lookup.output()?,
            &B::UnsignedInteger(IntegerWidth::W64),
        )
    {
        return None;
    }
    let set_shape = schema_certificate(plan, &h, &k, &s)?;
    steps.push(b.clone());
    // Every pre-guard consumer and alias stays on the original unconditional
    // producer-finish chain. A no-action bridge is the only permitted gap.
    let mut current = next(plan, data.finish())?;
    for step in &steps {
        current = skip_none(plan, actions, current, data.parent())?;
        if current != step.site() || !same_scope(plan, current, data.parent()) {
            return None;
        }
        current = next(plan, current)?;
    }
    let guard = guard(plan, actions, current, b.output()?, data.parent())?;
    let expected_uses = guard
        .tests
        .iter()
        .map(|site| Read::Terminator(*site))
        .collect::<Vec<_>>();
    if uses.get(b.output()?.get() as usize)? != &expected_uses {
        return None;
    }
    let true_start = skip_none(plan, actions, guard.true_body, data.parent())?;
    if true_start != lookup.site() || !same_scope(plan, true_start, data.parent()) {
        return None;
    }
    let (true_copy, result, join) = arm_copy(
        plan,
        actions,
        next(plan, true_start)?,
        lookup.output()?,
        data.parent(),
    )?;
    if uses.get(lookup.output()?.get() as usize)?.as_slice() != [Read::Action(true_copy)] {
        return None;
    }
    let false_copy = skip_none(plan, actions, guard.false_body, data.parent())?;
    let Action::Scalar(Scalar::Copy {
        source: zero,
        output: false_result,
    }) = actions.get(false_copy as usize)?
    else {
        return None;
    };
    if *false_result != result
        || !matches!(plan.scalar_constant(*zero), Some(NativeScalarValue::U64(0)))
        || next(plan, false_copy)? != join
        || !plain(plan, result, &B::UnsignedInteger(IntegerWidth::W64))
        || !same_scope(plan, join, data.parent())
        || !matches!(actions.get(join as usize)?, Action::Scalar(Scalar::None))
        || next(plan, join).is_none()
    {
        return None;
    }
    steps.push(lookup);
    steps.push(Step {
        site: join,
        kind: Kind::Cleanup,
        template: None,
        operands: Box::new([]),
        output: None,
    });
    let mut sites = std::collections::BTreeSet::new();
    if steps.iter().any(|step| !sites.insert(step.site())) {
        return None;
    }
    Some(Sink {
        query,
        steps: steps.into(),
        set_shape,
        match_result: result,
        zero: *zero,
        bool_tests: guard.tests,
        true_copy,
        false_copy,
    })
}
