use crate::intrinsics::*;
#[cfg(any(
    feature = "set",
    feature = "set_comprehensions",
    feature = "matrix_comprehensions",
    feature = "matrix_horzcat",
    feature = "matrix_vertcat"
))]
use std::sync::LazyLock;

#[cfg(feature = "matrix_comprehensions")]
pub(crate) static PURE_MATRIX_COMPREHENSION_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Variadic {
            prefix: Box::new([]),
            repeated: InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            },
            min_repetitions: 0,
        },
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: vec!["matrix".to_owned(), "concatenate".to_owned()]
                        .into_boxed_slice(),
                    contract_name: "horizontal-output".to_owned(),
                },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[cfg(any(feature = "matrix_horzcat", feature = "matrix_vertcat"))]
fn matrix_concatenation_contract(contract_name: &str) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Variadic {
            prefix: Box::new([]),
            repeated: InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            },
            min_repetitions: 1,
        },
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: vec!["matrix".to_owned(), "concatenate".to_owned()]
                        .into_boxed_slice(),
                    contract_name: contract_name.to_owned(),
                },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

#[cfg(feature = "matrix_horzcat")]
pub(crate) static PURE_MATRIX_HORZCAT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| matrix_concatenation_contract("horizontal-output"));

#[cfg(feature = "matrix_vertcat")]
pub(crate) static PURE_MATRIX_VERTCAT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| matrix_concatenation_contract("vertical-output"));

#[cfg(feature = "set")]
pub(crate) static PURE_SET_DEFINE_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Variadic {
            prefix: Box::new([]),
            repeated: InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            },
            min_repetitions: 0,
        },
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: vec!["set".to_owned()].into_boxed_slice(),
                    contract_name: "define-output".to_owned(),
                },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[cfg(feature = "set_comprehensions")]
pub(crate) static PURE_SET_COMPREHENSION_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Variadic {
            prefix: Box::new([]),
            repeated: InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            },
            min_repetitions: 0,
        },
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: vec!["set".to_owned()].into_boxed_slice(),
                    contract_name: "comprehension-output".to_owned(),
                },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[cfg(any(
    feature = "set_comprehensions",
    feature = "matrix_comprehensions",
    feature = "matrix_horzcat",
    feature = "matrix_vertcat"
))]
fn variadic_ports(
    invocation: &FunctionInvocation,
) -> MResult<(FunctionValueOutput, Vec<FunctionValueInput>)> {
    if invocation.input_count() == 0 {
        if invocation.expect_nullary().is_err() {
            invocation.expect_variadic()?;
        }
    } else {
        invocation.expect_variadic()?;
    }
    Ok((
        invocation.output().value(),
        invocation.inputs().map(FunctionInputPort::value).collect(),
    ))
}

// Set -----------------------------------------------------------------------

#[cfg(feature = "set")]
pub struct SetDefine;

#[cfg(all(feature = "set", feature = "functions", feature = "semantic-compiler"))]
impl CanonicalFunctionSpecializer for SetDefine {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        let inputs = invocation.inputs().iter().collect::<Vec<_>>();
        let descriptor = context.resolved_output_descriptor(
            0,
            vec![inputs.len() as u64].into_boxed_slice(),
            &inputs,
        )?;
        let draft = crate::structures::canonical_set_from_inputs(invocation.inputs().to_vec())?
            .snapshot()?
            .canonical_data_draft()
            .map_err(|error| {
                MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
            })?;
        let output = ValueCell::from_resolved_descriptor_data(&descriptor, draft)?;
        let runtime_invocation = FunctionInvocation::nullary(output);
        let implementation = ValueSet::new_invocation(runtime_invocation.clone())?;
        context.certify_instance_for_inputs(
            FunctionInstance::new(implementation, runtime_invocation),
            mech_core::RuntimeFunctionId::from_name("ValueSet"),
            mech_core::ExecutionTarget::DirectRuntime,
            &inputs,
        )
    }
}

/// Runtime implementation for `set/define`.
#[cfg(feature = "set")]
#[derive(Debug)]
pub struct ValueSet {
    output: FunctionValueOutput,
}

#[cfg(all(feature = "set", feature = "functions"))]
impl MechFunctionImpl for ValueSet {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn reactive_dependency_scopes(
        &self,
        argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyScope>> {
        Some(vec![ReactiveDependencyScope::None; argument_count])
    }

    fn to_string(&self) -> String {
        format!("{self:#?}")
    }

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.cell().clone()]
    }
}

#[cfg(all(feature = "set", feature = "functions"))]
impl MechFunctionFactory for ValueSet {
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::nullary(FunctionValueRepresentation::Set);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let output = invocation.expect_nullary()?.value();
        let SchemaBody::Set { .. } = output.cell().closed_schema_body()? else {
            return Err(comprehension_output_error(
                "set/define",
                output.representation(),
            ));
        };
        Ok(Box::new(Self { output }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_DEFINE_CONTRACT)
    }
}

#[cfg(all(feature = "set", feature = "semantic-compiler"))]
impl MechFunctionCompiler for ValueSet {
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = self.output.compile_register(context)?;
        context.emit_nullop(hash_str("set/define"), output);
        Ok(output)
    }
}

// Set comprehensions --------------------------------------------------------

#[cfg(any(feature = "set", feature = "set_comprehensions"))]
#[derive(Debug, Clone)]
struct SetComprehensionOutputKindMismatchError {
    found: FunctionValueRepresentation,
}

#[cfg(any(feature = "set", feature = "set_comprehensions"))]
impl MechErrorKind for SetComprehensionOutputKindMismatchError {
    fn name(&self) -> &str {
        "SetComprehensionOutputKindMismatch"
    }

    fn message(&self) -> String {
        format!(
            "Set comprehension bytecode output must be a set, but found {:?}.",
            self.found
        )
    }
}

#[cfg(any(feature = "set", feature = "set_comprehensions"))]
fn comprehension_output_error(
    _operation: &'static str,
    found: FunctionValueRepresentation,
) -> MechError {
    MechError::new(SetComprehensionOutputKindMismatchError { found }, None).with_compiler_loc()
}

/// Runtime implementation for `set/comprehension`.
#[cfg(feature = "set_comprehensions")]
#[derive(Debug)]
pub struct ValueSetComprehension {
    arguments: Vec<FunctionValueInput>,
    output: FunctionValueOutput,
}

#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl MechFunctionImpl for ValueSetComprehension {
    fn solve_result(&self) -> MResult<()> {
        let values = self
            .arguments
            .iter()
            .map(FunctionValueInput::snapshot)
            .collect::<MResult<Vec<_>>>()?
            .into_iter()
            .map(|value| value.data().clone())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        self.output.replace_set(values)
    }

    fn to_string(&self) -> String {
        format!("{self:#?}")
    }
}

#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl MechFunctionFactory for ValueSetComprehension {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::variadic(
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::AnyValue,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (output, arguments) = variadic_ports(&invocation)?;
        let SchemaBody::Set { .. } = output.cell().closed_schema_body()? else {
            return Err(comprehension_output_error(
                "set/comprehension",
                output.representation(),
            ));
        };
        Ok(Box::new(Self { arguments, output }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_COMPREHENSION_CONTRACT)
    }
}

#[cfg(all(feature = "set_comprehensions", feature = "semantic-compiler"))]
impl MechFunctionCompiler for ValueSetComprehension {
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = self.output.compile_register(context)?;
        let arguments = self
            .arguments
            .iter()
            .map(|argument| argument.compile_register(context))
            .collect::<MResult<Vec<_>>>()?;
        context.emit_varop(hash_str("set/comprehension"), output, arguments);
        Ok(output)
    }
}

// Matrix comprehensions -----------------------------------------------------

#[cfg(any(
    feature = "matrix_comprehensions",
    feature = "matrix_horzcat",
    feature = "matrix_vertcat"
))]
fn matrix_input_cells(input: &ValueCell) -> MResult<(usize, usize, Vec<ValueCell>)> {
    if let Some(elements) = input.matrix_elements()? {
        let SchemaBody::Matrix { dimensions, .. } = input.closed_schema_body()? else {
            unreachable!("matrix elements retain a matrix schema")
        };
        let [
            DimensionExpr::Constant(rows),
            DimensionExpr::Constant(columns),
        ] = dimensions.as_ref()
        else {
            return Err(matrix_comprehension_error(
                "matrix input must have exactly two dimensions",
            ));
        };
        return Ok((
            usize::try_from(*rows)
                .map_err(|_| matrix_comprehension_error("matrix row extent exceeds usize"))?,
            usize::try_from(*columns)
                .map_err(|_| matrix_comprehension_error("matrix column extent exceeds usize"))?,
            elements,
        ));
    }
    Ok((1, 1, vec![input.clone()]))
}

#[cfg(all(
    test,
    feature = "f32",
    feature = "matrixd",
    feature = "matrix_horzcat",
    feature = "matrix_vertcat"
))]
mod matrix_input_tests {
    use super::*;
    use mech_core::{
        FloatWidth, ValueData, ValueDataDraft,
        snapshot::{F32Bits, SequenceView},
    };

    fn matrix(rows: u64, columns: u64, values: &[f32]) -> ValueCell {
        ValueCell::from_schema_data(
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W32)),
                dimensions: vec![
                    DimensionExpr::Constant(rows),
                    DimensionExpr::Constant(columns),
                ]
                .into_boxed_slice(),
            },
            ValueDataDraft::Matrix(
                values
                    .iter()
                    .map(|value| ValueDataDraft::F32(F32Bits::from_f32(*value)))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        )
        .unwrap()
    }

    fn values(cell: &ValueCell) -> Vec<f32> {
        let snapshot = cell.snapshot().unwrap();
        let ValueData::Matrix(matrix) = snapshot.data() else {
            panic!("concatenation output must remain a matrix")
        };
        let SequenceView::F32(values) = matrix.elements() else {
            panic!("concatenation output must retain f32 elements")
        };
        values.iter().map(|value| value.to_f32()).collect()
    }

    #[test]
    fn fixed_matrix_extents_drive_horizontal_and_vertical_concatenation() {
        let left = matrix(2, 1, &[1.0, 2.0]);
        let right = matrix(2, 1, &[3.0, 4.0]);
        assert!(left.shape().parameter_values().is_empty());

        let horizontal = matrix_concatenation_output(&[left, right], false, None).unwrap();
        assert_eq!(horizontal.shape().parameter_values(), &[2, 2]);
        assert_eq!(values(&horizontal), vec![1.0, 3.0, 2.0, 4.0]);

        let top = matrix(1, 2, &[1.0, 2.0]);
        let bottom = matrix(1, 2, &[3.0, 4.0]);
        let vertical = matrix_concatenation_output(&[top, bottom], true, None).unwrap();
        assert_eq!(vertical.shape().parameter_values(), &[2, 2]);
        assert_eq!(values(&vertical), vec![1.0, 2.0, 3.0, 4.0]);
    }
}

#[cfg(any(feature = "matrix_comprehensions", feature = "matrix_horzcat"))]
fn horizontal_comprehension_cells(
    arguments: &[ValueCell],
) -> MResult<(usize, usize, Vec<ValueCell>)> {
    if arguments.is_empty() {
        return Ok((0, 0, Vec::new()));
    }
    let parts = arguments
        .iter()
        .map(matrix_input_cells)
        .collect::<MResult<Vec<_>>>()?;
    let rows = parts[0].0;
    if parts.iter().any(|(candidate, _, _)| *candidate != rows) {
        return Err(matrix_comprehension_error(
            "horizontal inputs must have the same row extent",
        ));
    }
    let columns = parts.iter().try_fold(0_usize, |total, (_, columns, _)| {
        total
            .checked_add(*columns)
            .ok_or_else(|| matrix_comprehension_error("matrix column extent overflowed"))
    })?;
    let mut cells = Vec::with_capacity(rows.saturating_mul(columns));
    for row in 0..rows {
        for (_, part_columns, part_cells) in &parts {
            let start = row.saturating_mul(*part_columns);
            cells.extend_from_slice(&part_cells[start..start + part_columns]);
        }
    }
    Ok((rows, columns, cells))
}

#[cfg(feature = "matrix_vertcat")]
fn vertical_concatenation_cells(
    arguments: &[ValueCell],
) -> MResult<(usize, usize, Vec<ValueCell>)> {
    if arguments.is_empty() {
        return Ok((0, 0, Vec::new()));
    }
    let parts = arguments
        .iter()
        .map(matrix_input_cells)
        .collect::<MResult<Vec<_>>>()?;
    let columns = parts[0].1;
    if parts.iter().any(|(_, candidate, _)| *candidate != columns) {
        return Err(matrix_comprehension_error(
            "vertical inputs must have the same column extent",
        ));
    }
    let rows = parts.iter().try_fold(0_usize, |total, (rows, _, _)| {
        total
            .checked_add(*rows)
            .ok_or_else(|| matrix_comprehension_error("matrix row extent overflowed"))
    })?;
    let cells = parts.into_iter().flat_map(|(_, _, cells)| cells).collect();
    Ok((rows, columns, cells))
}

#[cfg(any(
    feature = "matrix_comprehensions",
    feature = "matrix_horzcat",
    feature = "matrix_vertcat"
))]
fn matrix_comprehension_error(reason: impl Into<String>) -> MechError {
    MechError::new(GenericError { msg: reason.into() }, None).with_compiler_loc()
}

#[cfg(all(feature = "matrix_comprehensions", feature = "semantic-compiler"))]
pub(crate) fn matrix_comprehension_output(arguments: &[ValueCell]) -> MResult<ValueCell> {
    if arguments.is_empty() {
        return ValueCell::dynamic_matrix(
            SchemaBody::Tuple(Box::new([])),
            vec![0, 0].into_boxed_slice(),
            Box::new([]),
        );
    }
    let (rows, columns, cells) = horizontal_comprehension_cells(arguments)?;
    ValueCell::dynamic_matrix_from_cells(rows, columns, &cells)
}

#[cfg(any(feature = "matrix_horzcat", feature = "matrix_vertcat"))]
fn matrix_concatenation_cells(
    arguments: &[ValueCell],
    vertical: bool,
) -> MResult<(usize, usize, Vec<ValueCell>)> {
    if vertical {
        #[cfg(feature = "matrix_vertcat")]
        {
            vertical_concatenation_cells(arguments)
        }
        #[cfg(not(feature = "matrix_vertcat"))]
        unreachable!()
    } else {
        #[cfg(feature = "matrix_horzcat")]
        {
            horizontal_comprehension_cells(arguments)
        }
        #[cfg(not(feature = "matrix_horzcat"))]
        unreachable!()
    }
}

#[cfg(any(feature = "matrix_horzcat", feature = "matrix_vertcat"))]
fn matrix_concatenation_output(
    arguments: &[ValueCell],
    vertical: bool,
    resolved: Option<&ResolvedType>,
) -> MResult<ValueCell> {
    let (rows, columns, cells) = matrix_concatenation_cells(arguments, vertical)?;
    if let Some(resolved) = resolved {
        return ValueCell::matrix_from_resolved_type_cells(
            resolved, rows, columns, &cells, arguments,
        );
    }
    if cells.is_empty() {
        return ValueCell::dynamic_matrix(
            SchemaBody::Tuple(Box::new([])),
            vec![rows as u64, columns as u64].into_boxed_slice(),
            Box::new([]),
        );
    }
    ValueCell::dynamic_matrix_from_cells(rows, columns, &cells)
}

#[cfg(any(feature = "matrix_horzcat", feature = "matrix_vertcat"))]
#[derive(Debug)]
pub struct ValueMatrixConcatenation<const VERTICAL: bool> {
    arguments: Vec<FunctionValueInput>,
    output: FunctionValueOutput,
}

#[cfg(any(feature = "matrix_horzcat", feature = "matrix_vertcat"))]
impl<const VERTICAL: bool> MechFunctionImpl for ValueMatrixConcatenation<VERTICAL> {
    fn solve_result(&self) -> MResult<()> {
        let arguments = self
            .arguments
            .iter()
            .map(|argument| argument.cell().clone())
            .collect::<Vec<_>>();
        let (rows, columns, cells) = matrix_concatenation_cells(&arguments, VERTICAL)?;
        let values = cells
            .iter()
            .map(|cell| {
                cell.snapshot()?.canonical_data_draft().map_err(|_| {
                    matrix_comprehension_error(
                        "matrix concatenation input could not be materialized",
                    )
                })
            })
            .collect::<MResult<Vec<_>>>()?
            .into_boxed_slice();
        self.output
            .replace_matrix_drafts(vec![rows as u64, columns as u64].into_boxed_slice(), values)
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        if VERTICAL {
            #[cfg(feature = "matrix_vertcat")]
            return Some(&PURE_MATRIX_VERTCAT_CONTRACT);
            #[cfg(not(feature = "matrix_vertcat"))]
            unreachable!();
        }
        #[cfg(feature = "matrix_horzcat")]
        return Some(&PURE_MATRIX_HORZCAT_CONTRACT);
        #[cfg(not(feature = "matrix_horzcat"))]
        unreachable!();
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        Some(if VERTICAL {
            "matrix/vertcat"
        } else {
            "matrix/horzcat"
        })
    }

    fn to_string(&self) -> String {
        if VERTICAL {
            "ValueVerticalConcatenation".to_owned()
        } else {
            "ValueHorizontalConcatenation".to_owned()
        }
    }
}

#[cfg(all(
    feature = "semantic-compiler",
    any(feature = "matrix_horzcat", feature = "matrix_vertcat")
))]
impl<const VERTICAL: bool> MechFunctionCompiler for ValueMatrixConcatenation<VERTICAL> {
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = self.output.compile_register(context)?;
        let arguments = self
            .arguments
            .iter()
            .map(|argument| argument.compile_register(context))
            .collect::<MResult<Vec<_>>>()?;
        context.emit_varop(
            hash_str(if VERTICAL {
                "matrix/vertcat"
            } else {
                "matrix/horzcat"
            }),
            output,
            arguments,
        );
        Ok(output)
    }
}

#[cfg(any(feature = "matrix_horzcat", feature = "matrix_vertcat"))]
impl<const VERTICAL: bool> ValueMatrixConcatenation<VERTICAL> {
    pub(crate) fn specialize(
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        let arguments = invocation
            .inputs()
            .iter()
            .map(|input| Ok(input.cell()?.clone()))
            .collect::<MResult<Vec<_>>>()?;
        let output =
            matrix_concatenation_output(&arguments, VERTICAL, Some(context.resolved_output(0)?))?;
        let invocation = FunctionInvocation::variadic(output.clone(), arguments.into_boxed_slice());
        let implementation = Self {
            arguments: invocation.inputs().map(FunctionInputPort::value).collect(),
            output: invocation.output().value(),
        };
        context.certify_instance(
            FunctionInstance::new(Box::new(implementation), invocation),
            mech_core::RuntimeFunctionId::from_name(if VERTICAL {
                "matrix/vertcat"
            } else {
                "matrix/horzcat"
            }),
            mech_core::ExecutionTarget::DirectRuntime,
        )
    }
}

#[cfg(any(feature = "matrix_horzcat", feature = "matrix_vertcat"))]
impl<const VERTICAL: bool> MechFunctionFactory for ValueMatrixConcatenation<VERTICAL> {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::variadic(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (output, arguments) = variadic_ports(&invocation)?;
        let SchemaBody::Matrix { .. } = output.cell().closed_schema_body()? else {
            return Err(matrix_comprehension_error(format!(
                "matrix concatenation output must be a matrix, found {:?}",
                output.representation(),
            )));
        };
        Ok(Box::new(Self { arguments, output }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        if VERTICAL {
            #[cfg(feature = "matrix_vertcat")]
            return Some(&PURE_MATRIX_VERTCAT_CONTRACT);
            #[cfg(not(feature = "matrix_vertcat"))]
            unreachable!();
        }
        #[cfg(feature = "matrix_horzcat")]
        return Some(&PURE_MATRIX_HORZCAT_CONTRACT);
        #[cfg(not(feature = "matrix_horzcat"))]
        unreachable!();
    }
}

#[cfg(feature = "matrix_horzcat")]
pub type ValueHorizontalConcatenation = ValueMatrixConcatenation<false>;

#[cfg(feature = "matrix_vertcat")]
pub type ValueVerticalConcatenation = ValueMatrixConcatenation<true>;

/// Runtime implementation for `matrix/comprehension`.
#[cfg(feature = "matrix_comprehensions")]
#[derive(Debug)]
pub struct ValueMatrixComprehension {
    arguments: Vec<FunctionValueInput>,
    output: FunctionValueOutput,
}

#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
impl MechFunctionImpl for ValueMatrixComprehension {
    fn solve_result(&self) -> MResult<()> {
        let arguments = self
            .arguments
            .iter()
            .map(|argument| argument.cell().clone())
            .collect::<Vec<_>>();
        let (rows, columns, cells) = horizontal_comprehension_cells(&arguments)?;
        let drafts = cells
            .iter()
            .map(|cell| {
                cell.snapshot()?
                    .canonical_data_draft()
                    .map_err(|error| matrix_comprehension_error(format!("{error:?}")))
            })
            .collect::<MResult<Vec<_>>>()?
            .into_boxed_slice();
        self.output
            .replace_matrix_drafts(vec![rows as u64, columns as u64].into_boxed_slice(), drafts)
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_MATRIX_COMPREHENSION_CONTRACT)
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        Some("matrix/comprehension")
    }

    fn to_string(&self) -> String {
        format!("{self:#?}")
    }
}

#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
impl MechFunctionFactory for ValueMatrixComprehension {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::variadic(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (output, arguments) = variadic_ports(&invocation)?;
        let SchemaBody::Matrix { .. } = output.cell().closed_schema_body()? else {
            return Err(matrix_comprehension_error(format!(
                "matrix comprehension output must be a matrix, found {:?}",
                output.representation(),
            )));
        };
        Ok(Box::new(Self { arguments, output }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_MATRIX_COMPREHENSION_CONTRACT)
    }
}

#[cfg(all(feature = "matrix_comprehensions", feature = "semantic-compiler"))]
impl MechFunctionCompiler for ValueMatrixComprehension {
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = self.output.compile_register(context)?;
        let arguments = self
            .arguments
            .iter()
            .map(|argument| argument.compile_register(context))
            .collect::<MResult<Vec<_>>>()?;
        if arguments.is_empty() {
            context.emit_nullop(hash_str("matrix/comprehension"), output);
        } else {
            context.emit_varop(hash_str("matrix/comprehension"), output, arguments);
        }
        Ok(output)
    }
}
