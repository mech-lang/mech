use crate::*;
// Equal ---------------------------------------------------------------

macro_rules! eq_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$lhs).len() {
                (&mut (*$out))[i] = (&(*$lhs))[i] == (*$rhs);
            }
        }
    };
}

macro_rules! eq_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$rhs).len() {
                (&mut (*$out))[i] = (*$lhs) == (&(*$rhs))[i];
            }
        }
    };
}

macro_rules! eq_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$lhs).len() {
                (&mut (*$out))[i] = (&(*$lhs))[i] == (&(*$rhs))[i];
            }
        }
    };
}

macro_rules! eq_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            (*$out) = (*$lhs) == (*$rhs);
        }
    };
}

macro_rules! eq_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = lhs_col[i] == rhs_deref[i];
                }
            }
        }
    };
}

macro_rules! eq_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = lhs_deref[i] == rhs_col[i];
                }
            }
        }
    };
}

macro_rules! eq_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = lhs_row[i] == rhs_deref[i];
                }
            }
        }
    };
}

macro_rules! eq_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = lhs_deref[i] == rhs_row[i];
                }
            }
        }
    };
}

impl_compare_fxns!(EQ);

#[cfg(feature = "atom")]
#[derive(Debug)]
pub struct AtomEq {
    lhs: FunctionValueInput,
    rhs: FunctionValueInput,
    pub out: Ref<bool>,
}
#[cfg(feature = "atom")]
impl MechFunctionFactory for AtomEq {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::Atom,
        FunctionValueRepresentation::Atom,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        let lhs = lhs.value();
        let rhs = rhs.value();
        let out: Ref<bool> = out.try_ref()?;
        Ok(Box::new(AtomEq { lhs, rhs, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_COMPARE_SCALAR_CONTRACT)
    }
}
#[cfg(feature = "atom")]
impl MechFunctionImpl for AtomEq {
    fn solve_result(&self) -> MResult<()> {
        let next = self.lhs.snapshot_eq(&self.rhs)?;
        *self.out.borrow_mut() = next;
        Ok(())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_COMPARE_SCALAR_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
}
#[cfg(feature = "atom")]
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for AtomEq {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("AtomEq");
        let destination = compile_register_brrw!(self.out, ctx);
        let lhs = self.lhs.compile_register(ctx)?;
        let rhs = self.rhs.compile_register(ctx)?;
        ctx.emit_binop(hash_str(&name), destination, lhs, rhs);
        Ok(destination)
    }
}

#[cfg(feature = "table")]
#[derive(Debug)]
pub struct TableEq {
    lhs: FunctionValueInput,
    rhs: FunctionValueInput,
    pub out: Ref<bool>,
}
#[cfg(feature = "table")]
impl MechFunctionFactory for TableEq {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::Table,
        FunctionValueRepresentation::Table,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        let lhs = lhs.value();
        let rhs = rhs.value();
        let out: Ref<bool> = out.try_ref()?;
        Ok(Box::new(TableEq { lhs, rhs, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_COMPARE_SCALAR_CONTRACT)
    }
}
#[cfg(feature = "table")]
impl MechFunctionImpl for TableEq {
    fn solve_result(&self) -> MResult<()> {
        let next = self.lhs.snapshot_eq(&self.rhs)?;
        *self.out.borrow_mut() = next;
        Ok(())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_COMPARE_SCALAR_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
}
#[cfg(feature = "table")]
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for TableEq {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("TableEq");
        let destination = compile_register_brrw!(self.out, ctx);
        let lhs = self.lhs.compile_register(ctx)?;
        let rhs = self.rhs.compile_register(ctx)?;
        ctx.emit_binop(hash_str(&name), destination, lhs, rhs);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
pub struct CompareEqual;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for CompareEqual {
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
        let lhs = specialization.input(0).expect("validated comparison lhs");
        let rhs = specialization.input(1).expect("validated comparison rhs");

        let extents = crate::semantic_compare_extents(&[lhs, rhs])?;
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
            ExecutionTarget::DirectRuntime,
            vec![extents].into_boxed_slice(),
            &[lhs, rhs],
        )
    }
}

#[cfg(all(
    test,
    feature = "runtime",
    feature = "f64",
    feature = "bool",
    feature = "matrix2",
    feature = "matrixd",
    feature = "atom",
    feature = "table"
))]
mod invocation_port_tests {
    use super::*;
    use mech_core::snapshot::*;
    use nalgebra::{DMatrix, Matrix2};
    use std::rc::Rc;

    fn canonical_value(body: SchemaBody, data: ValueDataDraft) -> ValueCell {
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

    fn canonical_bool_output() -> (Ref<bool>, ValueCell) {
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

    #[test]
    fn scalar_comparison_uses_exact_ports_identity_and_state() {
        let output = ValueCell::from_exact(false).unwrap();
        let alias = output.clone();
        let function = EQSS::<f64>::new_invocation(FunctionInvocation::binary(
            output.clone(),
            ValueCell::from_exact(3.0_f64).unwrap(),
            ValueCell::from_exact(3.0_f64).unwrap(),
        ))
        .unwrap();
        function.solve_result().unwrap();
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::Bool(true)
        ));
        assert!(output.same_cell(&alias));
        assert_eq!(
            function.reactive_output_cell_ids(),
            vec![output.reactive_cell_id()]
        );

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
            output.replace(&ValueCell::from_exact(false)?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::Bool(true)
        ));
    }

    #[test]
    fn fixed_and_dynamic_comparisons_preserve_exact_storage() {
        let lhs = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let rhs = Ref::new(Matrix2::new(1.0_f64, 0.0, 3.0, 5.0));
        let out = Ref::new(Matrix2::from_element(false));
        EQM2M2::<f64>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(out.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(lhs, 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(rhs, 2, 2).unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(*out.borrow(), Matrix2::new(true, false, true, false));

        let dynamic_lhs = Ref::new(DMatrix::from_row_slice(1, 2, &[1.0_f64, 2.0]));
        let dynamic_rhs = Ref::new(DMatrix::from_row_slice(1, 2, &[1.0_f64, 0.0]));
        let dynamic_out = Ref::new(DMatrix::from_element(1, 2, false));
        let function = EQMDMD::<f64>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(dynamic_out.clone(), 1, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(dynamic_lhs, 1, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(dynamic_rhs, 1, 2).unwrap(),
        ))
        .unwrap();
        function.solve_result().unwrap();
        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
            *dynamic_out.borrow_mut() = DMatrix::from_element(2, 1, false);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(
            *dynamic_out.borrow(),
            DMatrix::from_row_slice(1, 2, &[true, false])
        );
    }

    #[test]
    fn comparison_rejects_wrong_exact_types_and_layouts() {
        assert!(
            EQSS::<f64>::new_invocation(FunctionInvocation::binary(
                ValueCell::from_exact(false).unwrap(),
                ValueCell::from_exact(1.0_f64).unwrap(),
                ValueCell::from_exact(1_usize).unwrap(),
            ))
            .is_err()
        );
        assert!(
            EQSS::<f64>::new_invocation(FunctionInvocation::unary(
                ValueCell::from_exact(false).unwrap(),
                ValueCell::from_exact(1.0_f64).unwrap(),
            ))
            .is_err()
        );
    }

    #[test]
    fn atom_and_table_comparisons_use_canonical_snapshots() {
        let nominal = NominalKey::from_bytes([9; 32]);
        let (atom_out, atom_output) = canonical_bool_output();
        AtomEq::new_invocation(FunctionInvocation::binary(
            atom_output,
            canonical_value(SchemaBody::Atom(nominal), ValueDataDraft::Atom),
            canonical_value(SchemaBody::Atom(nominal), ValueDataDraft::Atom),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert!(*atom_out.borrow());

        let table_body = SchemaBody::Table {
            columns: Box::new([]),
            rows: CardinalitySpec::Exact(DimensionExpr::Constant(0)),
        };
        let (table_out, table_output) = canonical_bool_output();
        TableEq::new_invocation(FunctionInvocation::binary(
            table_output,
            canonical_value(table_body.clone(), ValueDataDraft::Table(Box::new([]))),
            canonical_value(table_body, ValueDataDraft::Table(Box::new([]))),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert!(*table_out.borrow());
    }
}
