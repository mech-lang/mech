use crate::*;
use paste::paste;

#[cfg(feature = "variable_define")]
use crate::stdlib::define::*;

// Statements
// ----------------------------------------------------------------------------

pub fn statement(stmt: &Statement, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  match stmt {
    Statement::ImportDeclaration(_) => Ok(Value::Empty),
    Statement::ExportDeclaration(_) => Ok(Value::Empty),
    Statement::ContextDeclaration(ctx) => context_declaration(ctx, p),
    #[cfg(feature = "tuple")]
    Statement::TupleDestructure(tpl_dstrct) => tuple_destructure(&tpl_dstrct, p),
    #[cfg(feature = "invariant_define")]
    Statement::InvariantDefine(inv_def) => invariant_define(&inv_def, p),
    #[cfg(feature = "variable_define")]
    Statement::VariableDefine(var_def) => variable_define(&var_def, p),
    #[cfg(feature = "variable_assign")]
    Statement::VariableAssign(var_assgn) => variable_assign(&var_assgn, env, p),
    #[cfg(feature = "kind_define")]
    Statement::KindDefine(knd_def) => kind_define(&knd_def, p),
    #[cfg(feature = "enum")]
    Statement::EnumDefine(enm_def) => {
      enum_define(&enm_def, p)?;
      Ok(Value::Empty)
    }
    #[cfg(feature = "math")]
    Statement::OpAssign(op_assgn) => op_assign(&op_assgn, env, p),
    #[cfg(feature = "state_machines")]
    Statement::FsmDeclare(fsm_decl) => fsm_declare(fsm_decl, env, p),
    //Statement::SplitTable => todo!(),
    //Statement::FlattenTable => todo!(),
    x => return Err(MechError::new(
        FeatureNotEnabledError,
        None
      ).with_compiler_loc().with_tokens(x.tokens())
    ),
  }
}

#[cfg(all(test, feature = "program", feature = "functions", feature = "variables", feature = "variable_define", feature = "variable_assign", feature = "f64", feature = "math", feature = "assign"))]
mod dirty_scheduler_integration_tests {
  use super::*;

  fn symbol(interpreter: &Interpreter, name: &str) -> Value { interpreter.symbols().borrow().get(hash_str(name)).unwrap_or_else(|| panic!("missing symbol {name}")).borrow().clone() }
  fn root_cell(interpreter: &Interpreter, name: &str) -> ReactiveCellId { let cells=symbol(interpreter,name).reactive_root_cell_ids(); assert_eq!(cells.len(),1); cells[0] }
  fn set_f64(interpreter: &Interpreter, name: &str, value: f64) { *symbol(interpreter,name).as_f64().unwrap().borrow_mut()=value; }
  fn output_nodes(interpreter: &Interpreter, cell: ReactiveCellId) -> Vec<ReactiveNodeId> { let plan=interpreter.plan(); plan.borrow().nodes.iter().filter(|node| node.outputs.contains(&cell)).map(|node|node.id).collect() }
  fn executed_outputs(interpreter: &Interpreter, executed: &[ReactiveNodeId], cell: ReactiveCellId) -> bool { let plan=interpreter.plan(); let plan=plan.borrow(); executed.iter().any(|id|plan.node(*id).unwrap().outputs.contains(&cell)) }
  fn value(interpreter: &Interpreter, name: &str) -> f64 { *symbol(interpreter,name).as_f64().unwrap().borrow() }

  #[test]
  fn dirty_scheduler_updates_only_reachable_combinational_nodes() {
    let tree=mech_syntax::parser::parse("a := 1.0\nb := a + 1.0\nc := b + 1.0\n\nu := 100.0\nv := u + 1.0").unwrap(); let mut interpreter=Interpreter::new_with_full_stdlib(0); interpreter.interpret(&tree).unwrap();
    assert_eq!((value(&interpreter,"b"),value(&interpreter,"c"),value(&interpreter,"v")),(2.0,3.0,101.0)); let a=root_cell(&interpreter,"a");let b=root_cell(&interpreter,"b");let c=root_cell(&interpreter,"c");let v=root_cell(&interpreter,"v"); set_f64(&interpreter,"a",10.0); let outcome=interpreter.plan().solve_dirty_cells(&[a]).unwrap();
    assert_eq!((value(&interpreter,"b"),value(&interpreter,"c"),value(&interpreter,"v")),(11.0,12.0,101.0)); assert!(executed_outputs(&interpreter,&outcome.executed_nodes,b));assert!(executed_outputs(&interpreter,&outcome.executed_nodes,c));assert!(!executed_outputs(&interpreter,&outcome.executed_nodes,v));assert!(outcome.pending_register_nodes.is_empty()); let plan=interpreter.plan();let plan=plan.borrow();assert!(outcome.executed_nodes.windows(2).all(|pair|plan.node(pair[0]).unwrap().plan_index < plan.node(pair[1]).unwrap().plan_index));let unique=outcome.executed_nodes.iter().collect::<std::collections::HashSet<_>>();assert_eq!(unique.len(),outcome.executed_nodes.len());
  }

  #[test]
  fn dirty_scheduler_stops_at_register_boundary() {
    let tree=mech_syntax::parser::parse("~x := 1.0\ny := 2.0\nx += y\nz := x + 1.0").unwrap();let mut interpreter=Interpreter::new_with_full_stdlib(0);interpreter.interpret(&tree).unwrap();assert_eq!((value(&interpreter,"x"),value(&interpreter,"z")),(3.0,4.0));let x=root_cell(&interpreter,"x");let y=root_cell(&interpreter,"y");let z=root_cell(&interpreter,"z");let registers={let plan=interpreter.plan();let plan=plan.borrow();plan.nodes.iter().filter(|n|n.kind==ReactiveNodeKind::Register&&n.outputs.contains(&x)).map(|n|n.id).collect::<Vec<_>>()};assert_eq!(registers.len(),1);let register=registers[0];assert!(output_nodes(&interpreter,x).contains(&register));set_f64(&interpreter,"y",10.0);let outcome=interpreter.plan().solve_dirty_cells(&[y]).unwrap();assert_eq!(outcome.pending_register_nodes,vec![register]);assert_eq!((value(&interpreter,"x"),value(&interpreter,"z")),(3.0,4.0));assert!(!outcome.executed_nodes.contains(&register));assert!(!executed_outputs(&interpreter,&outcome.executed_nodes,z));
  }
}





#[cfg(all(
    test,
    feature = "program",
    feature = "functions",
    feature = "variables",
    feature = "variable_define",
    feature = "variable_assign",
    feature = "f64",
    feature = "math",
    feature = "assign"
))]
mod activation_scope_tests {
    use super::*;
    use std::collections::HashSet;

    const PURE: &str = "tick := 0.0\nx := 10.0\nradius := 2.0\n~> tick {\n left := x - radius\n doubled := left * 2.0\n}";
    const REGISTER: &str =
        "tick := 0.0\n~x := 10.0\n\n~> tick {\n  next-x := x + 1.0\n  x = next-x\n}";
    const TWO_REGISTERS: &str = "tick := 0.0\n\n~x := 0.0\n~y := 0.0\n\n~> tick {\n  next-x := x + 1.0\n  next-y := y + 2.0\n\n  x = next-x\n  y = next-y\n}";
    fn interpret(source: &str) -> Interpreter {
        let t = mech_syntax::parser::parse(source).unwrap();
        let mut i = Interpreter::new_with_full_stdlib(0);
        i.interpret(&t).unwrap();
        i
    }
    fn load() -> Interpreter {
        interpret(PURE)
    }
    fn cell(i: &Interpreter, n: &str) -> ReactiveCellId {
        i.symbols()
            .borrow()
            .get(hash_str(n))
            .unwrap()
            .borrow()
            .reactive_root_cell_ids()[0]
    }
    fn value(i: &Interpreter, n: &str) -> f64 {
        *i.symbols()
            .borrow()
            .get(hash_str(n))
            .unwrap()
            .borrow()
            .as_f64()
            .unwrap()
            .borrow()
    }
    fn nodes_for_output(i: &Interpreter, name: &str, kind: ReactiveNodeKind) -> Vec<ReactiveNodeId> {
        let output = cell(i, name);
        let p = i.plan();
        p
            .borrow()
            .nodes
            .iter()
            .filter(|n| n.kind == kind && n.outputs.contains(&output))
            .map(|n| n.id)
            .collect()
    }
    fn unique_register_for(i: &Interpreter, name: &str) -> ReactiveNodeId { let found=nodes_for_output(i,name,ReactiveNodeKind::Register);assert_eq!(found.len(),1,"expected one register node for {name}");found[0] }
    fn activation_nodes(i: &Interpreter, trigger_name: &str, kind: ReactiveNodeKind) -> Vec<ReactiveNodeId> { let trigger=cell(i,trigger_name);let p=i.plan();p.borrow().nodes.iter().filter(|node|node.kind==kind&&node.inputs.iter().any(|dependency|dependency.cell==trigger&&dependency.kind==ReactiveDependencyKind::Reactive)).map(|node|node.id).collect() }
    fn combinational_node_for_output_and_dependency(i: &Interpreter, output_name: &str, input_name: &str, input_kind: ReactiveDependencyKind) -> ReactiveNodeId { let output=cell(i,output_name);let input=cell(i,input_name);let p=i.plan();let found=p.borrow().nodes.iter().filter(|node|node.kind==ReactiveNodeKind::Combinational&&node.outputs.contains(&output)&&node.inputs.iter().any(|dependency|dependency.cell==input&&dependency.kind==input_kind)).map(|node|node.id).collect::<Vec<_>>();assert_eq!(found.len(),1,"expected one combinational node for {output_name} consuming {input_name}");found[0] }
    fn nodes(i: &Interpreter) -> Vec<ReactiveNodeId> { activation_nodes(i,"tick",ReactiveNodeKind::Combinational) }
    fn two_register_nodes(i: &Interpreter) -> Vec<ReactiveNodeId> {
        let registers = activation_nodes(i,"tick",ReactiveNodeKind::Register);
        assert_eq!(registers.len(), 2, "exactly two activation registers");
        let p=i.plan();let p=p.borrow();let outputs=registers.iter().flat_map(|id|p.node(*id).unwrap().outputs.iter()).copied().collect::<HashSet<_>>();assert_eq!(outputs,[cell(i,"x"),cell(i,"y")].into_iter().collect());
        registers
    }
    fn snapshot(
        i: &Interpreter,
    ) -> (
        usize,
        Vec<(
            usize,
            usize,
            ReactiveNodeKind,
            Vec<u64>,
            Vec<(u64, ReactiveDependencyKind)>,
        )>,
        Vec<(u64, Vec<usize>)>,
        Vec<(u64, Vec<usize>)>,
    ) {
        let p = i.plan();
        let p = p.borrow();
        let nodes = p
            .nodes
            .iter()
            .map(|n| {
                (
                    n.id,
                    n.plan_index,
                    n.kind,
                    n.outputs.iter().map(|c| c.get()).collect(),
                    n.inputs.iter().map(|d| (d.cell.get(), d.kind)).collect(),
                )
            })
            .collect();
        let mut reactive = p
            .reactive_consumers
            .iter()
            .map(|(c, n)| (c.get(), n.clone()))
            .collect::<Vec<_>>();
        let mut sampled = p
            .sampled_consumers
            .iter()
            .map(|(c, n)| (c.get(), n.clone()))
            .collect::<Vec<_>>();
        reactive.sort_by_key(|(c, _)| *c);
        sampled.sort_by_key(|(c, _)| *c);
        (p.len(), nodes, reactive, sampled)
    }
    #[test]
    fn activation_scope_does_not_execute_during_load() {
        let i = interpret(REGISTER);
        let (next_x, register) = (
            nodes_for_output(&i, "next-x", ReactiveNodeKind::Combinational), unique_register_for(&i,"x"),
        );
        assert_eq!(value(&i, "x"), 10.);
        assert!(!next_x.is_empty());
        assert_eq!(
            i.plan()
                .borrow()
                .nodes
                .iter()
                .filter(
                    |n| n.kind == ReactiveNodeKind::Register && n.outputs.contains(&cell(&i, "x"))
                )
                .map(|n| n.id)
                .collect::<Vec<_>>(),
            vec![register]
        );
        assert!(!i.has_pending_reactive_registers());
        assert!(!i.plan().activation_registration_active());
    }
    #[test]
    fn activation_scope_trigger_is_reactive() {
        let i = load();
        let t = cell(&i, "tick");
        let p = i.plan();
        assert!(nodes(&i).iter().all(|n| {
            p.borrow()
                .node(*n)
                .unwrap()
                .inputs
                .iter()
                .any(|d| d.cell == t && d.kind == ReactiveDependencyKind::Reactive)
        }));
    }
    #[test]
    fn activation_scope_external_inputs_are_sampled() {
        let i = load();
        let left = combinational_node_for_output_and_dependency(&i,"left","x",ReactiveDependencyKind::Sampled);
        let p = i.plan();
        let p = p.borrow();
        for input in ["x", "radius"] {
            let c = cell(&i, input);
            assert!(
                p.node(left)
                    .unwrap()
                    .inputs
                    .iter()
                    .any(|d| d.cell == c && d.kind == ReactiveDependencyKind::Sampled)
            );
            assert!(p.sampled_consumers_for(c).contains(&left));
            assert!(!p.reactive_consumers_for(c).contains(&left));
        }
    }
    #[test]
    fn activation_scope_local_outputs_are_reactive() {
        let i = load();
        let p = i.plan();
        assert!(
            p.borrow()
                .node(combinational_node_for_output_and_dependency(&i,"doubled","left",ReactiveDependencyKind::Reactive))
                .unwrap()
                .inputs
                .iter()
                .any(|d| d.cell == cell(&i, "left") && d.kind == ReactiveDependencyKind::Reactive)
        );
    }
    #[test]
    fn activation_scope_runs_once_on_trigger() {
        let mut i = load();
        let body = nodes(&i);
        let t = cell(&i, "tick");
        let o = i.advance_reactive_turn(&[t]).unwrap();
        let executed = o
            .before_commit
            .executed_nodes
            .iter()
            .chain(o.after_commit.executed_nodes.iter())
            .copied()
            .collect::<Vec<_>>();
        let unique = executed.iter().copied().collect::<HashSet<_>>();
        assert_eq!(unique.len(), executed.len(), "no node runs twice");
        for node in body {
            assert_eq!(
                executed.iter().filter(|id| **id == node).count(),
                1,
                "body node {node} runs exactly once"
            );
        }
        assert_eq!((value(&i, "left"), value(&i, "doubled")), (8., 16.));
    }
    #[test]
    fn activation_scope_ignores_external_value_change() {
        let mut i = load();
        let x = cell(&i, "x");
        let o = i.advance_reactive_turn(&[x]).unwrap();
        assert!(
            nodes(&i)
                .iter()
                .all(|n| !o.before_commit.executed_nodes.contains(n))
        );
    }
    #[test]
    fn activation_scope_samples_latest_external_value() {
        let mut i = load();
        let x = i
            .symbols()
            .borrow()
            .get(hash_str("x"))
            .unwrap()
            .borrow()
            .clone();
        *x.as_f64().unwrap().borrow_mut() = 20.;
        let t = cell(&i, "tick");
        i.advance_reactive_turn(&[t]).unwrap();
        assert_eq!(
            *i.symbols()
                .borrow()
                .get(hash_str("left"))
                .unwrap()
                .borrow()
                .as_f64()
                .unwrap()
                .borrow(),
            18.
        );
    }
    #[test]
    fn activation_scope_registers_commit_atomically() {
        let mut i = interpret(TWO_REGISTERS);
        let registers = two_register_nodes(&i);
        assert_eq!((value(&i, "x"), value(&i, "y")), (0., 0.));
        let o = i.advance_reactive_turn(&[cell(&i, "tick")]).unwrap();
        assert_eq!(o.before_commit.pending_register_nodes, registers);
        assert_eq!(o.register_commit.staged_nodes, registers);
        assert_eq!(o.register_commit.committed_nodes, registers);
        assert_eq!(
            o.register_commit.dirty_cells,
            vec![cell(&i, "x"), cell(&i, "y")]
        );
        assert_eq!((value(&i, "x"), value(&i, "y")), (1., 2.));
    }
    #[test]
    fn activation_scope_register_commit_does_not_reactivate_body() {
        let mut i = interpret(TWO_REGISTERS);
        let combinational=activation_nodes(&i,"tick",ReactiveNodeKind::Combinational);let registers=two_register_nodes(&i);assert!(!combinational.is_empty());
        let o = i.advance_reactive_turn(&[cell(&i, "tick")]).unwrap();
        assert!(
            combinational.iter().all(|id| o.before_commit.executed_nodes.iter().filter(|node| **node==*id).count()==1)
        );
        assert_eq!(o.before_commit.pending_register_nodes.iter().copied().collect::<HashSet<_>>(),registers.iter().copied().collect());
        assert_eq!(o.before_commit.pending_register_nodes.len(),registers.len());
        assert_eq!(o.register_commit.staged_nodes.iter().copied().collect::<HashSet<_>>(),registers.iter().copied().collect());
        assert_eq!(o.register_commit.staged_nodes.len(),registers.len());
        assert_eq!(o.register_commit.committed_nodes.iter().copied().collect::<HashSet<_>>(),registers.iter().copied().collect());
        assert_eq!(o.register_commit.committed_nodes.len(),registers.len());
        assert_eq!((value(&i, "x"), value(&i, "y")), (1., 2.));
        assert!(
            combinational
                .iter()
                .all(|id| !o.after_commit.executed_nodes.contains(id))
        );
    }
    #[test]
    fn activation_scope_failed_elaboration_clears_registration_state() {
        let mut i = interpret("tick := 0.0\nx := 1.0");
        let before = snapshot(&i);
        let failing=mech_syntax::parser::parse("~> tick {\n  registered-first := x + 1.0\n  fails-later := function-that-does-not-exist(registered-first)\n}").unwrap();
        let error = i.interpret(&failing).unwrap_err();
        assert!(format!("{error:?}").contains("Function"));
        assert_eq!(snapshot(&i), before);
        assert!(!i.symbols().borrow().contains(hash_str("registered-first")));
        assert!(!i.symbols().borrow().contains(hash_str("fails-later")));
        assert!(!i.plan().activation_registration_active());
        let ordinary = mech_syntax::parser::parse("ordinary := x + 2.0").unwrap();
        i.interpret(&ordinary).unwrap();
        let ordinary_nodes=nodes_for_output(&i,"ordinary",ReactiveNodeKind::Combinational);assert!(!ordinary_nodes.is_empty());
        let p = i.plan();
        let p = p.borrow();
        assert!(
            ordinary_nodes.iter().all(|node|!p.node(*node).unwrap().inputs.iter().any(|d|d.cell==cell(&i,"tick")))
        );
        assert!(
            ordinary_nodes.iter().any(|node|p.node(*node).unwrap().inputs.iter().any(|d|d.cell==cell(&i,"x")&&d.kind==ReactiveDependencyKind::Reactive))
        );
        assert!(!i.plan().activation_registration_active());
    }
    #[test]
    fn activation_scope_rejects_whole_assignment_to_trigger() {
        let mut i = Interpreter::new_with_full_stdlib(0);
        let setup = mech_syntax::parser::parse("~tick := 0.0").unwrap();
        i.interpret(&setup).unwrap();
        let before = snapshot(&i);
        let t = mech_syntax::parser::parse("~> tick {\n tick = tick + 1.0\n}").unwrap();
        assert!(
            format!("{:?}", i.interpret(&t).unwrap_err())
                .contains("ActivationScopeTriggerWriteUnsupported")
        );
        assert_eq!(snapshot(&i), before);
        assert!(!i.plan().activation_registration_active());
    }
    #[test]
    fn activation_scope_rejects_operator_assignment_to_trigger() {
        let mut i = Interpreter::new_with_full_stdlib(0);
        let setup = mech_syntax::parser::parse("~tick := 0.0").unwrap();
        i.interpret(&setup).unwrap();
        let before = snapshot(&i);
        let t = mech_syntax::parser::parse("~> tick {\n tick += 1.0\n}").unwrap();
        assert!(
            format!("{:?}", i.interpret(&t).unwrap_err())
                .contains("ActivationScopeTriggerWriteUnsupported")
        );
        assert_eq!(snapshot(&i), before);
        assert!(!i.plan().activation_registration_active());
    }
    #[test]
    fn activation_scope_plan_is_stable_across_triggers() {
        let mut i = load();
        let before = snapshot(&i);
        let t = cell(&i, "tick");
        for _ in 0..3 {
            i.advance_reactive_turn(&[t]).unwrap();
        }
        assert_eq!(snapshot(&i), before);
    }
}

// Interpreter-local context bindings are for direct interpreter execution.
// Host runtime resource bindings are owned by MechRuntime.resource_bindings.
pub fn context_declaration(ctx: &ContextDeclaration, p: &Interpreter) -> MResult<Value> {
  match &ctx.base {
    ContextBase::ResourceUri(uri) => {
      p.bind_context(&ctx.name, uri.chars.iter().collect::<String>());
      Ok(Value::Empty)
    }
    ContextBase::Context(base) => {
      match p.context_binding(base) {
        Some(binding) => {
          p.bind_context(&ctx.name, binding.base_uri);
          Ok(Value::Empty)
        }
        None => Err(MechError::new(
          GenericError { msg: format!("Context `@{}` is not defined", base.to_string()) },
          None,
        ).with_compiler_loc().with_tokens(base.tokens())),
      }
    }
  }
}

#[cfg(feature = "tuple")]
pub fn tuple_destructure(tpl_dstrct: &TupleDestructure, p: &Interpreter) -> MResult<Value> {
  let source = expression(&tpl_dstrct.expression, None, p)?;
  let tpl = match &source {
    Value::Tuple(tpl) => tpl,
    Value::MutableReference(r) => {
      let r_brrw = r.borrow();
      &match &*r_brrw {
        Value::Tuple(tpl) => tpl.clone(),
        _ => return Err(MechError::new(
          DestructureExpectedTupleError{ value: source.kind() },
          None
        ).with_compiler_loc().with_tokens(tpl_dstrct.expression.tokens())),
      }
    },
    _ => return Err(MechError::new(
      DestructureExpectedTupleError{ value: source.kind() },
      None
    ).with_compiler_loc().with_tokens(tpl_dstrct.expression.tokens())),
  };
  let symbols = p.symbols();
  let mut symbols_brrw = symbols.borrow_mut();
  for (i, var) in tpl_dstrct.vars.iter().enumerate() {
    let id = var.hash();
    if symbols_brrw.contains(id) {
      return Err(MechError::new(
        VariableAlreadyDefinedError { id },
        None
      ).with_compiler_loc().with_tokens(var.tokens()));
    }
    if let Some(element) = tpl.borrow().get(i) {
      symbols_brrw.insert(id, element.clone(), true);
      symbols_brrw.dictionary.borrow_mut().insert(id, var.name.to_string());
    } else {
      return Err(MechError::new(
        TupleDestructureTooManyVarsError{ value: source.kind() },
        None
      ).with_compiler_loc().with_tokens(var.tokens()));
    }
  }
  Ok(source)
}

fn assignment_registration_operand(value: &Value) -> Value {
  match value {
    Value::MutableReference(reference) => {
      let inner = reference.borrow();
      assignment_registration_operand(&inner)
    }
    _ => value.clone(),
  }
}

#[cfg(feature = "math")]
pub fn op_assign(op_assgn: &OpAssign, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let mut source = expression(&op_assgn.expression, env, p)?;
  let slc = &op_assgn.target;
  if slc.context.is_some() {
    return Err(MechError::new(AddressedAssignmentUnsupported, None)
      .with_compiler_loc()
      .with_tokens(slc.tokens()));
  }
  let id = slc.name.hash();
  let sink = { 
    let mut state_brrw = p.state.borrow_mut();
    match state_brrw.get_mutable_symbol(id) {
      Some(val) => val.borrow().clone(),
      None => {
        match state_brrw.contains_symbol(id) {
          true => return Err(MechError::new(
            NotMutableError { id },
            Some("(!)> Mutable variables are defined with the `~` operator. *e.g.*: {{~x := 123}}".to_string()),
          ).with_compiler_loc().with_tokens(slc.name.tokens())),
          false => return Err(MechError::new(
            UndefinedVariableError { id, name: slc.name.to_string() },
            Some("(!)> Variables are defined with the `:=` operator. *e.g.*: {{x := 123}}".to_string()),
          ).with_compiler_loc().with_tokens(slc.name.tokens())),
        }
      }
    }
  };
  match &slc.subscript {
    Some(sbscrpt) => {
      // todo: this only works for the first subscript, it needs to work for multiple subscripts
      for s in sbscrpt {
        let fxn = match op_assgn.op {
          #[cfg(feature = "math_add_assign")]
          OpAssignOp::Add => add_assign(&s, &sink, &source, env, p)?,
          #[cfg(feature = "math_sub_assign")]
          OpAssignOp::Sub => sub_assign(&s, &sink, &source, env, p)?,
          #[cfg(feature = "math_div_assign")]
          OpAssignOp::Div => div_assign(&s, &sink, &source, env, p)?,
          #[cfg(feature = "math_mul_assign")]
          OpAssignOp::Mul => mul_assign(&s, &sink, &source, env, p)?,
          _ => todo!(),
        };
        return Ok(fxn);
      }
    }
    None => {
      let plan = p.plan();
      let registration_source = assignment_registration_operand(&source);
      let compile_arguments = vec![sink, source];
      let registration_arguments = vec![registration_source];
      return match op_assgn.op {
        #[cfg(feature = "math_add_assign")]
        OpAssignOp::Add => execute_initialized_indexed_compiler_with_registration_arguments(&plan, &AddAssignValue{}, compile_arguments, registration_arguments),
        #[cfg(feature = "math_sub_assign")]
        OpAssignOp::Sub => execute_initialized_indexed_compiler_with_registration_arguments(&plan, &SubAssignValue{}, compile_arguments, registration_arguments),
        #[cfg(feature = "math_div_assign")]
        OpAssignOp::Div => execute_initialized_indexed_compiler_with_registration_arguments(&plan, &DivAssignValue{}, compile_arguments, registration_arguments),
        #[cfg(feature = "math_mul_assign")]
        OpAssignOp::Mul => execute_initialized_indexed_compiler_with_registration_arguments(&plan, &MulAssignValue{}, compile_arguments, registration_arguments),
        _ => todo!(),
      };
    }
  }
  unreachable!(); // subscript should have thrown an error if we can't access an element
}

#[cfg(feature = "variable_assign")]
pub fn variable_assign(var_assgn: &VariableAssign, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let mut source = expression(&var_assgn.expression, env, p)?;
  let slc = &var_assgn.target;
  if slc.context.is_some() {
    return Err(MechError::new(AddressedAssignmentUnsupported, None)
      .with_compiler_loc()
      .with_tokens(slc.tokens()));
  }
  let id = slc.name.hash();
  let sink = {
    let symbols = p.symbols();
    let symbols_brrw = symbols.borrow();
    match symbols_brrw.get_mutable(id) {
      Some(val) => val.borrow().clone(),
      None => {
        if !symbols_brrw.contains(id) {
          return Err(MechError::new(
            UndefinedVariableError { id, name: slc.name.to_string() },
            Some("(!)> Variables are defined with the `:=` operator. *e.g.*: {{x := 123}}".to_string()),
          ).with_compiler_loc().with_tokens(slc.name.tokens()));
        } else { 
          return Err(MechError::new(
            NotMutableError { id },
            Some("(!)> Mutable variables are defined with the `~` operator. *e.g.*: {{~x := 123}}".to_string()),
          ).with_compiler_loc().with_tokens(slc.name.tokens()));
        }
      }
    }
  };
  match &slc.subscript {
    Some(sbscrpt) => {
      #[cfg(feature = "subscript")]
      for s in sbscrpt {
        let s_result = subscript_ref(&s, &sink, &source, env, p)?;
        return Ok(s_result);
      }
    }
    #[cfg(feature = "assign")]
    None => {
      let plan = p.plan();
      let registration_source = assignment_registration_operand(&source);
      return execute_initialized_indexed_compiler_with_registration_arguments(
        &plan,
        &AssignValue{},
        vec![sink, source],
        vec![registration_source],
      );
    }
    _ => return Err(MechError::new(
      FeatureNotEnabledError,
      None
    ).with_compiler_loc().with_tokens(var_assgn.target.tokens())),
  }
  unreachable!(); // subscript should have thrown an error if we can't access an element
}

#[cfg(feature = "enum")]
pub fn enum_define(enm_def: &EnumDefine, p: &Interpreter) -> MResult<()> {
  let id = enm_def.name.hash();
  let mut variants: Vec<(u64, Option<Value>)> = Vec::new();
  {
    let mut state_brrw = p.state.borrow_mut();
    for v in &enm_def.variants {
      let payload = match &v.value {
        Some(kind_annotation_node) => {
          let knd = kind_annotation(&kind_annotation_node.kind, p)?;
          let vk = knd.to_value_kind(&mut state_brrw.kinds)?;
          Some(Value::Kind(vk))
        }
        None => None,
      };
      variants.push((v.name.hash(), payload));
    }
  }
  let state = &p.state;
  let mut state_brrw = state.borrow_mut();
  let dictionary = state_brrw.dictionary.clone();
  {
    let mut dictionary_brrw = dictionary.borrow_mut();
    dictionary_brrw.insert(enm_def.name.hash(), enm_def.name.to_string());
    for variant in &enm_def.variants {
      dictionary_brrw.insert(variant.name.hash(), variant.name.to_string());
    }
  }
  let enm = MechEnum{id, variants, names: dictionary};
  let val = Value::Enum(Ref::new(enm.clone()));
  state_brrw.enums.insert(id, enm.clone());
  state_brrw.kinds.insert(id, val.kind());
  Ok(())
}

#[cfg(feature = "kind_define")]
pub fn kind_define(knd_def: &KindDefine, p: &Interpreter) -> MResult<Value> {
  let id = knd_def.name.hash();
  let kind = kind_annotation(&knd_def.kind.kind, p)?;
  let value_kind = kind.to_value_kind(&p.state.borrow().kinds)?;
  let functions = p.functions();
  let mut kinds = &mut p.state.borrow_mut().kinds;
  kinds.insert(id, value_kind.clone());
  Ok(Value::Kind(value_kind))
}

#[cfg(feature = "invariant_define")]
pub fn invariant_define(inv_def: &InvariantDefine, p: &Interpreter) -> MResult<Value> {
  let invariant_id = inv_def.name.hash();
  let invariant_name = inv_def.name.to_string();
  let invariant_expression = tokens_to_string(&inv_def.expression.tokens());
  {
    let symbols = p.symbols();
    if symbols.borrow().contains(invariant_id) {
      return Err(MechError::new(
        VariableAlreadyDefinedError { id: invariant_id },
        None
      ).with_compiler_loc().with_tokens(inv_def.name.tokens()));
    }
  }
  let plan = p.plan();
  let result = expression(&inv_def.expression, None, p)?;
  let rhs_ref = value_to_ref(result.clone());
  let detached_result = detach_variable_value(&result);
  {
    let mut state_brrw = p.state.borrow_mut();
    state_brrw.save_symbol(invariant_id, invariant_name.clone(), detached_result.clone(), false);
    let var_define_arguments = vec![detached_result.clone(), Value::String(Ref::new(invariant_name.clone())), Value::Bool(Ref::new(false))];
    let var_def_fxn = VarDefine{}.compile(&var_define_arguments)?;
    plan.register_function(var_def_fxn, &[])?;
  }
  p.state.borrow_mut().invariant_expressions.insert(invariant_id, invariant_expression.clone());
  #[cfg(all(feature = "invariant_define", feature = "symbol_table"))]
  {
    let invariant_value = {
      let state_brrw = p.state.borrow();
      state_brrw.get_symbol(invariant_id)
    };
    if let Some(invariant_value) = invariant_value {
      p.state.borrow_mut().invariants.insert(invariant_id, (invariant_name.clone(), invariant_value));
    }
  }
  let violation_error = match &result {
    Value::Bool(b) => if *b.borrow() { None } else { Some("evaluated to false".to_string()) },
    other => Some(format!("must evaluate to bool, got {}", other.kind())),
  };
  let operand_detail = invariant_operand_refs(inv_def, p);
  let (lhs_addr, lhs_value, operator, rhs_addr, rhs_value) = match operand_detail {
    Some((lhs, op, rhs)) => {
      let lhs_addr = lhs.as_ref().map(|v| v.addr() as u64);
      let lhs_value = lhs.as_ref().map(|v| format!("{:?}", v.borrow()));
      let rhs_addr = rhs.as_ref().map(|v| v.addr() as u64);
      let rhs_value = rhs.as_ref().map(|v| format!("{:?}", v.borrow()));
      (lhs_addr, lhs_value, op, rhs_addr, rhs_value)
    }
    None => (None, None, None, Some(rhs_ref.addr() as u64), Some(format!("{:?}", rhs_ref.borrow()))),
  };
  {
    let reason = violation_error.clone().unwrap_or_else(|| "evaluated to true".to_string());
    let actual = lhs_value.clone().unwrap_or_else(|| format!("{:?}", rhs_ref.borrow()));
    let expected = rhs_value.clone().unwrap_or_else(|| format!("{:?}", rhs_ref.borrow()));
    p.state.borrow_mut().invariant_evaluations.insert(invariant_id, InvariantEvaluation {
      reason,
      evaluated_kind: result.kind().to_string(),
      actual,
      expected,
    });
  }
  if let Some(error) = violation_error {
    let err = MechError::new(
      InvariantViolationError{
        invariant_name: invariant_name.clone(),
        expression: invariant_expression,
        lhs_addr,
        lhs_value,
        operator,
        rhs_addr,
        rhs_value,
        reason: error,
        evaluated_kind: result.kind().to_string(),
      },
      None
    ).with_compiler_loc().with_tokens(inv_def.expression.tokens());
    p.state.borrow_mut().invariant_violations.push(InvariantViolation { id: invariant_id, error: err });
  }
  Ok(result)
}

#[cfg(feature = "invariant_define")]
fn tokens_to_string(tokens: &[Token]) -> String {
  tokens.iter().flat_map(|t| t.chars.clone()).collect::<String>()
}

#[cfg(feature = "invariant_define")]
fn value_to_ref(value: Value) -> ValRef {
  match value {
    Value::MutableReference(r) => r.clone(),
    other => Ref::new(other),
  }
}

#[cfg(feature = "invariant_define")]
fn invariant_operand_refs(inv_def: &InvariantDefine, p: &Interpreter) -> Option<(Option<ValRef>, Option<FormulaOperator>, Option<ValRef>)> {
  let factor = match &inv_def.expression {
    Expression::Formula(f) => f,
    _ => return None,
  };
  let term = match factor {
    Factor::Term(t) => t,
    _ => return None,
  };
  let (op, rhs_factor) = term.rhs.first()?;
  let lhs_value = expression(&Expression::Formula(term.lhs.clone()), None, p).ok().map(value_to_ref);
  let rhs_value = expression(&Expression::Formula(rhs_factor.clone()), None, p).ok().map(value_to_ref);
  Some((lhs_value, Some(op.clone()), rhs_value))
}

#[cfg(all(feature = "enum", feature = "atom"))]
fn value_matches_enum_variant(value: &Value, enum_id: u64, state: &ProgramState) -> bool {
  let my_enum = match state.enums.get(&enum_id) {
    Some(enm) => enm,
    None => return false,
  };
  let names_brrw = my_enum.names.borrow();
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
      let (_, declared_payload_kind) = match my_enum.variants.iter().find(|(known_variant, _)| {
        *known_variant == *variant_id
      }) {
        Some(v) => v,
        None => return false,
      };
      match (payload, declared_payload_kind) {
        (None, None) => true,
        (Some(payload_value), Some(Value::Kind(expected_kind))) => match expected_kind {
          ValueKind::Enum(inner_enum_id, _) => value_matches_enum_variant(payload_value, *inner_enum_id, state),
          _ => {
            payload_value.kind() == expected_kind.clone() ||
            ConvertKind{}.compile(&vec![payload_value.clone(), Value::Kind(expected_kind.clone())]).is_ok()
          }
        },
        _ => false,
      }
    }
    Value::Atom(atom_variant) => {
      let atom_brrw = atom_variant.borrow();
      let variant_id = atom_brrw.id();
      let atom_name = atom_brrw.name();
      my_enum.variants.iter().any(|(known_variant, payload_kind)| {
        atom_matches_variant(*known_variant, variant_id, &atom_name) && payload_kind.is_none()
      })
    }
    #[cfg(feature = "tuple")]
    Value::Tuple(tuple_val) => {
      let tuple_brrw = tuple_val.borrow();
      if tuple_brrw.elements.len() != 2 {
        return false;
      }
      let variant_atom = match tuple_brrw.elements[0].as_ref() {
        Value::Atom(atom) => atom.borrow(),
        _ => return false,
      };
      let variant_id = variant_atom.id();
      let atom_name = variant_atom.name();
      let payload = tuple_brrw.elements[1].as_ref();
      let (_, declared_payload_kind) = match my_enum.variants.iter().find(|(known_variant, _)| {
        atom_matches_variant(*known_variant, variant_id, &atom_name)
      }) {
        Some(v) => v,
        None => return false,
      };
      match declared_payload_kind {
        Some(Value::Kind(expected_kind)) => match expected_kind {
          ValueKind::Enum(inner_enum_id, _) => value_matches_enum_variant(payload, *inner_enum_id, state),
          _ => {
            payload.kind() == expected_kind.clone() ||
            ConvertKind{}.compile(&vec![payload.clone(), Value::Kind(expected_kind.clone())]).is_ok()
          }
        },
        _ => false,
      }
    }
    _ => false,
  }
}

#[cfg(feature = "variable_define")]
pub fn variable_define(var_def: &VariableDefine, p: &Interpreter) -> MResult<Value> {
  if var_def.var.context.is_some() {
    return Err(MechError::new(AddressedAssignmentUnsupported, None)
      .with_compiler_loc()
      .with_tokens(var_def.var.tokens()));
  }
  let var_id = var_def.var.name.hash();
  let var_name = var_def.var.name.to_string();
  {
    let symbols = p.symbols();
    if symbols.borrow().contains(var_id) {
      return Err(MechError::new(
        VariableAlreadyDefinedError { id: var_id },
        None
      ).with_compiler_loc().with_tokens(var_def.var.name.tokens()));
    }
  }
  let plan = p.plan();
  #[cfg(feature = "subscript_formula")]
  reset_current_string_access_expression_live(p);
  let mut result = expression(&var_def.expression, None, p)?;
  #[cfg(feature = "subscript_formula")]
  let string_access_result_is_live = take_current_string_access_expression_live(p);
  #[cfg(all(feature = "kind_annotation", feature = "convert"))]
  if let Some(knd_anntn) =  &var_def.var.kind {
    let knd = kind_annotation(&knd_anntn.kind,p)?;
    let mut state_brrw = &mut p.state.borrow_mut();
    let target_knd = knd.to_value_kind(&mut state_brrw.kinds)?;
    // Do kind checking
    match (&result, &target_knd) {
      // Atom is a variant of an enum
      #[cfg(all(feature = "atom", feature = "enum"))]
      (Value::Atom(atom_variant), ValueKind::Enum(enum_id, target_enum_variant_name)) => {
        let atom_name = atom_variant.borrow().name();
        if !value_matches_enum_variant(&result, *enum_id, &*state_brrw) {
          return Err(MechError::new(
            UnableToConvertAtomToEnumVariantError { atom_name: atom_name.clone(), target_enum_variant_name: target_enum_variant_name.clone() },
            None
          ).with_compiler_loc().with_tokens(var_def.expression.tokens()));
        }
      }
      #[cfg(all(feature = "tuple", feature = "atom", feature = "enum"))]
      (Value::Tuple(tuple_val), ValueKind::Enum(enum_id, target_enum_variant_name)) => {
        let atom_name = format!("{:?}", tuple_val);
        if !value_matches_enum_variant(&result, *enum_id, &*state_brrw) {
          return Err(MechError::new(
            UnableToConvertAtomToEnumVariantError { atom_name, target_enum_variant_name: target_enum_variant_name.clone() },
            None
          ).with_compiler_loc().with_tokens(var_def.expression.tokens()));
        }
      }
      // Atoms can't convert into anything else.
      #[cfg(feature = "atom")]
      (Value::Atom(given_variant_id), target_kind) => {
        return Err(MechError::new(
          UnableToConvertAtomError { atom_id: given_variant_id.borrow().0.0},
          None
        ).with_compiler_loc().with_tokens(var_def.expression.tokens()));
      }
      #[cfg(feature = "record")]
      (Value::Record(rec), ref target_kind @ ValueKind::Record(target_rec_knd)) => {
        let rec_brrw = rec.borrow();
        let rec_knd = rec_brrw.kind();
        if &rec_knd != *target_kind {
          return Err(MechError::new(
            UnableToConvertRecordError { source_record_kind: rec_knd.clone(), target_record_kind: (*target_kind).clone() },
            None
          ).with_compiler_loc().with_tokens(var_def.expression.tokens()));
        }
      }
      #[cfg(feature = "matrix")]
      (Value::MutableReference(v), ValueKind::Matrix(target_matrix_knd,_)) => {
        let value = v.borrow().clone();
        if value.is_matrix() {
          result = execute_initialized_indexed_compiler(&plan, &ConvertMatToMat{}, vec![result.clone(), Value::Kind(target_knd.clone())])?;
        } else {
          let value_kind = value.kind();
          if value_kind.deref_kind() != target_matrix_knd.as_ref().clone() && value_kind != *target_matrix_knd.clone() {
            result = execute_initialized_indexed_compiler(&plan, &ConvertKind{}, vec![result.clone(), Value::Kind(target_matrix_knd.as_ref().clone())])?;
          };
          result = execute_initialized_indexed_compiler(&plan, &ConvertScalarToMat{}, vec![result.clone(), Value::Kind(target_knd.clone())])?;
        }
      }
      #[cfg(feature = "matrix")]
      (value, ValueKind::Matrix(target_matrix_knd,_)) => {
        if value.is_matrix() {
          result = execute_initialized_indexed_compiler(&plan, &ConvertMatToMat{}, vec![result.clone(), Value::Kind(target_knd.clone())])?;
        } else {
          let value_kind = value.kind();
          if value_kind.deref_kind() != target_matrix_knd.as_ref().clone() && value_kind != *target_matrix_knd.clone() {
            result = execute_initialized_indexed_compiler(&plan, &ConvertKind{}, vec![result.clone(), Value::Kind(target_matrix_knd.as_ref().clone())])?;
          };
          result = execute_initialized_indexed_compiler(&plan, &ConvertScalarToMat{}, vec![result.clone(), Value::Kind(target_knd.clone())])?;
        }
      }
      // Kind isn't checked
      x => {
        result = execute_initialized_indexed_compiler(&plan, &ConvertKind{}, vec![result.clone(), Value::Kind(target_knd)])?;
      },
    };
    let detached_result = detach_variable_value(&result);
    #[cfg(feature = "subscript_formula")]
    if string_access_result_is_live {
      mark_string_access_value_live(p, &detached_result);
    }
    // Save symbol to interpreter
    let val_ref = state_brrw.save_symbol(var_id, var_name.clone(), detached_result.clone(), var_def.mutable);
    // Add variable define step to plan
    let var_define_arguments = vec![detached_result.clone(), Value::String(Ref::new(var_name.clone())), Value::Bool(Ref::new(var_def.mutable))];
    let var_def_fxn = VarDefine{}.compile(&var_define_arguments)?;
    plan.register_function(var_def_fxn, &[])?;
    return Ok(detached_result);
  } 
  let mut state_brrw = p.state.borrow_mut();
  let detached_result = detach_variable_value(&result);
  #[cfg(feature = "subscript_formula")]
  if string_access_result_is_live {
    mark_string_access_value_live(p, &detached_result);
  }
  // Save symbol to interpreter
  let val_ref = state_brrw.save_symbol(var_id,var_name.clone(),detached_result.clone(),var_def.mutable);
  // Add variable define step to plan
  let var_define_arguments = vec![detached_result.clone(), Value::String(Ref::new(var_name.clone())), Value::Bool(Ref::new(var_def.mutable))];
  let var_def_fxn = VarDefine{}.compile(&var_define_arguments)?;
  plan.register_function(var_def_fxn, &[])?;
  return Ok(detached_result);
}

#[cfg(feature = "state_machines")]
pub fn fsm_declare(fsm_decl: &FsmDeclare, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let result = crate::state_machines::execute_fsm_pipe(&fsm_decl.pipe, env, p)?;
  let id = fsm_decl.fsm.name.hash();
  let name = fsm_decl.fsm.name.to_string();
  #[cfg(feature = "symbol_table")]
  {
    let symbols = p.symbols();
    let mut symbols_brrw = symbols.borrow_mut();
    symbols_brrw.insert(id, detach_variable_value(&result), false);
    symbols_brrw.dictionary.borrow_mut().insert(id, name);
  }
  Ok(result)
}

fn detach_variable_value(value: &Value) -> Value {
  match value {
    Value::MutableReference(reference) => detach_variable_value(&reference.borrow()),
    _ => value.clone(),
  }
}

macro_rules! op_assign {
  ($fxn_name:ident, $op:tt) => {
    paste!{
      pub fn $fxn_name(sbscrpt: &Subscript, sink: &Value, source: &Value, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
        let plan = p.plan();
        match sbscrpt {
          Subscript::Dot(x) => {
            todo!()
          },
          Subscript::DotInt(x) => {
            todo!()
          },
          Subscript::Swizzle(x) => {
            todo!()
          },
          Subscript::Bracket(subs) => {
            let mut fxn_input = vec![sink.clone()];
            match &subs[..] {
              [Subscript::Formula(ix)] => {
                fxn_input.push(source.clone());
                let ixes = subscript_formula_ix(&subs[0], env, p)?;
                let shape = ixes.shape();
                fxn_input.push(ixes);
                match shape[..] {
                  [1,1] => { plan.borrow_mut().push(MatrixAssignScalar{}.compile(&fxn_input)?); }
                  [1,n] => { plan.borrow_mut().push([<$op AssignRange>]{}.compile(&fxn_input)?); }
                  [n,1] => { plan.borrow_mut().push([<$op AssignRange>]{}.compile(&fxn_input)?); }
                  _ => todo!(),
                }
              },
              [Subscript::Formula(ix1),Subscript::All] => {
                fxn_input.push(source.clone());
                let ix = subscript_formula_ix(&subs[0], env, p)?;
                let shape = ix.shape();
                fxn_input.push(ix);
                fxn_input.push(Value::IndexAll);
                match shape[..] {
                  [1,1] => { plan.borrow_mut().push(MatrixAssignScalarAll{}.compile(&fxn_input)?); }
                  [1,n] => { plan.borrow_mut().push([<$op AssignRangeAll>]{}.compile(&fxn_input)?); }
                  [n,1] => { plan.borrow_mut().push([<$op AssignRangeAll>]{}.compile(&fxn_input)?); }
                  _ => todo!(),
                }
              },
              [Subscript::Range(ix)] => {
                fxn_input.push(source.clone());
                let ixes = subscript_range(&subs[0], env, p)?;
                fxn_input.push(ixes);
                plan.borrow_mut().push([<$op AssignRange>]{}.compile(&fxn_input)?);
              },
              [Subscript::Range(ix), Subscript::All] => {
                fxn_input.push(source.clone());
                let ixes = subscript_range(&subs[0], env, p)?;
                fxn_input.push(ixes);
                fxn_input.push(Value::IndexAll);
                plan.borrow_mut().push([<$op AssignRangeAll>]{}.compile(&fxn_input)?);
              },
              x => todo!("{:?}", x),
            };
            let plan_brrw = plan.borrow();
            let mut new_fxn = &plan_brrw.last().unwrap();
            new_fxn.solve();
            let res = new_fxn.out();
            return Ok(res);
          },
          Subscript::Brace(x) => todo!(),
          x => todo!("{:?}", x),
        }
      }
    }
  };
}

#[cfg(feature = "math_add_assign")]
op_assign!(add_assign, Add);
#[cfg(feature = "math_sub_assign")]
op_assign!(sub_assign, Sub);
#[cfg(feature = "math_div_assign")]
op_assign!(mul_assign, Mul);
#[cfg(feature = "math_mul_assign")]
op_assign!(div_assign, Div);
//#[cfg(feature = "math_pow")]
//op_assign!(pow_assign, Pow);

#[cfg(all(feature = "subscript", feature = "assign"))]
pub fn subscript_ref(sbscrpt: &Subscript, sink: &Value, source: &Value, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let symbols = p.symbols();
  let functions = p.functions();
  match sbscrpt {
    Subscript::Dot(x) => {
      let key = x.hash();
      let fxn_input: Vec<Value> = vec![sink.clone(), source.clone(), Value::Id(key)];
      let new_fxn = AssignColumn{}.compile(&fxn_input)?;
      new_fxn.solve();
      let res = new_fxn.out();
      plan.borrow_mut().push(new_fxn);
      return Ok(res);
    },
    #[cfg(feature = "tuple")]
    Subscript::DotInt(x) => {
      let ix = real(x, p)?.as_index()?;
      let mut fxn_input: Vec<Value> = vec![sink.clone(), source.clone(), ix.clone()];
      let new_fxn = TupleAssignScalar{}.compile(&fxn_input)?;
      new_fxn.solve();
      let res = new_fxn.out();
      plan.borrow_mut().push(new_fxn);
      return Ok(res);
    },
    Subscript::Swizzle(x) => {
      unreachable!()
    },
    Subscript::Bracket(subs) => {
      let mut fxn_input = vec![sink.clone()];
      match &subs[..] {
        #[cfg(feature = "subscript_formula")]
        [Subscript::Formula(ix)] => {
          fxn_input.push(source.clone());
          let ixes = subscript_formula_ix(&subs[0], env, p)?;
          let shape = ixes.shape();
          fxn_input.push(ixes);
          match shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => { plan.borrow_mut().push(MatrixAssignScalar{}.compile(&fxn_input)?); }
            #[cfg(all(feature = "matrix", feature = "subscript_range", feature = "assign"))]
            [1,n] => { plan.borrow_mut().push(MatrixAssignRange{}.compile(&fxn_input)?); }
            #[cfg(all(feature = "matrix", feature = "subscript_range", feature = "assign"))]
            [n,1] => { plan.borrow_mut().push(MatrixAssignRange{}.compile(&fxn_input)?); }
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::Range(ix)] => {
          fxn_input.push(source.clone());
          let ixes = subscript_range(&subs[0], env, p)?;
          fxn_input.push(ixes);
          plan.borrow_mut().push(MatrixAssignRange{}.compile(&fxn_input)?);
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::All] => {
          fxn_input.push(source.clone());
          fxn_input.push(Value::IndexAll);
          plan.borrow_mut().push(MatrixAssignAll{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::All] => todo!(),
        #[cfg(feature = "subscript_formula")]
        [Subscript::Formula(ix1),Subscript::Formula(ix2)] => {
          fxn_input.push(source.clone());
          let result1 = subscript_formula_ix(&subs[0], env, p)?;
          let result2 = subscript_formula_ix(&subs[1], env, p)?;
          let shape1 = result1.shape();
          let shape2 = result2.shape();
          fxn_input.push(result1);
          fxn_input.push(result2);
          match ((shape1[0],shape1[1]),(shape2[0],shape2[1])) {
            #[cfg(feature = "matrix")]
            ((1,1),(1,1)) => { plan.borrow_mut().push(MatrixAssignScalarScalar{}.compile(&fxn_input)?); }
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            ((1,1),(m,1)) => { plan.borrow_mut().push(MatrixAssignScalarRange{}.compile(&fxn_input)?); }
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            ((n,1),(1,1)) => { plan.borrow_mut().push(MatrixAssignRangeScalar{}.compile(&fxn_input)?); }
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            ((n,1),(m,1)) => { plan.borrow_mut().push(MatrixAssignRangeRange{}.compile(&fxn_input)?); }
            _ => unreachable!(),
          }          
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::Range(ix1),Subscript::Range(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_range(&subs[0], env, p)?;
          fxn_input.push(result);
          let result = subscript_range(&subs[1], env, p)?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixAssignRangeRange{}.compile(&fxn_input)?);
        },
        #[cfg(all(feature = "matrix", feature = "subscript_formula"))]
        [Subscript::All,Subscript::Formula(ix2)] => {
          fxn_input.push(source.clone());
          fxn_input.push(Value::IndexAll);
          let ix = subscript_formula_ix(&subs[1], env, p)?;
          let shape = ix.shape();
          fxn_input.push(ix);
          match shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => { plan.borrow_mut().push(MatrixAssignAllScalar{}.compile(&fxn_input)?); }
            #[cfg(feature = "matrix")]
            [1,n] => { plan.borrow_mut().push(MatrixAssignAllRange{}.compile(&fxn_input)?); }
            #[cfg(feature = "matrix")]
            [n,1] => { plan.borrow_mut().push(MatrixAssignAllRange{}.compile(&fxn_input)?); }
            _ => todo!(),
          }
        }
        #[cfg(feature = "subscript_formula")]
        [Subscript::Formula(ix1),Subscript::All] => {
          fxn_input.push(source.clone());
          let ix = subscript_formula_ix(&subs[0], env, p)?;
          let shape = ix.shape();
          fxn_input.push(ix);
          fxn_input.push(Value::IndexAll);
          match shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => { plan.borrow_mut().push(MatrixAssignScalarAll{}.compile(&fxn_input)?); }
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            [1,n] => { plan.borrow_mut().push(MatrixAssignRangeAll{}.compile(&fxn_input)?); }
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            [n,1] => { plan.borrow_mut().push(MatrixAssignRangeAll{}.compile(&fxn_input)?); }
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "subscript_formula", feature = "subscript_range"))]
        [Subscript::Range(ix1),Subscript::Formula(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_range(&subs[0], env, p)?;
          fxn_input.push(result);
          let result = subscript_formula_ix(&subs[1], env, p)?;
          let shape = result.shape();
          fxn_input.push(result);
          match &shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => { plan.borrow_mut().push(MatrixAssignRangeScalar{}.compile(&fxn_input)?); }
            #[cfg(feature = "matrix")]
            [1,n] => { plan.borrow_mut().push(MatrixAssignRangeRange{}.compile(&fxn_input)?); }
            #[cfg(feature = "matrix")]
            [n,1] => { plan.borrow_mut().push(MatrixAssignRangeRange{}.compile(&fxn_input)?); }
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "subscript_formula", feature = "subscript_range"))]
        [Subscript::Formula(ix1),Subscript::Range(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_formula_ix(&subs[0], env, p)?;
          let shape = result.shape();
          fxn_input.push(result);
          let result = subscript_range(&subs[1], env, p)?;
          fxn_input.push(result);
          match &shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => { plan.borrow_mut().push(MatrixAssignScalarRange{}.compile(&fxn_input)?); }
            #[cfg(feature = "matrix")]
            [1,n] => { plan.borrow_mut().push(MatrixAssignRangeRange{}.compile(&fxn_input)?); }
            #[cfg(feature = "matrix")]
            [n,1] => { plan.borrow_mut().push(MatrixAssignRangeRange{}.compile(&fxn_input)?); }
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::All,Subscript::Range(ix2)] => {
          fxn_input.push(source.clone());
          fxn_input.push(Value::IndexAll);
          let result = subscript_range(&subs[1], env, p)?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixAssignAllRange{}.compile(&fxn_input)?);
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::Range(ix1),Subscript::All] => {
          fxn_input.push(source.clone());
          let result = subscript_range(&subs[0], env, p)?;
          fxn_input.push(result);
          fxn_input.push(Value::IndexAll);
          plan.borrow_mut().push(MatrixAssignRangeAll{}.compile(&fxn_input)?);
        },
        _ => unreachable!(),
      };
      let plan_brrw = plan.borrow();
      let mut new_fxn = &plan_brrw.last().unwrap();
      new_fxn.solve();
      let res = new_fxn.out();
      return Ok(res);
    },
    Subscript::Brace(subs) => {
      let mut fxn_input = vec![sink.clone()];
      match &subs[..] {
        #[cfg(feature = "subscript_formula")]
        [Subscript::Formula(ix)] => {
          fxn_input.push(source.clone());
          let ixes = subscript_formula(&subs[0], env, p)?;
          let shape = ixes.shape();
          fxn_input.push(ixes);
          match shape[..] {
            #[cfg(feature = "map")]
            [1,1] => { plan.borrow_mut().push(MapAssignScalar{}.compile(&fxn_input)?); }
            //#[cfg(all(feature = "matrix", feature = "subscript_range", feature = "assign"))]
            //[1,n] => { plan.borrow_mut().push(MatrixAssignRange{}.compile(&fxn_input)?); }
            //#[cfg(all(feature = "matrix", feature = "subscript_range", feature = "assign"))]
            //[n,1] => { plan.borrow_mut().push(MatrixAssignRange{}.compile(&fxn_input)?); }
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::Range(ix)] => {
          todo!();
          //fxn_input.push(source.clone());
          //let ixes = subscript_range(&subs[0], env, p)?;
          //fxn_input.push(ixes);
          //plan.borrow_mut().push(MatrixAssignRange{}.compile(&fxn_input)?);
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::All] => {
          todo!();
          //fxn_input.push(source.clone());
          //fxn_input.push(Value::IndexAll);
          //plan.borrow_mut().push(MatrixAssignAll{}.compile(&fxn_input)?);
        },
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


#[derive(Debug, Clone)]
pub struct AddressedAssignmentUnsupported;
impl MechErrorKind for AddressedAssignmentUnsupported {
  fn name(&self) -> &str { "AddressedAssignmentUnsupported" }
  fn message(&self) -> String {
    "addressed assignment is not supported yet".to_string()
  }
}

#[derive(Debug, Clone)]
pub struct UnableToConvertAtomToEnumVariantError {
  pub atom_name: String,
  pub target_enum_variant_name: String,
}
impl MechErrorKind for UnableToConvertAtomToEnumVariantError {
  fn name(&self) -> &str {
    "UnableToConvertAtomToEnumVariant"
  }
  fn message(&self) -> String {
    format!("Unable to convert atom variant `{} to enum <{}>", self.atom_name, self.target_enum_variant_name)
  }
}

#[derive(Debug, Clone)]
pub struct UnableToConvertAtomError {
  pub atom_id: u64,
}
impl MechErrorKind for UnableToConvertAtomError {
  fn name(&self) -> &str {
    "UnableToConvertAtom"
  }
  fn message(&self) -> String {
    format!("Unable to atom  {}", self.atom_id)
  }
}

#[derive(Debug, Clone)]
pub struct VariableAlreadyDefinedError {
  pub id: u64,
}
impl MechErrorKind for VariableAlreadyDefinedError {
  fn name(&self) -> &str { "VariableAlreadyDefined" }
  fn message(&self) -> String {
    format!("Variable already defined: {}", self.id)
  }
}

#[derive(Debug, Clone)]
pub struct UndefinedVariableError {
  pub id: u64,
  pub name: String,
}
impl MechErrorKind for UndefinedVariableError {
  fn name(&self) -> &str { "UndefinedVariable" }

  fn message(&self) -> String {
    format!("Undefined variable `{}` (id: {})", self.name, self.id)
  }
}

#[derive(Debug, Clone)]
pub struct NotMutableError {
  pub id: u64,
}
impl MechErrorKind for NotMutableError {
  fn name(&self) -> &str { "NotMutable" }
  fn message(&self) -> String {
    format!("Variable is not mutable: {}", self.id)
  }
}

#[cfg(feature = "record")]
#[derive(Debug, Clone)]
pub struct UnableToConvertRecordError {
  pub source_record_kind: ValueKind,
  pub target_record_kind: ValueKind,
}
#[cfg(feature = "record")]
impl MechErrorKind for UnableToConvertRecordError {
  fn name(&self) -> &str {
    "UnableToConvertRecord"
  }
  fn message(&self) -> String {
    format!("Unable to convert record of kind `{:?}` to record of kind `{:?}`", self.source_record_kind, self.target_record_kind)
  }
}

#[cfg(all(
  test,
  feature = "variable_define",
  feature = "f64",
  feature = "string",
  feature = "bool",
))]
mod variable_define_dependency_tests {
  use super::*;

  #[test]
  fn var_define_registration_has_no_reactive_inputs() {
    let plan = Plan::new();
    let value = Ref::new(1.0);
    let value_cell = ReactiveCellId::new(value.id());
    let arguments = vec![
      Value::F64(value),
      Value::String(Ref::new("defined value".to_string())),
      Value::Bool(Ref::new(false)),
    ];
    let function = VarDefine {}.compile(&arguments).unwrap();

    plan.register_function(function, &[]).unwrap();

    let plan_borrow = plan.borrow();
    let node = plan_borrow.node(0).unwrap();
    assert_eq!(plan_borrow.len(), 1);
    assert!(node.inputs.is_empty());
    assert!(plan_borrow.reactive_consumers.is_empty());
    assert!(plan_borrow.sampled_consumers.is_empty());
    assert!(node.outputs.contains(&value_cell));
  }
}

#[cfg(all(
  test,
  feature = "functions",
  feature = "variables",
  feature = "variable_define",
  feature = "variable_assign",
  feature = "assign",
  feature = "f64",
  feature = "program",
  feature = "compiler",
))]
mod whole_assignment_register_tests {
  use super::*;

  fn symbol(interpreter: &Interpreter, name: &str) -> Value {
    interpreter.symbols().borrow().get(hash_str(name))
      .unwrap_or_else(|| panic!("missing symbol {name}"))
      .borrow()
      .clone()
  }

  fn root_cell(value: &Value) -> ReactiveCellId {
    let cells = value.reactive_root_cell_ids();
    assert_eq!(cells.len(), 1);
    cells[0]
  }

  fn register_node_id_for_output(interpreter: &Interpreter, output_cell: ReactiveCellId) -> ReactiveNodeId {
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node_ids = plan.nodes.iter()
      .filter(|node| node.kind == ReactiveNodeKind::Register && node.outputs == vec![output_cell])
      .map(|node| node.id)
      .collect::<Vec<_>>();
    assert_eq!(node_ids.len(), 1);
    node_ids[0]
  }

  #[derive(Debug, PartialEq, Eq)]
  struct RegisterGraphShape {
    output_count: usize,
    input_kinds: Vec<ReactiveDependencyKind>,
    output_is_first_input: bool,
    source_is_second_input: bool,
    output_is_sampled_consumer: bool,
    output_is_reactive_consumer: bool,
    source_is_reactive_consumer: bool,
    source_is_sampled_consumer: bool,
  }

  fn distinct_assignment_graph_shape(interpreter: &Interpreter, target_name: &str, source_name: &str) -> RegisterGraphShape {
    let target_cell = root_cell(&symbol(interpreter, target_name));
    let source_cell = root_cell(&symbol(interpreter, source_name));
    assert_ne!(target_cell, source_cell);
    let node_id = register_node_id_for_output(interpreter, target_cell);
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert_eq!(node.outputs, vec![target_cell]);
    assert_eq!(node.inputs.len(), 2);
    assert_eq!(node.inputs[0].cell, target_cell);
    assert_eq!(node.inputs[0].kind, ReactiveDependencyKind::Sampled);
    assert_eq!(node.inputs[1].cell, source_cell);
    assert_eq!(node.inputs[1].kind, ReactiveDependencyKind::Reactive);
    RegisterGraphShape {
      output_count: node.outputs.len(),
      input_kinds: node.inputs.iter().map(|input| input.kind).collect(),
      output_is_first_input: node.inputs[0].cell == target_cell,
      source_is_second_input: node.inputs[1].cell == source_cell,
      output_is_sampled_consumer: plan.sampled_consumers_for(target_cell).contains(&node_id),
      output_is_reactive_consumer: plan.reactive_consumers_for(target_cell).contains(&node_id),
      source_is_reactive_consumer: plan.reactive_consumers_for(source_cell).contains(&node_id),
      source_is_sampled_consumer: plan.sampled_consumers_for(source_cell).contains(&node_id),
    }
  }

  fn decoded_assignment_graph_shape(interpreter: &Interpreter, output: &Value) -> RegisterGraphShape {
    let resolved_output = match output {
      Value::MutableReference(reference) => reference.borrow().clone(),
      other => other.clone(),
    };
    let output_cell = root_cell(&resolved_output);
    let node_id = register_node_id_for_output(interpreter, output_cell);
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert_eq!(node.outputs, vec![output_cell]);
    assert_eq!(node.outputs.len(), 1);
    assert_eq!(node.inputs.len(), 2);
    assert_eq!(node.inputs[0].cell, output_cell);
    assert_eq!(node.inputs[0].kind, ReactiveDependencyKind::Sampled);
    assert_ne!(node.inputs[1].cell, output_cell);
    assert_eq!(node.inputs[1].kind, ReactiveDependencyKind::Reactive);
    let source_cell = node.inputs[1].cell;
    RegisterGraphShape {
      output_count: node.outputs.len(),
      input_kinds: node.inputs.iter().map(|input| input.kind).collect(),
      output_is_first_input: node.inputs[0].cell == output_cell,
      source_is_second_input: node.inputs[1].cell == source_cell,
      output_is_sampled_consumer: plan.sampled_consumers_for(output_cell).contains(&node_id),
      output_is_reactive_consumer: plan.reactive_consumers_for(output_cell).contains(&node_id),
      source_is_reactive_consumer: plan.reactive_consumers_for(source_cell).contains(&node_id),
      source_is_sampled_consumer: plan.sampled_consumers_for(source_cell).contains(&node_id),
    }
  }

  fn expected_distinct_assignment_shape() -> RegisterGraphShape {
    RegisterGraphShape {
      output_count: 1,
      input_kinds: vec![ReactiveDependencyKind::Sampled, ReactiveDependencyKind::Reactive],
      output_is_first_input: true,
      source_is_second_input: true,
      output_is_sampled_consumer: true,
      output_is_reactive_consumer: false,
      source_is_reactive_consumer: true,
      source_is_sampled_consumer: false,
    }
  }

  #[test]
  fn whole_variable_assignment_registers_state_node() {
    let source = "~x := 1.0; y := 2.0; x = y; x";
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    let output = interpreter.interpret(&tree).unwrap();
    assert_eq!(*output.as_f64().unwrap().borrow(), 2.0);
    assert_eq!(distinct_assignment_graph_shape(&interpreter, "x", "y"), expected_distinct_assignment_shape());
  }

  #[cfg(feature = "math_add_assign")]
  #[test]
  fn whole_add_assignment_registers_state_node() {
    let source = "~x := 1.0; y := 2.0; x += y; x";
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    let output = interpreter.interpret(&tree).unwrap();
    assert_eq!(*output.as_f64().unwrap().borrow(), 3.0);
    assert_eq!(distinct_assignment_graph_shape(&interpreter, "x", "y"), expected_distinct_assignment_shape());
  }

  #[cfg(feature = "math_add_assign")]
  #[test]
  fn whole_add_assignment_alias_is_sampled_once() {
    let source = "~x := 2.0; x += x; x";
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    let output = interpreter.interpret(&tree).unwrap();
    assert_eq!(*output.as_f64().unwrap().borrow(), 4.0);
    let x_cell = root_cell(&symbol(&interpreter, "x"));
    let node_id = register_node_id_for_output(&interpreter, x_cell);
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert_eq!(node.outputs, vec![x_cell]);
    assert_eq!(node.inputs.len(), 1);
    assert_eq!(node.inputs[0].cell, x_cell);
    assert_eq!(node.inputs[0].kind, ReactiveDependencyKind::Sampled);
    assert!(plan.sampled_consumers_for(x_cell).contains(&node_id));
    assert!(!plan.reactive_consumers_for(x_cell).contains(&node_id));
  }

  #[test]
  fn decoded_whole_variable_assignment_matches_source_graph() {
    let source = "~x := 1.0; y := 2.0; x = y; x";
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut source_interpreter = Interpreter::new_with_full_stdlib(0);
    let source_output = source_interpreter.interpret(&tree).unwrap();
    assert_eq!(*source_output.as_f64().unwrap().borrow(), 2.0);
    let source_shape = distinct_assignment_graph_shape(&source_interpreter, "x", "y");
    let bytecode = source_interpreter.compile().unwrap();
    let program = ParsedProgram::from_bytes(&bytecode).unwrap();
    let mut decoded_interpreter = Interpreter::new_with_full_stdlib(0);
    let decoded_output = decoded_interpreter.run_program(&program).unwrap();
    assert_eq!(*decoded_output.as_f64().unwrap().borrow(), 2.0);
    let decoded_shape = decoded_assignment_graph_shape(&decoded_interpreter, &decoded_output);
    assert_eq!(source_shape, expected_distinct_assignment_shape());
    assert_eq!(decoded_shape, expected_distinct_assignment_shape());
    assert_eq!(source_shape, decoded_shape);
  }

  #[cfg(feature = "math_add_assign")]
  #[test]
  fn decoded_whole_add_assignment_matches_source_graph() {
    let source = "~x := 1.0; y := 2.0; x += y; x";
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut source_interpreter = Interpreter::new_with_full_stdlib(0);
    let source_output = source_interpreter.interpret(&tree).unwrap();
    assert_eq!(*source_output.as_f64().unwrap().borrow(), 3.0);
    let source_shape = distinct_assignment_graph_shape(&source_interpreter, "x", "y");
    let bytecode = source_interpreter.compile().unwrap();
    let program = ParsedProgram::from_bytes(&bytecode).unwrap();
    let mut decoded_interpreter = Interpreter::new_with_full_stdlib(0);
    let decoded_output = decoded_interpreter.run_program(&program).unwrap();
    assert_eq!(*decoded_output.as_f64().unwrap().borrow(), 3.0);
    let decoded_shape = decoded_assignment_graph_shape(&decoded_interpreter, &decoded_output);
    assert_eq!(source_shape, expected_distinct_assignment_shape());
    assert_eq!(decoded_shape, expected_distinct_assignment_shape());
    assert_eq!(source_shape, decoded_shape);
  }

  #[cfg(all(feature = "matrix", feature = "row_vectord"))]
  #[test]
  fn whole_matrix_assignment_uses_root_cells() {
    let source = "~x := [1.0 2.0]; y := [3.0 4.0]; x = y; x";
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    let output = interpreter.interpret(&tree).unwrap();
    let x = symbol(&interpreter, "x");
    let y = symbol(&interpreter, "y");
    let x_root_cells = x.reactive_root_cell_ids();
    let y_root_cells = y.reactive_root_cell_ids();
    assert_eq!(x_root_cells.len(), 1);
    assert_eq!(y_root_cells.len(), 1);
    let x_cell = x_root_cells[0];
    let y_cell = y_root_cells[0];
    let node_id = register_node_id_for_output(&interpreter, x_cell);
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert_eq!(node.outputs, vec![x_cell]);
    assert_eq!(node.outputs.len(), 1);
    assert_eq!(node.inputs.len(), 2);
    assert_eq!(node.inputs[0].cell, x_cell);
    assert_eq!(node.inputs[0].kind, ReactiveDependencyKind::Sampled);
    assert_eq!(node.inputs[1].cell, y_cell);
    assert_eq!(node.inputs[1].kind, ReactiveDependencyKind::Reactive);
    assert!(plan.sampled_consumers_for(x_cell).contains(&node_id));
    assert!(!plan.reactive_consumers_for(x_cell).contains(&node_id));
    assert!(plan.reactive_consumers_for(y_cell).contains(&node_id));
    assert!(!plan.sampled_consumers_for(y_cell).contains(&node_id));
    let resolved_output = match output {
      Value::MutableReference(reference) => reference.borrow().clone(),
      other => other,
    };
    assert_eq!(resolved_output, y);
  }
}

#[cfg(all(test, feature = "program", feature = "compiler", feature = "functions", feature = "variables", feature = "variable_define", feature = "variable_assign", feature = "assign", feature = "f64", feature = "math", feature = "math_add_assign"))]
mod register_commit_integration_tests {
  use super::*;
  fn symbol(i:&Interpreter,n:&str)->Value{i.symbols().borrow().get(hash_str(n)).unwrap().borrow().clone()}
  fn cell(i:&Interpreter,n:&str)->ReactiveCellId {let c=symbol(i,n).reactive_root_cell_ids();assert_eq!(c.len(),1);c[0]}
  fn value(i:&Interpreter,n:&str)->f64 {*symbol(i,n).as_f64().unwrap().borrow()}
  fn set(i:&Interpreter,n:&str,v:f64){*symbol(i,n).as_f64().unwrap().borrow_mut()=v;}
  fn register(i:&Interpreter,c:ReactiveCellId)->ReactiveNodeId {let p=i.plan();let p=p.borrow();let v=p.nodes.iter().filter(|n|n.kind==ReactiveNodeKind::Register&&n.outputs.contains(&c)).map(|n|n.id).collect::<Vec<_>>();assert_eq!(v.len(),1);v[0]}
  #[test] fn register_commit_plain_assignment_updates_register_only() {let t=mech_syntax::parser::parse("~x := 1.0\ny := 2.0\nx = y\nz := x + 1.0").unwrap();let mut i=Interpreter::new_with_full_stdlib(0);i.interpret(&t).unwrap();assert_eq!((value(&i,"x"),value(&i,"z")),(2.,3.));let(x,y)=(cell(&i,"x"),cell(&i,"y"));let r=register(&i,x);set(&i,"y",10.);let s=i.plan().solve_dirty_cells(&[y]).unwrap();assert_eq!(s.pending_register_nodes,vec![r]);let c=i.plan().commit_pending_registers(&s.pending_register_nodes).unwrap();assert_eq!(c.staged_nodes,vec![r]);assert_eq!(c.committed_nodes,vec![r]);assert_eq!(c.dirty_cells,vec![x]);assert_eq!((value(&i,"x"),value(&i,"z")),(10.,3.));}
  #[test] fn register_commit_add_assignment_updates_register_only() {let t=mech_syntax::parser::parse("~x := 1.0\ny := 2.0\nx += y\nz := x + 1.0").unwrap();let mut i=Interpreter::new_with_full_stdlib(0);i.interpret(&t).unwrap();assert_eq!((value(&i,"x"),value(&i,"z")),(3.,4.));let(x,y)=(cell(&i,"x"),cell(&i,"y"));set(&i,"y",10.);let s=i.plan().solve_dirty_cells(&[y]).unwrap();let c=i.plan().commit_pending_registers(&s.pending_register_nodes).unwrap();assert_eq!(c.dirty_cells,vec![x]);assert_eq!((value(&i,"x"),value(&i,"z")),(13.,4.));}
  #[test] fn register_commit_simultaneous_assignments_use_precommit_state() {let t=mech_syntax::parser::parse("~x := 1.0\n~y := 2.0\nx += y\ny += x").unwrap();let mut i=Interpreter::new_with_full_stdlib(0);i.interpret(&t).unwrap();assert_eq!((value(&i,"x"),value(&i,"y")),(3.,5.));let(x,y)=(cell(&i,"x"),cell(&i,"y"));let(rx,ry)=(register(&i,x),register(&i,y));let s=i.plan().solve_dirty_cells(&[x,y]).unwrap();assert_eq!(s.pending_register_nodes,vec![rx,ry]);let c=i.plan().commit_pending_registers(&[ry,rx]).unwrap();assert_eq!(c.staged_nodes,vec![rx,ry]);assert_eq!(c.committed_nodes,vec![rx,ry]);assert_eq!(c.dirty_cells,vec![x,y]);assert_eq!((value(&i,"x"),value(&i,"y")),(8.,8.));}
  #[test]
  fn decoded_register_commit_add_assignment_uses_staging() {
    let tree = mech_syntax::parser::parse(
      "~x := 1.0\n\
       y := 2.0\n\
       x += y\n\
       x",
    )
    .unwrap();
    let mut source_interpreter = Interpreter::new_with_full_stdlib(0);
    source_interpreter.interpret(&tree).unwrap();
    let bytes = source_interpreter.compile().unwrap();
    let program = ParsedProgram::from_bytes(&bytes).unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    let output = interpreter.run_program(&program).unwrap();
    assert_eq!(*output.as_f64().unwrap().borrow(), 3.0);
    let output_cells = output.reactive_root_cell_ids();
    assert_eq!(output_cells.len(), 1);
    let output_cell = output_cells[0];
    let register_node = register(&interpreter, output_cell);
    let source_cell = {
      let plan = interpreter.plan();
      let plan = plan.borrow();
      let node = plan.node(register_node).unwrap();
      let dependencies = node.inputs.iter().filter(|dependency| {
        dependency.kind == ReactiveDependencyKind::Reactive
          && dependency.cell != output_cell
      }).collect::<Vec<_>>();
      assert_eq!(dependencies.len(), 1, "decoded register must have exactly one distinct reactive source");
      dependencies[0].cell
    };
    let scheduling = interpreter.plan().solve_dirty_cells(&[source_cell]).unwrap();
    assert_eq!(scheduling.pending_register_nodes, vec![register_node]);
    let commit = interpreter.plan().commit_pending_registers(&scheduling.pending_register_nodes).unwrap();
    assert_eq!(commit.staged_nodes, vec![register_node]);
    assert_eq!(commit.committed_nodes, vec![register_node]);
    assert_eq!(commit.dirty_cells, vec![output_cell]);
    assert_eq!(*output.as_f64().unwrap().borrow(), 5.0);
  }
  #[test]
  fn reactive_turn_updates_downstream_after_register_commit() {
    let tree = mech_syntax::parser::parse("~x := 1.0\ny := 2.0\nx += y\nz := x + 1.0").unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    interpreter.interpret(&tree).unwrap();
    let (x_cell, y_cell, z_cell) = (cell(&interpreter, "x"), cell(&interpreter, "y"), cell(&interpreter, "z"));
    let x_register = register(&interpreter, x_cell);
    let x_consumers = {
      let plan_handle = interpreter.plan();
      let plan = plan_handle.borrow();
      let consumers = plan.reactive_consumers_for(x_cell).to_vec();
      assert!(!consumers.is_empty());
      for node_id in &consumers {
        assert_eq!(plan.node(*node_id).unwrap().kind, ReactiveNodeKind::Combinational);
      }
      consumers
    };
    set(&interpreter, "y", 10.0);
    let mut turn_state = ReactiveTurnState::default();
    let outcome = interpreter.plan().advance_reactive_turn(&mut turn_state, &[y_cell]).unwrap();
    assert_eq!((value(&interpreter, "x"), value(&interpreter, "z")), (13.0, 14.0));
    assert_eq!(outcome.before_commit.pending_register_nodes, vec![x_register]);
    assert_eq!(outcome.register_commit.staged_nodes, vec![x_register]);
    assert_eq!(outcome.register_commit.committed_nodes, vec![x_register]);
    assert_eq!(outcome.register_commit.dirty_cells, vec![x_cell]);
    for node_id in &x_consumers { assert!(outcome.after_commit.executed_nodes.contains(node_id)); }
    let executed_z_nodes = { let plan_handle=interpreter.plan(); let plan=plan_handle.borrow(); outcome.after_commit.executed_nodes.iter().copied().filter(|node_id| plan.node(*node_id).unwrap().outputs.contains(&z_cell)).collect::<Vec<_>>() };
    assert!(!executed_z_nodes.is_empty());
    assert!(turn_state.pending_register_nodes.is_empty());
  }
  #[test]
  fn reactive_turn_defers_second_register_layer() {
    let tree = mech_syntax::parser::parse("input := 1.0\n~a := 0.0\n~b := 0.0\na = input\nmiddle := a + 1.0\nb = middle\noutput := b + 1.0").unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    interpreter.interpret(&tree).unwrap();
    assert_eq!((value(&interpreter,"input"),value(&interpreter,"a"),value(&interpreter,"middle"),value(&interpreter,"b"),value(&interpreter,"output")),(1.,1.,2.,2.,3.));
    let (input, a, b) = (cell(&interpreter,"input"),cell(&interpreter,"a"),cell(&interpreter,"b"));
    let (a_register, b_register) = (register(&interpreter,a),register(&interpreter,b));
    set(&interpreter,"input",10.0);
    let mut turn_state = ReactiveTurnState::default();
    let first = interpreter.plan().advance_reactive_turn(&mut turn_state,&[input]).unwrap();
    assert_eq!((value(&interpreter,"a"),value(&interpreter,"middle"),value(&interpreter,"b"),value(&interpreter,"output")),(10.,11.,2.,3.));
    assert_eq!(first.register_commit.committed_nodes,vec![a_register]);
    assert_eq!(first.after_commit.pending_register_nodes,vec![b_register]);
    assert_eq!(turn_state.pending_register_nodes,vec![b_register]);
    let second = interpreter.plan().advance_reactive_turn(&mut turn_state,&[]).unwrap();
    assert_eq!((value(&interpreter,"a"),value(&interpreter,"middle"),value(&interpreter,"b"),value(&interpreter,"output")),(10.,11.,11.,12.));
    assert_eq!(second.register_commit.committed_nodes,vec![b_register]);
    assert!(!second.register_commit.committed_nodes.contains(&a_register));
    assert!(turn_state.pending_register_nodes.is_empty());
  }
  #[test]
  fn decoded_reactive_turn_reuses_compiled_plan() {
    let tree = mech_syntax::parser::parse("~x := 1.0\ny := 2.0\nx += y\nz := x + 1.0\nz").unwrap();
    let mut source_interpreter = Interpreter::new_with_full_stdlib(0);
    source_interpreter.interpret(&tree).unwrap();
    let bytes = source_interpreter.compile().unwrap();
    let program = ParsedProgram::from_bytes(&bytes).unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    let decoded_output = interpreter.run_program(&program).unwrap();
    assert_eq!(*decoded_output.as_f64().unwrap().borrow(), 4.0);
    let z_cell = decoded_output.reactive_root_cell_ids()[0];
    let (x_register, x_ref, x_cell, source_cell, x_consumers, plan_length, node_ids, output_cells) = {
      let plan_handle = interpreter.plan(); let plan = plan_handle.borrow();
      let registers=plan.nodes.iter().filter(|node| node.kind==ReactiveNodeKind::Register).collect::<Vec<_>>();
      assert_eq!(registers.len(),1); let x_register=registers[0].id;
      let x_output=plan.node(x_register).unwrap().function.out(); let x_ref=x_output.as_f64().unwrap().clone(); let x_cell=x_output.reactive_root_cell_ids()[0];
      let source_dependencies=plan.node(x_register).unwrap().inputs.iter().filter(|dependency| dependency.kind==ReactiveDependencyKind::Reactive&&dependency.cell!=x_cell).collect::<Vec<_>>(); assert_eq!(source_dependencies.len(),1);
      let x_consumers=plan.reactive_consumers_for(x_cell).to_vec(); assert!(!x_consumers.is_empty());
      (x_register,x_ref,x_cell,source_dependencies[0].cell,x_consumers,plan.len(),plan.nodes.iter().map(|node|node.id).collect::<Vec<_>>(),plan.nodes.iter().map(|node|node.outputs.clone()).collect::<Vec<_>>())
    };
    assert_eq!(*x_ref.borrow(),3.0);
    let mut turn_state=ReactiveTurnState::default();
    for (expected_x,expected_z) in [(5.0,6.0),(7.0,8.0)] {
      let outcome=interpreter.plan().advance_reactive_turn(&mut turn_state,&[source_cell]).unwrap();
      assert_eq!(outcome.before_commit.pending_register_nodes,vec![x_register]); assert_eq!(outcome.register_commit.staged_nodes,vec![x_register]); assert_eq!(outcome.register_commit.committed_nodes,vec![x_register]); assert_eq!(outcome.register_commit.dirty_cells,vec![x_cell]); for node_id in &x_consumers { assert!(outcome.after_commit.executed_nodes.contains(node_id)); }
      let executed_z_nodes={let plan_handle=interpreter.plan();let plan=plan_handle.borrow();outcome.after_commit.executed_nodes.iter().copied().filter(|node_id|plan.node(*node_id).unwrap().outputs.contains(&z_cell)).collect::<Vec<_>>()};assert!(!executed_z_nodes.is_empty());
      assert_eq!(*x_ref.borrow(),expected_x); assert_eq!(*decoded_output.as_f64().unwrap().borrow(),expected_z); assert!(turn_state.pending_register_nodes.is_empty());
      let plan_handle=interpreter.plan();let plan=plan_handle.borrow();assert_eq!(plan.len(),plan_length);assert_eq!(plan.nodes.iter().map(|node|node.id).collect::<Vec<_>>(),node_ids);assert_eq!(plan.nodes.iter().map(|node|node.outputs.clone()).collect::<Vec<_>>(),output_cells);
    }
  }
}
