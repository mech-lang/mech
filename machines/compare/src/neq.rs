use crate::*;

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
            let out_deref = &mut (*$out);
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
            let out_deref = &mut (*$out);
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
            let out_deref = &mut (*$out);
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
            let out_deref = &mut (*$out);
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
    lhs: FunctionValueInput,
    rhs: FunctionValueInput,
    pub out: Ref<bool>,
}
#[cfg(feature = "atom")]
impl MechFunctionFactory for AtomNeq {
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
        let out: Ref<bool> = out.try_ref()?;
        Ok(Box::new(AtomNeq { lhs, rhs, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_COMPARE_SCALAR_CONTRACT)
    }
}
#[cfg(feature = "atom")]
impl MechFunctionImpl for AtomNeq {
    fn solve_result(&self) -> MResult<()> {
        let next = !self.lhs.snapshot_eq(&self.rhs)?;
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
impl MechFunctionCompiler for AtomNeq {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("AtomNeq");
        let destination = compile_register_brrw!(self.out, ctx);
        let lhs = self.lhs.compile_register(ctx)?;
        let rhs = self.rhs.compile_register(ctx)?;
        ctx.emit_binop(hash_str(&name), destination, lhs, rhs);
        Ok(destination)
    }
}

#[cfg(feature = "table")]
#[derive(Debug)]
pub struct TableNeq {
    lhs: FunctionValueInput,
    rhs: FunctionValueInput,
    pub out: Ref<bool>,
}
#[cfg(feature = "table")]
impl MechFunctionFactory for TableNeq {
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
        let out: Ref<bool> = out.try_ref()?;
        Ok(Box::new(TableNeq { lhs, rhs, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_COMPARE_SCALAR_CONTRACT)
    }
}
#[cfg(feature = "table")]
impl MechFunctionImpl for TableNeq {
    fn solve_result(&self) -> MResult<()> {
        let next = !self.lhs.snapshot_eq(&self.rhs)?;
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
impl MechFunctionCompiler for TableNeq {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("TableNeq");
        let destination = compile_register_brrw!(self.out, ctx);
        let lhs = self.lhs.compile_register(ctx)?;
        let rhs = self.rhs.compile_register(ctx)?;
        ctx.emit_binop(hash_str(&name), destination, lhs, rhs);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
pub struct CompareNotEqual;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for CompareNotEqual {
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
