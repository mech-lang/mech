//! Lift proved immutable arguments out of changing native tail state.
//! The proof uses resolved control values, never variable spelling. A whole
//! initial argument pack remains the evaluation fence for the call's inputs.
use super::*;

#[derive(Clone, Copy)]
struct PackEdit {
    block: crate::ControlBlockId,
    operation: usize,
}

struct Proof {
    invariant: Vec<bool>,
    packs: Vec<PackEdit>,
    valid: bool,
}

fn origin(
    value: PendingControlValue,
    parameters: &[Option<usize>],
    locals: &[Option<usize>],
) -> Option<usize> {
    match value {
        PendingControlValue::Parameter(index) => parameters.get(index as usize).copied().flatten(),
        PendingControlValue::Local(index) => locals.get(index as usize).copied().flatten(),
        PendingControlValue::Constant(_) => None,
    }
}

fn whole_binding(pattern: &crate::MatchPattern<usize, SchemaDraft>, local: u32) -> bool {
    matches!(pattern, crate::MatchPattern::Structural(crate::CollectionPattern::Bind { local: bound, .. }) if *bound == local)
}

fn inspect(
    block: &PendingControlBlock,
    parameters: &[Option<usize>],
    depth: usize,
    proof: &mut Proof,
) -> (Option<usize>, bool) {
    let mut locals = Vec::with_capacity(block.operations.len());
    let mut uses = vec![0usize; block.operations.len()];
    for value in block
        .operations
        .iter()
        .flat_map(|operation| &operation.inputs)
        .chain([&block.yield_value])
    {
        if let PendingControlValue::Local(index) = value {
            let Some(count) = uses.get_mut(*index as usize) else {
                proof.valid = false;
                return (None, false);
            };
            *count += 1;
        }
    }
    let mut has_call = false;
    for (index, operation) in block.operations.iter().enumerate() {
        let tail = matches!(block.yield_value, PendingControlValue::Local(local) if local as usize == index)
            && index + 1 == block.operations.len();
        let result = match &operation.body {
            PendingControlOperationBody::Recur(ancestor) if *ancestor as usize == depth => {
                has_call = true;
                let [PendingControlValue::Local(pack)] = operation.inputs.as_slice() else {
                    proof.valid = false;
                    return (None, true);
                };
                let pack = *pack as usize;
                let Some(packed) = block.operations.get(pack).filter(|_| pack < index) else {
                    proof.valid = false;
                    return (None, true);
                };
                if !tail
                    || uses[pack] != 1
                    || packed.inputs.len() != proof.invariant.len()
                    || !matches!(&packed.body, PendingControlOperationBody::Operation { operation, .. }
                        if operation.canonical_name() == "core/composite-pack")
                {
                    proof.valid = false;
                    return (None, true);
                }
                for (argument, value) in packed.inputs.iter().enumerate() {
                    proof.invariant[argument] &=
                        origin(*value, parameters, &locals) == Some(argument);
                }
                proof.packs.push(PackEdit {
                    block: block.id,
                    operation: pack,
                });
                None
            }
            PendingControlOperationBody::Match(control) => {
                let arguments = operation
                    .inputs
                    .iter()
                    .map(|value| origin(*value, parameters, &locals))
                    .collect::<Vec<_>>();
                let mut all = None;
                let mut first = true;
                let mut nested_call = false;
                for arm in &control.arms {
                    let sources = |child: &PendingControlBlock| {
                        child
                            .parameters
                            .iter()
                            .map(|(source, _)| match source {
                                crate::ControlParameterSource::Capture(capture) => control
                                    .captures
                                    .get(*capture as usize)
                                    .and_then(|(input, _, _)| arguments.get(*input as usize))
                                    .copied()
                                    .flatten(),
                                crate::ControlParameterSource::Scrutinee => {
                                    arguments.first().copied().flatten()
                                }
                                crate::ControlParameterSource::PatternBinding(local)
                                    if whole_binding(&arm.pattern, *local) =>
                                {
                                    arguments.first().copied().flatten()
                                }
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                    };
                    if let Some(guard) = &arm.guard {
                        let (_, calls) = inspect(guard, &sources(guard), depth + 1, proof);
                        if calls {
                            proof.valid = false;
                        }
                    }
                    let (value, calls) = inspect(&arm.body, &sources(&arm.body), depth + 1, proof);
                    nested_call |= calls;
                    if first {
                        all = value;
                        first = false;
                    } else if all != value {
                        all = None;
                    }
                }
                if nested_call && !tail {
                    proof.valid = false;
                }
                has_call |= nested_call;
                // A partial match wraps its selected value in an option. It
                // cannot preserve the identity of an original argument.
                if control.partial { None } else { all }
            }
            // Counted/comprehension bodies have separate invocation/capture
            // semantics. Suspension can refresh live input captures. Neither
            // may be crossed by this optional tail-state transformation.
            PendingControlOperationBody::Comprehension(_)
            | PendingControlOperationBody::Iteration(_)
            | PendingControlOperationBody::Suspend
            | PendingControlOperationBody::Publish => {
                proof.valid = false;
                None
            }
            _ => None,
        };
        locals.push(result);
    }
    (origin(block.yield_value, parameters, &locals), has_call)
}

fn map(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Map { key, value, .. }
        if key.as_ref() == &SchemaBody::String
            && value.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64))
}
fn word(body: &SchemaBody) -> bool {
    *body == SchemaBody::UnsignedInteger(IntegerWidth::W64)
}
fn map_payload(schema: &SchemaDraft) -> bool {
    if !schema.dimension_parameters.is_empty() {
        return false;
    }
    match &schema.body {
        body if map(body) => true,
        SchemaBody::Tuple(fields) => {
            !fields.is_empty()
                && fields.len() <= 32
                && fields.iter().filter(|field| map(field)).count() == 1
                && fields.iter().all(|field| map(field) || word(field))
        }
        SchemaBody::Record(fields) => {
            !fields.is_empty()
                && fields.len() <= 32
                && fields.iter().filter(|field| map(&field.schema)).count() == 1
                && fields
                    .iter()
                    .all(|field| map(&field.schema) || word(&field.schema))
        }
        _ => false,
    }
}

fn numeric_body(body: &SchemaBody, depth: usize) -> bool {
    if depth > 16 {
        return false;
    }
    match body {
        SchemaBody::Bool => true,
        body if word(body) => true,
        SchemaBody::Matrix {
            element,
            dimensions,
        } => word(element) && dimensions.len() == 2,
        SchemaBody::Tuple(fields) => {
            !fields.is_empty()
                && fields.len() <= 32
                && fields.iter().all(|field| numeric_body(field, depth + 1))
        }
        SchemaBody::Record(fields) => {
            !fields.is_empty()
                && fields.len() <= 32
                && fields
                    .iter()
                    .all(|field| numeric_body(&field.schema, depth + 1))
        }
        _ => false,
    }
}

fn has_numeric_matrix(body: &SchemaBody) -> bool {
    match body {
        SchemaBody::Matrix {
            element,
            dimensions,
        } => word(element) && dimensions.len() == 2,
        SchemaBody::Tuple(fields) => fields.iter().any(has_numeric_matrix),
        SchemaBody::Record(fields) => fields.iter().any(|field| has_numeric_matrix(&field.schema)),
        _ => false,
    }
}

// Use the ordinary function argument policy for each new standalone field.
// Free Turn axes get local first-use identities; bounds and other lifetimes
// keep their declarations. Unused free dimensions do not belong to a child.
fn local_schema(schema: &SchemaDraft, anchor: SourceSemanticAnchor) -> Option<SchemaDraft> {
    let mut dimensions = Vec::new();
    let body = function_argument_shape::embed(schema, &mut dimensions, anchor).ok()?;
    let local = SchemaDraft {
        dimension_parameters: dimensions.into(),
        body,
    };
    structural_component_schema_draft(&local, &local.body)
}

fn validate_packs(
    block: &PendingControlBlock,
    edits: &[PackEdit],
    kept: &[usize],
    schemas: &[SchemaDraft],
    anchor: SourceSemanticAnchor,
) -> bool {
    for edit in edits.iter().filter(|edit| edit.block == block.id) {
        let operation = &block.operations[edit.operation];
        let SchemaBody::Tuple(fields) = &operation.schema.body else {
            return false;
        };
        if fields.len() != schemas.len() {
            return false;
        }
        for &argument in kept {
            // The existing checked pack, rather than a possibly less precise
            // projected operand, declares each recursive argument's shape.
            // Retain exactly those checks for every remaining field. A tuple
            // result may expose a free row axis even when this constructor
            // correctly checks that the recursive row has extent one.
            let schema = SchemaDraft {
                dimension_parameters: operation.schema.dimension_parameters.clone(),
                body: fields[argument].clone(),
            };
            if local_schema(&schema, anchor).as_ref() != Some(&schemas[argument]) {
                return false;
            }
        }
    }
    block
        .operations
        .iter()
        .all(|operation| match &operation.body {
            PendingControlOperationBody::Match(control) => control.arms.iter().all(|arm| {
                arm.guard
                    .as_ref()
                    .is_none_or(|guard| validate_packs(guard, edits, kept, schemas, anchor))
                    && validate_packs(&arm.body, edits, kept, schemas, anchor)
            }),
            _ => true,
        })
}

fn rewrite(
    block: &mut PendingControlBlock,
    edits: &[PackEdit],
    kept: &[usize],
    schema: &SchemaDraft,
    contract: &OperationContractDeclaration,
) {
    for edit in edits.iter().filter(|edit| edit.block == block.id) {
        let operation = &mut block.operations[edit.operation];
        operation.inputs = kept.iter().map(|index| operation.inputs[*index]).collect();
        operation.schema = schema.clone();
        let PendingControlOperationBody::Operation {
            contract: current, ..
        } = &mut operation.body
        else {
            unreachable!()
        };
        *current = contract.clone();
    }
    for operation in &mut block.operations {
        if let PendingControlOperationBody::Match(control) = &mut operation.body {
            for arm in &mut control.arms {
                if let Some(guard) = &mut arm.guard {
                    rewrite(guard, edits, kept, schema, contract);
                }
                rewrite(&mut arm.body, edits, kept, schema, contract);
            }
        }
    }
}

impl SemanticBuilder {
    pub(super) fn promote_invariant_arguments(
        &mut self,
        arguments: &[PendingValue],
        scrutinee: &mut PendingValue,
        pattern: &mut crate::MatchPattern<usize, SchemaDraft>,
        block: &mut PendingControlBlock,
        result: &SchemaDraft,
        inputs: &mut Vec<PendingValue>,
        captures: &mut Vec<(u16, SchemaDraft, bool)>,
        call: &SyntaxNode,
    ) -> Result<(), SourceSemanticError> {
        let map_result = map_payload(result);
        let numeric_result = numeric_body(&result.body, 0);
        if arguments.len() < 2 || !(map_result || numeric_result) {
            return Ok(());
        }
        let crate::MatchPattern::Structural(crate::CollectionPattern::Tuple(bindings)) = pattern
        else {
            return Ok(());
        };
        if bindings.len() != arguments.len() || bindings.iter().enumerate().any(|(index, binding)| {
            !matches!(binding, crate::CollectionPattern::Bind { local, .. } if *local as usize == index)
        }) { return Ok(()); }
        let mut proof = Proof {
            invariant: vec![true; arguments.len()],
            packs: Vec::new(),
            valid: true,
        };
        let sources = block
            .parameters
            .iter()
            .map(|(source, _)| match source {
                crate::ControlParameterSource::PatternBinding(local)
                    if (*local as usize) < arguments.len() =>
                {
                    Some(*local as usize)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        inspect(block, &sources, 0, &mut proof);
        if !proof.valid || proof.packs.is_empty() {
            return Ok(());
        }
        let actual_schemas = arguments
            .iter()
            .map(|value| self.schema_draft_of(*value))
            .collect::<Result<Vec<_>, _>>()?;
        let anchor = SourceSemanticAnchor::for_node(call);
        let Some(schemas) = actual_schemas
            .iter()
            .map(|schema| local_schema(schema, anchor))
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(());
        };
        for (index, schema) in schemas.iter().enumerate() {
            // Scalar-only calls retain their existing word transport. Numeric
            // calls only lift arguments containing immutable U64 matrices.
            proof.invariant[index] &= (if map_result {
                !word(&schema.body)
            } else {
                numeric_body(&schema.body, 0) && has_numeric_matrix(&schema.body)
            }) && actual_schemas[index]
                .finalize_ref()
                .is_ok_and(|schema| crate::is_control_capture_schema(&schema));
        }
        let kept = proof
            .invariant
            .iter()
            .enumerate()
            .filter_map(|(index, invariant)| (!invariant).then_some(index))
            .collect::<Vec<_>>();
        if kept.len() == arguments.len() || kept.is_empty() {
            return Ok(());
        }
        if self
            .nodes
            .len()
            .checked_add(kept.len() + 2)
            .is_none_or(|count| count > u16::MAX as usize)
            || self
                .constants
                .len()
                .checked_add(kept.len())
                .is_none_or(|count| count > u32::MAX as usize)
        {
            return Ok(());
        }
        let mut dimensions = Vec::new();
        let Some(fields) = kept
            .iter()
            .map(|index| {
                function_argument_shape::embed(&schemas[*index], &mut dimensions, anchor).ok()
            })
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(());
        };
        let state = SchemaDraft {
            dimension_parameters: dimensions.into(),
            body: SchemaBody::Tuple(fields.into()),
        };
        if !(if map_result {
            map_payload(&state)
        } else {
            numeric_body(&state.body, 0)
                && state
                    .finalize_ref()
                    .is_ok_and(|schema| crate::is_control_pattern_schema(&schema))
        }) || !validate_packs(block, &proof.packs, &kept, &schemas, anchor)
        {
            return Ok(());
        }
        let Some(contract) =
            self.source_operation_contract("core/composite-pack", kept.len(), &state)
        else {
            return Ok(());
        };
        let mut new_inputs = inputs.clone();
        let mut new_captures = captures.clone();
        let mut promoted = vec![None; arguments.len()];
        for (index, argument) in arguments
            .iter()
            .enumerate()
            .filter(|(index, _)| proof.invariant[*index])
        {
            let input = match new_inputs.iter().position(|value| value == argument) {
                Some(input) => input,
                None => {
                    new_inputs.push(*argument);
                    new_inputs.len() - 1
                }
            };
            let Ok(input) = u16::try_from(input) else {
                return Ok(());
            };
            let capture = match new_captures
                .iter()
                .position(|(existing, _, _)| *existing == input)
            {
                // A promoted state argument is frozen for this invocation.
                // Never strengthen a separate live-input capture's lifetime.
                Some(capture) if !new_captures[capture].2 => return Ok(()),
                Some(capture) => capture,
                None => {
                    new_captures.push((input, actual_schemas[index].clone(), true));
                    new_captures.len() - 1
                }
            };
            let Ok(capture) = u16::try_from(capture) else {
                return Ok(());
            };
            promoted[index] = Some(capture);
        }
        // The original complete pack evaluates every original argument in
        // order, including an unused invariant. Only project the changing
        // fields for the new recursive state; existing private-frame lowering
        // can keep this one-time fence unboxed.
        let mut seed = Vec::with_capacity(kept.len());
        for &index in &kept {
            let selector =
                self.constant_exact(SchemaBody::Index, ValueDataDraft::Index(index as u64 + 1));
            seed.push(self.emit_with_schema_draft(
                "access/scalar",
                vec![*scrutinee, selector],
                schemas[index].clone(),
                call,
                "invariant-argument-state",
                None,
            ));
        }
        let next_scrutinee = self.emit_with_schema_draft(
            "core/composite-pack",
            seed,
            state.clone(),
            call,
            "invariant-argument-state",
            None,
        );
        new_inputs[0] = next_scrutinee;
        let kept_bindings = kept
            .iter()
            .enumerate()
            .map(|(local, original)| {
                let crate::CollectionPattern::Bind { schema, .. } = &bindings[*original] else {
                    unreachable!()
                };
                crate::CollectionPattern::Bind {
                    local: local as u32,
                    schema: schema.clone(),
                }
            })
            .collect::<Vec<_>>();
        *pattern =
            crate::MatchPattern::Structural(crate::CollectionPattern::Tuple(kept_bindings.into()));
        for (source, _) in &mut block.parameters {
            if let crate::ControlParameterSource::PatternBinding(local) = *source {
                if let Some(capture) = promoted.get(local as usize).copied().flatten() {
                    *source = crate::ControlParameterSource::Capture(capture);
                } else if let Some(next) =
                    kept.iter().position(|original| *original == local as usize)
                {
                    *source = crate::ControlParameterSource::PatternBinding(next as u32);
                }
            }
        }
        rewrite(block, &proof.packs, &kept, &state, &contract);
        *inputs = new_inputs;
        *captures = new_captures;
        *scrutinee = next_scrutinee;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor() -> SourceSemanticAnchor {
        SourceSemanticAnchor {
            document: DocumentId(1),
            revision: Revision(1),
            range: TextRange::empty(mech_syntax::document::TextSize::ZERO),
        }
    }
    fn projected(body: SchemaBody) -> SchemaDraft {
        SchemaDraft {
            dimension_parameters: (0..3)
                .map(|i| DimensionParameterDeclaration {
                    id: DimensionParameterId::new(i),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime: DimensionLifetime::Turn,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: None,
                })
                .collect(),
            body,
        }
    }
    #[test]
    fn projected_argument_normalization_drops_unused_free_axes_and_keeps_repeated_axes() {
        let scalar = local_schema(
            &projected(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            anchor(),
        )
        .unwrap();
        assert!(scalar.dimension_parameters.is_empty());
        let matrix = projected(SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(2)); 2].into(),
        });
        let normalized = local_schema(&matrix, anchor()).unwrap();
        assert_eq!(normalized.dimension_parameters.len(), 1);
        let SchemaBody::Matrix { dimensions, .. } = &normalized.body else {
            panic!()
        };
        assert_eq!(
            dimensions.as_ref(),
            &[
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
            ]
        );
        assert_eq!(local_schema(&normalized, anchor()).unwrap(), normalized);
    }
    #[test]
    fn projected_argument_normalization_preserves_bounds_and_lifetimes() {
        for lifetime in [DimensionLifetime::Turn, DimensionLifetime::Activation] {
            let mut matrix = projected(SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                dimensions: vec![
                    DimensionExpr::Constant(1),
                    DimensionExpr::Parameter(DimensionParameterId::new(2)),
                ]
                .into(),
            });
            matrix.dimension_parameters[2].lifetime = lifetime;
            matrix.dimension_parameters[2].lower_bound = DimensionExpr::Constant(2);
            matrix.dimension_parameters[2].upper_bound = Some(DimensionExpr::Constant(9));
            let normalized = local_schema(&matrix, anchor()).unwrap();
            assert_eq!(normalized.dimension_parameters.len(), 1);
            let declaration = &normalized.dimension_parameters[0];
            assert_eq!(declaration.lifetime, lifetime);
            assert_eq!(declaration.lower_bound, DimensionExpr::Constant(2));
            assert_eq!(declaration.upper_bound, Some(DimensionExpr::Constant(9)));
        }
    }
}
