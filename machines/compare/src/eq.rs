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
        _context: &mut SpecializationContext<'_>,
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

        #[cfg(feature = "atom")]
        if lhs.representation() == Some(FunctionValueRepresentation::Atom)
            && rhs.representation() == Some(FunctionValueRepresentation::Atom)
        {
            return SpecializedFunction::bind_factory::<AtomEq>(
                ValueCell::from_exact(false)?,
                vec![lhs.cell()?.clone(), rhs.cell()?.clone()].into_boxed_slice(),
            );
        }
        #[cfg(feature = "table")]
        if lhs.representation() == Some(FunctionValueRepresentation::Table)
            && rhs.representation() == Some(FunctionValueRepresentation::Table)
        {
            return SpecializedFunction::bind_factory::<TableEq>(
                ValueCell::from_exact(false)?,
                vec![lhs.cell()?.clone(), rhs.cell()?.clone()].into_boxed_slice(),
            );
        }

        try_compare_binary_factories!(eq, lhs, rhs, EQ);
        Err(MechError::new(
            FunctionArgumentTypeMismatch {
                role: FunctionArgumentRole::Input(0),
                expected: "matching supported comparison inputs".into(),
                found: format!(
                    "{:?} and {:?}",
                    lhs.representation(),
                    rhs.representation(),
                ),
            },
            None,
        )
        .with_compiler_loc())
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

    fn binary_args<T, O>(out: &Ref<O>, lhs: &Ref<T>, rhs: &Ref<T>) -> FunctionArgs
    where
        Ref<T>: ToValue,
        Ref<O>: ToValue,
    {
        FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value())
    }

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
    fn scalar_legacy_and_invocation_entries_are_equivalent() {
        let lhs = Ref::new(3.0_f64);
        let rhs = Ref::new(3.0_f64);
        let legacy_out = Ref::new(false);
        let invocation_out = Ref::new(false);

        let legacy = EQSS::<f64>::new(binary_args(&legacy_out, &lhs, &rhs)).unwrap();
        let invocation = EQSS::<f64>::new_invocation(
            binary_args(&invocation_out, &lhs, &rhs).into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();

        assert!(*legacy_out.borrow());
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
        assert_eq!(
            invocation.reactive_output_cell_ids(),
            invocation_out.to_value().reactive_root_cell_ids(),
        );

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*invocation)?;
            *invocation_out.borrow_mut() = false;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(*invocation_out.borrow());
    }

    #[test]
    fn fixed_and_dynamic_matrix_factories_use_exact_ports() {
        let lhs = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let rhs = Ref::new(Matrix2::new(1.0_f64, 0.0, 3.0, 5.0));
        let out = Ref::new(Matrix2::from_element(false));
        let function = EQM2M2::<f64>::new_invocation(binary_args(&out, &lhs, &rhs).into()).unwrap();
        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), Matrix2::new(true, false, true, false));
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            *out.borrow_mut() = Matrix2::from_element(false);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(*out.borrow(), Matrix2::new(true, false, true, false));

        let dynamic_lhs = Ref::new(DMatrix::from_row_slice(1, 2, &[1.0_f64, 2.0]));
        let dynamic_rhs = Ref::new(DMatrix::from_row_slice(1, 2, &[1.0_f64, 0.0]));
        let dynamic_out = Ref::new(DMatrix::from_element(1, 2, false));
        let dynamic = EQMDMD::<f64>::new_invocation(
            binary_args(&dynamic_out, &dynamic_lhs, &dynamic_rhs).into(),
        )
        .unwrap();
        dynamic.solve_result().unwrap();
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*dynamic)?;
            *dynamic_out.borrow_mut() = DMatrix::from_element(2, 1, false);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(
            *dynamic_out.borrow(),
            DMatrix::from_row_slice(1, 2, &[true, false]),
        );

    }

    #[test]
    fn comparison_ports_reject_wrong_types_and_layouts() {
        let out = Ref::new(false);
        let lhs = Ref::new(1.0_f64);
        let rhs = Ref::new(1_i8);
        let type_error = EQSS::<f64>::new_invocation(
            FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value()).into(),
        )
        .err()
        .expect("wrong exact input representation must fail");
        assert_eq!(type_error.kind_name(), "FunctionArgumentTypeMismatch");

        let arity_error = EQSS::<f64>::new_invocation(
            FunctionArgs::Unary(out.to_value(), lhs.to_value()).into(),
        )
        .err()
        .expect("wrong layout must fail");
        assert_eq!(arity_error.kind_name(), "IncorrectNumberOfArguments");
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
