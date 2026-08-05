use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::*;

// Not Equal ---------------------------------------------------------------

macro_rules! neq_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$lhs).len() {
                (&mut (*$out))[i] = (&(*$lhs))[i] != (*$rhs);
            }
        }
    };
}

macro_rules! neq_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$rhs).len() {
                (&mut (*$out))[i] = (*$lhs) != (&(*$rhs))[i];
            }
        }
    };
}

macro_rules! neq_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$lhs).len() {
                (&mut (*$out))[i] = (&(*$lhs))[i] != (&(*$rhs))[i];
            }
        }
    };
}

macro_rules! neq_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            (*$out) = (*$lhs) != (*$rhs);
        }
    };
}

macro_rules! neq_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = lhs_col[i] != rhs_deref[i];
                }
            }
        }
    };
}

macro_rules! neq_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = lhs_deref[i] != rhs_col[i];
                }
            }
        }
    };
}

macro_rules! neq_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = lhs_row[i] != rhs_deref[i];
                }
            }
        }
    };
}

macro_rules! neq_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = lhs_deref[i] != rhs_row[i];
                }
            }
        }
    };
}

impl_compare_fxns!(NEQ);

#[cfg(feature = "atom")]
#[derive(Debug)]
pub struct AtomNeq {
    pub lhs: Ref<MechAtom>,
    pub rhs: Ref<MechAtom>,
    pub out: Ref<bool>,
}
#[cfg(feature = "atom")]
impl MechFunctionFactory for AtomNeq {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::Atom,
        FunctionValueRepresentation::Atom,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
                let lhs: Ref<MechAtom> = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                let rhs: Ref<MechAtom> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                let out: Ref<bool> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(AtomNeq { lhs, rhs, out }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}
#[cfg(feature = "atom")]
impl MechFunctionImpl for AtomNeq {
    fn solve(&self) {
        let lhs_ptr = self.lhs.as_ptr();
        let rhs_ptr = self.rhs.as_ptr();
        let mut out_ptr = self.out.as_mut_ptr();
        unsafe {
            *out_ptr = (*lhs_ptr) != (*rhs_ptr);
        }
    }
    fn out(&self) -> Value {
        self.out.to_value()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "atom")]
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for AtomNeq {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("AtomNeq");
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
    }
}

#[cfg(feature = "table")]
#[derive(Debug)]
pub struct TableNeq {
    pub lhs: Ref<MechTable>,
    pub rhs: Ref<MechTable>,
    pub out: Ref<bool>,
}
#[cfg(feature = "table")]
impl MechFunctionFactory for TableNeq {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::Table,
        FunctionValueRepresentation::Table,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
                let lhs: Ref<MechTable> = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                let rhs: Ref<MechTable> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                let out: Ref<bool> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(TableNeq { lhs, rhs, out }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}
#[cfg(feature = "table")]
impl MechFunctionImpl for TableNeq {
    fn solve(&self) {
        let lhs_ptr = self.lhs.as_ptr();
        let rhs_ptr = self.rhs.as_ptr();
        let mut out_ptr = self.out.as_mut_ptr();
        unsafe {
            *out_ptr = (*lhs_ptr) != (*rhs_ptr);
        }
    }
    fn out(&self) -> Value {
        self.out.to_value()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "table")]
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for TableNeq {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("TableNeq");
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
    }
}

#[cfg(feature = "source")]
fn impl_neq_fxn(lhs_value: Value, rhs_value: Value) -> MResult<Box<dyn MechFunction>> {
    match (&lhs_value, &rhs_value) {
        #[cfg(all(feature = "table"))]
        (Value::Table(lhs), Value::Table(rhs)) => {
            return Ok(Box::new(TableNeq {
                lhs: lhs.clone(),
                rhs: rhs.clone(),
                out: Ref::new(false),
            }));
        }
        #[cfg(feature = "atom")]
        (Value::Atom(lhs), Value::Atom(rhs)) => {
            return Ok(Box::new(AtomNeq {
                lhs: lhs.clone(),
                rhs: rhs.clone(),
                out: Ref::new(false),
            }));
        }
        _ => (),
    }
    impl_binop_match_arms!(
      NEQ,
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
impl_mech_binop_fxn!(CompareNotEqual, impl_neq_fxn, "compare/neq");
