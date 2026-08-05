#[macro_use]
use crate::intrinsics::*;

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn detach_comprehension_value(value: &Value) -> Value {
    match value {
        Value::MutableReference(reference) => reference.borrow().clone(),
        _ => value.clone(),
    }
}

// Set -----------------------------------------------------------------------

/// Runtime implementation for `set/define`.
#[cfg(feature = "set")]
#[derive(Debug)]
pub struct ValueSet {
    pub out: Ref<MechSet>,
}

#[cfg(all(feature = "set", feature = "functions"))]
impl MechFunctionImpl for ValueSet {
    fn solve(&self) {}

    fn out(&self) -> Value {
        Value::Set(self.out.clone())
    }

    fn reactive_dependency_scopes(
        &self,
        argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyScope>> {
        Some(vec![ReactiveDependencyScope::None; argument_count])
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(all(feature = "set", feature = "functions"))]
impl MechFunctionFactory for ValueSet {
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::nullary(FunctionValueRepresentation::Set);

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Nullary(out) => {
                let out: Ref<MechSet> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(ValueSet { out }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 0,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}

#[cfg(all(feature = "set", feature = "compiler"))]
impl MechFunctionCompiler for ValueSet {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        compile_nullop!("set/define", self.out, ctx);
    }
}

// Set comprehensions --------------------------------------------------------

#[cfg(feature = "set_comprehensions")]
#[derive(Debug, Clone)]
struct SetComprehensionOutputKindMismatchError {
    found: ValueKind,
}

#[cfg(feature = "set_comprehensions")]
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

/// Runtime implementation for `set/comprehension`.
#[cfg(feature = "set_comprehensions")]
#[derive(Debug)]
pub struct ValueSetComprehension {
    pub arguments: Vec<Value>,
    pub out: Ref<MechSet>,
}

#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl MechFunctionImpl for ValueSetComprehension {
    fn solve(&self) {
        let args = self
            .arguments
            .iter()
            .map(detach_comprehension_value)
            .collect::<Vec<Value>>();
        *self.out.borrow_mut() = MechSet::from_vec(args);
    }

    fn out(&self) -> Value {
        Value::Set(self.out.clone())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl MechFunctionFactory for ValueSetComprehension {
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::nullary(FunctionValueRepresentation::Set);

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Nullary(Value::Set(out)) => Ok(Box::new(ValueSetComprehension {
                arguments: Vec::new(),
                out,
            })),
            FunctionArgs::Nullary(out) => Err(MechError::new(
                SetComprehensionOutputKindMismatchError { found: out.kind() },
                None,
            )
            .with_compiler_loc()),
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 0,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}

#[cfg(all(feature = "set_comprehensions", feature = "compiler"))]
impl MechFunctionCompiler for ValueSetComprehension {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        compile_nullop!("set/comprehension", self.out, ctx);
    }
}

// Matrix comprehensions -----------------------------------------------------

/// Runtime implementation for `matrix/comprehension`.
#[cfg(feature = "matrix_comprehensions")]
#[derive(Debug)]
pub struct ValueMatrixComprehension {
    pub arguments: Vec<Value>,
    pub out: Ref<Value>,
}

#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
impl MechFunctionImpl for ValueMatrixComprehension {
    fn solve(&self) {
        let args = self
            .arguments
            .iter()
            .map(detach_comprehension_value)
            .collect::<Vec<Value>>();
        let out = if args.is_empty() {
            Value::MatrixValue(Matrix::from_vec(vec![], 0, 0))
        } else {
            let fxn = crate::intrinsics::horzcat::impl_horzcat_fxn(&args)
                .expect("matrix/comprehension input kinds changed to incompatible values");
            fxn.solve();
            fxn.out()
        };
        *self.out.borrow_mut() = out;
    }

    fn out(&self) -> Value {
        self.out.borrow().clone()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(vec![Value::MutableReference(self.out.clone())])
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
impl MechFunctionFactory for ValueMatrixComprehension {
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::nullary(FunctionValueRepresentation::AnyValue);

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Nullary(out) => Ok(Box::new(ValueMatrixComprehension {
                arguments: Vec::new(),
                out: Ref::new(out),
            })),
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 0,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}

#[cfg(all(feature = "matrix_comprehensions", feature = "compiler"))]
impl MechFunctionCompiler for ValueMatrixComprehension {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        compile_nullop!("matrix/comprehension", self.out, ctx);
    }
}
