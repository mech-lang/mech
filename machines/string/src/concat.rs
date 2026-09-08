use crate::*;

#[cfg(feature = "matrix")]
macro_rules! concat_scalar_lhs_op {
    () => {};
}

#[cfg(feature = "matrix")]
macro_rules! concat_scalar_rhs_op {
    () => {};
}

#[cfg(feature = "matrix")]
macro_rules! concat_vec_op {
    () => {};
}

macro_rules! concat_op {
    () => {};
}

#[cfg(feature = "matrix")]
macro_rules! concat_mat_vec_op {
    () => {};
}

#[cfg(feature = "matrix")]
macro_rules! concat_vec_mat_op {
    () => {};
}

#[cfg(feature = "matrix")]
macro_rules! concat_mat_row_op {
    () => {};
}

#[cfg(feature = "matrix")]
macro_rules! concat_row_mat_op {
    () => {};
}

impl_string_fxns!(Concat);

#[cfg(feature = "source")]
pub struct StringConcat;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for StringConcat {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: invocation.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let lhs = invocation.input(0).expect("validated concat lhs");
        let rhs = invocation.input(1).expect("validated concat rhs");
        let extents = [lhs, rhs]
            .into_iter()
            .map(|input| {
                input
                    .cell()?
                    .resolved_descriptor()?
                    .current_extents()
                    .map_err(MechError::from)
            })
            .collect::<MResult<Vec<_>>>()?;
        let output_extents = extents
            .iter()
            .find(|extent| !extent.is_empty())
            .cloned()
            .unwrap_or_else(|| Box::new([]));
        context.bind_resolved_runtime(
            mech_core::RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
            mech_core::ExecutionTarget::DirectRuntime,
            vec![output_extents].into_boxed_slice(),
            &[lhs, rhs],
        )
    }
}

#[cfg(all(test, feature = "string"))]
mod scalar_port_tests {
    use super::*;

    fn string_value(cell: &ValueCell) -> String {
        let snapshot = cell.snapshot().unwrap();
        let ValueData::String(value) = snapshot.data() else {
            panic!("expected canonical string output")
        };
        value.to_string()
    }

    #[test]
    fn scalar_concat_uses_exact_canonical_ports_and_state() {
        let output = ValueCell::from_exact(String::new()).unwrap();
        let alias = output.clone();
        let function = crate::test_managed_factory::<ConcatSS<String>>(
            FunctionInvocation::binary(
                output.clone(),
                ValueCell::from_exact("left".to_string()).unwrap(),
                ValueCell::from_exact("-right".to_string()).unwrap(),
            ),
            "string/concat",
        );
        function.instance().solve_result().unwrap();
        assert_eq!(string_value(&output), "left-right");
        assert!(output.same_cell(&alias));
        assert_eq!(
            function.instance().reactive_output_cell_ids(),
            vec![output.reactive_cell_id()]
        );

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            output.replace(&ValueCell::from_exact("changed".to_string())?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(string_value(&output), "left-right");

        assert!(
            ConcatSS::<String>::new_invocation(FunctionInvocation::unary(
                ValueCell::from_exact(String::new()).unwrap(),
                ValueCell::from_exact("wrong-layout".to_string()).unwrap(),
            ))
            .is_err()
        );
        assert!(
            ConcatSS::<String>::new_invocation(FunctionInvocation::binary(
                ValueCell::from_exact(String::new()).unwrap(),
                ValueCell::from_exact("left".to_string()).unwrap(),
                ValueCell::from_exact(1_usize).unwrap(),
            ))
            .is_err()
        );
    }

    #[test]
    fn bound_scalar_concat_replans_same_shape_payload_growth_and_shrink() {
        let output = ValueCell::from_exact(String::new()).unwrap();
        let lhs = ValueCell::from_exact("a".to_owned()).unwrap();
        let rhs = ValueCell::from_exact("!".to_owned()).unwrap();
        let function = crate::test_managed_factory::<ConcatSS<String>>(
            FunctionInvocation::binary(output.clone(), lhs.clone(), rhs),
            "string/concat",
        );

        function.instance().solve_result().unwrap();
        assert_eq!(string_value(&output), "a!");

        let larger = ValueCell::from_exact("payload-growth-without-shape-change".to_owned())
            .unwrap()
            .snapshot()
            .unwrap();
        lhs.replace(&larger).unwrap();
        function.instance().solve_result().unwrap();
        assert_eq!(
            string_value(&output),
            "payload-growth-without-shape-change!"
        );

        let smaller = ValueCell::from_exact("z".to_owned())
            .unwrap()
            .snapshot()
            .unwrap();
        lhs.replace(&smaller).unwrap();
        function.instance().solve_result().unwrap();
        assert_eq!(string_value(&output), "z!");
    }

    #[test]
    fn resident_concat_rejects_oversized_candidate_before_result_allocation_and_recovers() {
        let output = ValueCell::from_exact(String::new()).unwrap();
        let lhs = ValueCell::from_exact("a".to_owned()).unwrap();
        let rhs = ValueCell::from_exact("b".to_owned()).unwrap();
        let function = crate::test_managed_factory_for_target::<ConcatSS<String>>(
            FunctionInvocation::binary(output.clone(), lhs.clone(), rhs.clone()),
            "string/concat",
            ExecutionTarget::ResidentCpu,
        );
        function.instance().solve_result().unwrap();
        assert_eq!(string_value(&output), "ab");

        let operand_bytes = RESIDENT_MAX_BYTES as usize / 2 + 1_024;
        lhs.replace(
            &ValueCell::from_exact("x".repeat(operand_bytes))
                .unwrap()
                .snapshot()
                .unwrap(),
        )
        .unwrap();
        rhs.replace(
            &ValueCell::from_exact("y".repeat(operand_bytes))
                .unwrap()
                .snapshot()
                .unwrap(),
        )
        .unwrap();
        let version = output.published_version();
        let before = string_value(&output);
        let (result, maximum_allocation) =
            crate::allocation_probe::maximum_requested(|| function.instance().solve_result());
        assert!(result.is_err());
        assert_eq!(output.published_version(), version);
        assert_eq!(string_value(&output), before);
        assert!(
            maximum_allocation < operand_bytes,
            "rejected turn allocated a result-sized String ({maximum_allocation} bytes)"
        );

        lhs.replace(
            &ValueCell::from_exact("valid".to_owned())
                .unwrap()
                .snapshot()
                .unwrap(),
        )
        .unwrap();
        rhs.replace(
            &ValueCell::from_exact("-again".to_owned())
                .unwrap()
                .snapshot()
                .unwrap(),
        )
        .unwrap();
        function.instance().solve_result().unwrap();
        assert_eq!(string_value(&output), "valid-again");
    }

    #[cfg(feature = "source")]
    #[test]
    fn source_specialization_keeps_concat_behavior() {
        let mut builder = FunctionCatalogBuilder::new();
        crate::catalog::install_runtime(&mut builder).unwrap();
        crate::catalog::install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let function = crate::test_source_specialize(
            &catalog,
            "string/concat",
            vec![
                ValueCell::from_exact("source".to_string()).unwrap(),
                ValueCell::from_exact("-path".to_string()).unwrap(),
            ],
        );
        function.instance().solve_result().unwrap();
        assert!(matches!(
            function.output().snapshot().unwrap().data(),
            ValueData::String(value) if value.as_ref() == "source-path"
        ));
    }
}

#[cfg(all(test, feature = "string", feature = "matrix2", feature = "matrixd"))]
mod fixed_matrix_port_tests {
    use super::*;

    #[test]
    fn fixed_concat_preserves_storage_and_rejects_dynamic_inputs() {
        let lhs = ValueCell::from_exact(Matrix2::new(
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ))
        .unwrap();
        let rhs = ValueCell::from_exact(Matrix2::from_element("!".to_string())).unwrap();
        let out = ValueCell::from_exact(Matrix2::from_element(String::new())).unwrap();
        let alias = out.clone();
        let function = crate::test_managed_factory::<ConcatM2M2<String>>(
            FunctionInvocation::binary(out.clone(), lhs, rhs),
            "string/concat",
        );
        function.instance().solve_result().unwrap();
        assert!(out.same_logical_cell(&alias));
        let expected = ValueCell::from_exact(Matrix2::new(
            "a!".to_string(),
            "b!".to_string(),
            "c!".to_string(),
            "d!".to_string(),
        ))
        .unwrap()
        .snapshot()
        .unwrap();
        let actual = out.snapshot().unwrap();
        assert!(
            actual
                .language_eq(
                    &actual.schemas().unwrap(),
                    &expected,
                    &expected.schemas().unwrap(),
                )
                .unwrap()
        );

        let wrong = ValueCell::from_exact(DMatrix::from_element(2, 2, "x".to_string())).unwrap();
        assert!(
            ConcatM2M2::<String>::new_invocation(FunctionInvocation::binary(
                ValueCell::from_exact(Matrix2::from_element(String::new())).unwrap(),
                wrong,
                ValueCell::from_exact(Matrix2::from_element("y".to_string())).unwrap(),
            ))
            .is_err()
        );
    }
}

#[cfg(all(
    test,
    feature = "string",
    feature = "matrixd",
    feature = "vectord",
    feature = "row_vectord"
))]
mod dynamic_matrix_port_tests {
    use super::*;

    fn matrix(values: &[&str]) -> DMatrix<String> {
        DMatrix::from_row_slice(
            2,
            2,
            &values
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn dynamic_broadcast_orientation_and_shape_rollback_are_canonical() {
        let matrix_cell = ValueCell::from_exact(matrix(&["a", "b", "c", "d"])).unwrap();
        let vector =
            ValueCell::from_exact(DVector::from_vec(vec!["v1".to_string(), "v2".to_string()]))
                .unwrap();
        let row = ValueCell::from_exact(RowDVector::from_vec(vec![
            "r1".to_string(),
            "r2".to_string(),
        ]))
        .unwrap();

        let vector_out = ValueCell::from_exact(DMatrix::from_element(2, 2, String::new())).unwrap();
        let vector_function = crate::test_managed_factory::<ConcatMDVD<String>>(
            FunctionInvocation::binary(vector_out.clone(), matrix_cell.clone(), vector),
            "string/concat",
        );
        vector_function.instance().solve_result().unwrap();
        assert_matrix(&vector_out, &["av1", "bv1", "cv2", "dv2"]);

        let row_out = ValueCell::from_exact(DMatrix::from_element(2, 2, String::new())).unwrap();
        crate::test_managed_factory::<ConcatMDRD<String>>(
            FunctionInvocation::binary(row_out.clone(), matrix_cell, row),
            "string/concat",
        )
        .instance()
        .solve_result()
        .unwrap();
        assert_matrix(&row_out, &["ar1", "br2", "cr1", "dr2"]);

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(vector_function.instance())?;
            vector_out.replace(
                &ValueCell::from_exact(DMatrix::from_element(1, 3, "changed".to_string()))?
                    .snapshot()?,
            )?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_matrix(&vector_out, &["av1", "bv1", "cv2", "dv2"]);
    }

    fn assert_matrix(actual: &ValueCell, expected: &[&str]) {
        let expected = ValueCell::from_exact(matrix(expected))
            .unwrap()
            .snapshot()
            .unwrap();
        let actual = actual.snapshot().unwrap();
        assert!(
            actual
                .language_eq(
                    &actual.schemas().unwrap(),
                    &expected,
                    &expected.schemas().unwrap(),
                )
                .unwrap()
        );
    }
}
