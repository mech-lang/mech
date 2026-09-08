use crate::*;
// Equal ---------------------------------------------------------------

macro_rules! eq_scalar_lhs_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::RightScalar, |lhs, rhs| lhs == rhs)
    };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::RightScalar,
            |lhs, rhs| lhs == rhs,
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$lhs).len() {
                (&mut (*$out))[i] = (&(*$lhs))[i] == (*$rhs);
            }
        }
    };
}

macro_rules! eq_scalar_rhs_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::LeftScalar, |lhs, rhs| lhs == rhs)
    };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::LeftScalar,
            |lhs, rhs| lhs == rhs,
        )
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$rhs).len() {
                (&mut (*$out))[i] = (*$lhs) == (&(*$rhs))[i];
            }
        }
    };
}

macro_rules! eq_vec_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::Exact, |lhs, rhs| lhs == rhs)
    };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison($lhs, $rhs, $out, ComparisonBroadcast::Exact, |lhs, rhs| {
            lhs == rhs
        })
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$lhs).len() {
                (&mut (*$out))[i] = (&(*$lhs))[i] == (&(*$rhs))[i];
            }
        }
    };
}

macro_rules! eq_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::Exact, |lhs, rhs| lhs == rhs)
    };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison($lhs, $rhs, $out, ComparisonBroadcast::Exact, |lhs, rhs| {
            lhs == rhs
        })
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            (*$out) = (*$lhs) == (*$rhs);
        }
    };
}

macro_rules! eq_mat_vec_op {
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::RightColumn, |lhs, rhs| lhs == rhs)
    };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::RightColumn,
            |lhs, rhs| lhs == rhs,
        )
    };
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
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::LeftColumn, |lhs, rhs| lhs == rhs)
    };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::LeftColumn,
            |lhs, rhs| lhs == rhs,
        )
    };
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
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::RightRow, |lhs, rhs| lhs == rhs)
    };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::RightRow,
            |lhs, rhs| lhs == rhs,
        )
    };
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
    (canonical, $frame:expr, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_canonical_string_comparison($frame, $lhs, $rhs, $out, ComparisonBroadcast::LeftRow, |lhs, rhs| lhs == rhs)
    };
    (managed, $lhs:expr, $rhs:expr, $out:expr) => {
        apply_managed_comparison(
            $lhs,
            $rhs,
            $out,
            ComparisonBroadcast::LeftRow,
            |lhs, rhs| lhs == rhs,
        )
    };
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
    pub out: ManagedPort<bool>,
}
#[cfg(feature = "atom")]
impl MechFunctionFactory for AtomEq {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::Atom,
        FunctionValueRepresentation::Atom,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        let lhs = lhs.value();
        let rhs = rhs.value();
        let out = out.try_managed_element::<bool>()?;
        Ok(Box::new(AtomEq { lhs, rhs, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_COMPARE_SCALAR_CONTRACT)
    }
}
#[cfg(feature = "atom")]
impl MechFunctionImpl for AtomEq {
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        let next = frame.function_value_inputs_equal(&self.lhs, &self.rhs)?;
        frame.with_output_port_view(&self.out, |out| {
            out.try_fill_column_major(|_| Ok(next))
        })?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.out.cell()))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_COMPARE_SCALAR_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
    }
}
#[cfg(feature = "atom")]
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for AtomEq {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("AtomEq");
        let destination = compile_value_cell_register(self.out.cell(), ctx)?;
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
    pub out: ManagedPort<bool>,
}
#[cfg(feature = "table")]
impl MechFunctionFactory for TableEq {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::Table,
        FunctionValueRepresentation::Table,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        let lhs = lhs.value();
        let rhs = rhs.value();
        let out = out.try_managed_element::<bool>()?;
        Ok(Box::new(TableEq { lhs, rhs, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_COMPARE_SCALAR_CONTRACT)
    }
}
#[cfg(feature = "table")]
impl MechFunctionImpl for TableEq {
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        let next = frame.function_value_inputs_equal(&self.lhs, &self.rhs)?;
        frame.with_output_port_view(&self.out, |out| {
            out.try_fill_column_major(|_| Ok(next))
        })?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.out.cell()))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_COMPARE_SCALAR_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
    }
}
#[cfg(feature = "table")]
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for TableEq {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("TableEq");
        let destination = compile_value_cell_register(self.out.cell(), ctx)?;
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

    fn canonical_bool_output() -> ValueCell {
        ValueCell::from_exact(false).unwrap()
    }

    #[test]
    fn scalar_comparison_uses_exact_ports_identity_and_state() {
        let output = ValueCell::from_exact(false).unwrap();
        let alias = output.clone();
        let function = managed_test_function::<EQSS<f64>>(
            FunctionInvocation::binary(
                output.clone(),
                ValueCell::from_exact(3.0_f64).unwrap(),
                ValueCell::from_exact(3.0_f64).unwrap(),
            ),
            "compare/eq",
        );
        function.instance().solve_result().unwrap();
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::Bool(true)
        ));
        assert!(output.same_cell(&alias));
        assert_eq!(
            function.instance().reactive_output_cell_ids(),
            vec![output.reactive_cell_id()]
        );

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
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
    fn fixed_and_dynamic_comparisons_publish_managed_cells() {
        let lhs = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let rhs = Ref::new(Matrix2::new(1.0_f64, 0.0, 3.0, 5.0));
        let out =
            ValueCell::from_exact_matrix_ref(Ref::new(Matrix2::from_element(false)), 2, 2).unwrap();
        managed_test_function::<EQM2M2<f64>>(
            FunctionInvocation::binary(
                out.clone(),
                ValueCell::from_exact_matrix_ref(lhs, 2, 2).unwrap(),
                ValueCell::from_exact_matrix_ref(rhs, 2, 2).unwrap(),
            ),
            "compare/eq",
        )
        .instance()
        .solve_result()
        .unwrap();
        assert_bool_matrix(&out, &[true, false, true, false]);

        let dynamic_lhs = Ref::new(DMatrix::from_row_slice(1, 2, &[1.0_f64, 2.0]));
        let dynamic_rhs = Ref::new(DMatrix::from_row_slice(1, 2, &[1.0_f64, 0.0]));
        let dynamic_out =
            ValueCell::from_exact_matrix_ref(Ref::new(DMatrix::from_element(1, 2, false)), 1, 2)
                .unwrap();
        let function = managed_test_function::<EQMDMD<f64>>(
            FunctionInvocation::binary(
                dynamic_out.clone(),
                ValueCell::from_exact_matrix_ref(dynamic_lhs, 1, 2).unwrap(),
                ValueCell::from_exact_matrix_ref(dynamic_rhs, 1, 2).unwrap(),
            ),
            "compare/eq",
        );
        function.instance().solve_result().unwrap();
        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            dynamic_out.replace(
                &ValueCell::from_exact_matrix_ref(
                    Ref::new(DMatrix::from_element(2, 1, false)),
                    2,
                    1,
                )?
                .snapshot()?,
            )?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_bool_matrix(&dynamic_out, &[true, false]);
    }

    fn assert_bool_matrix(cell: &ValueCell, expected: &[bool]) {
        let snapshot = cell.snapshot().unwrap();
        let ValueData::Matrix(matrix) = snapshot.data() else {
            panic!("expected managed matrix")
        };
        let SequenceView::Bool(elements) = matrix.elements() else {
            panic!("expected Boolean elements")
        };
        assert_eq!(elements, expected);
    }

    #[cfg(all(
        feature = "vectord",
        feature = "row_vectord",
        feature = "lt",
        feature = "gt",
        feature = "min",
        feature = "max"
    ))]
    #[test]
    fn managed_comparison_preserves_scalar_row_column_and_minmax_semantics() {
        let matrix = || {
            ValueCell::from_exact_matrix_ref(
                Ref::new(DMatrix::from_row_slice(
                    2,
                    3,
                    &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0],
                )),
                2,
                3,
            )
            .unwrap()
        };
        let column = || {
            ValueCell::from_exact_matrix_ref(
                Ref::new(nalgebra::DVector::from_vec(vec![2.0_f64, 5.0])),
                2,
                1,
            )
            .unwrap()
        };
        let row = || {
            ValueCell::from_exact_matrix_ref(
                Ref::new(nalgebra::RowDVector::from_vec(vec![1.0_f64, 5.0, 6.0])),
                1,
                3,
            )
            .unwrap()
        };
        fn check<F: MechFunctionFactory>(
            lhs: ValueCell,
            rhs: ValueCell,
            operation: &'static str,
            expected: &[bool],
        ) {
            let output = ValueCell::from_exact_matrix_ref(
                Ref::new(DMatrix::from_element(2, 3, false)),
                2,
                3,
            )
            .unwrap();
            let function = managed_test_function::<F>(
                FunctionInvocation::binary(output.clone(), lhs, rhs),
                operation,
            );
            function.instance().solve_result().unwrap();
            assert_bool_matrix(&output, expected);
        }
        check::<EQMDVD<f64>>(
            matrix(),
            column(),
            "compare/eq",
            &[false, true, false, false, true, false],
        );
        check::<EQVDMD<f64>>(
            column(),
            matrix(),
            "compare/eq",
            &[false, true, false, false, true, false],
        );
        check::<EQMDRD<f64>>(
            matrix(),
            row(),
            "compare/eq",
            &[true, false, false, false, true, true],
        );
        check::<EQRDMD<f64>>(
            row(),
            matrix(),
            "compare/eq",
            &[true, false, false, false, true, true],
        );
        check::<crate::lt::LTSMD<f64>>(
            ValueCell::from_exact(3.0_f64).unwrap(),
            matrix(),
            "compare/lt",
            &[false, false, false, true, true, true],
        );
        check::<crate::gt::GTMDS<f64>>(
            matrix(),
            ValueCell::from_exact(3.0_f64).unwrap(),
            "compare/gt",
            &[false, false, false, true, true, true],
        );

        let min_output = ValueCell::from_exact(f64::NAN).unwrap();
        let max_output = ValueCell::from_exact(0.0_f64).unwrap();
        managed_test_function::<crate::min::MinSS<f64>>(
            FunctionInvocation::binary(
                min_output.clone(),
                ValueCell::from_exact(f64::NAN).unwrap(),
                ValueCell::from_exact(2.0_f64).unwrap(),
            ),
            "compare/min",
        )
        .instance()
        .solve_result()
        .unwrap();
        managed_test_function::<crate::max::MaxSS<f64>>(
            FunctionInvocation::binary(
                max_output.clone(),
                ValueCell::from_exact(2.0_f64).unwrap(),
                ValueCell::from_exact(f64::NAN).unwrap(),
            ),
            "compare/max",
        )
        .instance()
        .solve_result()
        .unwrap();
        let ValueData::F64(min_value) = *min_output.snapshot().unwrap().data() else {
            panic!("expected f64")
        };
        let ValueData::F64(max_value) = *max_output.snapshot().unwrap().data() else {
            panic!("expected f64")
        };
        assert!(min_value.to_f64().is_nan());
        assert_eq!(max_value.to_f64(), 2.0);
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
    fn every_enabled_fixed_width_comparison_rebinds_live_scalar_ports() {
        macro_rules! check_type {
            ($feature:literal, $type:ty, $first:expr, $second:expr) => {
                #[cfg(feature = $feature)]
                {
                    let input = ValueCell::from_exact($first).unwrap();
                    let output = ValueCell::from_exact(false).unwrap();
                    let consumer = managed_test_function::<EQSS<$type>>(
                        FunctionInvocation::binary(
                            output.clone(),
                            input.clone(),
                            ValueCell::from_exact($first).unwrap(),
                        ),
                        "compare/eq",
                    );
                    consumer.instance().solve_result().unwrap();
                    assert!(matches!(
                        output.snapshot().unwrap().data(),
                        ValueData::Bool(true)
                    ));
                    input
                        .replace(&ValueCell::from_exact($second).unwrap().snapshot().unwrap())
                        .unwrap();
                    consumer.instance().solve_result().unwrap();
                    assert!(matches!(
                        output.snapshot().unwrap().data(),
                        ValueData::Bool(false)
                    ));
                }
            };
        }
        check_type!("bool", bool, true, false);
        check_type!("u8", u8, 1_u8, 2_u8);
        check_type!("u16", u16, 1_u16, 2_u16);
        check_type!("u32", u32, 1_u32, 2_u32);
        check_type!("u64", u64, 1_u64, 2_u64);
        check_type!("u128", u128, 1_u128, 2_u128);
        check_type!("i8", i8, 1_i8, -2_i8);
        check_type!("i16", i16, 1_i16, -2_i16);
        check_type!("i32", i32, 1_i32, -2_i32);
        check_type!("i64", i64, 1_i64, -2_i64);
        check_type!("i128", i128, 1_i128, -2_i128);
        check_type!("f32", f32, 1_f32, 2_f32);
        check_type!("f64", f64, 1_f64, 2_f64);
        check_type!("r64", R64, R64::new(1, 2), R64::new(2, 3));
        check_type!("c64", C64, C64::new(1.0, 2.0), C64::new(2.0, 3.0));
    }

    #[test]
    fn atom_and_table_comparisons_use_canonical_snapshots() {
        let nominal = NominalKey::from_bytes([9; 32]);
        let atom_output = canonical_bool_output();
        managed_test_function::<AtomEq>(
            FunctionInvocation::binary(
                atom_output.clone(),
                canonical_value(SchemaBody::Atom(nominal), ValueDataDraft::Atom),
                canonical_value(SchemaBody::Atom(nominal), ValueDataDraft::Atom),
            ),
            "compare/eq",
        )
        .instance()
        .solve_result()
        .unwrap();
        assert!(matches!(
            atom_output.snapshot().unwrap().data(),
            ValueData::Bool(true)
        ));

        let table_body = SchemaBody::Table {
            columns: Box::new([]),
            rows: CardinalitySpec::Exact(DimensionExpr::Constant(0)),
        };
        let table_output = canonical_bool_output();
        managed_test_function::<TableEq>(
            FunctionInvocation::binary(
                table_output.clone(),
                canonical_value(table_body.clone(), ValueDataDraft::Table(Box::new([]))),
                canonical_value(table_body, ValueDataDraft::Table(Box::new([]))),
            ),
            "compare/eq",
        )
        .instance()
        .solve_result()
        .unwrap();
        assert!(matches!(
            table_output.snapshot().unwrap().data(),
            ValueData::Bool(true)
        ));
    }

    #[cfg(all(feature = "string", feature = "max"))]
    #[test]
    fn string_comparisons_stage_canonical_scalar_and_matrix_outputs() {
        let equal = ValueCell::from_exact(false).unwrap();
        managed_test_function::<EQSS<String>>(
            FunctionInvocation::binary(
                equal.clone(),
                ValueCell::from_exact(String::from("alpha")).unwrap(),
                ValueCell::from_exact(String::from("alpha")).unwrap(),
            ),
            "compare/eq-string",
        )
        .instance()
        .solve_result()
        .unwrap();
        assert!(matches!(
            equal.snapshot().unwrap().data(),
            ValueData::Bool(true)
        ));

        let maximum = ValueCell::from_exact(String::new()).unwrap();
        managed_test_function::<crate::max::MaxSS<String>>(
            FunctionInvocation::binary(
                maximum.clone(),
                ValueCell::from_exact(String::from("alpha")).unwrap(),
                ValueCell::from_exact(String::from("omega")).unwrap(),
            ),
            "compare/max-string",
        )
        .instance()
        .solve_result()
        .unwrap();
        let maximum_snapshot = maximum.snapshot().unwrap();
        let ValueData::String(maximum) = maximum_snapshot.data() else {
            panic!("expected canonical String output")
        };
        assert_eq!(maximum.as_ref(), "omega");

        let matrix_equal = ValueCell::from_exact(DMatrix::from_element(1, 2, false)).unwrap();
        managed_test_function::<EQMDMD<String>>(
            FunctionInvocation::binary(
                matrix_equal.clone(),
                ValueCell::from_exact(DMatrix::from_row_slice(
                    1,
                    2,
                    &[String::from("a"), String::from("b")],
                ))
                .unwrap(),
                ValueCell::from_exact(DMatrix::from_row_slice(
                    1,
                    2,
                    &[String::from("a"), String::from("c")],
                ))
                .unwrap(),
            ),
            "compare/eq-string-matrix",
        )
        .instance()
        .solve_result()
        .unwrap();
        assert_bool_matrix(&matrix_equal, &[true, false]);
    }
}
