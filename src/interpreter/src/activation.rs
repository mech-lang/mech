#![cfg(all(feature = "functions", feature = "symbol_table"))]
//! Statically elaborated structural dispatch for patterned activation scopes.
use crate::*;

macro_rules! activation_error {($n:ident,$m:expr)=>{#[derive(Debug,Clone)] pub(crate) struct $n; impl MechErrorKind for $n {fn name(&self)->&str{stringify!($n)} fn message(&self)->String{$m.into()}}};}
activation_error!(ActivationPatternExpressionUnsupported,"This activation pattern is not supported.");
activation_error!(ActivationPatternCaptureKindUnsupported,"The capture kind cannot be inferred from the activation trigger.");
activation_error!(ActivationPatternArmsNonExhaustive,"Patterned activations require a final unguarded wildcard arm.");
activation_error!(ActivationPatternWildcardMustBeLast,"The wildcard activation arm must be last.");
activation_error!(ActivationPatternGuardMustBePure,"Patterned activation guards are not supported yet.");
activation_error!(ActivationPatternRegisterWriteUnsupported,"Patterned activation register writes are not supported.");
activation_error!(ActivationPatternContextEffectUnsupported,"Patterned activation context effects are not supported.");
activation_error!(ActivationPatternTriggerInvariant,"Activation trigger root cells disagree with the resolved trigger.");

#[derive(Clone,Debug)] enum CompiledActivationPattern { Wildcard, Literal{expected:Value}, Capture{capture_index:usize}, Tuple{elements:Vec<Self>}, TupleStruct{tag:u64,payload:Vec<Self>} }
#[derive(Clone)] struct Capture { id:u64, kind:ValueKind, slot:Value }
#[derive(Clone)] struct CompiledArm { pattern:CompiledActivationPattern, captures:Vec<Capture> }
fn detached(v:&Value)->Value { match v {Value::MutableReference(r)=>detached(&r.borrow()), _=>v.clone()} }
fn clone_ref_value<T: Clone>(destination: &Ref<T>, source: &Ref<T>) { let source=source.borrow(); destination.borrow_mut().clone_from(&*source); }
fn create_capture_slot(sample:&Value)->MResult<Value>{match detached(sample){
#[cfg(feature="u8")] Value::U8(v)=>Ok(Value::U8(Ref::new(*v.borrow()))),
#[cfg(feature="u16")] Value::U16(v)=>Ok(Value::U16(Ref::new(*v.borrow()))),
#[cfg(feature="u32")] Value::U32(v)=>Ok(Value::U32(Ref::new(*v.borrow()))),
#[cfg(feature="u64")] Value::U64(v)=>Ok(Value::U64(Ref::new(*v.borrow()))),
#[cfg(feature="u128")] Value::U128(v)=>Ok(Value::U128(Ref::new(*v.borrow()))),
#[cfg(feature="i8")] Value::I8(v)=>Ok(Value::I8(Ref::new(*v.borrow()))),
#[cfg(feature="i16")] Value::I16(v)=>Ok(Value::I16(Ref::new(*v.borrow()))),
#[cfg(feature="i32")] Value::I32(v)=>Ok(Value::I32(Ref::new(*v.borrow()))),
#[cfg(feature="i64")] Value::I64(v)=>Ok(Value::I64(Ref::new(*v.borrow()))),
#[cfg(feature="i128")] Value::I128(v)=>Ok(Value::I128(Ref::new(*v.borrow()))),
#[cfg(feature="f32")] Value::F32(v)=>Ok(Value::F32(Ref::new(*v.borrow()))),
#[cfg(feature="f64")] Value::F64(v)=>Ok(Value::F64(Ref::new(*v.borrow()))),
#[cfg(feature="complex")] Value::C64(v)=>Ok(Value::C64(Ref::new(v.borrow().clone()))),
#[cfg(feature="rational")] Value::R64(v)=>Ok(Value::R64(Ref::new(v.borrow().clone()))),
#[cfg(any(feature="bool",feature="variable_define"))] Value::Bool(v)=>Ok(Value::Bool(Ref::new(*v.borrow()))),
#[cfg(any(feature="string",feature="variable_define"))] Value::String(v)=>Ok(Value::String(Ref::new(v.borrow().clone()))),
Value::Index(v)=>Ok(Value::Index(Ref::new(*v.borrow()))),
#[cfg(feature="atom")] Value::Atom(v)=>Ok(Value::Atom(Ref::new(v.borrow().clone()))),
_=>Err(MechError::new(ActivationPatternCaptureKindUnsupported,None))}}
fn commit_capture_slot(destination:&Value,source:&Value)->MResult<()>{let source=detached(source);match(destination,&source){
#[cfg(feature="u8")] (Value::U8(a),Value::U8(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="u16")] (Value::U16(a),Value::U16(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="u32")] (Value::U32(a),Value::U32(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="u64")] (Value::U64(a),Value::U64(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="u128")] (Value::U128(a),Value::U128(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="i8")] (Value::I8(a),Value::I8(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="i16")] (Value::I16(a),Value::I16(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="i32")] (Value::I32(a),Value::I32(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="i64")] (Value::I64(a),Value::I64(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="i128")] (Value::I128(a),Value::I128(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="f32")] (Value::F32(a),Value::F32(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="f64")] (Value::F64(a),Value::F64(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="complex")] (Value::C64(a),Value::C64(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="rational")] (Value::R64(a),Value::R64(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(any(feature="bool",feature="variable_define"))] (Value::Bool(a),Value::Bool(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(any(feature="string",feature="variable_define"))] (Value::String(a),Value::String(b))=>{clone_ref_value(a,b);Ok(())},
(Value::Index(a),Value::Index(b))=>{clone_ref_value(a,b);Ok(())},
#[cfg(feature="atom")] (Value::Atom(a),Value::Atom(b))=>{clone_ref_value(a,b);Ok(())},
_=>Err(MechError::new(ActivationPatternCaptureKindUnsupported,None))}}
fn compile(pat:&Pattern,sample:&Value,i:&Interpreter,caps:&mut Vec<Capture>)->MResult<CompiledActivationPattern>{match pat { Pattern::Wildcard=>Ok(CompiledActivationPattern::Wildcard), Pattern::Expression(Expression::Literal(x))=>Ok(CompiledActivationPattern::Literal{expected:crate::literal(x,i)?}), Pattern::Expression(Expression::Var(v))=>{let id=v.name.hash();if let Some(n)=caps.iter().position(|c|c.id==id){Ok(CompiledActivationPattern::Capture{capture_index:n})}else{let value=create_capture_slot(sample).map_err(|e|e.with_tokens(pat.tokens()))?;caps.push(Capture{id,kind:sample.kind(),slot:value});Ok(CompiledActivationPattern::Capture{capture_index:caps.len()-1})}}, #[cfg(feature="tuple")] Pattern::Tuple(t)=>{let Value::Tuple(tv)=detached(sample) else{return Err(MechError::new(ActivationPatternCaptureKindUnsupported,None))};let tv=tv.borrow();if tv.elements.len()!=t.0.len(){return Err(MechError::new(ActivationPatternCaptureKindUnsupported,None))}Ok(CompiledActivationPattern::Tuple{elements:t.0.iter().zip(tv.elements.iter()).map(|(p,v)|compile(p,v,i,caps)).collect::<MResult<_>>()?})}, _=>Err(MechError::new(ActivationPatternExpressionUnsupported,None).with_tokens(pat.tokens()))}}
fn matches_pattern(p:&CompiledActivationPattern,v:&Value,proposed:&mut Vec<Option<Value>>)->bool{match p {CompiledActivationPattern::Wildcard=>true,CompiledActivationPattern::Literal{expected}=>expected==&detached(v),CompiledActivationPattern::Capture{capture_index}=>{let x=detached(v);match &proposed[*capture_index]{Some(y)=>y==&x,None=>{proposed[*capture_index]=Some(x);true}}}, #[cfg(feature="tuple")] CompiledActivationPattern::Tuple{elements}=>match detached(v){Value::Tuple(t)=>{let t=t.borrow();t.elements.len()==elements.len()&&elements.iter().zip(t.elements.iter()).all(|(p,v)|matches_pattern(p,v,proposed))},_=>false}, CompiledActivationPattern::TupleStruct{..}=>false, #[cfg(not(feature="tuple"))] CompiledActivationPattern::Tuple{..}=>false}}
fn generation()->(Ref<usize>,Value){let r=Ref::new(0);(r.clone(),Value::Index(r))}
struct ScopePulse{out:Ref<usize>} impl MechFunctionImpl for ScopePulse{fn solve(&self){}fn solve_reactive(&self)->MResult<ReactiveSolveStatus>{*self.out.borrow_mut()+=1;Ok(ReactiveSolveStatus::Changed)}fn out(&self)->Value{Value::Index(self.out.clone())}fn reactive_dependency_scopes(&self,_:usize)->Option<Vec<ReactiveDependencyScope>>{Some(vec![ReactiveDependencyScope::Root])}fn to_string(&self)->String{"ActivationPatternScopePulse".into()}}
struct Matcher{pattern:CompiledActivationPattern,trigger:Value,captures:Vec<Capture>,matched:Ref<bool>,out:Ref<usize>} impl MechFunctionImpl for Matcher{fn solve(&self){}fn solve_reactive(&self)->MResult<ReactiveSolveStatus>{let mut p=vec![None;self.captures.len()];let ok=matches_pattern(&self.pattern,&self.trigger,&mut p);if ok{for(c,v)in self.captures.iter().zip(p.iter()){if let Some(v)=v{commit_capture_slot(&c.slot,v)?}}}*self.matched.borrow_mut()=ok;*self.out.borrow_mut()+=1;Ok(ReactiveSolveStatus::Changed)}fn out(&self)->Value{Value::Index(self.out.clone())}fn reactive_dependency_kinds(&self,_:usize)->Option<Vec<ReactiveDependencyKind>>{Some(vec![ReactiveDependencyKind::Reactive,ReactiveDependencyKind::Sampled])}fn to_string(&self)->String{"ActivationPatternMatcher".into()}}
struct Finalize{matched:Ref<bool>,eligible:Ref<bool>,out:Ref<usize>} impl MechFunctionImpl for Finalize{fn solve(&self){}fn solve_reactive(&self)->MResult<ReactiveSolveStatus>{*self.eligible.borrow_mut()=*self.matched.borrow();*self.out.borrow_mut()+=1;Ok(ReactiveSolveStatus::Changed)}fn out(&self)->Value{Value::Index(self.out.clone())}fn to_string(&self)->String{"ActivationPatternArmFinalize".into()}}
struct Select{eligible:Vec<Ref<bool>>,selected:Ref<usize>,out:Ref<usize>} impl MechFunctionImpl for Select{fn solve(&self){}fn solve_reactive(&self)->MResult<ReactiveSolveStatus>{*self.selected.borrow_mut()=self.eligible.iter().position(|x|*x.borrow()).unwrap_or(usize::MAX);*self.out.borrow_mut()+=1;Ok(ReactiveSolveStatus::Changed)}fn out(&self)->Value{Value::Index(self.out.clone())}fn to_string(&self)->String{"ActivationPatternSelectArm".into()}}
struct Gate{arm:usize,selected:Ref<usize>,out:Ref<usize>} impl MechFunctionImpl for Gate{fn solve(&self){}fn solve_reactive(&self)->MResult<ReactiveSolveStatus>{if*self.selected.borrow()==self.arm{*self.out.borrow_mut()+=1;Ok(ReactiveSolveStatus::Changed)}else{Ok(ReactiveSolveStatus::Unchanged)}}fn out(&self)->Value{Value::Index(self.out.clone())}fn to_string(&self)->String{"ActivationPatternArmGate".into()}}

#[cfg(feature="compiler")] macro_rules! interpreter_only {($t:ty)=>{impl MechFunctionCompiler for $t { fn compile(&self,_:&mut CompileCtx)->MResult<Register>{Err(MechError::new(GenericError{msg:"Activation pattern dispatch is interpreter-only.".into()},None))} }};}
#[cfg(feature="compiler")] interpreter_only!(ScopePulse);
#[cfg(feature="compiler")] interpreter_only!(Matcher);
#[cfg(feature="compiler")] interpreter_only!(Finalize);
#[cfg(feature="compiler")] interpreter_only!(Select);
#[cfg(feature="compiler")] interpreter_only!(Gate);

pub(crate) fn elaborate_patterned_activation(scope:&ActivationScope,arms:&[ActivationArm],trigger:Value,trigger_cells:Vec<ReactiveCellId>,i:&Interpreter)->MResult<Value>{
 let last=arms.last().ok_or_else(||MechError::new(ActivationPatternArmsNonExhaustive,None).with_tokens(scope.tokens()))?;
 if !matches!(last.pattern,Pattern::Wildcard)||last.guard.is_some(){return Err(MechError::new(ActivationPatternArmsNonExhaustive,None).with_tokens(scope.tokens()))} if arms[..arms.len()-1].iter().any(|x|matches!(x.pattern,Pattern::Wildcard)){return Err(MechError::new(ActivationPatternWildcardMustBeLast,None).with_tokens(scope.tokens()))} if arms.iter().any(|x|x.guard.is_some()){return Err(MechError::new(ActivationPatternGuardMustBePure,None).with_tokens(scope.tokens()))}
 for arm in arms {if let ActivationArmBody::Block(body)=&arm.body{for(code,_)in body{match code{MechCode::Statement(Statement::VariableAssign(_))|MechCode::Statement(Statement::OpAssign(_))=>return Err(MechError::new(ActivationPatternRegisterWriteUnsupported,None).with_tokens(code.tokens())),MechCode::Statement(Statement::ContextSend(_))=>return Err(MechError::new(ActivationPatternContextEffectUnsupported,None).with_tokens(code.tokens())),_=>{}}}}}
 let mut compiled=Vec::new();for arm in arms{let mut captures=Vec::new();let pattern=compile(&arm.pattern,&trigger,i,&mut captures)?;compiled.push(CompiledArm{pattern,captures})}
 if trigger.reactive_root_cell_ids()!=trigger_cells{return Err(MechError::new(ActivationPatternTriggerInvariant,None).with_tokens(scope.tokens()))}
 let plan=i.plan();let(scope_gen,scope_v)=generation();let scope_node=plan.borrow_mut().register(Box::new(ScopePulse{out:scope_gen}),&[trigger.clone()])?;
 let(mut matcher_nodes,mut completions,mut matched)=(Vec::new(),Vec::new(),Vec::new());for arm in &compiled{let(o,v)=generation();let f=Ref::new(false);let n=plan.borrow_mut().register(Box::new(Matcher{pattern:arm.pattern.clone(),trigger:trigger.clone(),captures:arm.captures.clone(),matched:f.clone(),out:o}),&[scope_v.clone(),trigger.clone()])?;matcher_nodes.push(n);completions.push(v);matched.push(f)}
 let(mut finalizers,mut eligible,mut done)=(Vec::new(),Vec::new(),Vec::new());for(f,c)in matched.iter().zip(completions.iter()){let(o,v)=generation();let e=Ref::new(false);finalizers.push(plan.borrow_mut().register(Box::new(Finalize{matched:f.clone(),eligible:e.clone(),out:o}),&[c.clone()])?);eligible.push(e);done.push(v)} let(o,selection)=generation();let selected=Ref::new(usize::MAX);let selector=plan.borrow_mut().register(Box::new(Select{eligible:eligible.clone(),selected:selected.clone(),out:o}),&done)?;
 let(mut gates,mut pulses)=(Vec::new(),Vec::new());for arm in 0..arms.len(){let(o,v)=generation();gates.push(plan.borrow_mut().register(Box::new(Gate{arm,selected:selected.clone(),out:o}),&[selection.clone()])?);pulses.push(v)}
 let mut ranges=Vec::new();for(arm,compiled_arm)in arms.iter().zip(compiled.iter()){let symbols=i.symbols();let mut s=symbols.borrow_mut();let old=compiled_arm.captures.iter().map(|c|(c.id,s.symbols.get(&c.id).cloned(),s.mutable_variables.get(&c.id).cloned())).collect::<Vec<_>>();for c in &compiled_arm.captures{s.insert(c.id,c.slot.clone(),false);}drop(s);let start=plan.len();plan.push_activation_registration_scope(pulses[ranges.len()].reactive_root_cell_ids());let result=match &arm.body{ActivationArmBody::Block(b)=>{for(c,_)in b{crate::mech_code(c,i)?;}Ok(())},ActivationArmBody::Expression(e)=>crate::expression(e,None,i).map(|_|())};plan.pop_activation_registration_scope();let mut s=symbols.borrow_mut();for(id,oldv,oldm)in old{s.symbols.remove(&id);s.mutable_variables.remove(&id);if let Some(v)=oldv{s.symbols.insert(id,v);}if let Some(v)=oldm{s.mutable_variables.insert(id,v);}}drop(s);result?;ranges.push((start,plan.len()))}
 let registration=PatternActivationRegistration{scope_pulse_node:scope_node,selector_node:selector,arms:(0..arms.len()).map(|n|PatternActivationArmRegistration{matcher_node:matcher_nodes[n],finalizer_node:finalizers[n],gate_node:gates[n],pulse_cell:pulses[n].reactive_root_cell_ids()[0],body_node_start:ranges[n].0,body_node_end:ranges[n].1,captures:compiled[n].captures.iter().map(|c|PatternActivationCaptureRegistration{id:c.id,kind:c.kind.clone(),cell:c.slot.reactive_root_cell_ids()[0]}).collect()}).collect()};plan.borrow_mut().register_pattern_activation(registration);Ok(Value::Empty)
}
