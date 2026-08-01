use super::{
    ComprehensionGeneratorError, Environment, SetComprehensionOutputKindMismatchError, expression,
};
use crate::patterns::PatternBindingSink;
#[cfg(feature = "compiler")]
use crate::{
    BytecodeCompilerContext, CompileConst, FeatureFlag, FeatureKind, MechFunctionCompiler, Register,
};
use crate::{
    ComprehensionQualifier, FunctionArgs, IncorrectNumberOfArguments, Interpreter,
    InterpreterExecution, MResult, MechError, MechFunction, MechFunctionFactory, MechFunctionImpl,
    NativeFunctionCompiler, Ref, ToValue, Value, execute_catalog_operation, hash_str,
};
use crate::{FunctionCompilerDescriptor, FunctionDescriptor};
#[cfg(feature = "matrix_comprehensions")]
use crate::{Matrix, MatrixComprehension};
#[cfg(feature = "set_comprehensions")]
use crate::{MechSet, SetComprehension};
use std::collections::HashMap;

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn comprehension_environments(
    qualifiers: &[ComprehensionQualifier],
    comprehension_id: u64,
    p: &InterpreterExecution<'_>,
) -> MResult<(Vec<Environment>, Interpreter)> {
    let mut envs: Vec<Environment> = vec![HashMap::new()];
    let mut new_p: Interpreter = (**p).clone();
    new_p.id = comprehension_id;
    new_p.clear_plan();
    for qual in qualifiers {
        envs = match qual {
            ComprehensionQualifier::Generator((pttrn, expr)) => {
                let compiled = crate::patterns::compile_pattern(pttrn, None, &new_p)?;
                let mut new_envs = Vec::new();
                for env in &envs {
                    let collection = p.with_interpreter(&new_p, |execution| {
                        expression(expr, Some(env), execution)
                    })?;
                    for elmnt in comprehension_generator_values(&collection)? {
                        let mut new_env = env.clone();
                        let pattern_match = p.with_interpreter(&new_p, |execution| {
                            crate::patterns::match_compiled_pattern_with_environment_constraints(
                                &compiled, &elmnt, &new_env, execution,
                            )
                        })?;
                        if pattern_match.matched {
                            crate::patterns::EnvironmentBindingSink::new(&mut new_env)
                                .commit(&pattern_match)?;
                            new_envs.push(new_env);
                        }
                    }
                }
                new_envs
            }
            ComprehensionQualifier::Filter(expr) => envs
                .into_iter()
                .filter(|env| {
                    let result = p.with_interpreter(&new_p, |execution| {
                        expression(expr, Some(env), execution)
                    });
                    match result {
                        Ok(Value::Bool(v)) => v.borrow().clone(),
                        Ok(_) => false,
                        Err(_) => false,
                    }
                })
                .collect(),
            ComprehensionQualifier::Let(var_def) => envs
                .into_iter()
                .map(|mut env| -> MResult<_> {
                    let val = p.with_interpreter(&new_p, |execution| {
                        expression(&var_def.expression, Some(&env), execution)
                    })?;
                    env.insert(var_def.var.name.hash(), val);
                    Ok(env)
                })
                .collect::<MResult<Vec<_>>>()?,
        };
    }
    Ok((envs, new_p))
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn comprehension_generator_values(collection: &Value) -> MResult<Vec<Value>> {
    match collection {
        #[cfg(feature = "set")]
        Value::Set(mset) => Ok(mset.borrow().set.iter().cloned().collect()),
        #[cfg(feature = "matrix")]
        Value::MatrixIndex(matrix) => Ok(matrix
            .as_vec()
            .into_iter()
            .map(|value| Value::Index(Ref::new(value)))
            .collect()),
        #[cfg(all(feature = "matrix", feature = "bool"))]
        Value::MatrixBool(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "u8"))]
        Value::MatrixU8(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "u16"))]
        Value::MatrixU16(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "u32"))]
        Value::MatrixU32(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "u64"))]
        Value::MatrixU64(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "u128"))]
        Value::MatrixU128(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "i8"))]
        Value::MatrixI8(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "i16"))]
        Value::MatrixI16(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "i32"))]
        Value::MatrixI32(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "i64"))]
        Value::MatrixI64(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "i128"))]
        Value::MatrixI128(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "f32"))]
        Value::MatrixF32(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "f64"))]
        Value::MatrixF64(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "string"))]
        Value::MatrixString(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "rational"))]
        Value::MatrixR64(matrix) => Ok(matrix
            .as_vec()
            .into_iter()
            .map(|value| value.to_value())
            .collect()),
        #[cfg(all(feature = "matrix", feature = "complex"))]
        Value::MatrixC64(matrix) => Ok(matrix
            .as_vec()
            .into_iter()
            .map(|value| value.to_value())
            .collect()),
        #[cfg(feature = "matrix")]
        Value::MatrixValue(matrix) => Ok(matrix.as_vec()),
        Value::MutableReference(reference) => comprehension_generator_values(&reference.borrow()),
        x => Err(
            MechError::new(ComprehensionGeneratorError { found: x.kind() }, None)
                .with_compiler_loc(),
        ),
    }
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn detach_comprehension_value(value: &Value) -> Value {
    match value {
        Value::MutableReference(reference) => reference.borrow().clone(),
        _ => value.clone(),
    }
}

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
        compile_nullop!(
            "set/comprehension",
            self.out,
            ctx,
            FeatureFlag::Builtin(FeatureKind::SetComprehensions)
        );
    }
}
#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
register_descriptor! {
  FunctionDescriptor {
    name: "set/comprehension",
    ptr: ValueSetComprehension::new,
  }
}
#[cfg(feature = "set_comprehensions")]
pub struct SetComprehensionDefine {}
#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl NativeFunctionCompiler for SetComprehensionDefine {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        Ok(Box::new(ValueSetComprehension {
            arguments: arguments.clone(),
            out: Ref::new(MechSet::from_vec(arguments.clone())),
        }))
    }
}
#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
register_descriptor! {
  FunctionCompilerDescriptor {
    name: "set/comprehension",
    ptr: &SetComprehensionDefine{},
  }
}

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
            let fxn = crate::stdlib::horzcat::impl_horzcat_fxn(&args)
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
        compile_nullop!(
            "matrix/comprehension",
            self.out,
            ctx,
            FeatureFlag::Builtin(FeatureKind::MatrixComprehensions)
        );
    }
}
#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
register_descriptor! {
  FunctionDescriptor {
    name: "matrix/comprehension",
    ptr: ValueMatrixComprehension::new,
  }
}
#[cfg(feature = "matrix_comprehensions")]
pub struct MatrixComprehensionDefine {}
#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
impl NativeFunctionCompiler for MatrixComprehensionDefine {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        let out = if arguments.is_empty() {
            Value::MatrixValue(Matrix::from_vec(vec![], 0, 0))
        } else {
            let fxn = crate::stdlib::horzcat::impl_horzcat_fxn(arguments)?;
            fxn.solve();
            fxn.out()
        };
        Ok(Box::new(ValueMatrixComprehension {
            arguments: arguments.clone(),
            out: Ref::new(out),
        }))
    }
}
#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
register_descriptor! {
  FunctionCompilerDescriptor {
    name: "matrix/comprehension",
    ptr: &MatrixComprehensionDefine{},
  }
}

#[cfg(feature = "set_comprehensions")]
pub fn set_comprehension(
    set_comp: &SetComprehension,
    p: &InterpreterExecution<'_>,
) -> MResult<Value> {
    let comprehension_id = hash_str(&format!("{:?}", set_comp));
    let (envs, new_p) = comprehension_environments(&set_comp.qualifiers, comprehension_id, p)?;
    let mut values = Vec::new();
    for env in envs {
        let val = p.with_interpreter(&new_p, |execution| {
            expression(&set_comp.expression, Some(&env), execution)
        })?;
        values.push(val);
    }
    let plan = p.plan();
    execute_catalog_operation(p, &plan, "set/comprehension", values)
}

#[cfg(feature = "matrix_comprehensions")]
pub fn matrix_comprehension(
    matrix_comp: &MatrixComprehension,
    p: &InterpreterExecution<'_>,
) -> MResult<Value> {
    let comprehension_id = hash_str(&format!("{:?}", matrix_comp));
    let (envs, new_p) = comprehension_environments(&matrix_comp.qualifiers, comprehension_id, p)?;
    let mut values = Vec::new();
    for env in envs {
        values.push(p.with_interpreter(&new_p, |execution| {
            expression(&matrix_comp.expression, Some(&env), execution)
        })?);
    }
    let plan = p.plan();
    execute_catalog_operation(p, &plan, "matrix/comprehension", values)
}
