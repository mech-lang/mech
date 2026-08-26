use crate::intrinsics::*;
#[cfg(feature = "matrix_comprehensions")]
use std::sync::LazyLock;

#[cfg(feature = "matrix_comprehensions")]
static PURE_MATRIX_COMPREHENSION_CONTRACT: LazyLock<OperationContractDeclaration> =
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

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn detach_comprehension_value(value: &LegacyValue) -> LegacyValue {
    match value {
        LegacyValue::MutableReference(reference) => reference.borrow().clone(),
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
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn out(&self) -> LegacyValue {
        LegacyValue::Set(self.out.clone())
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

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
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

#[cfg(all(feature = "set", feature = "semantic-compiler"))]
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
    pub arguments: Vec<LegacyValue>,
    pub out: Ref<MechSet>,
}

#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl MechFunctionImpl for ValueSetComprehension {
    fn solve_result(&self) -> MResult<()> {
        let args = self
            .arguments
            .iter()
            .map(detach_comprehension_value)
            .collect::<Vec<LegacyValue>>();
        *self.out.borrow_mut() = MechSet::from_vec(args);
        Ok(())
    }

    fn out(&self) -> LegacyValue {
        LegacyValue::Set(self.out.clone())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl MechFunctionFactory for ValueSetComprehension {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::variadic(
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::AnyValue,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            // Bytecode v1 uses RuntimeNullary for a variadic operation with
            // zero inputs. Accept that canonical encoding as the empty
            // argument list while retaining the checked set output lane.
            FunctionArgs::Nullary(LegacyValue::Set(out)) => Ok(Box::new(ValueSetComprehension {
                arguments: Vec::new(),
                out,
            })),
            FunctionArgs::Variadic(LegacyValue::Set(out), arguments) => {
                Ok(Box::new(ValueSetComprehension { arguments, out }))
            }
            FunctionArgs::Nullary(out) | FunctionArgs::Variadic(out, _) => Err(MechError::new(
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

#[cfg(all(feature = "set_comprehensions", feature = "semantic-compiler"))]
impl MechFunctionCompiler for ValueSetComprehension {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = LegacyValue::Set(self.out.clone());
        let destination = compile_value_register(&output, self.out.addr(), ctx)?;
        let arguments = self
            .arguments
            .iter()
            .map(|argument| {
                compile_value_register(argument, core::ptr::from_ref(argument).addr(), ctx)
            })
            .collect::<MResult<Vec<_>>>()?;
        ctx.emit_varop(hash_str("set/comprehension"), destination, arguments);
        Ok(destination)
    }
}

// Matrix comprehensions -----------------------------------------------------

/// Runtime implementation for `matrix/comprehension`.
#[cfg(feature = "matrix_comprehensions")]
#[derive(Debug)]
pub struct ValueMatrixComprehension {
    pub arguments: Vec<LegacyValue>,
    pub out: Ref<LegacyValue>,
}

#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
impl MechFunctionImpl for ValueMatrixComprehension {
    fn solve_result(&self) -> MResult<()> {
        let args = self
            .arguments
            .iter()
            .map(detach_comprehension_value)
            .collect::<Vec<LegacyValue>>();
        let out = if args.is_empty() {
            LegacyValue::MatrixValue(Matrix::from_vec(vec![], 0, 0))
        } else {
            let fxn = crate::intrinsics::horzcat::impl_horzcat_fxn(&args)?;
            fxn.solve_result()?;
            fxn.out()
        };
        *self.out.borrow_mut() = out;
        Ok(())
    }

    fn out(&self) -> LegacyValue {
        self.out.borrow().clone()
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_MATRIX_COMPREHENSION_CONTRACT)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(vec![LegacyValue::MutableReference(self.out.clone())])
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
impl MechFunctionFactory for ValueMatrixComprehension {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::variadic(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            // Bytecode v1 represents a zero-input variadic operation as a
            // nullary call. Empty comprehensions are valid 0x0 matrices, so
            // retain that canonical encoding instead of imposing an
            // accidental one-element minimum at reconstruction time.
            FunctionArgs::Nullary(out) => Ok(Box::new(ValueMatrixComprehension {
                arguments: Vec::new(),
                out: Ref::new(out),
            })),
            FunctionArgs::Variadic(out, arguments) => Ok(Box::new(ValueMatrixComprehension {
                arguments,
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

#[cfg(all(feature = "matrix_comprehensions", feature = "semantic-compiler"))]
impl MechFunctionCompiler for ValueMatrixComprehension {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = self.out.borrow().clone();
        let destination = compile_value_register(&output, self.out.addr(), ctx)?;
        let arguments = self
            .arguments
            .iter()
            .map(|argument| {
                compile_value_register(argument, core::ptr::from_ref(argument).addr(), ctx)
            })
            .collect::<MResult<Vec<_>>>()?;
        ctx.emit_varop(hash_str("matrix/comprehension"), destination, arguments);
        Ok(destination)
    }
}
