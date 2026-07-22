use crate::*;
use std::collections::HashMap;
#[cfg(feature = "enum")]
use std::collections::HashSet;

// Expressions
// ----------------------------------------------------------------------------

pub type Environment = HashMap<u64, Value>;

pub fn expression(expr: &Expression, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
    match &expr {
        #[cfg(feature = "variables")]
        Expression::Var(v) => var(v, env, p),
        #[cfg(feature = "range")]
        Expression::Range(rng) => range(&rng, env, p),
        #[cfg(all(feature = "subscript_slice", feature = "access"))]
        Expression::Slice(slc) => slice(&slc, env, p),
        #[cfg(feature = "formulas")]
        Expression::Formula(fctr) => factor(fctr, env, p),
        Expression::Structure(strct) => structure(strct, env, p),
        Expression::Literal(ltrl) => literal(&ltrl, p),
        #[cfg(feature = "functions")]
        Expression::FunctionCall(fxn_call) => function_call(fxn_call, env, p),
        #[cfg(feature = "set_comprehensions")]
        Expression::SetComprehension(set_comp) => set_comprehension(set_comp, p),
        #[cfg(feature = "matrix_comprehensions")]
        Expression::MatrixComprehension(matrix_comp) => matrix_comprehension(matrix_comp, p),
        Expression::Match(match_expr) => match_expression(match_expr, env, p),
        #[cfg(feature = "state_machines")]
        Expression::FsmPipe(fsm_pipe) => crate::state_machines::execute_fsm_pipe(fsm_pipe, env, p),
        x => Err(MechError::new(FeatureNotEnabledError, None)
            .with_compiler_loc()
            .with_tokens(x.tokens())),
    }
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
pub fn pattern_match_value(
    pattern: &Pattern,
    value: &Value,
    env: &mut Environment,
    p: &Interpreter,
) -> MResult<()> {
    match pattern {
        Pattern::Wildcard => Ok(()),
        Pattern::Expression(expr) => match expr {
            Expression::Var(var) if crate::patterns::pattern_var_is_binding(var) => {
                let id = &var.name.hash();
                match env.get(id) {
                    Some(existing) if existing == value => Ok(()),
                    Some(existing) => Err(MechError::new(
                        PatternMatchError {
                            var: var.name.to_string(),
                            expected: existing.to_string(),
                            found: value.to_string(),
                        },
                        None,
                    )
                    .with_compiler_loc()),
                    None => {
                        env.insert(id.clone(), value.clone());
                        Ok(())
                    }
                }
            }
            _ => {
                let expected = expression(expr, Some(env), p)?;
                if detach_comprehension_value(&expected) == detach_comprehension_value(value) {
                    Ok(())
                } else {
                    Err(MechError::new(
                        PatternMatchError {
                            var: "<expression>".to_string(),
                            expected: expected.to_string(),
                            found: value.to_string(),
                        },
                        None,
                    )
                    .with_compiler_loc())
                }
            }
        },
        #[cfg(feature = "tuple")]
        Pattern::Tuple(pat_tuple) => match value {
            Value::Tuple(values) => {
                let values_brrw = values.borrow();
                if pat_tuple.0.len() != values_brrw.elements.len() {
                    return Err(MechError::new(
                        ArityMismatchError {
                            expected: pat_tuple.0.len(),
                            found: values_brrw.elements.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                for (pttrn, val) in pat_tuple.0.iter().zip(values_brrw.elements.iter()) {
                    pattern_match_value(pttrn, val, env, p)?;
                }
                Ok(())
            }
            _ => Err(MechError::new(
                PatternExpectedTupleError {
                    found: value.kind(),
                },
                None,
            )
            .with_compiler_loc()),
        },
        Pattern::TupleStruct(pat_struct) => {
            todo!("Implement tuple struct pattern matching")
        }
        _ => Err(MechError::new(FeatureNotEnabledError, None).with_compiler_loc()),
    }
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn comprehension_environments(
    qualifiers: &[ComprehensionQualifier],
    comprehension_id: u64,
    p: &Interpreter,
) -> MResult<(Vec<Environment>, Interpreter)> {
    let mut envs: Vec<Environment> = vec![HashMap::new()];
    let mut new_p = p.clone();
    new_p.id = comprehension_id;
    new_p.clear_plan();
    for qual in qualifiers {
        envs = match qual {
            ComprehensionQualifier::Generator((pttrn, expr)) => {
                let mut new_envs = Vec::new();
                for env in &envs {
                    let collection = expression(expr, Some(env), &new_p)?;
                    for elmnt in comprehension_generator_values(&collection)? {
                        let mut new_env = env.clone();
                        if pattern_match_value(pttrn, &elmnt, &mut new_env, &new_p).is_ok() {
                            new_envs.push(new_env);
                        }
                    }
                }
                new_envs
            }
            ComprehensionQualifier::Filter(expr) => envs
                .into_iter()
                .filter(|env| {
                    let result = expression(expr, Some(env), &new_p);
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
                    let val = expression(&var_def.expression, Some(&env), &new_p)?;
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
}
#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl MechFunctionFactory for ValueSetComprehension {
    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Nullary(out) => {
                let out: Ref<MechSet> = unsafe { out.as_unchecked().clone() };
                Ok(Box::new(ValueSetComprehension {
                    arguments: Vec::new(),
                    out,
                }))
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
#[cfg(all(feature = "set_comprehensions", feature = "compiler"))]
impl MechFunctionCompiler for ValueSetComprehension {
    fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
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
            let fxn = MatrixHorzCat {}
                .compile(&args)
                .expect("matrix/comprehension input kinds changed to incompatible values");
            fxn.solve();
            fxn.out()
        };
        *self.out.borrow_mut() = out;
    }
    fn out(&self) -> Value {
        self.out.borrow().clone()
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
    fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
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
            let fxn = MatrixHorzCat {}.compile(arguments)?;
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
pub fn set_comprehension(set_comp: &SetComprehension, p: &Interpreter) -> MResult<Value> {
    let comprehension_id = hash_str(&format!("{:?}", set_comp));
    let (envs, new_p) = comprehension_environments(&set_comp.qualifiers, comprehension_id, p)?;
    let mut values = Vec::new();
    for env in envs {
        let val = expression(&set_comp.expression, Some(&env), &new_p)?;
        values.push(val);
    }
    let functions = p.functions();
    let set_define_id = hash_str("set/comprehension");
    let set_define = {
        functions
            .borrow()
            .function_compilers
            .get(&set_define_id)
            .cloned()
    };
    match set_define {
        Some(compiler) => execute_native_function_compiler(compiler, &values, p),
        None => Err(MechError::new(
            MissingFunctionError {
                function_id: set_define_id,
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "matrix_comprehensions")]
pub fn matrix_comprehension(matrix_comp: &MatrixComprehension, p: &Interpreter) -> MResult<Value> {
    let comprehension_id = hash_str(&format!("{:?}", matrix_comp));
    let (envs, new_p) = comprehension_environments(&matrix_comp.qualifiers, comprehension_id, p)?;
    let mut values = Vec::new();
    for env in envs {
        values.push(expression(&matrix_comp.expression, Some(&env), &new_p)?);
    }
    let functions = p.functions();
    let horzcat_id = hash_str("matrix/comprehension");
    let horzcat = {
        functions
            .borrow()
            .function_compilers
            .get(&horzcat_id)
            .cloned()
    };
    match horzcat {
        Some(compiler) => execute_native_function_compiler(compiler, &values, p),
        None => Err(MechError::new(
            MissingFunctionError {
                function_id: horzcat_id,
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "functions")]
fn register_initialized_expression_function(
  plan: &Plan,
  function: Box<dyn MechFunction>,
  arguments: &[Value],
) -> MResult<Value> {
  let node_id = plan.register_function(function, arguments)?;
  let plan_borrow = plan.borrow();
  let function = &plan_borrow[node_id];
  if !plan.activation_registration_active() { function.solve(); }
  Ok(function.out())
}

#[cfg(feature = "functions")]
fn register_expression_function_batch(
  plan: &Plan,
  functions: Vec<(Box<dyn MechFunction>, Vec<Value>)>,
) -> MResult<()> {
  for (function, arguments) in functions {
    plan.register_function(function, &arguments)?;
  }
  Ok(())
}

#[cfg(all(test, feature = "functions", feature = "f64"))]
mod indexed_expression_registration_tests {
  use super::*;
  use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

  struct IndexedExpressionTestFunction { output: Value, solve_calls: Arc<AtomicUsize> }
  impl MechFunctionImpl for IndexedExpressionTestFunction {
    fn solve(&self) { self.solve_calls.fetch_add(1, Ordering::SeqCst); }
    fn out(&self) -> Value { self.output.clone() }
    fn to_string(&self) -> String { "indexed-expression-test".to_string() }
  }
  #[cfg(feature = "compiler")]
  impl MechFunctionCompiler for IndexedExpressionTestFunction {
    fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> { Ok(0) }
  }
  fn scalar(value: f64) -> (Value, ReactiveCellId) {
    let reference = Ref::new(value); let cell = ReactiveCellId::new(reference.id());
    (Value::F64(reference), cell)
  }
  fn function(output: Value, calls: Arc<AtomicUsize>) -> Box<dyn MechFunction> {
    Box::new(IndexedExpressionTestFunction { output, solve_calls: calls })
  }
  #[test]
  fn indexed_expression_registration_records_dependencies() {
    let plan = Plan::new(); let (first, a) = scalar(1.0); let (second, b) = scalar(2.0); let (third, c) = scalar(3.0); let (output, out) = scalar(4.0); let calls = Arc::new(AtomicUsize::new(0));
    let result = register_initialized_expression_function(&plan, function(output, calls.clone()), &[first, second, third]).unwrap();
    let plan_borrow = plan.borrow(); let node = plan_borrow.node(0).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1); assert_eq!(result.reactive_cell_ids(), vec![out]);
    assert_eq!(node.inputs.iter().map(|d| d.cell).collect::<Vec<_>>(), vec![a,b,c]);
    assert!(node.inputs.iter().all(|d| d.kind == ReactiveDependencyKind::Reactive));
    for cell in [a,b,c] { assert_eq!(plan_borrow.reactive_consumers_for(cell), &[0]); assert!(plan_borrow.sampled_consumers_for(cell).is_empty()); }
    assert!(node.outputs.contains(&out)); assert!(!node.inputs.iter().any(|d| d.cell == out));
  }
  #[test]
  fn indexed_expression_registration_deduplicates_aliases() {
    let plan=Plan::new(); let (input, cell)=scalar(1.0); let (output,_)=scalar(2.0); let calls=Arc::new(AtomicUsize::new(0));
    register_initialized_expression_function(&plan, function(output,calls.clone()), &[input.clone(),input]).unwrap();
    let p=plan.borrow(); assert_eq!(calls.load(Ordering::SeqCst),1); assert_eq!(p.node(0).unwrap().inputs.len(),1); assert_eq!(p.reactive_consumers_for(cell),&[0]);
  }
  #[test]
  fn binary_term_batch_registration_preserves_order_and_edges() {
    let plan=Plan::new(); let (a,ac)=scalar(1.0); let (b,bc)=scalar(2.0); let (c,cc)=scalar(3.0); let (mid,mc)=scalar(4.0); let (final_out,fc)=scalar(5.0); let first=Arc::new(AtomicUsize::new(0)); let second=Arc::new(AtomicUsize::new(0)); let f1=function(mid.clone(),first.clone()); let f2=function(final_out,second.clone()); f1.solve(); f2.solve();
    register_expression_function_batch(&plan,vec![(f1,vec![a,b]),(f2,vec![mid,c])]).unwrap(); let p=plan.borrow();
    assert_eq!(p.len(),2); assert_eq!(p.node(0).unwrap().inputs.iter().map(|d|d.cell).collect::<Vec<_>>(),vec![ac,bc]); assert_eq!(p.node(1).unwrap().inputs.iter().map(|d|d.cell).collect::<Vec<_>>(),vec![mc,cc]); assert!(p.node(1).unwrap().outputs.contains(&fc)); assert_eq!(p.reactive_consumers_for(mc),&[1]); assert_eq!(first.load(Ordering::SeqCst),1); assert_eq!(second.load(Ordering::SeqCst),1);
  }
}

#[cfg(all(test, feature = "functions", feature = "record", feature = "tuple", feature = "f64", feature = "program", feature = "compiler"))]
mod structural_alias_access_tests {
  use super::*;

  fn symbol(interpreter: &Interpreter, name: &str) -> Value {
    interpreter.symbols().borrow().get(hash_str(name)).unwrap().borrow().clone()
  }

  fn alias_node(plan: &Plan, name: &str) -> usize {
    let plan = plan.borrow();
    (0..plan.len()).find_map(|node_id| {
      let node = plan.node(node_id).unwrap();
      node.function.to_string().contains(name).then_some(node_id)
    }).unwrap_or_else(|| panic!("missing {name} node"))
  }

  fn assert_alias_node(plan: &Plan, name: &str, output: &Value, container: &Value) {
    let output_cell = output.reactive_root_cell_ids()[0];
    let container_cell = container.reactive_root_cell_ids()[0];
    let node_id = alias_node(plan, name);
    let plan_borrow = plan.borrow();
    let node = plan_borrow.node(node_id).unwrap();
    assert!(node.inputs.is_empty());
    assert_eq!(node.outputs.as_slice(), &[output_cell]);
    assert!(!node.inputs.iter().any(|input| input.cell == container_cell));
    assert!(!plan_borrow.reactive_consumers_for(container_cell).contains(&node_id));
    assert!(!plan_borrow.sampled_consumers_for(container_cell).contains(&node_id));
  }

  #[test]
  fn record_field_access_registers_structural_node() {
    let tree = mech_syntax::parser::parse("record := {field: 2}; record.field").unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    let output = interpreter.interpret(&tree).unwrap();
    assert_eq!(*output.as_f64().unwrap().borrow(), 2.0);
    assert_alias_node(&interpreter.plan(), "RecordAccessField", &output, &symbol(&interpreter, "record"));
  }

  #[test]
  fn tuple_element_access_registers_structural_node() {
    let tree = mech_syntax::parser::parse("tuple := (1, 2); tuple.2").unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    let output = interpreter.interpret(&tree).unwrap();
    assert_eq!(*output.as_f64().unwrap().borrow(), 2.0);
    assert_alias_node(&interpreter.plan(), "TupleAccessElement", &output, &symbol(&interpreter, "tuple"));
  }

  #[test]
  fn record_field_consumer_depends_on_member_cell() {
    let tree = mech_syntax::parser::parse("record := {field: 2}; record.field + 1").unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    let output = interpreter.interpret(&tree).unwrap();
    assert_eq!(*output.as_f64().unwrap().borrow(), 3.0);
    let record = symbol(&interpreter, "record");
    let record_cell = record.reactive_root_cell_ids()[0];
    let field_cell = {
      let Value::Record(record) = record else { panic!("expected record") };
      record.borrow().get(&hash_str("field")).unwrap().reactive_root_cell_ids()[0]
    };
    let plan = interpreter.plan();
    let alias_id = alias_node(&plan, "RecordAccessField");
    assert!(plan.borrow().node(alias_id).unwrap().inputs.is_empty());
    let output_cell = output.reactive_root_cell_ids()[0];
    let plan = plan.borrow();
    let (consumer_id, consumer) = (0..plan.len()).find_map(|node_id| {
      let node = plan.node(node_id).unwrap();
      (node_id != alias_id && node.outputs.contains(&output_cell)).then_some((node_id, node))
    }).expect("missing computed field consumer");
    assert!(consumer.inputs.iter().any(|input| input.cell == field_cell));
    assert!(plan.reactive_consumers_for(field_cell).contains(&consumer_id));
    assert!(!consumer.inputs.iter().any(|input| input.cell == record_cell));
  }

  #[test]
  fn decoded_structural_alias_access_matches_source() {
    for (source, name) in [("tuple := (1, 2); tuple.2", "TupleAccessElement")] {
      let tree = mech_syntax::parser::parse(source).unwrap();
      let mut interpreter = Interpreter::new_with_full_stdlib(0);
      let source_output = interpreter.interpret(&tree).unwrap();
      {
        let source_plan = interpreter.plan();
        let source_node = alias_node(&source_plan, name);
        let source_plan = source_plan.borrow();
        let source_node = source_plan.node(source_node).unwrap();
        assert!(source_node.inputs.is_empty());
        assert_eq!(source_node.outputs.as_slice(), &source_output.reactive_root_cell_ids());
      }
      let bytecode = interpreter.compile().unwrap();
      let program = ParsedProgram::from_bytes(&bytecode).unwrap();
      interpreter.clear_plan();
      let decoded_output = interpreter.run_program(&program).unwrap();
      assert_eq!(decoded_output, source_output);
      let decoded_node = alias_node(&interpreter.plan(), name);
      let decoded_plan = interpreter.plan();
      let decoded_plan = decoded_plan.borrow();
      let decoded_node = decoded_plan.node(decoded_node).unwrap();
      assert!(decoded_node.inputs.is_empty());
      assert_eq!(decoded_node.outputs.as_slice(), &decoded_output.reactive_root_cell_ids());
    }
  }
}

#[cfg(all(test, feature = "functions", feature = "f64", feature = "u64", feature = "convert", feature = "kind_annotation", feature = "variable_define", feature = "variables"))]
mod variable_kind_cast_dependency_tests {
  use super::*;

  #[test]
  fn variable_kind_cast_is_indexed() {
    let tree = mech_syntax::parser::parse("value := 1; value<f64>").unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    let output = interpreter.interpret(&tree).unwrap();
    assert_eq!(*output.as_f64().unwrap().borrow(), 1.0);
    let output_cell = output.reactive_root_cell_ids()[0];
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let (node_id, node) = (0..plan.len()).find_map(|node_id| {
      let node = plan.node(node_id).unwrap();
      (node.outputs.contains(&output_cell) && !node.inputs.is_empty()).then_some((node_id, node))
    }).expect("converted variable read should be registered in the plan");
    assert!(node.inputs.iter().all(|dependency| dependency.kind == ReactiveDependencyKind::Reactive));
    assert!(node.outputs.contains(&output_cell));
    assert!(!node.inputs.iter().any(|dependency| dependency.cell == output_cell));
    for dependency in &node.inputs {
      assert!(plan.reactive_consumers_for(dependency.cell).contains(&node_id));
      assert!(plan.sampled_consumers_for(dependency.cell).is_empty());
    }
  }
}

#[cfg(feature = "range")]
pub fn range(rng: &RangeExpression, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let start = factor(&rng.start, env, p)?;
  let terminal = factor(&rng.terminal, env, p)?;
  let (function, arguments) = match &rng.increment {
    Some((_, increment)) => {
      let step = factor(increment, env, p)?;
      let arguments = vec![start, step, terminal];
      let function = match &rng.operator {
        #[cfg(feature = "range_exclusive")]
        RangeOp::Exclusive => RangeIncrementExclusive {}.compile(&arguments)?,
        #[cfg(feature = "range_inclusive")]
        RangeOp::Inclusive => RangeIncrementInclusive {}.compile(&arguments)?,
        _ => unreachable!(),
      };
      (function, arguments)
    }
    None => {
      let arguments = vec![start, terminal];
      let function = match &rng.operator {
        #[cfg(feature = "range_exclusive")]
        RangeOp::Exclusive => RangeExclusive {}.compile(&arguments)?,
        #[cfg(feature = "range_inclusive")]
        RangeOp::Inclusive => RangeInclusive {}.compile(&arguments)?,
        _ => unreachable!(),
      };
      (function, arguments)
    }
  };
  register_initialized_expression_function(&plan, function, &arguments)
}

fn addressed_identifier_name(name: &Identifier, context: &Option<Identifier>) -> String {
    match context {
        Some(context) => format!("@{}/{}", context.to_string(), name.to_string()),
        None => name.to_string(),
    }
}

fn addressed_identifier_hash(name: &Identifier, context: &Option<Identifier>) -> u64 {
    match context {
        Some(_) => hash_str(&addressed_identifier_name(name, context)),
        None => name.hash(),
    }
}

#[cfg(all(feature = "subscript_slice", feature = "access"))]
pub fn slice(slc: &Slice, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
    let id = addressed_identifier_hash(&slc.name, &slc.context);
    let name = addressed_identifier_name(&slc.name, &slc.context);
    let val: Value = if let Some(env) = env {
        if let Some(val) = env.get(&id) {
            val.clone()
        } else {
            // fallback to global symbols
            {
                let symbols = p.symbols();
                let symbols_brrw = symbols.borrow();
                match symbols_brrw.get(id) {
                    Some(val) => match symbols_brrw.get_mutable(id) {
                        Some(_) => Value::MutableReference(val.clone()),
                        None => val.borrow().clone(),
                    },
                    None => {
                        return Err(MechError::new(UndefinedVariableError { id, name: name.clone() }, None)
                            .with_compiler_loc()
                            .with_tokens(slc.tokens()));
                    }
                }
            }
        }
    } else {
        let symbols = p.symbols();
        let symbols_brrw = symbols.borrow();
        match symbols_brrw.get(id) {
            Some(val) => match symbols_brrw.get_mutable(id) {
                Some(_) => Value::MutableReference(val.clone()),
                None => val.borrow().clone(),
            },
            None => {
                return Err(MechError::new(UndefinedVariableError { id, name: name.clone() }, None)
                    .with_compiler_loc()
                    .with_tokens(slc.tokens()));
            }
        }
    };
    let mut v = val;
    for s in &slc.subscript {
        v = subscript(s, &v, env, p)?;
    }
    Ok(v)
}

#[cfg(feature = "subscript_formula")]
pub fn subscript_formula(
    sbscrpt: &Subscript,
    env: Option<&Environment>,
    p: &Interpreter,
) -> MResult<Value> {
    match sbscrpt {
        Subscript::Formula(fctr) => factor(fctr, env, p),
        _ => unreachable!(),
    }
}

#[cfg(feature = "subscript_formula")]
pub fn subscript_formula_ix(
    sbscrpt: &Subscript,
    env: Option<&Environment>,
    p: &Interpreter,
) -> MResult<Value> {
    match sbscrpt {
        Subscript::Formula(fctr) => {
            let result = factor(fctr, env, p)?;
            result.as_index()
        }
        _ => unreachable!(),
    }
}


#[cfg(feature = "subscript_formula")]
pub(crate) fn reset_current_string_access_expression_live(p: &Interpreter) {
    *p.current_string_access_expression_live.borrow_mut() = false;
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn current_string_access_expression_live(p: &Interpreter) -> bool {
    *p.current_string_access_expression_live.borrow()
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn take_current_string_access_expression_live(p: &Interpreter) -> bool {
    let value = *p.current_string_access_expression_live.borrow();
    *p.current_string_access_expression_live.borrow_mut() = false;
    value
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn mark_current_string_access_expression_live(p: &Interpreter) {
    *p.current_string_access_expression_live.borrow_mut() = true;
}

#[cfg(feature = "subscript_formula")]
fn string_access_scalar_addr(value: &Value) -> Option<usize> {
    match value {
        Value::MutableReference(reference) => string_access_scalar_addr(&reference.borrow()),
        Value::Typed(value, _) => string_access_scalar_addr(value),
        Value::String(value) => Some(value.addr()),
        Value::Index(value) => Some(value.addr()),

        #[cfg(feature = "u8")]
        Value::U8(value) => Some(value.addr()),
        #[cfg(feature = "u16")]
        Value::U16(value) => Some(value.addr()),
        #[cfg(feature = "u32")]
        Value::U32(value) => Some(value.addr()),
        #[cfg(feature = "u64")]
        Value::U64(value) => Some(value.addr()),
        #[cfg(feature = "u128")]
        Value::U128(value) => Some(value.addr()),

        #[cfg(feature = "i8")]
        Value::I8(value) => Some(value.addr()),
        #[cfg(feature = "i16")]
        Value::I16(value) => Some(value.addr()),
        #[cfg(feature = "i32")]
        Value::I32(value) => Some(value.addr()),
        #[cfg(feature = "i64")]
        Value::I64(value) => Some(value.addr()),
        #[cfg(feature = "i128")]
        Value::I128(value) => Some(value.addr()),

        #[cfg(feature = "f32")]
        Value::F32(value) => Some(value.addr()),
        #[cfg(feature = "f64")]
        Value::F64(value) => Some(value.addr()),

        _ => None,
    }
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn mark_string_access_value_live(p: &Interpreter, value: &Value) {
    if let Some(addr) = string_access_scalar_addr(value) {
        p.string_access_live_values.borrow_mut().insert(addr);
    }
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn string_access_value_is_marked_live(p: &Interpreter, value: &Value) -> bool {
    string_access_scalar_addr(value)
        .map(|addr| p.string_access_live_values.borrow().contains(&addr))
        .unwrap_or(false)
}

#[cfg(feature = "subscript_formula")]
fn subscript_formula_is_mutable_symbol(
    sbscrpt: &Subscript,
    env: Option<&Environment>,
    p: &Interpreter,
) -> bool {
    if env.is_some() {
        return false;
    }
    let Subscript::Formula(fctr) = sbscrpt else {
        return false;
    };
    let Factor::Expression(expr) = fctr else {
        return false;
    };
    let Expression::Var(var) = expr.as_ref() else {
        return false;
    };
    let id = addressed_identifier_hash(&var.name, &var.context);
    let state_brrw = p.state.borrow();
    let symbols_brrw = state_brrw.symbol_table.borrow();
    symbols_brrw.get_mutable(id).is_some()
}

#[cfg(feature = "subscript_formula")]
fn mutable_reference_is_mutable_symbol(reference: &MutableReference, p: &Interpreter) -> bool {
    let state_brrw = p.state.borrow();
    let symbols_brrw = state_brrw.symbol_table.borrow();
    symbols_brrw
        .mutable_variables
        .values()
        .any(|symbol| std::rc::Rc::ptr_eq(&symbol.0, &reference.0))
}

#[cfg(feature = "subscript_formula")]
fn value_is_mutable_symbol_reference(value: &Value, p: &Interpreter) -> bool {
    match value {
        Value::MutableReference(reference) => mutable_reference_is_mutable_symbol(reference, p),
        _ => false,
    }
}

#[cfg(feature = "subscript_formula")]
fn mutable_reference_is_live_plan_output(reference: &MutableReference, p: &Interpreter) -> bool {
    let current = reference.borrow();
    string_access_value_is_marked_live(p, &current)
}

#[cfg(feature = "subscript_formula")]
fn string_access_argument_is_live(value: &Value, p: &Interpreter) -> bool {
    string_access_value_is_marked_live(p, value)
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn string_access_input_is_live(value: &Value, p: &Interpreter) -> bool {
    value_is_mutable_symbol_reference(value, p) || string_access_argument_is_live(value, p)
}

#[cfg(feature = "subscript_formula")]
fn string_access_source_argument(value: &Value, p: &Interpreter) -> Value {
    match value {
        Value::MutableReference(reference)
            if matches!(value.deref_kind(), ValueKind::String)
                && !mutable_reference_is_mutable_symbol(reference, p)
                && !mutable_reference_is_live_plan_output(reference, p) =>
        {
            reference.borrow().clone()
        }
        _ => value.clone(),
    }
}

#[cfg(feature = "subscript_formula")]
fn string_access_index_argument(
    raw_index: Value,
    sbscrpt: &Subscript,
    env: Option<&Environment>,
    p: &Interpreter,
) -> MResult<Value> {
    match &raw_index {
        Value::MutableReference(reference)
            if subscript_formula_is_mutable_symbol(sbscrpt, env, p)
                || mutable_reference_is_live_plan_output(reference, p) =>
        {
            reference.borrow().as_index()?;
            Ok(raw_index)
        }
        _ => raw_index.as_index(),
    }
}

#[cfg(feature = "subscript_range")]
pub fn subscript_range(
    sbscrpt: &Subscript,
    env: Option<&Environment>,
    p: &Interpreter,
) -> MResult<Value> {
    match sbscrpt {
        Subscript::Range(rng) => {
            let result = range(rng, env, p)?;
            match result.as_vecusize() {
                Ok(v) => Ok(v.to_value()),
                Err(_) => Err(MechError::new(
                    InvalidIndexKindError {
                        kind: result.kind(),
                    },
                    None,
                )
                .with_compiler_loc()
                .with_tokens(rng.tokens())),
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(all(feature = "subscript", feature = "access"))]
pub fn subscript(
    sbscrpt: &Subscript,
    val: &Value,
    env: Option<&Environment>,
    p: &Interpreter,
) -> MResult<Value> {
    let plan = p.plan();
    match sbscrpt {
        #[cfg(feature = "table")]
        Subscript::Dot(x) => {
            let key = x.hash();
            let fxn_input: Vec<Value> = vec![val.clone(), Value::Id(key)];
            #[cfg(feature = "record")]
            if matches!(val.deref_kind(), ValueKind::Record(..)) {
                let function = AccessColumn {}.compile(&fxn_input)?;
                return register_initialized_expression_function(&plan, function, &[]);
            }
            let new_fxn = AccessColumn {}.compile(&fxn_input)?;
            new_fxn.solve();
            let res = new_fxn.out();
            plan.borrow_mut().push(new_fxn);
            return Ok(res);
        }
        Subscript::DotInt(x) => {
            let mut fxn_input = vec![val.clone()];
            let result = real(&x.clone(), p)?;
            fxn_input.push(result.as_index()?);
            match val.deref_kind() {
                #[cfg(feature = "matrix")]
                ValueKind::Matrix(..) => {
                    let new_fxn = MatrixAccessScalar {}.compile(&fxn_input)?;
                    new_fxn.solve();
                    let res = new_fxn.out();
                    plan.borrow_mut().push(new_fxn);
                    return Ok(res);
                }
                #[cfg(feature = "tuple")]
                ValueKind::Tuple(..) => {
                    let function = TupleAccess {}.compile(&fxn_input)?;
                    return register_initialized_expression_function(&plan, function, &[]);
                }
                /*ValueKind::Record(_) => {
                  let new_fxn = RecordAccessScalar{}.compile(&fxn_input)?;
                  new_fxn.solve();
                  let res = new_fxn.out();
                  plan.borrow_mut().push(new_fxn);
                  return Ok(res);
                },*/
                _ => todo!("Implement access for dot int"),
            }
        }
        #[cfg(feature = "swizzle")]
        Subscript::Swizzle(x) => {
            let mut keys = x
                .iter()
                .map(|x| Value::Id(x.hash()))
                .collect::<Vec<Value>>();
            let mut fxn_input: Vec<Value> = vec![val.clone()];
            fxn_input.append(&mut keys);
            let new_fxn = AccessSwizzle {}.compile(&fxn_input)?;
            new_fxn.solve();
            let res = new_fxn.out();
            plan.borrow_mut().push(new_fxn);
            return Ok(res);
        }
        Subscript::Brace(subs) => {
            let mut fxn_input = vec![val.clone()];
            match &subs[..] {
                #[cfg(feature = "subscript_formula")]
                [Subscript::Formula(ix)] => {
                    let result = subscript_formula(&subs[0], env, p)?;
                    let shape = result.shape();
                    fxn_input.push(result);
                    match shape[..] {
                        [1, 1] => { plan.borrow_mut().push(AccessScalar {}.compile(&fxn_input)?); }
                        #[cfg(feature = "subscript_range")]
                        [n, 1] => { plan.borrow_mut().push(AccessRange {}.compile(&fxn_input)?); }
                        #[cfg(feature = "subscript_range")]
                        [1, n] => { plan.borrow_mut().push(AccessRange {}.compile(&fxn_input)?); }
                        _ => todo!(),
                    }
                }
                #[cfg(feature = "subscript_range")]
                [Subscript::Range(ix)] => {
                    let result = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(result);
                    plan.borrow_mut().push(AccessRange {}.compile(&fxn_input)?);
                }
                /*[Subscript::All] => {
                  fxn_input.push(Value::IndexAll);
                  #[cfg(feature = "matrix")]
                  plan.borrow_mut().push(MapAccessAll{}.compile(&fxn_input)?);
                },*/
                _ => {
                    todo!("Implement brace subscript")
                }
            }
            let plan_brrw = plan.borrow();
            let mut new_fxn = &plan_brrw.last().unwrap();
            new_fxn.solve();
            let res = new_fxn.out();
            return Ok(res);
        }
        #[cfg(feature = "subscript_slice")]
        Subscript::Bracket(subs) => {
            let string_source_is_live = matches!(val.deref_kind(), ValueKind::String)
                && string_access_argument_is_live(val, p);
            let mut fxn_input = if matches!(val.deref_kind(), ValueKind::String) {
                vec![string_access_source_argument(val, p)]
            } else {
                vec![val.clone()]
            };
            match &subs[..] {
                #[cfg(feature = "subscript_formula")]
                [Subscript::Formula(ix)] => {
                    let raw_index = subscript_formula(&subs[0], env, p)?;
                    let index_arg = if matches!(val.deref_kind(), ValueKind::String) {
                        string_access_index_argument(raw_index, &subs[0], env, p)?
                    } else {
                        raw_index.as_index()?
                    };
                    if matches!(val.deref_kind(), ValueKind::String)
                        && matches!(fxn_input.first(), Some(Value::String(_)))
                        && matches!(&index_arg, Value::Index(_))
                    {
                        let mode = if current_string_access_expression_live(p)
                            || string_source_is_live
                            || string_access_argument_is_live(&index_arg, p)
                        {
                            StringAccessCompileMode::LiveDirect
                        } else {
                            StringAccessCompileMode::Constant
                        };
                        set_next_string_access_compile_mode(mode);
                    }
                    let shape = index_arg.shape();
                    fxn_input.push(index_arg);
                    match shape[..] {
                        [1, 1] => { plan.borrow_mut().push(AccessScalar {}.compile(&fxn_input)?); }
                        #[cfg(feature = "subscript_range")]
                        [1, n] => { plan.borrow_mut().push(AccessRange {}.compile(&fxn_input)?); }
                        #[cfg(feature = "subscript_range")]
                        [n, 1] => { plan.borrow_mut().push(AccessRange {}.compile(&fxn_input)?); }
                        _ => todo!(),
                    }
                }
                #[cfg(feature = "subscript_range")]
                [Subscript::Range(ix)] => {
                    let result = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(result);
                    plan.borrow_mut().push(AccessRange {}.compile(&fxn_input)?);
                }
                [Subscript::All] => {
                    fxn_input.push(Value::IndexAll);
                    #[cfg(feature = "matrix")]
                    plan.borrow_mut()
                        .push(MatrixAccessAll {}.compile(&fxn_input)?);
                }
                [Subscript::All, Subscript::All] => todo!(),
                #[cfg(feature = "subscript_formula")]
                [Subscript::Formula(ix1), Subscript::Formula(ix2)] => {
                    let result = subscript_formula_ix(&subs[0], env, p)?;
                    let shape1 = result.shape();
                    fxn_input.push(result);
                    let result = subscript_formula_ix(&subs[1], env, p)?;
                    let shape2 = result.shape();
                    fxn_input.push(result);
                    match ((shape1[0], shape1[1]), (shape2[0], shape2[1])) {
                        #[cfg(feature = "matrix")]
                        ((1, 1), (1, 1)) => { plan.borrow_mut().push(MatrixAccessScalarScalar {}.compile(&fxn_input)?); }
                        #[cfg(feature = "matrix")]
                        ((1, 1), (m, 1)) => { plan.borrow_mut().push(MatrixAccessScalarRange {}.compile(&fxn_input)?); }
                        #[cfg(feature = "matrix")]
                        ((n, 1), (1, 1)) => { plan.borrow_mut().push(MatrixAccessRangeScalar {}.compile(&fxn_input)?); }
                        #[cfg(feature = "matrix")]
                        ((n, 1), (m, 1)) => { plan.borrow_mut().push(MatrixAccessRangeRange {}.compile(&fxn_input)?); }
                        _ => unreachable!(),
                    }
                }
                #[cfg(feature = "subscript_range")]
                [Subscript::Range(ix1), Subscript::Range(ix2)] => {
                    let result = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(result);
                    let result = subscript_range(&subs[1], env, p)?;
                    fxn_input.push(result);
                    #[cfg(feature = "matrix")]
                    plan.borrow_mut()
                        .push(MatrixAccessRangeRange {}.compile(&fxn_input)?);
                }
                #[cfg(all(feature = "subscript_range", feature = "subscript_formula"))]
                [Subscript::All, Subscript::Formula(ix2)] => {
                    fxn_input.push(Value::IndexAll);
                    let result = subscript_formula_ix(&subs[1], env, p)?;
                    let shape = result.shape();
                    fxn_input.push(result);
                    match &shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => { plan.borrow_mut().push(MatrixAccessAllScalar {}.compile(&fxn_input)?); }
                        #[cfg(feature = "matrix")]
                        [1, n] => { plan.borrow_mut().push(MatrixAccessAllRange {}.compile(&fxn_input)?); }
                        #[cfg(feature = "matrix")]
                        [n, 1] => { plan.borrow_mut().push(MatrixAccessAllRange {}.compile(&fxn_input)?); }
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "subscript_range", feature = "subscript_formula"))]
                [Subscript::Formula(ix1), Subscript::All] => {
                    let result = subscript_formula_ix(&subs[0], env, p)?;
                    let shape = result.shape();
                    fxn_input.push(result);
                    fxn_input.push(Value::IndexAll);
                    match &shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => { plan.borrow_mut().push(MatrixAccessScalarAll {}.compile(&fxn_input)?); }
                        #[cfg(feature = "matrix")]
                        [1, n] => { plan.borrow_mut().push(MatrixAccessRangeAll {}.compile(&fxn_input)?); }
                        #[cfg(feature = "matrix")]
                        [n, 1] => { plan.borrow_mut().push(MatrixAccessRangeAll {}.compile(&fxn_input)?); }
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "subscript_range", feature = "subscript_formula"))]
                [Subscript::Range(ix1), Subscript::Formula(ix2)] => {
                    let result = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(result);
                    let result = subscript_formula_ix(&subs[1], env, p)?;
                    let shape = result.shape();
                    fxn_input.push(result);
                    match &shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => { plan.borrow_mut().push(MatrixAccessRangeScalar {}.compile(&fxn_input)?); }
                        #[cfg(feature = "matrix")]
                        [1, n] => { plan.borrow_mut().push(MatrixAccessRangeRange {}.compile(&fxn_input)?); }
                        #[cfg(feature = "matrix")]
                        [n, 1] => { plan.borrow_mut().push(MatrixAccessRangeRange {}.compile(&fxn_input)?); }
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "subscript_range", feature = "subscript_formula"))]
                [Subscript::Formula(ix1), Subscript::Range(ix2)] => {
                    let result = subscript_formula_ix(&subs[0], env, p)?;
                    let shape = result.shape();
                    fxn_input.push(result);
                    let result = subscript_range(&subs[1], env, p)?;
                    fxn_input.push(result);
                    match &shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => { plan.borrow_mut().push(MatrixAccessScalarRange {}.compile(&fxn_input)?); }
                        #[cfg(feature = "matrix")]
                        [1, n] => { plan.borrow_mut().push(MatrixAccessRangeRange {}.compile(&fxn_input)?); }
                        #[cfg(feature = "matrix")]
                        [n, 1] => { plan.borrow_mut().push(MatrixAccessRangeRange {}.compile(&fxn_input)?); }
                        _ => todo!(),
                    }
                }
                #[cfg(feature = "subscript_range")]
                [Subscript::All, Subscript::Range(ix2)] => {
                    fxn_input.push(Value::IndexAll);
                    let result = subscript_range(&subs[1], env, p)?;
                    fxn_input.push(result);
                    #[cfg(feature = "matrix")]
                    plan.borrow_mut()
                        .push(MatrixAccessAllRange {}.compile(&fxn_input)?);
                }
                #[cfg(feature = "subscript_range")]
                [Subscript::Range(ix1), Subscript::All] => {
                    let result = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(result);
                    fxn_input.push(Value::IndexAll);
                    #[cfg(feature = "matrix")]
                    plan.borrow_mut()
                        .push(MatrixAccessRangeAll {}.compile(&fxn_input)?);
                }
                _ => unreachable!(),
            };
            let plan_brrw = plan.borrow();
            let mut new_fxn = &plan_brrw.last().unwrap();
            new_fxn.solve();
            let res = new_fxn.out();
            return Ok(res);
        }
        _ => unreachable!(),
    }
}

#[cfg(feature = "symbol_table")]
pub fn var(v: &Var, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
    let plan = p.plan();
    let maybe_cast_to_kind = |value: Value| -> MResult<Value> {
        match &v.kind {
            Some(kind_anntn) => {
                let target_kind = {
                    let state_brrw = p.state.borrow();
                    kind_annotation(&kind_anntn.kind, p)?.to_value_kind(&state_brrw.kinds)?
                };
                execute_initialized_indexed_compiler(
                    &plan,
                    &ConvertKind {},
                    vec![value, Value::Kind(target_kind)],
                )
            }
            None => Ok(value),
        }
    };

    let id = addressed_identifier_hash(&v.name, &v.context);
    let name = addressed_identifier_name(&v.name, &v.context);
    let mark_if_live_symbol = |value: &MutableReference| {
        #[cfg(feature = "subscript_formula")]
        {
            let state_brrw = p.state.borrow();
            let symbols_brrw = state_brrw.symbol_table.borrow();
            if symbols_brrw.get_mutable(id).is_some() || string_access_value_is_marked_live(p, &value.borrow()) {
                mark_current_string_access_expression_live(p);
            }
        }
        #[cfg(not(feature = "subscript_formula"))]
        {
            let _ = value;
        }
    };
    match env {
        Some(env) => match env.get(&id) {
            Some(value) => maybe_cast_to_kind(value.clone()),
            None => {
                let state_brrw = p.state.borrow();
                let symbols_brrw = state_brrw.symbol_table.borrow();
                let symbol_value = symbols_brrw.get(id);
                drop(symbols_brrw);
                drop(state_brrw);
                match symbol_value {
                    Some(value) => {
                        mark_if_live_symbol(&value);
                        maybe_cast_to_kind(Value::MutableReference(value))
                    },
                    None => Err(MechError::new(UndefinedVariableError { id, name: name.clone() }, None)
                        .with_compiler_loc()
                        .with_tokens(v.tokens())),
                }
            }
        },
        None => {
            let state_brrw = p.state.borrow();
            let symbols_brrw = state_brrw.symbol_table.borrow();
            let symbol_value = symbols_brrw.get(id);
            drop(symbols_brrw);
            drop(state_brrw);
            match symbol_value {
                Some(value) => {
                    mark_if_live_symbol(&value);
                    maybe_cast_to_kind(Value::MutableReference(value))
                },
                None => Err(MechError::new(UndefinedVariableError { id, name: name.clone() }, None)
                    .with_compiler_loc()
                    .with_tokens(v.tokens())),
            }
        }
    }
}

pub fn match_expression(
    match_expr: &MatchExpression,
    env: Option<&Environment>,
    p: &Interpreter,
) -> MResult<Value> {
    let source = expression(&match_expr.source, env, p)?;
    let detached_source = match &source {
        Value::MutableReference(reference) => reference.borrow().clone(),
        _ => source.clone(),
    };
    let mut base_env = env.cloned().unwrap_or_default();
    if let Expression::Var(var) = &match_expr.source {
        base_env.insert(var.name.hash(), detached_source.clone());
    }
    if !match_expr
        .arms
        .iter()
        .any(|arm| matches!(arm.pattern, Pattern::Wildcard))
    {
        #[cfg(feature = "enum")]
        if let Some((enum_name, missing_patterns)) =
            infer_missing_enum_match_patterns(match_expr, &detached_source, p)
        {
            if missing_patterns.is_empty() {
                // Exhaustive enum matches do not require a wildcard arm.
                validate_match_arm_output_kinds(match_expr, &base_env, p)?;
            } else {
                return Err(MechError::new(
                    MatchNonExhaustiveVariantsError {
                        enum_name,
                        missing_patterns,
                    },
                    None,
                )
                .with_compiler_loc()
                .with_tokens(match_expr.source.tokens()));
            }
        } else {
            return Err(MechError::new(MatchNonExhaustiveError, None)
                .with_compiler_loc()
                .with_tokens(match_expr.source.tokens()));
        }
    }
    if value_contains_empty(&detached_source) && !has_identity_wildcard_coalesce_arms(match_expr) {
        if let Some(arm) = match_expr
            .arms
            .iter()
            .find(|arm| matches!(arm.pattern, Pattern::Wildcard))
        {
            let passed_guard = match &arm.guard {
                Some(guard) => guard_expression_true(guard, &base_env, p)?,
                None => true,
            };
            if passed_guard {
                return expression(&arm.expression, Some(&base_env), p);
            }
        }
    }

    for (arm_ix, arm) in match_expr.arms.iter().enumerate() {
        let mut guard_env = base_env.clone();
        let matched = match &arm.pattern {
            Pattern::Wildcard => true,
            _ => crate::patterns::pattern_matches_value(
                &arm.pattern,
                &detached_source,
                &mut guard_env,
                p,
            )?,
        };
        if !matched {
            continue;
        }
        let passed_guard = match &arm.guard {
            Some(guard) => guard_expression_true(guard, &guard_env, p)?,
            None => true,
        };
        if passed_guard {
            #[cfg(feature = "matrix")]
            if value_contains_empty(&detached_source) && is_identity_option_matrix_arm(arm) {
                if let Some(wildcard_arm) = match_expr
                    .arms
                    .iter()
                    .find(|arm| matches!(arm.pattern, Pattern::Wildcard))
                {
                    let wildcard_passed = match &wildcard_arm.guard {
                        Some(guard) => guard_expression_true(guard, &base_env, p)?,
                        None => true,
                    };
                    if wildcard_passed {
                        let fallback = expression(&wildcard_arm.expression, Some(&base_env), p)?;
                        let coalesced =
                            coalesce_option_matrix_with_fallback(&detached_source, &fallback)?;
                        return Ok(coalesced);
                    }
                }
            }
            let output = expression(&arm.expression, Some(&guard_env), p)?;
            match_validate_arm_kinds(
                match_expr,
                arm_ix,
                &output.kind(),
                &detached_source,
                &base_env,
                p,
            )?;
            return Ok(output);
        }
    }

    Err(MechError::new(MatchNoArmMatchedError, None)
        .with_compiler_loc()
        .with_tokens(match_expr.source.tokens()))
}

#[cfg(feature = "enum")]
fn infer_missing_enum_match_patterns(
    match_expr: &MatchExpression,
    source: &Value,
    p: &Interpreter,
) -> Option<(String, Vec<String>)> {
    let (source_enum_id, source_tag) = match source {
        Value::Enum(enum_value) => {
            let enum_brrw = enum_value.borrow();
            if enum_brrw.variants.len() != 1 {
                (Some(enum_brrw.id), None)
            } else {
                (Some(enum_brrw.id), Some(enum_brrw.variants[0].0))
            }
        }
        Value::Atom(atom) => (None, Some(atom.borrow().id())),
        #[cfg(feature = "tuple")]
        Value::Tuple(tuple_val) => {
            let tuple_brrw = tuple_val.borrow();
            match tuple_brrw.elements.first() {
                Some(tag) => match tag.as_ref() {
                    Value::Atom(atom) => (None, Some(atom.borrow().id())),
                    _ => (None, None),
                },
                None => (None, None),
            }
        }
        _ => (None, None),
    };
    let source_tag = source_tag?;

    let mut arm_tags: HashSet<u64> = HashSet::new();
    for arm in &match_expr.arms {
        match &arm.pattern {
            Pattern::Expression(Expression::Literal(Literal::Atom(atom))) => {
                arm_tags.insert(atom.name.hash());
            }
            #[cfg(feature = "atom")]
            Pattern::TupleStruct(pattern_tuple_struct) => {
                arm_tags.insert(pattern_tuple_struct.name.hash());
            }
            _ => {}
        }
    }
    if arm_tags.is_empty() {
        return None;
    }

    let state_brrw = p.state.borrow();
    let enum_def = if let Some(enum_id) = source_enum_id {
        state_brrw.enums.get(&enum_id)?
    } else {
        let candidates: Vec<&MechEnum> = state_brrw
            .enums
            .values()
            .filter(|enm| {
                let variant_ids: HashSet<u64> = enm.variants.iter().map(|(id, _)| *id).collect();
                variant_ids.contains(&source_tag) && arm_tags.is_subset(&variant_ids)
            })
            .collect();
        if candidates.len() != 1 {
            return None;
        }
        candidates[0]
    };
    let variant_ids: HashSet<u64> = enum_def.variants.iter().map(|(id, _)| *id).collect();
    let missing_ids: Vec<u64> = variant_ids.difference(&arm_tags).cloned().collect();
    let names_brrw = enum_def.names.borrow();
    let missing_patterns = enum_def
        .variants
        .iter()
        .filter(|(id, _)| missing_ids.contains(id))
        .map(|(id, payload_kind)| {
            let variant_name = names_brrw
                .get(id)
                .cloned()
                .unwrap_or_else(|| id.to_string());
            if payload_kind.is_some() {
                format!(":{}(…)", variant_name)
            } else {
                format!(":{}", variant_name)
            }
        })
        .collect::<Vec<String>>();
    Some((enum_def.name(), missing_patterns))
}

fn match_validate_arm_kinds(
    match_expr: &MatchExpression,
    matched_arm_ix: usize,
    matched_kind: &ValueKind,
    source: &Value,
    base_env: &Environment,
    p: &Interpreter,
) -> MResult<()> {
    for (ix, arm) in match_expr.arms.iter().enumerate() {
        if ix == matched_arm_ix {
            continue;
        }
        if matches!(arm.pattern, Pattern::Wildcard) {
            continue;
        }
        let mut arm_env = base_env.clone();
        let applicable = match arm.pattern {
            Pattern::Wildcard => true,
            _ => crate::patterns::pattern_matches_value(
                &arm.pattern,
                source,
                &mut arm_env,
                p,
            )?,
        };
        if !applicable {
            continue;
        }
        let passed_guard = match &arm.guard {
            Some(guard) => guard_expression_true(guard, &arm_env, p)?,
            None => true,
        };
        if !passed_guard {
            continue;
        }
        let arm_value = expression(&arm.expression, Some(&arm_env), p)?;
        let arm_kind = arm_value.kind();
        if arm_kind != *matched_kind {
            return Err(MechError::new(
                MatchArmKindMismatchError {
                    expected: matched_kind.clone(),
                    found: arm_kind,
                },
                None,
            )
            .with_compiler_loc()
            .with_tokens(arm.expression.tokens()));
        }
    }
    Ok(())
}

fn validate_match_arm_output_kinds(
    match_expr: &MatchExpression,
    env: &Environment,
    p: &Interpreter,
) -> MResult<()> {
    let mut expected: Option<ValueKind> = None;
    for arm in &match_expr.arms {
        let arm_kind = match expression(&arm.expression, Some(env), p) {
            Ok(value) => value.kind(),
            Err(_) => continue,
        };
        if let Some(expected_kind) = &expected {
            if *expected_kind != arm_kind {
                return Err(MechError::new(
                    MatchArmKindMismatchError {
                        expected: expected_kind.clone(),
                        found: arm_kind,
                    },
                    None,
                )
                .with_compiler_loc()
                .with_tokens(arm.expression.tokens()));
            }
        } else {
            expected = Some(arm_kind);
        }
    }
    Ok(())
}

fn guard_expression_true(guard: &Expression, env: &Environment, p: &Interpreter) -> MResult<bool> {
  let guard_result = expression(guard, Some(env), p)?;
  match guard_result {
      #[cfg(feature = "bool")]
    Value::Bool(flag) => Ok(*flag.borrow()),
    _ => Err(MechError::new(
      InvalidGuardExpressionError {
        found: guard_result.kind(),
      },
      None,
    )
    .with_compiler_loc()
    .with_tokens(guard.tokens())),
  }
}

fn is_identity_option_matrix_arm(arm: &MatchArm) -> bool {
  match (&arm.pattern, &arm.expression) {
      (Pattern::Expression(Expression::Var(pattern_var)), Expression::Var(expr_var)) => {
        pattern_var.name.hash() == expr_var.name.hash()
      }
      _ => false,
  }
}

fn has_identity_wildcard_coalesce_arms(match_expr: &MatchExpression) -> bool {
  let has_identity = match_expr.arms.iter().any(is_identity_option_matrix_arm);
  let has_wildcard = match_expr
      .arms
      .iter()
      .any(|arm| matches!(arm.pattern, Pattern::Wildcard));
  has_identity && has_wildcard
}

#[cfg(feature = "matrix")]
fn coalesce_option_matrix_with_fallback(source: &Value, fallback: &Value) -> MResult<Value> {
  let source_kind = source.kind();
  if let ValueKind::Option(inner_kind) = source_kind.clone() {
    let raw = match source {
        Value::Typed(inner, _) => inner.as_ref().clone(),
        value => value.clone(),
    };
    let candidate = match raw {
        Value::Empty | Value::EmptyKind(_) => fallback.clone(),
        value => value,
    };
    return candidate.convert_to(inner_kind.as_ref()).ok_or_else(|| {
        MechError::new(
            CannotConvertToTypeError {
                target_type: "requested type",
            },
            None,
        )
        .with_compiler_loc()
    });
  }
  let (inner_kind, shape) = match source_kind {
    ValueKind::Matrix(element_kind, shape) => match *element_kind {
        ValueKind::Option(inner) => (*inner, shape),
        _ => return Ok(source.clone()),
    },
    _ => return Ok(source.clone()),
  };
  let values = match crate::patterns::matrix_like_values(source) {
    Some(values) => values,
    None => return Ok(source.clone()),
  };
  let fill_value = fallback
    .convert_to(&inner_kind)
    .ok_or_else(|| {
        MechError::new(
            CannotConvertToTypeError {
                target_type: "requested type",
            },
            None,
        )
        .with_compiler_loc()
    })?;
  let converted_values = values
    .into_iter()
    .map(|value| {
        let raw = match value {
            Value::Empty | Value::EmptyKind(_) => fill_value.clone(),
            other => other,
        };
        raw.convert_to(&inner_kind).ok_or_else(|| {
            MechError::new(
                CannotConvertToTypeError {
                    target_type: "requested type",
                },
                None,
            )
            .with_compiler_loc()
        })
    })
    .collect::<MResult<Vec<Value>>>()?;
  Ok(Value::MatrixValue(Matrix::from_vec(
    converted_values,
    shape[0],
    shape[1],
  )))
}

fn value_contains_empty(value: &Value) -> bool {
  match value {
    Value::Empty | Value::EmptyKind(_) => true,
    #[cfg(feature = "matrix")]
    Value::MatrixValue(matrix) => matrix
        .as_vec()
        .iter()
        .any(|value| value_contains_empty(value)),
    #[cfg(feature = "tuple")]
    Value::Tuple(tuple) => tuple
        .borrow()
        .elements
        .iter()
        .any(|value| value_contains_empty(value.as_ref())),
    Value::Typed(value, _) => value_contains_empty(value),
    Value::MutableReference(reference) => value_contains_empty(&reference.borrow()),
    _ => false,
  }
}

#[cfg(feature = "formulas")]
pub fn factor(fctr: &Factor, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  match fctr {
    Factor::Term(trm) => {
      let result = term(trm, env, p)?;
      Ok(result)
    }
    Factor::Parenthetical(paren) => factor(&*paren, env, p),
    Factor::Expression(expr) => expression(expr, env, p),
    #[cfg(feature = "math_neg")]
    Factor::Negate(neg) => {
      let value = factor(neg, env, p)?;
      #[cfg(feature = "subscript_formula")]
      let value_is_live = current_string_access_expression_live(p) || string_access_input_is_live(&value, p);
      let arguments = vec![value];
      let function = MathNegate {}.compile(&arguments)?;
      let plan = p.plan();
      let out = register_initialized_expression_function(&plan, function, &arguments)?;
      #[cfg(feature = "subscript_formula")]
      if value_is_live {
        mark_current_string_access_expression_live(p);
        mark_string_access_value_live(p, &out);
      }
      Ok(out)
    }
    #[cfg(feature = "logic_not")]
    Factor::Not(neg) => {
      let value = factor(neg, env, p)?;
      #[cfg(feature = "subscript_formula")]
      let value_is_live = current_string_access_expression_live(p) || string_access_input_is_live(&value, p);
      let arguments = vec![value];
      let function = LogicNot {}.compile(&arguments)?;
      let plan = p.plan();
      let out = register_initialized_expression_function(&plan, function, &arguments)?;
      #[cfg(feature = "subscript_formula")]
      if value_is_live {
        mark_current_string_access_expression_live(p);
        mark_string_access_value_live(p, &out);
      }
      Ok(out)
    }
    #[cfg(feature = "matrix_transpose")]
    Factor::Transpose(fctr) => {
      use mech_matrix::MatrixTranspose;
      let value = factor(fctr, env, p)?;
      #[cfg(feature = "subscript_formula")]
      let value_is_live = current_string_access_expression_live(p) || string_access_input_is_live(&value, p);
      let arguments = vec![value];
      let function = MatrixTranspose {}.compile(&arguments)?;
      let plan = p.plan();
      let out = register_initialized_expression_function(&plan, function, &arguments)?;
      #[cfg(feature = "subscript_formula")]
      if value_is_live {
        mark_current_string_access_expression_live(p);
        mark_string_access_value_live(p, &out);
      }
      Ok(out)
    }
    _ => todo!(),
  }
}

#[cfg(feature = "formulas")]
pub fn term(trm: &Term, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let mut lhs = factor(&trm.lhs, env, p)?;
  let mut term_plan: Vec<(Box<dyn MechFunction>, Vec<Value>)> = Vec::new();
  for (op, rhs) in &trm.rhs {
    let rhs = factor(&rhs, env, p)?;
    let dependency_arguments = vec![lhs.clone(), rhs.clone()];
    #[cfg(feature = "subscript_formula")]
    let new_fxn_is_live = current_string_access_expression_live(p)
      || string_access_input_is_live(&lhs, p)
      || string_access_input_is_live(&rhs, p);
    let new_fxn: Box<dyn MechFunction> = match op {
      // Math
      FormulaOperator::AddSub(AddSubOp::Add) => match (&lhs, &rhs) {
        #[cfg(feature = "string_concat")]
        (_, value) | (value, _) if value.is_string() => {
          StringConcat {}.compile(&vec![lhs, rhs])?
        }
        #[cfg(feature = "math_add")]
        _ => MathAdd {}.compile(&vec![lhs, rhs])?,
      },
      #[cfg(feature = "math_sub")]
      FormulaOperator::AddSub(AddSubOp::Sub) => MathSub {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "math_mul")]
      FormulaOperator::MulDiv(MulDivOp::Mul) => MathMul {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "math_div")]
      FormulaOperator::MulDiv(MulDivOp::Div) => MathDiv {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "math_mod")]
      FormulaOperator::MulDiv(MulDivOp::Mod) => MathMod {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "math_pow")]
      FormulaOperator::Power(PowerOp::Pow) => MathPow {}.compile(&vec![lhs, rhs])?,

      // Matrix
      #[cfg(feature = "matrix_matmul")]
      FormulaOperator::Vec(VecOp::MatMul) => MatrixMatMul {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "matrix_solve")]
      FormulaOperator::Vec(VecOp::Solve) => MatrixSolve {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "matrix_cross")]
      FormulaOperator::Vec(VecOp::Cross) => todo!(),
      #[cfg(feature = "matrix_dot")]
      FormulaOperator::Vec(VecOp::Dot) => MatrixDot {}.compile(&vec![lhs, rhs])?,

      // Compare
      #[cfg(feature = "compare_eq")]
      FormulaOperator::Comparison(ComparisonOp::Equal) => CompareEqual {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_seq")]
      FormulaOperator::Comparison(ComparisonOp::StrictEqual) => CompareStrictEqual {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_neq")]
      FormulaOperator::Comparison(ComparisonOp::NotEqual) => CompareNotEqual {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_sneq")]
      FormulaOperator::Comparison(ComparisonOp::StrictNotEqual) => CompareStrictNotEqual {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_lte")]
      FormulaOperator::Comparison(ComparisonOp::LessThanEqual) => CompareLessThanEqual {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_gte")]
      FormulaOperator::Comparison(ComparisonOp::GreaterThanEqual) => CompareGreaterThanEqual {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_lt")]
      FormulaOperator::Comparison(ComparisonOp::LessThan) => CompareLessThan {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_gt")]
      FormulaOperator::Comparison(ComparisonOp::GreaterThan) => CompareGreaterThan {}.compile(&vec![lhs, rhs])?,

      // Logic
      #[cfg(feature = "logic_and")]
      FormulaOperator::Logic(LogicOp::And) => LogicAnd {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "logic_or")]
      FormulaOperator::Logic(LogicOp::Or) => LogicOr {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "logic_not")]
      FormulaOperator::Logic(LogicOp::Not) => LogicNot {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "logic_xor")]
      FormulaOperator::Logic(LogicOp::Xor) => LogicXor {}.compile(&vec![lhs, rhs])?,

      // Table
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::InnerJoin) => TableInnerJoin {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::LeftOuterJoin) => TableLeftOuterJoin {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::RightOuterJoin) => TableRightOuterJoin {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::FullOuterJoin) => TableFullOuterJoin {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::LeftSemiJoin) => TableLeftSemiJoin {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::LeftAntiJoin) => TableLeftAntiJoin {}.compile(&vec![lhs, rhs])?,

      // Set
      #[cfg(feature = "set_union")]
      FormulaOperator::Set(SetOp::Union) => SetUnion {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_intersection")]
      FormulaOperator::Set(SetOp::Intersection) => SetIntersection {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_difference")]
      FormulaOperator::Set(SetOp::Difference) => SetDifference {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_symmetric_difference")]
      FormulaOperator::Set(SetOp::SymmetricDifference) => SetSymmetricDifference {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_complement")]
      FormulaOperator::Set(SetOp::Complement) => todo!(),
      #[cfg(feature = "set_subset")]
      FormulaOperator::Set(SetOp::Subset) => SetSubset {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_superset")]
      FormulaOperator::Set(SetOp::Superset) => SetSuperset {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_proper_subset")]
      FormulaOperator::Set(SetOp::ProperSubset) => SetProperSubset {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_proper_superset")]
      FormulaOperator::Set(SetOp::ProperSuperset) => SetProperSuperset {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_element_of")]
      FormulaOperator::Set(SetOp::ElementOf) => {
        #[cfg(feature = "kind_annotation")]
        if let Value::Kind(kind) = &rhs {
          lhs = Value::Bool(Ref::new(value_in_kind(&lhs, kind, p)));
          continue;
        }
        SetElementOf {}.compile(&vec![lhs, rhs])?
      }
      #[cfg(feature = "set_not_element_of")]
      FormulaOperator::Set(SetOp::NotElementOf) => {
        #[cfg(feature = "kind_annotation")]
        if let Value::Kind(kind) = &rhs {
          lhs = Value::Bool(Ref::new(!value_in_kind(&lhs, kind, p)));
          continue;
        }
        SetNotElementOf {}.compile(&vec![lhs, rhs])?
      }
      x => {
        return Err(MechError::new(
          UnhandledFormulaOperatorError {
            operator: x.clone(),
          },
          None,
        )
        .with_compiler_loc()
        .with_tokens(trm.tokens()));
      }
    };
    new_fxn.solve();
    let res = new_fxn.out();
    #[cfg(feature = "subscript_formula")]
    if new_fxn_is_live {
      mark_current_string_access_expression_live(p);
      mark_string_access_value_live(p, &res);
    }
    term_plan.push((new_fxn, dependency_arguments));
    lhs = res;
  }
  register_expression_function_batch(&plan, term_plan)?;
  Ok(lhs)
}

#[cfg(all(feature = "kind_annotation", feature = "enum", feature = "atom"))]
  fn enum_value_matches_kind(value: &Value, enum_id: u64, state: &ProgramState) -> bool {
  let enum_def = match state.enums.get(&enum_id) {
    Some(enm) => enm,
    None => return false,
  };
  let names_brrw = enum_def.names.borrow();
  let atom_matches_variant = |variant_id: u64, atom_id: u64, atom_name: &str| {
    if variant_id == atom_id {
      return true;
    }
    let variant_name = match names_brrw.get(&variant_id) {
      Some(name) => name.as_str(),
      None => return false,
    };
    let short_variant = variant_name.rsplit('/').next().unwrap_or(variant_name);
    let short_atom = atom_name.rsplit('/').next().unwrap_or(atom_name);
    short_variant == short_atom
  };
  match value {
    Value::Enum(enum_value) => {
      let enum_value_brrw = enum_value.borrow();
      if enum_value_brrw.id != enum_id {
        return false;
      }
      if enum_value_brrw.variants.len() != 1 {
        return false;
      }
      let (variant_id, payload) = &enum_value_brrw.variants[0];
      let (_, declared_payload_kind) = match enum_def
        .variants
        .iter()
        .find(|(known_variant, _)| *known_variant == *variant_id)
      {
        Some(entry) => entry,
        None => return false,
      };
      match (payload, declared_payload_kind) {
        (None, None) => true,
        (Some(payload_value), Some(Value::Kind(expected_kind))) => match expected_kind {
          ValueKind::Enum(inner_enum_id, _) => {
            enum_value_matches_kind(payload_value, *inner_enum_id, state)
          }
          _ => payload_value.kind() == expected_kind.clone() || payload_value.convert_to(expected_kind).is_some(),
        },
        _ => false,
      }
    }
    Value::Atom(atom) => {
      let atom_brrw = atom.borrow();
      let variant_id = atom_brrw.id();
      let atom_name = atom_brrw.name();
      enum_def
        .variants
        .iter()
        .any(|(known_variant, payload_kind)| atom_matches_variant(*known_variant, variant_id, &atom_name) && payload_kind.is_none())
    }
    #[cfg(feature = "tuple")]
    Value::Tuple(tuple_val) => {
      let tuple_brrw = tuple_val.borrow();
      if tuple_brrw.elements.len() != 2 {
        return false;
      }
      let (tag, tag_name) = match tuple_brrw.elements[0].as_ref() {
        Value::Atom(atom) => {
          let atom_brrw = atom.borrow();
          (atom_brrw.id(), atom_brrw.name())
        }
        _ => return false,
      };
      let payload = tuple_brrw.elements[1].as_ref();
      let (_, declared_payload_kind) = match enum_def
        .variants
        .iter()
        .find(|(known_variant, _)| atom_matches_variant(*known_variant, tag, &tag_name))
      {
        Some(entry) => entry,
        None => return false,
      };
      match declared_payload_kind {
        Some(Value::Kind(expected_kind)) => match expected_kind {
          ValueKind::Enum(inner_enum_id, _) => {
            enum_value_matches_kind(payload, *inner_enum_id, state)
          }
          _ => payload.kind() == expected_kind.clone() || payload.convert_to(expected_kind).is_some(),
        },
        _ => false,
      }
    }
    _ => false,
  }
}

#[cfg(feature = "kind_annotation")]
fn value_in_kind(value: &Value, kind: &ValueKind, p: &Interpreter) -> bool {
  let detached = detach_value(value);
  #[cfg(all(feature = "enum", feature = "atom"))]
  if let ValueKind::Enum(enum_id, _) = kind {
    let state_brrw = p.state.borrow();
    return enum_value_matches_kind(&detached, *enum_id, &state_brrw);
  }
  detached.convert_to(kind).is_some()
}

// Errors
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct UnhandledFormulaOperatorError {
  pub operator: FormulaOperator,
}
impl MechErrorKind for UnhandledFormulaOperatorError {
  fn name(&self) -> &str {
    "UnhandledFormulaOperator"
  }
  fn message(&self) -> String {
    format!("Unhandled formula operator: {:#?}", self.operator)
  }
}

#[derive(Debug, Clone)]
pub struct UndefinedVariableError {
  pub id: u64,
  pub name: String,
}
impl MechErrorKind for UndefinedVariableError {
  fn name(&self) -> &str {
    "UndefinedVariable"
  }

  fn message(&self) -> String {
    format!("Undefined variable `{}` (id: {})", self.name, self.id)
  }
}
#[derive(Debug, Clone)]
pub struct InvalidIndexKindError {
  kind: ValueKind,
}
impl MechErrorKind for InvalidIndexKindError {
  fn name(&self) -> &str {
    "InvalidIndexKind"
  }
  fn message(&self) -> String {
    "Invalid index kind".to_string()
  }
}

#[derive(Debug, Clone)]
pub struct ComprehensionGeneratorError {
  found: ValueKind,
}

impl MechErrorKind for ComprehensionGeneratorError {
  fn name(&self) -> &str {
    "ComprehensionGenerator"
  }
  fn message(&self) -> String {
      format!(
        "Comprehension generator must produce a set or matrix, found kind: {:?}",
        self.found
      )
  }
}

#[derive(Debug, Clone)]
pub struct PatternExpectedTupleError {
  found: ValueKind,
}
impl MechErrorKind for PatternExpectedTupleError {
  fn name(&self) -> &str {
    "PatternExpectedTuple"
  }
  fn message(&self) -> String {
    format!("Pattern expected a tuple, found kind: {:?}", self.found)
  }
}

#[derive(Debug, Clone)]
pub struct ArityMismatchError {
  expected: usize,
  found: usize,
}
impl MechErrorKind for ArityMismatchError {
  fn name(&self) -> &str {
    "ArityMismatch"
  }
  fn message(&self) -> String {
    format!(
      "Arity mismatch: expected {}, found {}",
      self.expected, self.found
    )
  }
}

#[derive(Debug, Clone)]
pub struct PatternMatchError {
  pub var: String,
  pub expected: String,
  pub found: String,
}

#[derive(Debug, Clone)]
pub struct MatchNoArmMatchedError;
impl MechErrorKind for MatchNoArmMatchedError {
  fn name(&self) -> &str {
    "MatchNoArmMatched"
  }
  fn message(&self) -> String {
    format!("No match arm matched the provided value.")
  }
}

#[derive(Debug, Clone)]
pub struct MatchArmKindMismatchError {
  expected: ValueKind,
  found: ValueKind,
}
impl MechErrorKind for MatchArmKindMismatchError {
  fn name(&self) -> &str {
    "MatchArmKindMismatch"
  }
  fn message(&self) -> String {
    format!(
      "Expected {:?}, found {:?}",
      self.expected, self.found
    )
  }
}

#[derive(Debug, Clone)]
pub struct MatchNonExhaustiveError;
impl MechErrorKind for MatchNonExhaustiveError {
  fn name(&self) -> &str {
    "MatchNonExhaustive"
  }
  fn message(&self) -> String {
    "Match expression must include a wildcard (`*`) arm.".to_string()
  }
}

#[derive(Debug, Clone)]
pub struct MatchNonExhaustiveVariantsError {
  pub enum_name: String,
  pub missing_patterns: Vec<String>,
}
impl MechErrorKind for MatchNonExhaustiveVariantsError {
  fn name(&self) -> &str {
    "MatchNonExhaustive"
  }
  fn message(&self) -> String {
    format!(
      "Match over enum '{}' is non-exhaustive. Missing variants: {}. Handle the missing variants or add a wildcard (`*`) arm to catch all cases.",
      self.enum_name,
      self.missing_patterns.join(", ")
    )
  }
}

impl MechErrorKind for PatternMatchError {
  fn name(&self) -> &str {
    "PatternMatchError"
  }
  fn message(&self) -> String {
    format!(
      "Pattern match error for variable '{}': expected value {}, found value {}",
      self.var, self.expected, self.found
    )
  }
}

#[derive(Debug, Clone)]
pub struct InvalidGuardExpressionError {
  found: ValueKind,
}

impl MechErrorKind for InvalidGuardExpressionError {
  fn name(&self) -> &str {
    "InvalidGuardExpression"
  }
  fn message(&self) -> String {
    format!(
      "Guard expressions must evaluate to a boolean value. Found kind: {:?}",
      self.found
    )
  }
}
