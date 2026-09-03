//! Closed kind and dimension constraint solving.

use super::{
    BuiltinKindPredicate, BuiltinKindPredicateSet, ConversionPlan, KindPredicateEvidence,
    ResolvedType, TypeConstraintFailure, TypeConstraintOrigin, TypeResolutionError,
    exact_type_equal, format_fields, permitted_conversion, plan_implicit_conversion,
    plan_numeric_promotion, semantic_dimension_name, semantic_kind_name,
};
use crate::dimension::{
    collect_dimension_references, normalize_dimension, rewrite_dimension_references,
};
use crate::kind_expr::{
    collect_kind_dimension_references, rewrite_kind_dimensions, visit_kind_dimensions,
    visit_kind_parameters,
};
use crate::{
    DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
    InputKindScheme, KindConstraint, KindExpr, KindField, KindParameterId, KindScheme,
    TableJoinMode,
};

#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::ToString,
    vec,
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::ToString,
    vec,
    vec::Vec,
};

type DimensionInterval = (u64, Option<u64>);

fn prove_less_equal(left: DimensionInterval, right: DimensionInterval) -> bool {
    left.1.is_some_and(|left_max| left_max <= right.0)
}

fn prove_greater_equal(left: DimensionInterval, right: DimensionInterval) -> bool {
    right.1.is_some_and(|right_max| left.0 >= right_max)
}

fn kind_predicates(
    kind: &KindExpr,
    bindings: &BTreeMap<KindParameterId, KindBinding>,
) -> BuiltinKindPredicateSet {
    if let KindExpr::Parameter(id) = kind {
        return bindings
            .get(id)
            .map(|binding| binding.predicates)
            .unwrap_or_else(BuiltinKindPredicateSet::empty);
    }
    let child_predicates = match kind {
        KindExpr::Option(element)
        | KindExpr::Matrix { element, .. }
        | KindExpr::Set { element, .. } => vec![kind_predicates(element, bindings)],
        KindExpr::Tuple(elements) => elements
            .iter()
            .map(|element| kind_predicates(element, bindings))
            .collect(),
        KindExpr::Record(fields)
        | KindExpr::Table {
            columns: fields, ..
        } => fields
            .iter()
            .map(|field| kind_predicates(&field.kind, bindings))
            .collect(),
        KindExpr::Map { key, value, .. } => {
            vec![
                kind_predicates(key, bindings),
                kind_predicates(value, bindings),
            ]
        }
        _ => Vec::new(),
    };
    super::intrinsic_kind_predicates(kind, &child_predicates)
}

/// Formats semantic kinds without exposing Rust or runtime storage names.
#[derive(Clone, Debug)]
struct KindBinding {
    kind: KindExpr,
    predicates: BuiltinKindPredicateSet,
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct OverloadScore {
    pub conversion_cost: u32,
    pub wildcard_matches: u32,
    pub unconstrained_kind_bindings: u32,
    pub unconstrained_dimension_bindings: u32,
    pub predicate_generality: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RigidDimensionOrigin {
    type_equivalence_group: u32,
    local_parameter: DimensionParameterId,
}

/// One semantic overload candidate. `id` is an opaque compiler/catalog key;
/// it does not participate in type equality or diagnostics.
#[derive(Clone, Copy)]
pub struct TypeOverloadCandidate<'a> {
    pub id: u64,
    pub scheme: &'a KindScheme,
}

/// The unique semantic result of overload resolution. Multiple candidate IDs
/// may remain only when they are semantically identical implementations of
/// the same closed result; execution binding chooses among those afterward.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedOverload {
    pub candidate_ids: Box<[u64]>,
    pub outputs: Box<[ResolvedType]>,
    pub conversions: Box<[ConversionPlan]>,
    pub conversion_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemeResolution {
    pub outputs: Box<[ResolvedType]>,
    pub conversions: Box<[ConversionPlan]>,
    pub conversion_count: u32,
    score: OverloadScore,
}

/// Resolves overloads by schemes and constraints. Runtime representations are
/// intentionally absent from this API.
pub fn resolve_type_overloads(
    origin: TypeConstraintOrigin,
    candidates: &[TypeOverloadCandidate<'_>],
    inputs: &[ResolvedType],
    expected_outputs: Option<&[ResolvedType]>,
) -> Result<ResolvedOverload, TypeResolutionError> {
    let mut successes = Vec::new();
    let mut failures = Vec::new();
    for candidate in candidates {
        match TypeConstraintEnvironment::new(origin.clone()).solve_scheme(
            candidate.scheme,
            inputs,
            expected_outputs,
        ) {
            Ok(solution) => successes.push((candidate.id, solution)),
            Err(TypeResolutionError::Incompatible {
                failures: candidate_failures,
                ..
            }) => failures.extend(candidate_failures.into_vec()),
            Err(error @ TypeResolutionError::Ambiguous { .. }) => return Err(error),
        }
    }

    if successes.is_empty() {
        failures.sort_by(|left, right| format!("{left:?}").cmp(&format!("{right:?}")));
        failures.dedup();
        if failures.is_empty() {
            failures.push(TypeConstraintFailure::InvalidScheme {
                reason: "the overload set is empty".to_string(),
            });
        }
        return Err(TypeResolutionError::Incompatible {
            origin,
            failures: failures.into_boxed_slice(),
        });
    }

    let Some(best_cost) = successes.iter().map(|(_, solution)| solution.score).min() else {
        return Err(TypeResolutionError::incompatible(
            origin.semantic_name.clone(),
            TypeConstraintFailure::InvalidScheme {
                reason: "the overload set produced no comparable result".into(),
            },
        ));
    };
    successes.retain(|(_, solution)| solution.score == best_cost);
    successes.sort_by_key(|(id, _)| *id);

    let mut alternatives: Vec<(Box<[ResolvedType]>, Box<[ConversionPlan]>, u32, Vec<u64>)> =
        Vec::new();
    for (id, solution) in successes {
        if let Some((_, _, _conversion_count, ids)) =
            alternatives
                .iter_mut()
                .find(|(outputs, conversions, conversion_count, _)| {
                    **outputs == *solution.outputs
                        && **conversions == *solution.conversions
                        && *conversion_count == solution.conversion_count
                })
        {
            ids.push(id);
        } else {
            alternatives.push((
                solution.outputs,
                solution.conversions,
                solution.conversion_count,
                vec![id],
            ));
        }
    }

    if alternatives.len() != 1 {
        return Err(TypeResolutionError::Ambiguous {
            origin,
            alternatives: alternatives
                .into_iter()
                .map(|(outputs, _, _, _)| outputs)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        });
    }
    let Some((outputs, conversions, conversion_count, mut candidate_ids)) = alternatives.pop()
    else {
        return Err(TypeResolutionError::incompatible(
            origin.semantic_name,
            TypeConstraintFailure::InvalidScheme {
                reason: "the overload set produced no semantic alternative".into(),
            },
        ));
    };
    candidate_ids.sort_unstable();
    Ok(ResolvedOverload {
        candidate_ids: candidate_ids.into_boxed_slice(),
        outputs,
        conversions,
        conversion_count,
    })
}

/// The single environment used by Type System v1 for kind variables,
/// dimension variables, equality, conversions, keyability, nominal identity,
/// structural aggregates, options, and dynamic/fixed extent relationships.
pub struct TypeConstraintEnvironment {
    origin: TypeConstraintOrigin,
    dimensions: Vec<DimensionParameterDeclaration>,
    bindable_dimensions: BTreeSet<DimensionParameterId>,
    kind_bindings: BTreeMap<KindParameterId, KindBinding>,
    dimension_bindings: BTreeMap<DimensionParameterId, DimensionExpr>,
    imported_evidence: Vec<(KindExpr, BuiltinKindPredicateSet)>,
    imported_type_groups: Vec<(ResolvedType, u32)>,
    rigid_dimension_origins: BTreeMap<DimensionParameterId, RigidDimensionOrigin>,
    predicate_constrained_kinds: BTreeSet<KindParameterId>,
    score: OverloadScore,
    conversion_rewrites: Vec<(KindExpr, KindExpr)>,
    conversion_count: u32,
}

impl TypeConstraintEnvironment {
    pub fn new(origin: TypeConstraintOrigin) -> Self {
        Self {
            origin,
            dimensions: Vec::new(),
            bindable_dimensions: BTreeSet::new(),
            kind_bindings: BTreeMap::new(),
            dimension_bindings: BTreeMap::new(),
            imported_evidence: Vec::new(),
            imported_type_groups: Vec::new(),
            rigid_dimension_origins: BTreeMap::new(),
            predicate_constrained_kinds: BTreeSet::new(),
            score: OverloadScore::default(),
            conversion_rewrites: Vec::new(),
            conversion_count: 0,
        }
    }

    pub fn solve_scheme(
        mut self,
        scheme: &KindScheme,
        inputs: &[ResolvedType],
        expected_outputs: Option<&[ResolvedType]>,
    ) -> Result<SchemeResolution, TypeResolutionError> {
        let expected_input_count = match scheme.inputs() {
            InputKindScheme::Fixed(expected) => {
                if expected.len() != inputs.len() {
                    return self.fail(TypeConstraintFailure::Arity {
                        expected: expected.len().to_string(),
                        actual: inputs.len(),
                    });
                }
                expected.len()
            }
            InputKindScheme::Variadic {
                prefix,
                min_repetitions,
                ..
            } => {
                let minimum = prefix
                    .len()
                    .checked_add(*min_repetitions as usize)
                    .ok_or_else(|| {
                        self.error(TypeConstraintFailure::InvalidScheme {
                            reason: "variadic input arity exceeds the host index range".into(),
                        })
                    })?;
                if inputs.len() < minimum {
                    return self.fail(TypeConstraintFailure::Arity {
                        expected: format!("at least {minimum}"),
                        actual: inputs.len(),
                    });
                }
                inputs.len()
            }
        };

        let actual_inputs = inputs
            .iter()
            .map(|input| self.import_resolved_type(input))
            .collect::<Result<Vec<_>, _>>()?;
        let actual_outputs = expected_outputs
            .map(|outputs| {
                outputs
                    .iter()
                    .map(|output| self.import_resolved_type(output))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;
        let scheme = self.import_scheme(scheme)?;
        for constraint in scheme.constraints.iter() {
            match constraint {
                KindConstraint::Satisfies { kind, .. } | KindConstraint::Keyable(kind) => {
                    collect_kind_parameter_ids(kind, &mut self.predicate_constrained_kinds);
                }
                _ => {}
            }
        }
        let expected_inputs = match scheme.inputs {
            InputKindScheme::Fixed(inputs) => inputs.into_vec(),
            InputKindScheme::Variadic {
                prefix,
                repeated,
                min_repetitions: _,
            } => {
                let mut expanded = prefix.into_vec();
                expanded.extend((expanded.len()..expected_input_count).map(|_| repeated.clone()));
                expanded
            }
        };

        for ((expected, actual), predicates) in expected_inputs.iter().zip(&actual_inputs).zip(
            inputs
                .iter()
                .map(|input| input.predicates_for(input.kind())),
        ) {
            self.unify_kind(expected, actual, Some(predicates))?;
        }

        if let (Some(actual_outputs), Some(expected_output_types)) =
            (&actual_outputs, expected_outputs)
        {
            if actual_outputs.len() != scheme.outputs.len() {
                return self.fail(TypeConstraintFailure::Arity {
                    expected: format!("{} outputs", scheme.outputs.len()),
                    actual: actual_outputs.len(),
                });
            }
            for ((expected, actual), predicates) in scheme.outputs.iter().zip(actual_outputs).zip(
                expected_output_types
                    .iter()
                    .map(|output| output.predicates_for(output.kind())),
            ) {
                self.unify_kind(expected, actual, Some(predicates))?;
            }
        }

        // Solve equivalence groups before directional conversions and bounds.
        for constraint in &scheme.constraints {
            match constraint {
                KindConstraint::Equal(left, right) => self.unify_kind(left, right, None)?,
                KindConstraint::DimensionEqual(left, right) => self.unify_dimension(left, right)?,
                _ => {}
            }
        }
        for constraint in &scheme.constraints {
            if let KindConstraint::TableJoin {
                left,
                right,
                output,
                rows,
                mode,
            } = constraint
            {
                self.require_table_join(left, right, output, rows, *mode)?;
            }
        }
        for constraint in &scheme.constraints {
            match constraint {
                KindConstraint::Keyable(kind) => self.require_keyable(kind)?,
                KindConstraint::Satisfies { kind, predicate } => {
                    let kind = self.substitute_kind(kind)?;
                    if !self.known_predicates(&kind).contains(*predicate) {
                        return self.fail(TypeConstraintFailure::PredicateNotSatisfied {
                            kind: semantic_kind_name(&kind),
                            predicate: *predicate,
                        });
                    }
                    self.score.predicate_generality = self
                        .score
                        .predicate_generality
                        .checked_add(predicate_generality(*predicate))
                        .ok_or_else(|| {
                            self.error(TypeConstraintFailure::InvalidScheme {
                                reason: "predicate-generality score overflow".into(),
                            })
                        })?;
                }
                _ => {}
            }
        }
        for constraint in &scheme.constraints {
            if let KindConstraint::Promotes {
                left,
                right,
                output,
            } = constraint
            {
                let left = self.substitute_kind(left)?;
                let right = self.substitute_kind(right)?;
                let left_type = self.relation_type(left)?;
                let right_type = self.relation_type(right)?;
                let Some(promotion) = plan_numeric_promotion(&left_type, &right_type)? else {
                    return self.fail(TypeConstraintFailure::PromotionUnavailable {
                        left: left_type.semantic_name(),
                        right: right_type.semantic_name(),
                    });
                };
                self.add_conversion_cost(promotion.left.cost)?;
                self.add_conversion_cost(promotion.right.cost)?;
                if promotion.left.cost != 0 {
                    self.add_conversion_rewrite(&promotion.left)?;
                }
                if promotion.right.cost != 0 {
                    self.add_conversion_rewrite(&promotion.right)?;
                }
                self.unify_kind(output, promotion.result.kind(), None)?;
            }
        }
        for constraint in &scheme.constraints {
            if let KindConstraint::Convertible(source, target) = constraint {
                self.require_conversion(source, target)?;
            }
        }
        for constraint in &scheme.constraints {
            match constraint {
                KindConstraint::DimensionCompatible(left, right) => {
                    self.require_dimension_compatible(left, right)?;
                }
                KindConstraint::DimensionLessEqual(left, right) => {
                    self.require_dimension_less_equal(left, right)?;
                }
                _ => {}
            }
        }
        self.validate_kind_parameter_bounds(&scheme.kind_parameters)?;
        self.validate_dimension_parameter_bounds()?;

        let conversions = expected_inputs
            .iter()
            .zip(&actual_inputs)
            .zip(inputs)
            .map(|((expected, imported_actual), actual)| {
                self.close_input_conversion(expected, imported_actual, actual)
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_boxed_slice();

        let outputs = scheme
            .outputs
            .iter()
            .map(|output| self.close_output(output))
            .collect::<Result<Vec<_>, _>>()?
            .into_boxed_slice();
        Ok(SchemeResolution {
            outputs,
            conversions,
            conversion_count: self.conversion_count,
            score: self.score,
        })
    }

    fn import_resolved_type(
        &mut self,
        resolved: &ResolvedType,
    ) -> Result<KindExpr, TypeResolutionError> {
        let offset = self.dimensions.len();
        let mapping = dimension_offset_mapping(resolved.dimension_parameters().len(), offset)?;
        let type_equivalence_group = if let Some(index) = self
            .imported_type_groups
            .iter()
            .position(|(known, _)| exact_type_equal(known, resolved))
        {
            let predicate = self.imported_type_groups[index].1;
            let intersected = self.imported_type_groups[index]
                .0
                .with_intersected_evidence(resolved)
                .map_err(|error| error.with_origin(self.origin.clone()))?;
            self.imported_type_groups[index].0 = intersected;
            predicate
        } else {
            let predicate = u32::try_from(self.imported_type_groups.len()).map_err(|_| {
                self.error(TypeConstraintFailure::InvalidScheme {
                    reason: "rigid type namespace exhausted".into(),
                })
            })?;
            self.imported_type_groups
                .push((resolved.clone(), predicate));
            predicate
        };
        for declaration in resolved.dimension_parameters() {
            let shifted = shift_declaration(declaration, &mapping)?;
            self.rigid_dimension_origins.insert(
                shifted.id,
                RigidDimensionOrigin {
                    type_equivalence_group,
                    local_parameter: declaration.id,
                },
            );
            self.dimensions.push(shifted);
        }
        let imported = rewrite_kind_dimensions(resolved.kind(), &mapping)
            .map_err(TypeResolutionError::semantic)
            .map_err(|error| error.with_origin(self.origin.clone()))?;
        for evidence in resolved.evidence() {
            let known = rewrite_kind_dimensions(evidence.kind(), &mapping)
                .map_err(TypeResolutionError::semantic)
                .map_err(|error| error.with_origin(self.origin.clone()))?;
            self.merge_imported_evidence(known, evidence.predicates());
        }
        Ok(imported)
    }

    fn merge_imported_evidence(&mut self, kind: KindExpr, predicates: BuiltinKindPredicateSet) {
        if let Some((_, existing)) = self
            .imported_evidence
            .iter_mut()
            .find(|(known, _)| *known == kind)
        {
            *existing = existing.intersection(predicates);
        } else {
            self.imported_evidence.push((kind, predicates));
        }
    }

    fn import_scheme(
        &mut self,
        scheme: &KindScheme,
    ) -> Result<ImportedScheme, TypeResolutionError> {
        let offset = self.dimensions.len();
        let mapping = dimension_offset_mapping(scheme.dimension_parameters().len(), offset)?;
        let mut bindable = Vec::new();
        match scheme.inputs() {
            InputKindScheme::Fixed(inputs) => {
                for input in inputs {
                    collect_kind_dimension_references(input, &mut bindable);
                }
            }
            InputKindScheme::Variadic {
                prefix, repeated, ..
            } => {
                for input in prefix {
                    collect_kind_dimension_references(input, &mut bindable);
                }
                collect_kind_dimension_references(repeated, &mut bindable);
            }
        }
        for constraint in scheme.constraints() {
            match constraint {
                KindConstraint::Equal(left, right) | KindConstraint::Convertible(left, right) => {
                    collect_kind_dimension_references(left, &mut bindable);
                    collect_kind_dimension_references(right, &mut bindable);
                }
                KindConstraint::Keyable(kind) | KindConstraint::Satisfies { kind, .. } => {
                    collect_kind_dimension_references(kind, &mut bindable);
                }
                KindConstraint::Promotes {
                    left,
                    right,
                    output,
                } => {
                    collect_kind_dimension_references(left, &mut bindable);
                    collect_kind_dimension_references(right, &mut bindable);
                    collect_kind_dimension_references(output, &mut bindable);
                }
                KindConstraint::TableJoin {
                    left,
                    right,
                    output,
                    ..
                } => {
                    collect_kind_dimension_references(left, &mut bindable);
                    collect_kind_dimension_references(right, &mut bindable);
                    collect_kind_dimension_references(output, &mut bindable);
                }
                KindConstraint::DimensionEqual(left, right)
                | KindConstraint::DimensionCompatible(left, right)
                | KindConstraint::DimensionLessEqual(left, right) => {
                    collect_dimension_references(left, &mut bindable);
                    collect_dimension_references(right, &mut bindable);
                }
            }
        }
        for declaration in scheme.dimension_parameters() {
            let shifted = shift_declaration(declaration, &mapping)?;
            if bindable.contains(&declaration.id) {
                self.bindable_dimensions.insert(shifted.id);
            }
            self.dimensions.push(shifted);
        }
        let rewrite = |kind: &KindExpr| {
            rewrite_kind_dimensions(kind, &mapping)
                .map_err(TypeResolutionError::semantic)
                .map_err(|error| error.with_origin(self.origin.clone()))
        };
        let inputs = match scheme.inputs() {
            InputKindScheme::Fixed(inputs) => InputKindScheme::Fixed(
                inputs
                    .iter()
                    .map(rewrite)
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            ),
            InputKindScheme::Variadic {
                prefix,
                repeated,
                min_repetitions,
            } => InputKindScheme::Variadic {
                prefix: prefix
                    .iter()
                    .map(rewrite)
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
                repeated: rewrite(repeated)?,
                min_repetitions: *min_repetitions,
            },
        };
        Ok(ImportedScheme {
            kind_parameters: scheme.kind_parameters().to_vec().into_boxed_slice(),
            inputs,
            outputs: scheme
                .outputs()
                .iter()
                .map(rewrite)
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
            constraints: scheme
                .constraints()
                .iter()
                .map(|constraint| rewrite_constraint(constraint, &mapping, &self.origin))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        })
    }

    fn unify_kind(
        &mut self,
        expected: &KindExpr,
        actual: &KindExpr,
        actual_predicates: Option<BuiltinKindPredicateSet>,
    ) -> Result<(), TypeResolutionError> {
        let expected_parameter = match expected {
            KindExpr::Parameter(id) => Some(*id),
            _ => None,
        };
        let expected = self.substitute_kind(expected)?;
        let actual = self.substitute_kind(actual)?;
        if expected == actual {
            if let (Some(id), Some(predicates)) = (expected_parameter, actual_predicates)
                && let Some(binding) = self.kind_bindings.get_mut(&id)
            {
                binding.predicates = binding.predicates.intersection(predicates);
            }
            return Ok(());
        }
        match (&expected, &actual) {
            (KindExpr::Hole, _) | (_, KindExpr::Hole) => {
                self.fail(TypeConstraintFailure::UnresolvedKindHole)
            }
            (KindExpr::Parameter(id), _) => self.bind_kind(*id, &actual, actual_predicates),
            (_, KindExpr::Parameter(id)) => self.bind_kind(*id, &expected, None),
            (KindExpr::Wildcard, _) => {
                self.score.wildcard_matches =
                    self.score.wildcard_matches.checked_add(1).ok_or_else(|| {
                        self.error(TypeConstraintFailure::InvalidScheme {
                            reason: "overload wildcard score overflow".into(),
                        })
                    })?;
                Ok(())
            }
            (_, KindExpr::Wildcard) => self.fail(TypeConstraintFailure::ExactTypeMismatch {
                expected: semantic_kind_name(&expected),
                actual: semantic_kind_name(&actual),
            }),
            (KindExpr::Atom(_), KindExpr::Atom(_))
            | (KindExpr::Enum(_), KindExpr::Enum(_))
            | (KindExpr::Named(_), KindExpr::Named(_)) => {
                self.fail(TypeConstraintFailure::ConflictingNominalTypes {
                    expected: semantic_kind_name(&expected),
                    actual: semantic_kind_name(&actual),
                })
            }
            (
                KindExpr::Matrix {
                    element: expected_element,
                    dimensions: expected_dimensions,
                },
                KindExpr::Matrix {
                    element: actual_element,
                    dimensions: actual_dimensions,
                },
            ) => {
                if expected_dimensions.len() != actual_dimensions.len() {
                    return self.fail(TypeConstraintFailure::StructuralMismatch {
                        expected: semantic_kind_name(&expected),
                        actual: semantic_kind_name(&actual),
                    });
                }
                self.unify_kind(expected_element, actual_element, None)?;
                for (expected, actual) in expected_dimensions.iter().zip(actual_dimensions) {
                    self.unify_dimension(expected, actual)?;
                }
                Ok(())
            }
            (KindExpr::Option(expected), KindExpr::Option(actual))
            | (KindExpr::Reference(expected), KindExpr::Reference(actual))
            | (KindExpr::TypeOf(expected), KindExpr::TypeOf(actual)) => {
                self.unify_kind(expected, actual, None)
            }
            (KindExpr::Tuple(expected), KindExpr::Tuple(actual)) => {
                self.unify_kind_lists(expected, actual)
            }
            (KindExpr::Record(expected), KindExpr::Record(actual)) => {
                self.unify_fields(expected, actual)
            }
            (
                KindExpr::Table {
                    columns: expected_columns,
                    rows: expected_rows,
                },
                KindExpr::Table {
                    columns: actual_columns,
                    rows: actual_rows,
                },
            ) => {
                self.unify_fields(expected_columns, actual_columns)?;
                self.unify_dimension(expected_rows, actual_rows)
            }
            (
                KindExpr::Set {
                    element: expected_element,
                    cardinality: expected_cardinality,
                },
                KindExpr::Set {
                    element: actual_element,
                    cardinality: actual_cardinality,
                },
            ) => {
                self.unify_kind(expected_element, actual_element, None)?;
                self.unify_dimension(expected_cardinality, actual_cardinality)
            }
            (
                KindExpr::Map {
                    key: expected_key,
                    value: expected_value,
                    cardinality: expected_cardinality,
                },
                KindExpr::Map {
                    key: actual_key,
                    value: actual_value,
                    cardinality: actual_cardinality,
                },
            ) => {
                self.unify_kind(expected_key, actual_key, None)?;
                self.unify_kind(expected_value, actual_value, None)?;
                self.unify_dimension(expected_cardinality, actual_cardinality)
            }
            _ => self.fail(TypeConstraintFailure::StructuralMismatch {
                expected: semantic_kind_name(&expected),
                actual: semantic_kind_name(&actual),
            }),
        }
    }

    fn unify_kind_lists(
        &mut self,
        expected: &[KindExpr],
        actual: &[KindExpr],
    ) -> Result<(), TypeResolutionError> {
        if expected.len() != actual.len() {
            return self.fail(TypeConstraintFailure::StructuralMismatch {
                expected: format!("{} elements", expected.len()),
                actual: format!("{} elements", actual.len()),
            });
        }
        for (expected, actual) in expected.iter().zip(actual) {
            self.unify_kind(expected, actual, None)?;
        }
        Ok(())
    }

    fn unify_fields(
        &mut self,
        expected: &[KindField],
        actual: &[KindField],
    ) -> Result<(), TypeResolutionError> {
        if expected.len() != actual.len()
            || expected
                .iter()
                .zip(actual)
                .any(|(expected, actual)| expected.name != actual.name)
        {
            return self.fail(TypeConstraintFailure::StructuralMismatch {
                expected: format_fields("aggregate", expected),
                actual: format_fields("aggregate", actual),
            });
        }
        for (expected, actual) in expected.iter().zip(actual) {
            self.unify_kind(&expected.kind, &actual.kind, None)?;
        }
        Ok(())
    }

    fn bind_kind(
        &mut self,
        id: KindParameterId,
        kind: &KindExpr,
        predicates: Option<BuiltinKindPredicateSet>,
    ) -> Result<(), TypeResolutionError> {
        if matches!(kind, KindExpr::Parameter(other) if *other == id) {
            return Ok(());
        }
        if kind_contains_parameter(kind, id) {
            return self.fail(TypeConstraintFailure::CyclicKindBinding { parameter: id });
        }
        if let Some(existing) = self.kind_bindings.get(&id).cloned() {
            self.unify_kind(&existing.kind, kind, predicates)?;
            if let Some(predicates) = predicates {
                if let Some(binding) = self.kind_bindings.get_mut(&id) {
                    binding.predicates = binding.predicates.intersection(predicates);
                }
            }
            return Ok(());
        }
        if !self.predicate_constrained_kinds.contains(&id) {
            let penalty = unconstrained_binding_generality(kind);
            self.score.unconstrained_kind_bindings = self
                .score
                .unconstrained_kind_bindings
                .checked_add(penalty)
                .ok_or_else(|| {
                    self.error(TypeConstraintFailure::InvalidScheme {
                        reason: "overload kind-binding score overflow".into(),
                    })
                })?;
        }
        let predicates = predicates.unwrap_or_else(|| self.known_predicates(kind));
        self.kind_bindings.insert(
            id,
            KindBinding {
                kind: kind.clone(),
                predicates,
            },
        );
        Ok(())
    }

    fn unify_dimension(
        &mut self,
        expected: &DimensionExpr,
        actual: &DimensionExpr,
    ) -> Result<(), TypeResolutionError> {
        let expected = self.substitute_dimension(expected)?;
        let actual = self.substitute_dimension(actual)?;
        if expected == actual {
            return Ok(());
        }
        if matches!(expected, DimensionExpr::Hole) || matches!(actual, DimensionExpr::Hole) {
            return self.fail(TypeConstraintFailure::UnresolvedDimensionHole);
        }
        if let DimensionExpr::Parameter(id) = expected
            && self.bindable_dimensions.contains(&id)
        {
            return self.bind_dimension(id, &actual);
        }
        if let DimensionExpr::Parameter(id) = actual
            && self.bindable_dimensions.contains(&id)
        {
            return self.bind_dimension(id, &expected);
        }
        if self.rigid_expression_equivalent(&expected, &actual) {
            return Ok(());
        }
        if matches!(expected, DimensionExpr::Constant(_))
            && matches!(actual, DimensionExpr::Parameter(_))
        {
            return self.fail(TypeConstraintFailure::InvalidDynamicFixedExtent {
                expected: semantic_dimension_name(&expected),
                actual: semantic_dimension_name(&actual),
            });
        }
        self.fail(TypeConstraintFailure::IncompatibleDimensions {
            expected: semantic_dimension_name(&expected),
            actual: semantic_dimension_name(&actual),
        })
    }

    fn rigid_expression_equivalent(&self, left: &DimensionExpr, right: &DimensionExpr) -> bool {
        match (left, right) {
            (DimensionExpr::Constant(left), DimensionExpr::Constant(right)) => left == right,
            (DimensionExpr::Parameter(left), DimensionExpr::Parameter(right)) => {
                self.rigid_dimension_origins.get(left) == self.rigid_dimension_origins.get(right)
                    && self.rigid_dimension_origins.contains_key(left)
            }
            (DimensionExpr::Add(left), DimensionExpr::Add(right))
            | (DimensionExpr::Multiply(left), DimensionExpr::Multiply(right))
            | (DimensionExpr::Min(left), DimensionExpr::Min(right))
            | (DimensionExpr::Max(left), DimensionExpr::Max(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right.iter())
                        .all(|(left, right)| self.rigid_expression_equivalent(left, right))
            }
            _ => false,
        }
    }

    fn bind_dimension(
        &mut self,
        id: DimensionParameterId,
        expression: &DimensionExpr,
    ) -> Result<(), TypeResolutionError> {
        if matches!(expression, DimensionExpr::Parameter(other) if *other == id) {
            return Ok(());
        }
        if dimension_contains_parameter(expression, id) {
            return self.fail(TypeConstraintFailure::CyclicDimensionBinding { parameter: id });
        }
        if let Some(existing) = self.dimension_bindings.get(&id).cloned() {
            return self.unify_dimension(&existing, expression);
        }
        self.validate_dynamic_binding(id, expression)?;
        self.score.unconstrained_dimension_bindings = self
            .score
            .unconstrained_dimension_bindings
            .checked_add(1)
            .ok_or_else(|| {
                self.error(TypeConstraintFailure::InvalidScheme {
                    reason: "overload dimension-binding score overflow".into(),
                })
            })?;
        self.dimension_bindings.insert(id, expression.clone());
        Ok(())
    }

    fn validate_dynamic_binding(
        &self,
        expected: DimensionParameterId,
        actual: &DimensionExpr,
    ) -> Result<(), TypeResolutionError> {
        let Some(expected_declaration) = self.dimensions.get(expected.get() as usize) else {
            return self.fail(TypeConstraintFailure::UnresolvedDimensionVariable {
                parameter: expected,
            });
        };
        let actual_evolution = self.dimension_evolution(actual)?;
        let expected_evolution = declaration_evolution(expected_declaration);
        if actual_evolution > expected_evolution {
            if expected_evolution == 2 && actual_evolution == 3 {
                return self.fail(TypeConstraintFailure::DimensionBoundNotProven {
                    relation: format!(
                        "{} must satisfy the declared upper bound of d{}",
                        semantic_dimension_name(actual),
                        expected.get(),
                    ),
                });
            }
            return self.fail(TypeConstraintFailure::InvalidDynamicFixedExtent {
                expected: evolution_name(expected_evolution).into(),
                actual: evolution_name(actual_evolution).into(),
            });
        }
        Ok(())
    }

    fn dimension_evolution(&self, expression: &DimensionExpr) -> Result<u8, TypeResolutionError> {
        match expression {
            DimensionExpr::Hole => self.fail(TypeConstraintFailure::UnresolvedDimensionHole),
            DimensionExpr::Constant(_) => Ok(0),
            DimensionExpr::Parameter(id) => self
                .dimensions
                .get(id.get() as usize)
                .map(declaration_evolution)
                .ok_or_else(|| {
                    self.error(TypeConstraintFailure::UnresolvedDimensionVariable {
                        parameter: *id,
                    })
                }),
            DimensionExpr::Add(children)
            | DimensionExpr::Multiply(children)
            | DimensionExpr::Min(children)
            | DimensionExpr::Max(children) => children
                .iter()
                .map(|child| self.dimension_evolution(child))
                .try_fold(0_u8, |current, child| child.map(|child| current.max(child))),
        }
    }

    fn require_conversion(
        &mut self,
        source: &KindExpr,
        target: &KindExpr,
    ) -> Result<(), TypeResolutionError> {
        let source = self.substitute_kind(source)?;
        let target = self.substitute_kind(target)?;
        let source = self.relation_type(source)?;
        let target = self.relation_type(target)?;
        let plan = plan_implicit_conversion(&source, &target)
            .map_err(|error| error.with_origin(self.origin.clone()))?;
        self.add_conversion_cost(plan.cost)?;
        if plan.cost != 0 {
            self.add_conversion_rewrite(&plan)?;
        }
        Ok(())
    }

    fn add_conversion_rewrite(&mut self, plan: &ConversionPlan) -> Result<(), TypeResolutionError> {
        let source = plan.source.kind().clone();
        let target = plan.target.kind().clone();
        if let Some((_, existing)) = self
            .conversion_rewrites
            .iter()
            .find(|(known, _)| *known == source)
        {
            if *existing != target {
                return self.fail(TypeConstraintFailure::InvalidScheme {
                    reason: format!(
                        "one semantic input type is converted to both {} and {}",
                        semantic_kind_name(existing),
                        semantic_kind_name(&target),
                    ),
                });
            }
            return Ok(());
        }
        self.conversion_rewrites.push((source, target));
        Ok(())
    }

    fn close_input_conversion(
        &self,
        input: &KindExpr,
        imported_actual: &KindExpr,
        actual: &ResolvedType,
    ) -> Result<ConversionPlan, TypeResolutionError> {
        let input = self.substitute_kind(input)?;
        let input = substitute_kind_dimensions(&input, &mut |dimension| {
            self.substitute_dimension(dimension)
        })?;
        let input = materialize_input_wildcards(&input, imported_actual);
        let target = rewrite_kind_for_conversions(&input, &self.conversion_rewrites);
        let target = self.relation_type(target)?;
        plan_implicit_conversion(actual, &target)
            .map_err(|error| error.with_origin(self.origin.clone()))
    }

    fn relation_type(&self, kind: KindExpr) -> Result<ResolvedType, TypeResolutionError> {
        ResolvedType::new(kind, self.dimensions.clone().into_boxed_slice())
            .map_err(|error| error.with_origin(self.origin.clone()))
    }

    fn add_conversion_cost(&mut self, cost: u32) -> Result<(), TypeResolutionError> {
        if cost == 0 {
            return Ok(());
        }
        self.conversion_count = self.conversion_count.checked_add(1).ok_or_else(|| {
            self.error(TypeConstraintFailure::InvalidScheme {
                reason: "conversion count overflow".into(),
            })
        })?;
        self.score.conversion_cost =
            self.score
                .conversion_cost
                .checked_add(cost)
                .ok_or_else(|| {
                    self.error(TypeConstraintFailure::InvalidScheme {
                        reason: "conversion score overflow".into(),
                    })
                })?;
        Ok(())
    }

    fn require_keyable(&self, kind: &KindExpr) -> Result<(), TypeResolutionError> {
        let kind = self.substitute_kind(kind)?;
        let keyable = self
            .known_predicates(&kind)
            .contains(BuiltinKindPredicate::Keyable);
        if keyable {
            Ok(())
        } else {
            self.fail(TypeConstraintFailure::NotKeyable {
                kind: semantic_kind_name(&kind),
            })
        }
    }

    fn require_table_join(
        &mut self,
        left: &KindExpr,
        right: &KindExpr,
        output: &KindExpr,
        rows: &DimensionExpr,
        mode: TableJoinMode,
    ) -> Result<(), TypeResolutionError> {
        let left = self.substitute_kind(left)?;
        let right = self.substitute_kind(right)?;
        let (
            KindExpr::Table {
                columns: left_columns,
                ..
            },
            KindExpr::Table {
                columns: right_columns,
                ..
            },
        ) = (&left, &right)
        else {
            return self.fail(TypeConstraintFailure::ExactTypeMismatch {
                expected: "Table and Table".into(),
                actual: format!(
                    "{} and {}",
                    semantic_kind_name(&left),
                    semantic_kind_name(&right)
                ),
            });
        };

        let mut common_right = BTreeSet::new();
        for left_field in left_columns.iter() {
            if let Some((index, right_field)) = right_columns
                .iter()
                .enumerate()
                .find(|(_, field)| field.name == left_field.name)
            {
                self.unify_kind(&left_field.kind, &right_field.kind, None)?;
                common_right.insert(index);
            }
        }

        let optional = |kind: &KindExpr| match kind {
            KindExpr::Option(_) => kind.clone(),
            _ => KindExpr::Option(Box::new(kind.clone())),
        };
        let left_outer = matches!(mode, TableJoinMode::RightOuter | TableJoinMode::FullOuter);
        let right_outer = matches!(mode, TableJoinMode::LeftOuter | TableJoinMode::FullOuter);
        let left_only = matches!(mode, TableJoinMode::LeftSemi | TableJoinMode::LeftAnti);
        let mut columns = left_columns
            .iter()
            .map(|field| KindField {
                name: field.name.clone(),
                kind: if left_outer
                    && !right_columns
                        .iter()
                        .any(|candidate| candidate.name == field.name)
                {
                    optional(&field.kind)
                } else {
                    field.kind.clone()
                },
            })
            .collect::<Vec<_>>();
        if !left_only {
            columns.extend(
                right_columns
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| !common_right.contains(index))
                    .map(|(_, field)| KindField {
                        name: field.name.clone(),
                        kind: if right_outer {
                            optional(&field.kind)
                        } else {
                            field.kind.clone()
                        },
                    }),
            );
        }
        self.unify_kind(
            output,
            &KindExpr::Table {
                columns: columns.into_boxed_slice(),
                rows: rows.clone(),
            },
            None,
        )
    }

    fn known_predicates(&self, kind: &KindExpr) -> BuiltinKindPredicateSet {
        self.imported_evidence
            .iter()
            .find_map(|(imported, predicates)| (imported == kind).then_some(*predicates))
            .unwrap_or_else(|| kind_predicates(kind, &self.kind_bindings))
    }

    fn require_dimension_less_equal(
        &self,
        left: &DimensionExpr,
        right: &DimensionExpr,
    ) -> Result<(), TypeResolutionError> {
        let left = self.substitute_dimension(left)?;
        let right = self.substitute_dimension(right)?;
        if left == right {
            return Ok(());
        }
        let left_interval = self.dimension_interval(&left)?;
        let right_interval = self.dimension_interval(&right)?;
        if prove_less_equal(left_interval, right_interval) {
            Ok(())
        } else {
            self.fail(TypeConstraintFailure::DimensionBoundNotProven {
                relation: format!(
                    "{} <= {}",
                    semantic_dimension_name(&left),
                    semantic_dimension_name(&right)
                ),
            })
        }
    }

    fn require_dimension_compatible(
        &self,
        left: &DimensionExpr,
        right: &DimensionExpr,
    ) -> Result<(), TypeResolutionError> {
        let left = self.substitute_dimension(left)?;
        let right = self.substitute_dimension(right)?;
        if left == right || self.rigid_expression_equivalent(&left, &right) {
            return Ok(());
        }
        let left_evolution = self.dimension_evolution(&left)?;
        let right_evolution = self.dimension_evolution(&right)?;
        if left_evolution >= 2 || right_evolution >= 2 {
            return Ok(());
        }
        let left_interval = self.dimension_interval(&left)?;
        let right_interval = self.dimension_interval(&right)?;
        if left_interval.1 == Some(left_interval.0)
            && right_interval.1 == Some(right_interval.0)
            && left_interval.0 == right_interval.0
        {
            return Ok(());
        }
        self.fail(TypeConstraintFailure::IncompatibleDimensions {
            expected: semantic_dimension_name(&left),
            actual: semantic_dimension_name(&right),
        })
    }

    fn validate_kind_parameter_bounds(
        &self,
        parameters: &[crate::KindParameter],
    ) -> Result<(), TypeResolutionError> {
        for parameter in parameters {
            let Some(bound) = &parameter.upper_bound else {
                continue;
            };
            let Some(binding) = self.kind_bindings.get(&parameter.id) else {
                continue;
            };
            let upper = self.substitute_kind(bound)?;
            if !matches!(upper, KindExpr::Wildcard)
                && !permitted_conversion(
                    &self.relation_type(binding.kind.clone())?,
                    &self.relation_type(upper.clone())?,
                )
            {
                return self.fail(TypeConstraintFailure::ConversionNotPermitted {
                    source: semantic_kind_name(&binding.kind),
                    target: semantic_kind_name(&upper),
                });
            }
        }
        Ok(())
    }

    fn validate_dimension_parameter_bounds(&self) -> Result<(), TypeResolutionError> {
        for id in &self.bindable_dimensions {
            let Some(value) = self.dimension_bindings.get(id) else {
                continue;
            };
            let declaration = &self.dimensions[id.get() as usize];
            let value = self.dimension_interval(value)?;
            let lower = self.dimension_interval(&declaration.lower_bound)?;
            if !prove_greater_equal(value, lower) {
                return self.fail(TypeConstraintFailure::DimensionBoundNotProven {
                    relation: format!("d{} lower bound", id.get()),
                });
            }
            if let Some(upper) = &declaration.upper_bound {
                let upper = self.dimension_interval(upper)?;
                if !prove_less_equal(value, upper) {
                    return self.fail(TypeConstraintFailure::DimensionBoundNotProven {
                        relation: format!("d{} upper bound", id.get()),
                    });
                }
            }
        }
        Ok(())
    }

    fn close_output(&self, output: &KindExpr) -> Result<ResolvedType, TypeResolutionError> {
        let output = self.substitute_kind(output)?;
        let mut unresolved_kind = None;
        visit_kind_parameters(&output, &mut |id| {
            unresolved_kind = Some(id);
            Ok(())
        })
        .map_err(TypeResolutionError::semantic)
        .map_err(|error| error.with_origin(self.origin.clone()))?;
        if let Some(parameter) = unresolved_kind {
            return self.fail(TypeConstraintFailure::UnresolvedKindVariable { parameter });
        }

        let output = substitute_kind_dimensions(&output, &mut |dimension| {
            self.substitute_dimension(dimension)
        })?;
        let mut unresolved_dimension = None;
        visit_kind_dimensions(&output, &mut |dimension| {
            if matches!(dimension, DimensionExpr::Hole) {
                return Err(crate::SemanticModelError::UnresolvedDimensionHole);
            }
            let mut references = Vec::new();
            collect_dimension_references(&dimension, &mut references);
            if let Some(parameter) = references
                .into_iter()
                .find(|parameter| self.bindable_dimensions.contains(parameter))
            {
                unresolved_dimension = Some(parameter);
            }
            Ok(())
        })
        .map_err(TypeResolutionError::semantic)
        .map_err(|error| error.with_origin(self.origin.clone()))?;
        if let Some(parameter) = unresolved_dimension {
            return self.fail(TypeConstraintFailure::UnresolvedDimensionVariable { parameter });
        }
        let mut output_kinds = Vec::new();
        collect_kind_nodes(&output, &mut output_kinds);
        let evidence = output_kinds
            .into_iter()
            .map(|kind| KindPredicateEvidence::new(kind.clone(), self.known_predicates(kind)))
            .collect::<Vec<_>>();
        ResolvedType::new_with_evidence(
            output,
            self.dimensions.clone().into_boxed_slice(),
            evidence,
        )
        .map_err(|error| error.with_origin(self.origin.clone()))
    }

    fn substitute_kind(&self, kind: &KindExpr) -> Result<KindExpr, TypeResolutionError> {
        substitute_kind_parameters(kind, &self.kind_bindings, &self.origin)
    }

    fn substitute_dimension(
        &self,
        dimension: &DimensionExpr,
    ) -> Result<DimensionExpr, TypeResolutionError> {
        substitute_dimension_parameters(
            dimension,
            &self.dimension_bindings,
            self.dimensions.len(),
            &self.origin,
        )
    }

    fn dimension_interval(
        &self,
        dimension: &DimensionExpr,
    ) -> Result<(u64, Option<u64>), TypeResolutionError> {
        let dimension = self.substitute_dimension(dimension)?;
        match dimension {
            DimensionExpr::Hole => self.fail(TypeConstraintFailure::UnresolvedDimensionHole),
            DimensionExpr::Constant(value) => Ok((value, Some(value))),
            DimensionExpr::Parameter(id) => {
                let declaration = self.dimensions.get(id.get() as usize).ok_or_else(|| {
                    TypeResolutionError::Incompatible {
                        origin: self.origin.clone(),
                        failures: vec![TypeConstraintFailure::UnresolvedDimensionVariable {
                            parameter: id,
                        }]
                        .into_boxed_slice(),
                    }
                })?;
                let lower = self.dimension_interval(&declaration.lower_bound)?.0;
                let upper = declaration
                    .upper_bound
                    .as_ref()
                    .map(|upper| self.dimension_interval(upper).map(|interval| interval.1))
                    .transpose()?
                    .flatten();
                Ok((lower, upper))
            }
            DimensionExpr::Add(children) => {
                let mut lower = 0_u64;
                let mut upper = Some(0_u64);
                for child in children {
                    let child = self.dimension_interval(&child)?;
                    lower = lower.checked_add(child.0).ok_or_else(|| {
                        self.error(TypeConstraintFailure::DimensionBoundNotProven {
                            relation: "dimension addition overflow".to_string(),
                        })
                    })?;
                    upper = match (upper, child.1) {
                        (Some(left), Some(right)) => left.checked_add(right),
                        _ => None,
                    };
                }
                Ok((lower, upper))
            }
            DimensionExpr::Multiply(children) => {
                let mut lower = 1_u64;
                let mut upper = Some(1_u64);
                for child in children {
                    let child = self.dimension_interval(&child)?;
                    lower = lower.checked_mul(child.0).ok_or_else(|| {
                        self.error(TypeConstraintFailure::DimensionBoundNotProven {
                            relation: "dimension multiplication overflow".to_string(),
                        })
                    })?;
                    upper = match (upper, child.1) {
                        (Some(left), Some(right)) => left.checked_mul(right),
                        _ => None,
                    };
                }
                Ok((lower, upper))
            }
            DimensionExpr::Min(children) => {
                let intervals = children
                    .iter()
                    .map(|child| self.dimension_interval(child))
                    .collect::<Result<Vec<_>, _>>()?;
                let lower = intervals
                    .iter()
                    .map(|interval| interval.0)
                    .min()
                    .ok_or_else(|| {
                        self.error(TypeConstraintFailure::DimensionBoundNotProven {
                            relation: "empty min".to_string(),
                        })
                    })?;
                let upper = intervals.iter().filter_map(|interval| interval.1).min();
                Ok((lower, upper))
            }
            DimensionExpr::Max(children) => {
                let intervals = children
                    .iter()
                    .map(|child| self.dimension_interval(child))
                    .collect::<Result<Vec<_>, _>>()?;
                let lower = intervals
                    .iter()
                    .map(|interval| interval.0)
                    .max()
                    .ok_or_else(|| {
                        self.error(TypeConstraintFailure::DimensionBoundNotProven {
                            relation: "empty max".to_string(),
                        })
                    })?;
                let upper = if intervals.iter().all(|interval| interval.1.is_some()) {
                    intervals.iter().filter_map(|interval| interval.1).max()
                } else {
                    None
                };
                Ok((lower, upper))
            }
        }
    }

    fn error(&self, failure: TypeConstraintFailure) -> TypeResolutionError {
        TypeResolutionError::Incompatible {
            origin: self.origin.clone(),
            failures: vec![failure].into_boxed_slice(),
        }
    }

    fn fail<T>(&self, failure: TypeConstraintFailure) -> Result<T, TypeResolutionError> {
        Err(self.error(failure))
    }
}

fn unconstrained_binding_generality(kind: &KindExpr) -> u32 {
    match kind {
        KindExpr::Matrix { element, .. }
        | KindExpr::Option(element)
        | KindExpr::Set { element, .. }
        | KindExpr::Reference(element)
        | KindExpr::TypeOf(element) => 1 + unconstrained_binding_generality(element),
        KindExpr::Tuple(elements) => {
            1 + elements
                .iter()
                .map(unconstrained_binding_generality)
                .sum::<u32>()
        }
        KindExpr::Record(fields)
        | KindExpr::Table {
            columns: fields, ..
        } => {
            1 + fields
                .iter()
                .map(|field| unconstrained_binding_generality(&field.kind))
                .sum::<u32>()
        }
        KindExpr::Map { key, value, .. } => {
            1 + unconstrained_binding_generality(key) + unconstrained_binding_generality(value)
        }
        _ => 1,
    }
}

struct ImportedScheme {
    kind_parameters: Box<[crate::KindParameter]>,
    inputs: InputKindScheme,
    outputs: Box<[KindExpr]>,
    constraints: Box<[KindConstraint]>,
}

fn rewrite_kind_for_conversions(kind: &KindExpr, rewrites: &[(KindExpr, KindExpr)]) -> KindExpr {
    rewrites
        .iter()
        .fold(kind.clone(), |kind, (source, target)| {
            rewrite_kind_once(&kind, source, target)
        })
}

fn materialize_input_wildcards(expected: &KindExpr, actual: &KindExpr) -> KindExpr {
    match (expected, actual) {
        (KindExpr::Wildcard, actual) => actual.clone(),
        (
            KindExpr::Matrix {
                element,
                dimensions,
            },
            KindExpr::Matrix {
                element: actual_element,
                ..
            },
        ) => KindExpr::Matrix {
            element: Box::new(materialize_input_wildcards(element, actual_element)),
            dimensions: dimensions.clone(),
        },
        (KindExpr::Option(payload), KindExpr::Option(actual_payload)) => KindExpr::Option(
            Box::new(materialize_input_wildcards(payload, actual_payload)),
        ),
        (KindExpr::Tuple(elements), KindExpr::Tuple(actual_elements))
            if elements.len() == actual_elements.len() =>
        {
            KindExpr::Tuple(
                elements
                    .iter()
                    .zip(actual_elements)
                    .map(|(element, actual)| materialize_input_wildcards(element, actual))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )
        }
        (KindExpr::Record(fields), KindExpr::Record(actual_fields))
            if fields.len() == actual_fields.len() =>
        {
            KindExpr::Record(materialize_wildcard_fields(fields, actual_fields))
        }
        (
            KindExpr::Table { columns, rows },
            KindExpr::Table {
                columns: actual_columns,
                ..
            },
        ) if columns.len() == actual_columns.len() => KindExpr::Table {
            columns: materialize_wildcard_fields(columns, actual_columns),
            rows: rows.clone(),
        },
        (
            KindExpr::Set {
                element,
                cardinality,
            },
            KindExpr::Set {
                element: actual_element,
                ..
            },
        ) => KindExpr::Set {
            element: Box::new(materialize_input_wildcards(element, actual_element)),
            cardinality: cardinality.clone(),
        },
        (
            KindExpr::Map {
                key,
                value,
                cardinality,
            },
            KindExpr::Map {
                key: actual_key,
                value: actual_value,
                ..
            },
        ) => KindExpr::Map {
            key: Box::new(materialize_input_wildcards(key, actual_key)),
            value: Box::new(materialize_input_wildcards(value, actual_value)),
            cardinality: cardinality.clone(),
        },
        (KindExpr::Reference(inner), KindExpr::Reference(actual_inner)) => {
            KindExpr::Reference(Box::new(materialize_input_wildcards(inner, actual_inner)))
        }
        (KindExpr::TypeOf(inner), KindExpr::TypeOf(actual_inner)) => {
            KindExpr::TypeOf(Box::new(materialize_input_wildcards(inner, actual_inner)))
        }
        _ => expected.clone(),
    }
}

fn materialize_wildcard_fields(fields: &[KindField], actual: &[KindField]) -> Box<[KindField]> {
    fields
        .iter()
        .zip(actual)
        .map(|(field, actual)| KindField {
            name: field.name.clone(),
            kind: materialize_input_wildcards(&field.kind, &actual.kind),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn rewrite_kind_once(kind: &KindExpr, source: &KindExpr, target: &KindExpr) -> KindExpr {
    if kind == source {
        return target.clone();
    }
    match kind {
        KindExpr::Matrix {
            element,
            dimensions,
        } => KindExpr::Matrix {
            element: Box::new(rewrite_kind_once(element, source, target)),
            dimensions: dimensions.clone(),
        },
        KindExpr::Option(payload) => {
            KindExpr::Option(Box::new(rewrite_kind_once(payload, source, target)))
        }
        KindExpr::Tuple(elements) => KindExpr::Tuple(
            elements
                .iter()
                .map(|element| rewrite_kind_once(element, source, target))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        KindExpr::Record(fields) => {
            KindExpr::Record(rewrite_conversion_fields(fields, source, target))
        }
        KindExpr::Table { columns, rows } => KindExpr::Table {
            columns: rewrite_conversion_fields(columns, source, target),
            rows: rows.clone(),
        },
        KindExpr::Set {
            element,
            cardinality,
        } => KindExpr::Set {
            element: Box::new(rewrite_kind_once(element, source, target)),
            cardinality: cardinality.clone(),
        },
        KindExpr::Map {
            key,
            value,
            cardinality,
        } => KindExpr::Map {
            key: Box::new(rewrite_kind_once(key, source, target)),
            value: Box::new(rewrite_kind_once(value, source, target)),
            cardinality: cardinality.clone(),
        },
        KindExpr::Reference(inner) => {
            KindExpr::Reference(Box::new(rewrite_kind_once(inner, source, target)))
        }
        KindExpr::TypeOf(inner) => {
            KindExpr::TypeOf(Box::new(rewrite_kind_once(inner, source, target)))
        }
        KindExpr::Named(_)
        | KindExpr::Id
        | KindExpr::Index
        | KindExpr::Atom(_)
        | KindExpr::Enum(_)
        | KindExpr::Wildcard
        | KindExpr::Never
        | KindExpr::Hole
        | KindExpr::Parameter(_) => kind.clone(),
    }
}

fn rewrite_conversion_fields(
    fields: &[KindField],
    source: &KindExpr,
    target: &KindExpr,
) -> Box<[KindField]> {
    fields
        .iter()
        .map(|field| KindField {
            name: field.name.clone(),
            kind: rewrite_kind_once(&field.kind, source, target),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn dimension_offset_mapping(
    count: usize,
    offset: usize,
) -> Result<Vec<Option<DimensionParameterId>>, TypeResolutionError> {
    (0..count)
        .map(|index| {
            let shifted = offset.checked_add(index).ok_or_else(|| {
                TypeResolutionError::incompatible(
                    "type",
                    TypeConstraintFailure::InvalidScheme {
                        reason: "dimension variable space exhausted".to_string(),
                    },
                )
            })?;
            let shifted = u32::try_from(shifted).map_err(|_| {
                TypeResolutionError::incompatible(
                    "type",
                    TypeConstraintFailure::InvalidScheme {
                        reason: "dimension variable space exhausted".to_string(),
                    },
                )
            })?;
            Ok(Some(DimensionParameterId::new(shifted)))
        })
        .collect()
}

fn shift_declaration(
    declaration: &DimensionParameterDeclaration,
    mapping: &[Option<DimensionParameterId>],
) -> Result<DimensionParameterDeclaration, TypeResolutionError> {
    let id = mapping
        .get(declaration.id.get() as usize)
        .and_then(|id| *id)
        .ok_or_else(|| {
            TypeResolutionError::incompatible(
                "type",
                TypeConstraintFailure::UnresolvedDimensionVariable {
                    parameter: declaration.id,
                },
            )
        })?;
    Ok(DimensionParameterDeclaration {
        id,
        origin: declaration.origin,
        lifetime: declaration.lifetime,
        lower_bound: rewrite_dimension_references(&declaration.lower_bound, mapping)
            .map_err(TypeResolutionError::semantic)?,
        upper_bound: declaration
            .upper_bound
            .as_ref()
            .map(|upper| rewrite_dimension_references(upper, mapping))
            .transpose()
            .map_err(TypeResolutionError::semantic)?,
    })
}

fn rewrite_constraint(
    constraint: &KindConstraint,
    mapping: &[Option<DimensionParameterId>],
    origin: &TypeConstraintOrigin,
) -> Result<KindConstraint, TypeResolutionError> {
    let rewrite_kind = |kind: &KindExpr| {
        rewrite_kind_dimensions(kind, mapping)
            .map_err(TypeResolutionError::semantic)
            .map_err(|error| error.with_origin(origin.clone()))
    };
    let rewrite_dimension = |dimension: &DimensionExpr| {
        rewrite_dimension_references(dimension, mapping)
            .map_err(TypeResolutionError::semantic)
            .map_err(|error| error.with_origin(origin.clone()))
    };
    Ok(match constraint {
        KindConstraint::Equal(left, right) => {
            KindConstraint::Equal(rewrite_kind(left)?, rewrite_kind(right)?)
        }
        KindConstraint::Convertible(left, right) => {
            KindConstraint::Convertible(rewrite_kind(left)?, rewrite_kind(right)?)
        }
        KindConstraint::Keyable(kind) => KindConstraint::Keyable(rewrite_kind(kind)?),
        KindConstraint::Satisfies { kind, predicate } => KindConstraint::Satisfies {
            kind: rewrite_kind(kind)?,
            predicate: *predicate,
        },
        KindConstraint::Promotes {
            left,
            right,
            output,
        } => KindConstraint::Promotes {
            left: rewrite_kind(left)?,
            right: rewrite_kind(right)?,
            output: rewrite_kind(output)?,
        },
        KindConstraint::TableJoin {
            left,
            right,
            output,
            rows,
            mode,
        } => KindConstraint::TableJoin {
            left: rewrite_kind(left)?,
            right: rewrite_kind(right)?,
            output: rewrite_kind(output)?,
            rows: rewrite_dimension(rows)?,
            mode: *mode,
        },
        KindConstraint::DimensionEqual(left, right) => {
            KindConstraint::DimensionEqual(rewrite_dimension(left)?, rewrite_dimension(right)?)
        }
        KindConstraint::DimensionCompatible(left, right) => {
            KindConstraint::DimensionCompatible(rewrite_dimension(left)?, rewrite_dimension(right)?)
        }
        KindConstraint::DimensionLessEqual(left, right) => {
            KindConstraint::DimensionLessEqual(rewrite_dimension(left)?, rewrite_dimension(right)?)
        }
    })
}

fn substitute_kind_parameters(
    kind: &KindExpr,
    bindings: &BTreeMap<KindParameterId, KindBinding>,
    origin: &TypeConstraintOrigin,
) -> Result<KindExpr, TypeResolutionError> {
    fn substitute(
        kind: &KindExpr,
        bindings: &BTreeMap<KindParameterId, KindBinding>,
        visiting: &mut BTreeSet<KindParameterId>,
        origin: &TypeConstraintOrigin,
    ) -> Result<KindExpr, TypeResolutionError> {
        Ok(match kind {
            KindExpr::Parameter(id) => {
                let Some(binding) = bindings.get(id) else {
                    return Ok(kind.clone());
                };
                if !visiting.insert(*id) {
                    return Err(TypeResolutionError::Incompatible {
                        origin: origin.clone(),
                        failures: vec![TypeConstraintFailure::InvalidScheme {
                            reason: format!("cyclic kind variable T{}", id.get()),
                        }]
                        .into_boxed_slice(),
                    });
                }
                let result = substitute(&binding.kind, bindings, visiting, origin)?;
                visiting.remove(id);
                result
            }
            KindExpr::Matrix {
                element,
                dimensions,
            } => KindExpr::Matrix {
                element: Box::new(substitute(element, bindings, visiting, origin)?),
                dimensions: dimensions.clone(),
            },
            KindExpr::Option(element) => {
                KindExpr::Option(Box::new(substitute(element, bindings, visiting, origin)?))
            }
            KindExpr::Tuple(elements) => KindExpr::Tuple(
                elements
                    .iter()
                    .map(|element| substitute(element, bindings, visiting, origin))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            ),
            KindExpr::Record(fields) => {
                KindExpr::Record(substitute_fields(fields, bindings, visiting, origin)?)
            }
            KindExpr::Table { columns, rows } => KindExpr::Table {
                columns: substitute_fields(columns, bindings, visiting, origin)?,
                rows: rows.clone(),
            },
            KindExpr::Set {
                element,
                cardinality,
            } => KindExpr::Set {
                element: Box::new(substitute(element, bindings, visiting, origin)?),
                cardinality: cardinality.clone(),
            },
            KindExpr::Map {
                key,
                value,
                cardinality,
            } => KindExpr::Map {
                key: Box::new(substitute(key, bindings, visiting, origin)?),
                value: Box::new(substitute(value, bindings, visiting, origin)?),
                cardinality: cardinality.clone(),
            },
            KindExpr::Reference(element) => {
                KindExpr::Reference(Box::new(substitute(element, bindings, visiting, origin)?))
            }
            KindExpr::TypeOf(element) => {
                KindExpr::TypeOf(Box::new(substitute(element, bindings, visiting, origin)?))
            }
            other => other.clone(),
        })
    }
    substitute(kind, bindings, &mut BTreeSet::new(), origin)
}

fn substitute_fields(
    fields: &[KindField],
    bindings: &BTreeMap<KindParameterId, KindBinding>,
    visiting: &mut BTreeSet<KindParameterId>,
    origin: &TypeConstraintOrigin,
) -> Result<Box<[KindField]>, TypeResolutionError> {
    fields
        .iter()
        .map(|field| {
            Ok(KindField {
                name: field.name.clone(),
                kind: substitute_kind_parameters_with_visiting(
                    &field.kind,
                    bindings,
                    visiting,
                    origin,
                )?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn substitute_kind_parameters_with_visiting(
    kind: &KindExpr,
    bindings: &BTreeMap<KindParameterId, KindBinding>,
    _visiting: &mut BTreeSet<KindParameterId>,
    origin: &TypeConstraintOrigin,
) -> Result<KindExpr, TypeResolutionError> {
    // Bindings are cycle-checked when inserted. A fresh traversal keeps this
    // helper small while preserving deterministic failure for malformed data.
    substitute_kind_parameters(kind, bindings, origin)
}

fn substitute_dimension_parameters(
    dimension: &DimensionExpr,
    bindings: &BTreeMap<DimensionParameterId, DimensionExpr>,
    parameter_count: usize,
    origin: &TypeConstraintOrigin,
) -> Result<DimensionExpr, TypeResolutionError> {
    fn substitute(
        dimension: &DimensionExpr,
        bindings: &BTreeMap<DimensionParameterId, DimensionExpr>,
        visiting: &mut BTreeSet<DimensionParameterId>,
        origin: &TypeConstraintOrigin,
    ) -> Result<DimensionExpr, TypeResolutionError> {
        Ok(match dimension {
            DimensionExpr::Parameter(id) => {
                let Some(binding) = bindings.get(id) else {
                    return Ok(dimension.clone());
                };
                if !visiting.insert(*id) {
                    return Err(TypeResolutionError::Incompatible {
                        origin: origin.clone(),
                        failures: vec![TypeConstraintFailure::InvalidScheme {
                            reason: format!("cyclic dimension variable d{}", id.get()),
                        }]
                        .into_boxed_slice(),
                    });
                }
                let result = substitute(binding, bindings, visiting, origin)?;
                visiting.remove(id);
                result
            }
            DimensionExpr::Add(children) => DimensionExpr::Add(
                children
                    .iter()
                    .map(|child| substitute(child, bindings, visiting, origin))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            ),
            DimensionExpr::Multiply(children) => DimensionExpr::Multiply(
                children
                    .iter()
                    .map(|child| substitute(child, bindings, visiting, origin))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            ),
            DimensionExpr::Min(children) => DimensionExpr::Min(
                children
                    .iter()
                    .map(|child| substitute(child, bindings, visiting, origin))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            ),
            DimensionExpr::Max(children) => DimensionExpr::Max(
                children
                    .iter()
                    .map(|child| substitute(child, bindings, visiting, origin))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            ),
            other => other.clone(),
        })
    }
    let substituted = substitute(dimension, bindings, &mut BTreeSet::new(), origin)?;
    normalize_dimension(&substituted, parameter_count)
        .map_err(TypeResolutionError::semantic)
        .map_err(|error| error.with_origin(origin.clone()))
}

fn substitute_kind_dimensions(
    kind: &KindExpr,
    substitute: &mut impl FnMut(&DimensionExpr) -> Result<DimensionExpr, TypeResolutionError>,
) -> Result<KindExpr, TypeResolutionError> {
    Ok(match kind {
        KindExpr::Matrix {
            element,
            dimensions,
        } => KindExpr::Matrix {
            element: Box::new(substitute_kind_dimensions(element, substitute)?),
            dimensions: dimensions
                .iter()
                .map(&mut *substitute)
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        },
        KindExpr::Option(element) => {
            KindExpr::Option(Box::new(substitute_kind_dimensions(element, substitute)?))
        }
        KindExpr::Tuple(elements) => KindExpr::Tuple(
            elements
                .iter()
                .map(|element| substitute_kind_dimensions(element, substitute))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        KindExpr::Record(fields) => {
            KindExpr::Record(substitute_dimension_fields(fields, substitute)?)
        }
        KindExpr::Table { columns, rows } => KindExpr::Table {
            columns: substitute_dimension_fields(columns, substitute)?,
            rows: substitute(rows)?,
        },
        KindExpr::Set {
            element,
            cardinality,
        } => KindExpr::Set {
            element: Box::new(substitute_kind_dimensions(element, substitute)?),
            cardinality: substitute(cardinality)?,
        },
        KindExpr::Map {
            key,
            value,
            cardinality,
        } => KindExpr::Map {
            key: Box::new(substitute_kind_dimensions(key, substitute)?),
            value: Box::new(substitute_kind_dimensions(value, substitute)?),
            cardinality: substitute(cardinality)?,
        },
        KindExpr::Reference(element) => {
            KindExpr::Reference(Box::new(substitute_kind_dimensions(element, substitute)?))
        }
        KindExpr::TypeOf(element) => {
            KindExpr::TypeOf(Box::new(substitute_kind_dimensions(element, substitute)?))
        }
        other => other.clone(),
    })
}

fn substitute_dimension_fields(
    fields: &[KindField],
    substitute: &mut impl FnMut(&DimensionExpr) -> Result<DimensionExpr, TypeResolutionError>,
) -> Result<Box<[KindField]>, TypeResolutionError> {
    fields
        .iter()
        .map(|field| {
            Ok(KindField {
                name: field.name.clone(),
                kind: substitute_kind_dimensions(&field.kind, substitute)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn kind_contains_parameter(kind: &KindExpr, target: KindParameterId) -> bool {
    let mut found = false;
    let _ = visit_kind_parameters(kind, &mut |id| {
        found |= id == target;
        Ok(())
    });
    found
}

fn collect_kind_parameter_ids(kind: &KindExpr, output: &mut BTreeSet<KindParameterId>) {
    let _ = visit_kind_parameters(kind, &mut |id| {
        output.insert(id);
        Ok(())
    });
}

fn collect_kind_nodes<'a>(kind: &'a KindExpr, nodes: &mut Vec<&'a KindExpr>) {
    nodes.push(kind);
    match kind {
        KindExpr::Matrix { element, .. }
        | KindExpr::Option(element)
        | KindExpr::Reference(element)
        | KindExpr::TypeOf(element)
        | KindExpr::Set { element, .. } => collect_kind_nodes(element, nodes),
        KindExpr::Tuple(elements) => {
            for element in elements {
                collect_kind_nodes(element, nodes);
            }
        }
        KindExpr::Record(fields)
        | KindExpr::Table {
            columns: fields, ..
        } => {
            for field in fields {
                collect_kind_nodes(&field.kind, nodes);
            }
        }
        KindExpr::Map { key, value, .. } => {
            collect_kind_nodes(key, nodes);
            collect_kind_nodes(value, nodes);
        }
        KindExpr::Wildcard
        | KindExpr::Never
        | KindExpr::Hole
        | KindExpr::Parameter(_)
        | KindExpr::Named(_)
        | KindExpr::Id
        | KindExpr::Index
        | KindExpr::Atom(_)
        | KindExpr::Enum(_) => {}
    }
}

fn dimension_contains_parameter(dimension: &DimensionExpr, target: DimensionParameterId) -> bool {
    let mut references = Vec::new();
    collect_dimension_references(dimension, &mut references);
    references.contains(&target)
}

const fn declaration_evolution(declaration: &DimensionParameterDeclaration) -> u8 {
    match declaration.lifetime {
        DimensionLifetime::CompileTime => 0,
        DimensionLifetime::Activation => 1,
        DimensionLifetime::Turn if declaration.upper_bound.is_some() => 2,
        DimensionLifetime::Turn => 3,
    }
}

const fn evolution_name(evolution: u8) -> &'static str {
    match evolution {
        0 => "fixed",
        1 => "activation-fixed",
        2 => "turn-bounded",
        _ => "turn-unbounded",
    }
}

const fn predicate_generality(predicate: BuiltinKindPredicate) -> u32 {
    match predicate {
        BuiltinKindPredicate::FloatingPoint => 0,
        BuiltinKindPredicate::Integer
        | BuiltinKindPredicate::Negatable
        | BuiltinKindPredicate::RangeEndpoint => 1,
        BuiltinKindPredicate::Real | BuiltinKindPredicate::Ordered => 2,
        BuiltinKindPredicate::Number => 3,
        BuiltinKindPredicate::Equatable | BuiltinKindPredicate::Keyable => 4,
    }
}
