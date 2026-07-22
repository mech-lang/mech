//! Static scheduling support for patterned activation scopes.
//!
//! The small dispatch nodes in this file deliberately contain no syntax and
//! are registered once while a source tree is elaborated.  A reactive turn
//! only moves generation counters through that already-built graph.

use crate::*;

#[derive(Debug, Clone)] pub(crate) struct ActivationPatternExpressionUnsupported;
impl MechErrorKind for ActivationPatternExpressionUnsupported { fn name(&self)->&str{"ActivationPatternExpressionUnsupported"} fn message(&self)->String{"This activation pattern is not supported.".into()} }
#[derive(Debug, Clone)] pub(crate) struct ActivationPatternCaptureKindUnsupported;
impl MechErrorKind for ActivationPatternCaptureKindUnsupported { fn name(&self)->&str{"ActivationPatternCaptureKindUnsupported"} fn message(&self)->String{"The capture kind cannot be inferred from the activation trigger.".into()} }
#[derive(Debug, Clone)] pub(crate) struct ActivationPatternArmsNonExhaustive;
impl MechErrorKind for ActivationPatternArmsNonExhaustive { fn name(&self)->&str{"ActivationPatternArmsNonExhaustive"} fn message(&self)->String{"Patterned activations require a final unguarded wildcard arm.".into()} }
#[derive(Debug, Clone)] pub(crate) struct ActivationPatternWildcardMustBeLast;
impl MechErrorKind for ActivationPatternWildcardMustBeLast { fn name(&self)->&str{"ActivationPatternWildcardMustBeLast"} fn message(&self)->String{"The wildcard activation arm must be last.".into()} }
#[derive(Debug, Clone)] pub(crate) struct ActivationPatternGuardMustBePure;
impl MechErrorKind for ActivationPatternGuardMustBePure { fn name(&self)->&str{"ActivationPatternGuardMustBePure"} fn message(&self)->String{"Patterned activation guards must be pure.".into()} }
#[derive(Debug, Clone)] pub(crate) struct ActivationPatternRegisterWriteUnsupported;
impl MechErrorKind for ActivationPatternRegisterWriteUnsupported { fn name(&self)->&str{"ActivationPatternRegisterWriteUnsupported"} fn message(&self)->String{"Patterned activation register writes are not supported.".into()} }
#[derive(Debug, Clone)] pub(crate) struct ActivationPatternContextEffectUnsupported;
impl MechErrorKind for ActivationPatternContextEffectUnsupported { fn name(&self)->&str{"ActivationPatternContextEffectUnsupported"} fn message(&self)->String{"Patterned activation context effects are not supported.".into()} }

#[derive(Clone, Debug)]
enum CompiledActivationPattern { Wildcard, Literal(Value) }

fn compile_pattern(pattern: &Pattern, interpreter: &Interpreter) -> MResult<CompiledActivationPattern> {
  match pattern {
    Pattern::Wildcard => Ok(CompiledActivationPattern::Wildcard),
    Pattern::Expression(Expression::Literal(literal)) => Ok(CompiledActivationPattern::Literal(crate::literal(literal, interpreter)?)),
    // A variable is deliberately not treated as a catch-all: doing so loses
    // the type and stable backing reference required for a capture slot.
    Pattern::Expression(Expression::Var(_)) => Err(MechError::new(ActivationPatternCaptureKindUnsupported, None).with_tokens(pattern.tokens())),
    _ => Err(MechError::new(ActivationPatternExpressionUnsupported, None).with_tokens(pattern.tokens())),
  }
}

fn generation() -> (Ref<usize>, Value) { let cell=Ref::new(0usize); let value=Value::Index(cell.clone()); (cell,value) }

struct ScopePulse { out: Ref<usize> }
#[cfg(feature="compiler")] impl MechFunctionCompiler for ScopePulse { fn compile(&self,_:&mut CompileCtx)->MResult<Register>{Err(MechError::new(GenericError{msg:"Activation pattern dispatch is interpreter-only.".into()},None))} }
impl MechFunctionImpl for ScopePulse {
  fn solve(&self) {}
  fn solve_reactive(&self)->MResult<ReactiveSolveStatus>{ *self.out.borrow_mut()+=1; Ok(ReactiveSolveStatus::Changed) }
  fn out(&self)->Value { Value::Index(self.out.clone()) }
  fn reactive_dependency_scopes(&self,_:usize)->Option<Vec<ReactiveDependencyScope>> { Some(vec![ReactiveDependencyScope::Root]) }
  fn to_string(&self)->String{"ActivationPatternScopePulse".into()}
}
struct Matcher { pattern: CompiledActivationPattern, trigger: Value, matched: Ref<bool>, out: Ref<usize> }
#[cfg(feature="compiler")] impl MechFunctionCompiler for Matcher { fn compile(&self,_:&mut CompileCtx)->MResult<Register>{Err(MechError::new(GenericError{msg:"Activation pattern dispatch is interpreter-only.".into()},None))} }
impl MechFunctionImpl for Matcher {
  fn solve(&self) {}
  fn solve_reactive(&self)->MResult<ReactiveSolveStatus>{ *self.matched.borrow_mut()=match &self.pattern { CompiledActivationPattern::Wildcard=>true, CompiledActivationPattern::Literal(value)=>value==&self.trigger }; *self.out.borrow_mut()+=1; Ok(ReactiveSolveStatus::Changed) }
  fn out(&self)->Value{Value::Index(self.out.clone())}
  fn reactive_dependency_kinds(&self,_:usize)->Option<Vec<ReactiveDependencyKind>>{Some(vec![ReactiveDependencyKind::Reactive,ReactiveDependencyKind::Sampled])}
  fn to_string(&self)->String{"ActivationPatternMatcher".into()}
}
struct Finalize { matched: Ref<bool>, eligible: Ref<bool>, out: Ref<usize> }
#[cfg(feature="compiler")] impl MechFunctionCompiler for Finalize { fn compile(&self,_:&mut CompileCtx)->MResult<Register>{Err(MechError::new(GenericError{msg:"Activation pattern dispatch is interpreter-only.".into()},None))} }
impl MechFunctionImpl for Finalize { fn solve(&self){} fn solve_reactive(&self)->MResult<ReactiveSolveStatus>{*self.eligible.borrow_mut()=*self.matched.borrow();*self.out.borrow_mut()+=1;Ok(ReactiveSolveStatus::Changed)} fn out(&self)->Value{Value::Index(self.out.clone())} fn to_string(&self)->String{"ActivationPatternArmFinalize".into()} }
struct Select { eligible: Vec<Ref<bool>>, selected: Ref<usize>, out: Ref<usize> }
#[cfg(feature="compiler")] impl MechFunctionCompiler for Select { fn compile(&self,_:&mut CompileCtx)->MResult<Register>{Err(MechError::new(GenericError{msg:"Activation pattern dispatch is interpreter-only.".into()},None))} }
impl MechFunctionImpl for Select { fn solve(&self){} fn solve_reactive(&self)->MResult<ReactiveSolveStatus>{*self.selected.borrow_mut()=self.eligible.iter().position(|x|*x.borrow()).unwrap_or(usize::MAX);*self.out.borrow_mut()+=1;Ok(ReactiveSolveStatus::Changed)} fn out(&self)->Value{Value::Index(self.out.clone())} fn to_string(&self)->String{"ActivationPatternSelectArm".into()} }
struct Gate { arm: usize, selected: Ref<usize>, out: Ref<usize> }
#[cfg(feature="compiler")] impl MechFunctionCompiler for Gate { fn compile(&self,_:&mut CompileCtx)->MResult<Register>{Err(MechError::new(GenericError{msg:"Activation pattern dispatch is interpreter-only.".into()},None))} }
impl MechFunctionImpl for Gate { fn solve(&self){} fn solve_reactive(&self)->MResult<ReactiveSolveStatus>{if *self.selected.borrow()==self.arm {*self.out.borrow_mut()+=1;Ok(ReactiveSolveStatus::Changed)} else {Ok(ReactiveSolveStatus::Unchanged)}} fn out(&self)->Value{Value::Index(self.out.clone())} fn to_string(&self)->String{"ActivationPatternArmGate".into()} }

/// Elaborates the static, combinational dispatch graph for literal and
/// wildcard patterned activation arms.  Unsupported destructuring is rejected
/// during elaboration rather than deferred to a reactive turn.
pub(crate) fn elaborate_patterned_activation(scope: &ActivationScope, arms: &[ActivationArm], trigger: Value, trigger_cells: Vec<ReactiveCellId>, interpreter: &Interpreter) -> MResult<Value> {
  let final_arm=arms.last().ok_or_else(||MechError::new(ActivationPatternArmsNonExhaustive,None).with_tokens(scope.tokens()))?;
  if !matches!(final_arm.pattern,Pattern::Wildcard) || final_arm.guard.is_some() { return Err(MechError::new(ActivationPatternArmsNonExhaustive,None).with_tokens(scope.tokens())); }
  if arms[..arms.len()-1].iter().any(|arm|matches!(arm.pattern,Pattern::Wildcard)) { return Err(MechError::new(ActivationPatternWildcardMustBeLast,None).with_tokens(scope.tokens())); }
  if arms.iter().any(|arm|arm.guard.is_some()) { return Err(MechError::new(ActivationPatternGuardMustBePure,None).with_tokens(scope.tokens())); }
  for arm in arms { if let ActivationArmBody::Block(body)=&arm.body { for (code,_) in body { match code { MechCode::Statement(Statement::VariableAssign(_)) | MechCode::Statement(Statement::OpAssign(_)) => return Err(MechError::new(ActivationPatternRegisterWriteUnsupported,None).with_tokens(code.tokens())), MechCode::Statement(Statement::ContextSend(_)) => return Err(MechError::new(ActivationPatternContextEffectUnsupported,None).with_tokens(code.tokens())), _=>{} } } } }
  let patterns=arms.iter().map(|arm|compile_pattern(&arm.pattern,interpreter)).collect::<MResult<Vec<_>>>()?;
  let plan=interpreter.plan();
  let (scope_generation,scope_value)=generation();
  let scope_node=plan.borrow_mut().register(Box::new(ScopePulse{out:scope_generation}), &[trigger.clone()])?;
  let mut matcher_nodes=Vec::new(); let mut completions=Vec::new(); let mut matched=Vec::new();
  for pattern in patterns { let (complete,complete_value)=generation(); let flag=Ref::new(false); let id=plan.borrow_mut().register(Box::new(Matcher{pattern,trigger:trigger.clone(),matched:flag.clone(),out:complete}), &[scope_value.clone(),trigger.clone()])?; matcher_nodes.push(id); completions.push(complete_value); matched.push(flag); }
  let mut finalizer_nodes=Vec::new(); let mut eligible=Vec::new(); let mut arm_complete=Vec::new();
  for (flag, complete) in matched.iter().zip(completions.iter()) { let (done,done_value)=generation(); let ok=Ref::new(false); let id=plan.borrow_mut().register(Box::new(Finalize{matched:flag.clone(),eligible:ok.clone(),out:done}), &[complete.clone()])?; finalizer_nodes.push(id); eligible.push(ok); arm_complete.push(done_value); }
  let (selection,selection_value)=generation(); let selected=Ref::new(usize::MAX); let selector_node=plan.borrow_mut().register(Box::new(Select{eligible:eligible.clone(),selected:selected.clone(),out:selection}), &arm_complete)?;
  let mut gate_nodes=Vec::new(); let mut pulses=Vec::new();
  for arm in 0..arms.len() { let (pulse,pulse_value)=generation(); let id=plan.borrow_mut().register(Box::new(Gate{arm,selected:selected.clone(),out:pulse}), &[selection_value.clone()])?; gate_nodes.push(id); pulses.push(pulse_value); }
  let mut registrations=Vec::new();
  for (arm,pulse) in arms.iter().zip(pulses.iter()) { let start=plan.len(); plan.push_activation_registration_scope(pulse.reactive_root_cell_ids()); let result=match &arm.body { ActivationArmBody::Block(body)=>{for (code,_) in body { crate::mech_code(code,interpreter)?; } Ok(())}, ActivationArmBody::Expression(expression)=>{crate::expression(expression,None,interpreter).map(|_|())} }; plan.pop_activation_registration_scope(); result?; registrations.push((start,plan.len())); }
  let registration=PatternActivationRegistration { scope_pulse_node:scope_node, selector_node, arms:(0..arms.len()).map(|i|PatternActivationArmRegistration{matcher_node:matcher_nodes[i],finalizer_node:finalizer_nodes[i],gate_node:gate_nodes[i],pulse_cell:pulses[i].reactive_root_cell_ids()[0],body_node_start:registrations[i].0,body_node_end:registrations[i].1}).collect() };
  plan.borrow_mut().register_pattern_activation(registration);
  let _=trigger_cells; // The scope pulse's Root dependency is the trigger's root storage.
  Ok(Value::Empty)
}
