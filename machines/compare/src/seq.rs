use crate::*;

#[derive(Debug)]
pub struct StrictEqValue {
    lhs: FunctionValueInput,
    rhs: FunctionValueInput,
    pub out: Ref<bool>,
}

impl MechFunctionImpl for StrictEqValue {
    fn solve_result(&self) -> MResult<()> {
        *self.out.borrow_mut() = self.lhs.snapshot_eq(&self.rhs)?;
        Ok(())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::compare_full_write_contract(
            FunctionValueRepresentation::Bool,
        ))
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
}

impl MechFunctionFactory for StrictEqValue {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        Ok(Box::new(Self {
            lhs: lhs.value(),
            rhs: rhs.value(),
            out: out.try_ref()?,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(crate::compare_full_write_contract(
            FunctionValueRepresentation::Bool,
        ))
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for StrictEqValue {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = compile_register_brrw!(self.out, ctx);
        let lhs = self.lhs.compile_register(ctx)?;
        let rhs = self.rhs.compile_register(ctx)?;
        ctx.emit_binop(hash_str("compare/seq"), destination, lhs, rhs);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
pub struct CompareStrictEqual;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for CompareStrictEqual {
    fn specialize_invocation(
        &self,
        specialization: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if specialization.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: specialization.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let lhs = specialization.input(0).expect("validated lhs");
        let rhs = specialization.input(1).expect("validated rhs");
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
            ExecutionTarget::DirectRuntime,
            vec![Vec::<u64>::new().into_boxed_slice()].into_boxed_slice(),
            &[lhs, rhs],
        )
    }
}

#[cfg(all(test, feature = "runtime", feature = "bool", feature = "sneq"))]
mod canonical_strict_equality_tests {
    use super::*;
    use crate::StrictNotEqValue;
    use mech_core::snapshot::*;
    use std::rc::Rc;

    fn value_cell(body: SchemaBody, data: ValueDataDraft) -> ValueCell {
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body,
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let value = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data,
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        ValueCell::from_value(value, Rc::new(schemas)).unwrap()
    }

    fn bool_output() -> (Ref<bool>, ValueCell) {
        let reference = Ref::new(false);
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Bool,
        }
        .finalize()
        .unwrap();
        let shape = schema.instantiate_shape(Box::new([])).unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let cell = ValueCell::from_ref(reference.clone(), schema, shape, Rc::new(schemas)).unwrap();
        (reference, cell)
    }

    fn strict_results(lhs: ValueCell, rhs: ValueCell) -> (bool, bool) {
        let (equal, equal_cell) = bool_output();
        StrictEqValue::new_invocation(FunctionInvocation::binary(
            equal_cell,
            lhs.clone(),
            rhs.clone(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        let (not_equal, not_equal_cell) = bool_output();
        StrictNotEqValue::new_invocation(FunctionInvocation::binary(not_equal_cell, lhs, rhs))
            .unwrap()
            .solve_result()
            .unwrap();
        let results = (*equal.borrow(), *not_equal.borrow());
        results
    }

    #[test]
    fn strict_equality_covers_scalar_aggregate_option_nominal_and_matrix_values() {
        let scalar = || value_cell(SchemaBody::Index, ValueDataDraft::Index(7));
        assert_eq!(strict_results(scalar(), scalar()), (true, false));

        let tuple_body =
            SchemaBody::Tuple(vec![SchemaBody::Index, SchemaBody::Bool].into_boxed_slice());
        let tuple_data = ValueDataDraft::Tuple(
            vec![ValueDataDraft::Index(7), ValueDataDraft::Bool(true)].into_boxed_slice(),
        );
        assert_eq!(
            strict_results(
                value_cell(tuple_body.clone(), tuple_data.clone()),
                value_cell(tuple_body, tuple_data),
            ),
            (true, false),
        );

        let absent = || {
            value_cell(
                SchemaBody::Option(Box::new(SchemaBody::Bool)),
                ValueDataDraft::Option(OptionDraft {
                    present: false,
                    value: None,
                }),
            )
        };
        assert_eq!(strict_results(absent(), absent()), (true, false));

        let nominal = NominalKey::from_bytes([3; 32]);
        assert_eq!(
            strict_results(
                value_cell(SchemaBody::Atom(nominal), ValueDataDraft::Atom),
                value_cell(SchemaBody::Atom(nominal), ValueDataDraft::Atom),
            ),
            (true, false),
        );

        let matrix_body = SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)]
                .into_boxed_slice(),
        };
        let matrix_data = ValueDataDraft::Matrix(
            vec![
                ValueDataDraft::F64(F64Bits::from_f64(1.0)),
                ValueDataDraft::F64(F64Bits::from_f64(2.0)),
            ]
            .into_boxed_slice(),
        );
        assert_eq!(
            strict_results(
                value_cell(matrix_body.clone(), matrix_data.clone()),
                value_cell(matrix_body, matrix_data),
            ),
            (true, false),
        );
    }

    #[test]
    fn strict_equality_rejects_schema_distinct_equal_payloads() {
        let left = value_cell(
            SchemaBody::Atom(NominalKey::from_bytes([1; 32])),
            ValueDataDraft::Atom,
        );
        let right = value_cell(
            SchemaBody::Atom(NominalKey::from_bytes([2; 32])),
            ValueDataDraft::Atom,
        );
        assert_eq!(strict_results(left, right), (false, true));
    }
}
