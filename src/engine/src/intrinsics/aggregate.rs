//! Canonical aggregate preparation shared by registered resident constructors.
//! No source AST or interpreter is accepted at this boundary.
#[cfg(feature = "matrix_comprehensions")]
use crate::intrinsics::constructors::ValueMatrixComprehension;
#[cfg(feature = "set_comprehensions")]
use crate::intrinsics::constructors::ValueSetComprehension;
use crate::*;
#[cfg(all(
    test,
    feature = "functions",
    any(feature = "set_comprehensions", feature = "matrix_comprehensions")
))]
#[path = "aggregate_tests.rs"]
mod tests;
#[cfg(feature = "set")]
use mech_core::snapshot::OptionDraft;

#[cfg(feature = "set")]
fn snapshot_draft(cell: &ValueCell) -> MResult<ValueDataDraft> {
    cell.snapshot()?.canonical_data_draft().map_err(|error| {
        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })
}
#[cfg(feature = "set")]
fn schema_mismatch(context: &'static str, expected: &SchemaBody, actual: &SchemaBody) -> MechError {
    MechError::new(
        CanonicalAggregateSchemaMismatch {
            context,
            expected: format!("{expected:?}"),
            actual: format!("{actual:?}"),
        },
        None,
    )
    .with_compiler_loc()
}
#[cfg(feature = "set")]
pub(crate) fn canonical_set_from_inputs(inputs: Vec<SpecializationInput>) -> MResult<ValueCell> {
    let concrete = inputs
        .iter()
        .filter_map(|input| input.cell().ok())
        .collect::<Vec<_>>();
    let Some(first) = concrete.first() else {
        return Err(MechError::new(
            CanonicalAggregateTypeInferenceFailure {
                context: "empty or all-absent set",
            },
            None,
        )
        .with_compiler_loc());
    };
    let element = first.closed_schema_body()?;
    for value in concrete.iter().skip(1) {
        let actual = value.closed_schema_body()?;
        if actual != element {
            return Err(schema_mismatch("set element", &element, &actual));
        }
    }
    let optional = inputs.iter().any(SpecializationInput::is_absent);
    let schema = if optional {
        SchemaBody::Option(Box::new(element))
    } else {
        element
    };
    let values = inputs
        .into_iter()
        .map(|input| match input {
            SpecializationInput::Cell(value) if optional => snapshot_draft(&value).map(|value| {
                ValueDataDraft::Option(OptionDraft {
                    present: true,
                    value: Some(Box::new(value)),
                })
            }),
            SpecializationInput::Cell(value) => snapshot_draft(&value),
            SpecializationInput::Absent => Ok(ValueDataDraft::Option(OptionDraft {
                present: false,
                value: None,
            })),
            SpecializationInput::MatrixAllSelection => Err(MechError::new(
                CanonicalAggregateSourceAbsence {
                    context: "set element",
                },
                Some("matrix all-selection is not a set value".to_owned()),
            )
            .with_compiler_loc()),
        })
        .collect::<MResult<Vec<_>>>()?;
    let cardinality = values.len() as u64;
    ValueCell::from_schema_data(
        SchemaBody::Set {
            element: Box::new(schema),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(cardinality)),
        },
        ValueDataDraft::Set(values.into_boxed_slice()),
    )
}

#[cfg(feature = "set_comprehensions")]
pub struct SetComprehensionDefine {}
#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl CanonicalFunctionSpecializer for SetComprehensionDefine {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        let arguments = invocation
            .inputs()
            .iter()
            .map(|input| input.cell().cloned())
            .collect::<MResult<Vec<_>>>()?;
        let element = arguments
            .first()
            .map(ValueCell::closed_schema_body)
            .transpose()?
            .unwrap_or_else(|| SchemaBody::Tuple(Box::new([])));
        for argument in &arguments {
            if argument.closed_schema_body()? != element {
                return Err(MechError::new(
                    ComprehensionGeneratorError {
                        found: argument.resolved_type()?,
                    },
                    None,
                )
                .with_compiler_loc());
            }
        }
        let semantic_inputs = invocation.inputs().iter().collect::<Vec<_>>();
        let descriptor =
            context.resolved_output_descriptor(0, vec![0].into_boxed_slice(), &semantic_inputs)?;
        let output = ValueCell::allocate_for_descriptor(
            &descriptor,
            mech_core::FunctionValueRepresentation::Set,
        )?;
        let invocation = FunctionInvocation::variadic(output, arguments.into_boxed_slice());
        let implementation = ValueSetComprehension::new_invocation(invocation.clone())?;
        context.certify_instance(
            (implementation, invocation),
            mech_core::RuntimeFunctionId::from_name("set/comprehension"),
            mech_core::ExecutionTarget::DirectRuntime,
            mech_core::ImplementationMemoryClass::CanonicalSortUnique,
        )
    }
}
#[cfg(feature = "matrix_comprehensions")]
pub struct MatrixComprehensionDefine {}
#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
impl CanonicalFunctionSpecializer for MatrixComprehensionDefine {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        let arguments = invocation
            .inputs()
            .iter()
            .map(|input| input.cell().cloned())
            .collect::<MResult<Vec<_>>>()?;
        let output = crate::intrinsics::constructors::matrix_comprehension_output(&arguments)?;
        let invocation = FunctionInvocation::variadic(output, arguments.into_boxed_slice());
        let implementation = ValueMatrixComprehension::new_invocation(invocation.clone())?;
        context.certify_instance(
            (implementation, invocation),
            mech_core::RuntimeFunctionId::from_name("matrix/comprehension"),
            mech_core::ExecutionTarget::DirectRuntime,
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
        )
    }
}

#[cfg(feature = "set_comprehensions")]
#[derive(Debug, Clone)]
struct ComprehensionGeneratorError {
    found: ResolvedType,
}
#[cfg(feature = "set_comprehensions")]
impl MechErrorKind for ComprehensionGeneratorError {
    fn name(&self) -> &str {
        "ComprehensionGenerator"
    }
    fn message(&self) -> String {
        format!(
            "Comprehension generator must produce a set or matrix, found type: {}",
            self.found.semantic_name()
        )
    }
}

#[derive(Debug, Clone)]
pub struct CanonicalAggregateSourceAbsence {
    pub context: &'static str,
}

impl MechErrorKind for CanonicalAggregateSourceAbsence {
    fn name(&self) -> &str {
        "CanonicalAggregateSourceAbsence"
    }

    fn message(&self) -> String {
        format!("source absence is not a value in {}", self.context)
    }
}

#[derive(Debug, Clone)]
pub struct CanonicalAggregateSchemaMismatch {
    pub context: &'static str,
    pub expected: String,
    pub actual: String,
}

impl MechErrorKind for CanonicalAggregateSchemaMismatch {
    fn name(&self) -> &str {
        "CanonicalAggregateSchemaMismatch"
    }

    fn message(&self) -> String {
        format!(
            "{} expected schema {}, found {}",
            self.context, self.expected, self.actual
        )
    }
}

#[derive(Debug, Clone)]
pub struct CanonicalAggregateTypeInferenceFailure {
    pub context: &'static str,
}

impl MechErrorKind for CanonicalAggregateTypeInferenceFailure {
    fn name(&self) -> &str {
        "CanonicalAggregateTypeInferenceFailure"
    }

    fn message(&self) -> String {
        format!(
            "cannot infer a closed canonical schema for {}",
            self.context
        )
    }
}
