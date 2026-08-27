use crate::*;
#[cfg(all(feature = "matrix", feature = "source"))]
use mech_core::matrix::Matrix;

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
    pub lhs: Ref<MechAtom>,
    pub rhs: Ref<MechAtom>,
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
        let lhs: Ref<MechAtom> = lhs.try_ref()?;
        let rhs: Ref<MechAtom> = rhs.try_ref()?;
        let out: Ref<bool> = out.try_ref()?;
        Ok(Box::new(AtomEq { lhs, rhs, out }))
    }

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        Self::new_invocation(args.into())
    }
}
#[cfg(feature = "atom")]
impl MechFunctionImpl for AtomEq {
    fn solve_result(&self) -> MResult<()> {
        let lhs_ptr = self.lhs.as_ptr();
        let rhs_ptr = self.rhs.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        unsafe {
            *out_ptr = (*lhs_ptr) == (*rhs_ptr);
        };
        Ok(())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn out(&self) -> LegacyValue {
        self.out.to_value()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
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
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
    }
}

#[cfg(feature = "table")]
#[derive(Debug)]
pub struct TableEq {
    pub lhs: Ref<MechTable>,
    pub rhs: Ref<MechTable>,
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
        let lhs: Ref<MechTable> = lhs.try_ref()?;
        let rhs: Ref<MechTable> = rhs.try_ref()?;
        let out: Ref<bool> = out.try_ref()?;
        Ok(Box::new(TableEq { lhs, rhs, out }))
    }

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        Self::new_invocation(args.into())
    }
}
#[cfg(feature = "table")]
impl MechFunctionImpl for TableEq {
    fn solve_result(&self) -> MResult<()> {
        let lhs_ptr = self.lhs.as_ptr();
        let rhs_ptr = self.rhs.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        unsafe {
            *out_ptr = (*lhs_ptr) == (*rhs_ptr);
        };
        Ok(())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn out(&self) -> LegacyValue {
        self.out.to_value()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
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
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
    }
}

#[cfg(feature = "source")]
fn impl_eq_fxn(lhs_value: LegacyValue, rhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    match (&lhs_value, &rhs_value) {
        #[cfg(all(feature = "table"))]
        (LegacyValue::Table(lhs), LegacyValue::Table(rhs)) => {
            return Ok(Box::new(TableEq {
                lhs: lhs.clone(),
                rhs: rhs.clone(),
                out: Ref::new(false),
            }));
        }
        #[cfg(feature = "atom")]
        (LegacyValue::Atom(lhs), LegacyValue::Atom(rhs)) => {
            return Ok(Box::new(AtomEq {
                lhs: lhs.clone(),
                rhs: rhs.clone(),
                out: Ref::new(false),
            }));
        }
        _ => (),
    }
    impl_binop_match_arms!(
      EQ,
      (lhs_value, rhs_value),
      Bool, bool, "bool";
      I8,   bool, "i8";
      I16,  bool, "i16";
      I32,  bool, "i32";
      I64,  bool, "i64";
      I128, bool, "i128";
      U8,   bool, "u8";
      U16,  bool, "u16";
      U32,  bool, "u32";
      U64,  bool, "u64";
      U128, bool, "u128";
      F32,  bool, "f32";
      F64,  bool, "f64";
      String, bool, "string";
      R64, bool, "rational";
      C64, bool, "complex";
    )
}

#[cfg(feature = "source")]
impl_mech_binop_fxn!(CompareEqual, impl_eq_fxn, "compare/eq");

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
    use nalgebra::{DMatrix, Matrix2};

    fn binary_args<T, O>(out: &Ref<O>, lhs: &Ref<T>, rhs: &Ref<T>) -> FunctionArgs
    where
        Ref<T>: ToValue,
        Ref<O>: ToValue,
    {
        FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value())
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
            invocation.out().reactive_root_cell_ids(),
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
    fn fixed_matrix_atom_and_table_factories_use_exact_ports() {
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

        let atom_lhs = Ref::new(MechAtom::from_name("same"));
        let atom_rhs = Ref::new(MechAtom::from_name("same"));
        let atom_out = Ref::new(false);
        let atom = AtomEq::new_invocation(binary_args(&atom_out, &atom_lhs, &atom_rhs).into())
            .unwrap();
        atom.solve_result().unwrap();
        assert!(*atom_out.borrow());
        assert_eq!(
            atom.reactive_output_cell_ids(),
            atom.out().reactive_root_cell_ids(),
        );

        let table_lhs = Ref::new(MechTable::from_parts(0, 0, Vec::new(), Vec::new()));
        let table_rhs = Ref::new(MechTable::from_parts(0, 0, Vec::new(), Vec::new()));
        let table_out = Ref::new(false);
        let table =
            TableEq::new_invocation(binary_args(&table_out, &table_lhs, &table_rhs).into())
                .unwrap();
        table.solve_result().unwrap();
        assert!(*table_out.borrow());
        assert_eq!(
            table.reactive_output_cell_ids(),
            table.out().reactive_root_cell_ids(),
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
}
