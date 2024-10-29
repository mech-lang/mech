use crate::matrix::{Matrix, ToMatrix};
use crate::kind::Kind;

use crate::stdlib::{math::*,
                    logic::*,
                    compare::*,
                    matrix::*,
                    table::*,
                    convert::*
                  };
use crate::stdlib::range::{RangeInclusive, RangeExclusive};
use crate::*;
use crate::{MechError, MechErrorKind, hash_str, nodes::Kind as NodeKind, nodes::*};
use crate::nodes::Matrix as Mat;

use na::DMatrix;
use indexmap::set::IndexSet;
use indexmap::map::IndexMap;

// Interpreter 
// ----------------------------------------------------------------------------

pub struct Interpreter {
  pub symbols: SymbolTableRef,
  pub plan: Plan,
  pub functions: FunctionsRef,
}

impl Interpreter {
  pub fn new() -> Interpreter {
    
    // Preload functions
    let mut fxns = Functions::new();
    fxns.function_compilers.insert(hash_str("math/sin"),Box::new(MathSin{}));
    fxns.function_compilers.insert(hash_str("math/cos"),Box::new(MathCos{}));

    // Preload kinds
    fxns.kinds.insert(hash_str("u8"),ValueKind::U8);
    fxns.kinds.insert(hash_str("u16"),ValueKind::U16);
    fxns.kinds.insert(hash_str("u32"),ValueKind::U32);
    fxns.kinds.insert(hash_str("u64"),ValueKind::U64);
    fxns.kinds.insert(hash_str("u128"),ValueKind::U128);
    fxns.kinds.insert(hash_str("i8"),ValueKind::I8);
    fxns.kinds.insert(hash_str("i16"),ValueKind::I16);
    fxns.kinds.insert(hash_str("i32"),ValueKind::I32);
    fxns.kinds.insert(hash_str("i64"),ValueKind::I64);
    fxns.kinds.insert(hash_str("i128"),ValueKind::I128);
    fxns.kinds.insert(hash_str("f32"),ValueKind::F32);
    fxns.kinds.insert(hash_str("f64"),ValueKind::F64);
    fxns.kinds.insert(hash_str("string"),ValueKind::String);
    fxns.kinds.insert(hash_str("bool"),ValueKind::Bool);

    Interpreter {
      symbols: new_ref(SymbolTable::new()),
      plan: new_ref(Vec::new()),
      functions: new_ref(fxns),
    }
  }

  pub fn interpret(&mut self, tree: &Program) -> MResult<Value> {
    program(tree, self.plan.clone(), self.symbols.clone(), self.functions.clone())
  }
}

//-----------------------------------------------------------------------------

fn program(program: &Program, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  body(&program.body, plan.clone(), symbols.clone(), functions.clone())
}

fn body(body: &Body, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut result = None;
  for sec in &body.sections {
    result = Some(section(&sec, plan.clone(), symbols.clone(), functions.clone())?);
  }
  Ok(result.unwrap())
}

fn section(section: &Section, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut result = None;
  for el in &section.elements {
    result = Some(section_element(&el, plan.clone(), symbols.clone(), functions.clone())?);
  }
  Ok(result.unwrap())
}

fn section_element(element: &SectionElement, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let out = match element {
    SectionElement::MechCode(code) => {mech_code(&code, plan.clone(), symbols.clone(), functions.clone())?},
    SectionElement::Section(sctn) => Value::Empty,
    SectionElement::Comment(cmmnt) => Value::Empty,
    SectionElement::Paragraph(p) => Value::Empty,
    SectionElement::UnorderedList(ul) => Value::Empty,
    SectionElement::CodeBlock => Value::Empty,
    SectionElement::OrderedList => Value::Empty,
    SectionElement::BlockQuote => Value::Empty,
    SectionElement::ThematicBreak => Value::Empty,
    SectionElement::Image => Value::Empty,
  };
  Ok(out)
}

fn mech_code(code: &MechCode, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match &code {
    MechCode::Expression(expr) => expression(&expr, plan.clone(), symbols.clone(), functions.clone()),
    MechCode::Statement(stmt) => statement(&stmt, plan.clone(), symbols.clone(), functions.clone()),
    MechCode::FsmSpecification(_) => todo!(),
    MechCode::FsmImplementation(_) => todo!(),
    MechCode::FunctionDefine(fxn_def) => {
      let usr_fxn = function_define(&fxn_def, functions.clone())?;
      let mut fxns_brrw = functions.borrow_mut();
      fxns_brrw.functions.insert(usr_fxn.id, usr_fxn);
      Ok(Value::Empty)
    },
  }
}


pub fn function_define(fxn_def: &FunctionDefine, functions: FunctionsRef) -> MResult<FunctionDefinition> {
  let fxn_name_id = fxn_def.name.hash();
  let mut new_fxn = FunctionDefinition::new(fxn_name_id,fxn_def.name.to_string(), fxn_def.clone());
  for input_arg in &fxn_def.input {
    let arg_id = input_arg.name.hash();
    new_fxn.input.insert(arg_id,input_arg.kind.clone());
    let in_arg = Value::F64(new_ref(F64::new(0.0)));
    new_fxn.symbols.borrow_mut().insert(arg_id, in_arg);
  }
  let output_arg_ids = fxn_def.output.iter().map(|output_arg| {
    let arg_id = output_arg.name.hash();
    new_fxn.output.insert(arg_id,output_arg.kind.clone());
    arg_id
  }).collect::<Vec<u64>>();
  
  for stmnt in &fxn_def.statements {
    let result = statement(stmnt, new_fxn.plan.clone(), new_fxn.symbols.clone(), functions.clone());
  }    
  // get the output cell
  {
    let symbol_brrw = new_fxn.symbols.borrow();
    for arg_id in output_arg_ids {
      match symbol_brrw.get(arg_id) {
        Some(cell) => new_fxn.out = cell.clone(),
        None => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::OutputUndefinedInFunctionBody(arg_id)});} 
      }
    }
  }
  Ok(new_fxn)
}

fn statement(stmt: &Statement, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match stmt {
    Statement::VariableDefine(var_def) => variable_define(&var_def, plan.clone(), symbols.clone(), functions.clone()),
    Statement::VariableAssign(var_assgn) => variable_assign(&var_assgn, plan.clone(), symbols.clone(), functions.clone()),
    Statement::KindDefine(knd_def) => kind_define(&knd_def, plan.clone(), symbols.clone(), functions.clone()),
    Statement::EnumDefine(enm_def) => enum_define(&enm_def, plan.clone(), symbols.clone(), functions.clone()),
    Statement::FsmDeclare(_) => todo!(),
    Statement::SplitTable => todo!(),
    Statement::FlattenTable => todo!(),
  }
}

fn variable_assign(var_assgn: &VariableAssign, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut source = expression(&var_assgn.expression, plan.clone(), symbols.clone(), functions.clone())?;
  let slc = &var_assgn.target;
  let name = slc.name.hash();
  let symbols_brrw = symbols.borrow();
  let sink = match symbols_brrw.get(name) {
    Some(val) => val.borrow().clone(),
    None => {return Err(MechError{tokens: slc.name.tokens(), msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedVariable(name)});}
  };
  match &slc.subscript {
    Some(sbscrpt) => {
      for s in sbscrpt {
        let s_result = subscript_ref(&s, &sink, &source, plan.clone(), symbols.clone(), functions.clone())?;
        return Ok(s_result);
      }
    }
    None => {
      let args = vec![sink,source];
      let fxn = SetValue{}.compile(&args)?;
      fxn.solve();
      let mut plan_brrw = plan.borrow_mut();
      let res = fxn.out();
      plan_brrw.push(fxn);
      return Ok(res);
    }
  }
  unreachable!(); // subscript should have thrown an error if we can't access an element
}

fn enum_define(enm_def: &EnumDefine, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let id = enm_def.name.hash();
  let variants = enm_def.variants.iter().map(|v| (v.name.hash(),None)).collect::<Vec<(u64, Option<Value>)>>();
  let mut fxns_brrw = functions.borrow_mut();
  let enm = MechEnum{id, variants};
  let val = Value::Enum(Box::new(enm.clone()));
  fxns_brrw.enums.insert(id, enm.clone());
  fxns_brrw.kinds.insert(id, val.kind());
  Ok(val)
}

fn kind_define(knd_def: &KindDefine, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let id = knd_def.name.hash();
  let kind = kind_annotation(&knd_def.kind.kind, functions.clone())?;
  let value_kind = kind.to_value_kind(functions.clone())?;
  let mut fxns_brrw = functions.borrow_mut();
  fxns_brrw.kinds.insert(id, value_kind.clone());
  Ok(Value::Kind(value_kind))
}

fn variable_define(var_def: &VariableDefine, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let id = var_def.var.name.hash();
  let mut result = expression(&var_def.expression, plan.clone(), symbols.clone(), functions.clone())?;
  if let Some(knd_anntn) =  &var_def.var.kind {
    let knd = kind_annotation(&knd_anntn.kind,functions.clone())?;
    let target_knd = knd.to_value_kind(functions.clone())?;
    // Do type checking
    match (&result, &target_knd) {
      (Value::Atom(given_variant_id), ValueKind::Enum(enum_id)) => {
        let fxns_brrw = functions.borrow();
        let my_enum = match fxns_brrw.enums.get(enum_id) {
          Some(my_enum) => my_enum,
          None => todo!(),
        };
        if !my_enum.variants.iter().any(|(enum_variant, inner_value)| *given_variant_id == *enum_variant) {
          return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnknownEnumVairant(*enum_id,*given_variant_id)}); 
        }
      }
      (Value::Atom(given_variant_id), target_kind) => {
        return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnableToConvertValueKind}); 
      }
      x => {
        println!("{:?}",x);
      },
    }
    let convert_fxn = ConvertKind{}.compile(&vec![result.clone(), Value::Kind(target_knd)])?;
    convert_fxn.solve();
    let converted_result = convert_fxn.out();
    let mut plan_brrw = plan.borrow_mut();
    plan_brrw.push(convert_fxn);
    result = converted_result;
  };
  let mut symbols_brrw = symbols.borrow_mut();
  symbols_brrw.insert(id,result.clone());
  Ok(result)
}

fn kind_annotation(knd: &NodeKind, functions: FunctionsRef) -> MResult<Kind> {
  match knd {
    NodeKind::Scalar(id) => {
      let kind_id = id.hash();
      Ok(Kind::Scalar(kind_id))
    }
    NodeKind::Bracket((el_knds, size)) => {
      let mut knds = vec![];
      for knd in el_knds {
        let knd = kind_annotation(knd, functions.clone())?;
        knds.push(knd);
      }
      let mut dims = vec![];
      for dim in size {
        let dim_val = literal(dim, functions.clone())?;
        match dim_val.as_usize() {
          Some(size_val) => dims.push(size_val.clone()),
          None => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::ExpectedNumericForSize});} 
        }
      }
      if knds.len() != 1 {
        return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::MatrixMustHaveHomogenousKind});
      }
      Ok(Kind::Matrix(Box::new(knds[0].clone()),dims))
    }
    _ => todo!(),
  }
}

fn expression(expr: &Expression, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match &expr {
    Expression::Var(v) => var(&v, symbols.clone()),
    Expression::Range(rng) => range(&rng, plan.clone(), symbols.clone(), functions.clone()),
    Expression::Slice(slc) => slice(&slc, plan.clone(), symbols.clone(), functions.clone()),
    Expression::Formula(fctr) => factor(fctr, plan.clone(), symbols.clone(), functions.clone()),
    Expression::Structure(strct) => structure(strct, plan.clone(), symbols.clone(), functions.clone()),
    Expression::Literal(ltrl) => literal(&ltrl, functions.clone()),
    Expression::FunctionCall(fxn_call) => function_call(fxn_call, plan.clone(), symbols.clone(), functions.clone()),
    Expression::FsmPipe(_) => todo!(),
  }
}

fn function_call(fxn_call: &FunctionCall, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let fxn_name_id = fxn_call.name.hash();
  let fxns_brrw = functions.borrow();
  match fxns_brrw.functions.get(&fxn_name_id) {
    Some(fxn) => {
      let mut new_fxn = fxn.recompile(functions.clone())?; // This just calles function_define again, it should be smarter.
      for (ix,(arg_name, arg_expr)) in fxn_call.args.iter().enumerate() {
        // Get the value
        let value_ref: ValRef = match arg_name {
          // Arg is called with a name
          Some(arg_id) => {
            match new_fxn.input.get(&arg_id.hash()) {
              // Arg name matches expected name
              Some(kind) => {
                let symbols_brrw = new_fxn.symbols.borrow();
                symbols_brrw.get(arg_id.hash()).unwrap().clone()
              }
              // The argument name doesn't match
              None => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnknownFunctionArgument(arg_id.hash())});}
            }
          }
          // Arg is called positionally (no arg name supplied)
          None => {
            match &new_fxn.input.iter().nth(ix) {
              Some((arg_id,kind)) => {
                let symbols_brrw = new_fxn.symbols.borrow();
                symbols_brrw.get(**arg_id).unwrap().clone()
              }
              None => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::TooManyInputArguments(ix+1,new_fxn.input.len())});} 
            }
          }
        };
        let result = expression(&arg_expr, plan.clone(), symbols.clone(), functions.clone())?;
        let mut ref_brrw = value_ref.borrow_mut();
        // TODO check types
        match (&mut *ref_brrw, &result) {
          (Value::I64(arg_ref), Value::I64(i64_ref)) => {
            *arg_ref.borrow_mut() = i64_ref.borrow().clone();
          }
          (Value::F64(arg_ref), Value::F64(f64_ref)) => {
            *arg_ref.borrow_mut() = f64_ref.borrow().clone();
          }
          (x,y) => {return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::KindMismatch(x.kind(),y.kind())});}
        }
      }
      // schedule function
      let mut plan_brrw = plan.borrow_mut();
      let result = new_fxn.solve();
      let result_brrw = result.borrow();
      plan_brrw.push(Box::new(UserFunction{fxn: new_fxn.clone()}));
      return Ok(result_brrw.clone())
    }
    None => { 
      match fxns_brrw.function_compilers.get(&fxn_name_id) {
        Some(fxn_compiler) => {
          let mut input_arg_values = vec![];
          for (arg_name, arg_expr) in fxn_call.args.iter() {
            let result = expression(&arg_expr, plan.clone(), symbols.clone(), functions.clone())?;
            input_arg_values.push(result);
          }
          match fxn_compiler.compile(&input_arg_values) {
            Ok(new_fxn) => {
              let mut plan_brrw = plan.borrow_mut();
              new_fxn.solve();
              let result = new_fxn.out();
              plan_brrw.push(new_fxn);
              return Ok(result)
            }
            Err(x) => {return Err(x);}
          }
        }
        None => {return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::MissingFunction(fxn_name_id)});}
      }
    }
  }   
  unreachable!()
}

fn range(rng: &RangeExpression, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let start = factor(&rng.start, plan.clone(),symbols.clone(), functions.clone())?;
  let terminal = factor(&rng.terminal, plan.clone(),symbols.clone(), functions.clone())?;
  let new_fxn = match &rng.operator {
    RangeOp::Exclusive => RangeExclusive{}.compile(&vec![start,terminal])?,
    RangeOp::Inclusive => RangeInclusive{}.compile(&vec![start,terminal])?,
    x => unreachable!(),
  };
  let mut plan_brrw = plan.borrow_mut();
  plan_brrw.push(new_fxn);
  let step = plan_brrw.last().unwrap();
  step.solve();
  let res = step.out();
  Ok(res)
}

fn slice(slc: &Slice, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let name = slc.name.hash();
  let symbols_brrw = symbols.borrow();
  let val: Value = match symbols_brrw.get(name) {
    Some(val) => Value::MutableReference(val.clone()),
    None => {return Err(MechError{tokens: slc.name.tokens(), msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedVariable(name)});}
  };
  for s in &slc.subscript {
    let s_result = subscript(&s, &val, plan.clone(), symbols.clone(), functions.clone())?;
    return Ok(s_result);
  }
  unreachable!() // subscript should have thrown an error if we can't access an element
}

fn subscript_formula(sbscrpt: &Subscript, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match sbscrpt {
    Subscript::Formula(fctr) => {
      let result = factor(fctr,plan.clone(), symbols.clone(), functions.clone())?;
      result.as_index()
    }
    _ => unreachable!()
  }
}

fn subscript_range(sbscrpt: &Subscript, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match sbscrpt {
    Subscript::Range(rng) => {
      let result = range(rng,plan.clone(), symbols.clone(), functions.clone())?;
      match result.as_vecusize() {
        Some(v) => Ok(v.to_value()),
        None => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledIndexKind}),
      }
    }
    _ => unreachable!()
  }
}

fn subscript_ref(sbscrpt: &Subscript, sink: &Value, source: &Value, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match sbscrpt {
    Subscript::Dot(x) => {
      todo!()
    },
    Subscript::DotInt(x) => {
      todo!()
    },
    Subscript::Swizzle(x) => {
      unreachable!()
    },
    Subscript::Bracket(subs) => {
      let mut fxn_input = vec![sink.clone()];
      match &subs[..] {
        [Subscript::Formula(ix)] => {
          let ixes = subscript_formula(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = ixes.shape();
          fxn_input.push(source.clone());
          fxn_input.push(ixes);
          plan.borrow_mut().push(MatrixSetScalar{}.compile(&fxn_input)?);
          /*match shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixAccessScalar{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixAccessRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixAccessRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }*/
        },
        [Subscript::Range(ix)] => {
          todo!()
        },
        [Subscript::All] => {
          todo!()
        },
        [Subscript::All,Subscript::All] => todo!(),
        [Subscript::Formula(ix1),Subscript::Formula(ix2)] => {
          todo!()
        },
        [Subscript::Range(ix1),Subscript::Range(ix2)] => {
          todo!()
        },
        [Subscript::All,Subscript::Formula(ix2)] => {
          todo!()
        },
        [Subscript::Formula(ix1),Subscript::All] => {
          todo!()
        },
        [Subscript::Range(ix1),Subscript::Formula(ix2)] => {
          todo!()
        },
        [Subscript::Formula(ix1),Subscript::Range(ix2)] => {
          todo!()
        },
        [Subscript::All,Subscript::Range(ix2)] => {
          todo!()
        },
        [Subscript::Range(ix1),Subscript::All] => {
          todo!()
        },
        _ => unreachable!(),
      };
      let plan_brrw = plan.borrow();
      let mut new_fxn = &plan_brrw.last().unwrap();
      new_fxn.solve();
      let res = new_fxn.out();
      return Ok(res);
    },
    Subscript::Brace(x) => todo!(),
    _ => unreachable!(),
  }
}

fn subscript(sbscrpt: &Subscript, val: &Value, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match sbscrpt {
    Subscript::Dot(x) => {
      let key = x.hash();
      let fxn_input: Vec<Value> = vec![val.clone(), Value::Id(key)];
      let new_fxn = AccessColumn{}.compile(&fxn_input)?;
      new_fxn.solve();
      let res = new_fxn.out();
      plan.borrow_mut().push(new_fxn);
      return Ok(res);
    },
    Subscript::DotInt(x) => {
      let mut fxn_input = vec![val.clone()];
      let result = real(&x.clone());
      fxn_input.push(result.as_index()?);
      let new_fxn = MatrixAccessScalar{}.compile(&fxn_input)?; // This presumes the thing is a matrix...
      new_fxn.solve();
      let res = new_fxn.out();
      plan.borrow_mut().push(new_fxn);
      return Ok(res);
    },
    Subscript::Swizzle(x) => {
      let mut keys = x.iter().map(|x| Value::Id(x.hash())).collect::<Vec<Value>>();
      let mut fxn_input: Vec<Value> = vec![val.clone()];
      fxn_input.append(&mut keys);
      let new_fxn = AccessSwizzle{}.compile(&fxn_input)?;
      new_fxn.solve();
      let res = new_fxn.out();
      plan.borrow_mut().push(new_fxn);
      return Ok(res);
    },
    Subscript::Bracket(subs) => {
      let mut fxn_input = vec![val.clone()];
      match &subs[..] {
        [Subscript::Formula(ix)] => {
          let result = subscript_formula(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = result.shape();
          fxn_input.push(result);
          match shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixAccessScalar{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixAccessRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixAccessRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::Range(ix)] => {
          let result = subscript_range(&subs[0],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixAccessRange{}.compile(&fxn_input)?);
        },
        [Subscript::All] => {
          fxn_input.push(Value::IndexAll);
          plan.borrow_mut().push(MatrixAccessAll{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::All] => todo!(),
        [Subscript::Formula(ix1),Subscript::Formula(ix2)] => {
          let result = subscript_formula(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          let shape1 = result.shape();
          fxn_input.push(result);
          let result = subscript_formula(&subs[1], plan.clone(), symbols.clone(), functions.clone())?;
          let shape2 = result.shape();
          fxn_input.push(result);
          match ((shape1[0],shape1[1]),(shape2[0],shape2[1])) {
            ((1,1),(1,1)) => plan.borrow_mut().push(MatrixAccessScalarScalar{}.compile(&fxn_input)?),
            ((1,1),(1,m)) => plan.borrow_mut().push(MatrixAccessScalarRange{}.compile(&fxn_input)?),
            ((1,n),(1,1)) => plan.borrow_mut().push(MatrixAccessRangeScalar{}.compile(&fxn_input)?),
            ((n,1),(1,m)) |
            ((n,1),(m,1)) |
            ((1,n),(m,1)) |
            ((1,n),(1,m)) => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::Range(ix1),Subscript::Range(ix2)] => {
          let result = subscript_range(&subs[0],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          let result = subscript_range(&subs[1],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::Formula(ix2)] => {
          fxn_input.push(Value::IndexAll);
          let result = subscript_formula(&subs[1], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = result.shape();
          fxn_input.push(result);
          match &shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixAccessAllScalar{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixAccessAllRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixAccessAllRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::Formula(ix1),Subscript::All] => {
          let result = subscript_formula(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = result.shape();
          fxn_input.push(result);
          fxn_input.push(Value::IndexAll);
          match &shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixAccessScalarAll{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixAccessRangeAll{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixAccessRangeAll{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::Range(ix1),Subscript::Formula(ix2)] => {
          let result = subscript_range(&subs[0],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          let result = subscript_formula(&subs[1], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = result.shape();
          fxn_input.push(result);
          match &shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixAccessRangeScalar{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::Formula(ix1),Subscript::Range(ix2)] => {
          let result = subscript_formula(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = result.shape();
          fxn_input.push(result);
          let result = subscript_range(&subs[1],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          match &shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixAccessScalarRange{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::All,Subscript::Range(ix2)] => {
          fxn_input.push(Value::IndexAll);
          let result = subscript_range(&subs[1],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixAccessAllRange{}.compile(&fxn_input)?);
        },
        [Subscript::Range(ix1),Subscript::All] => {
          let result = subscript_range(&subs[0],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          fxn_input.push(Value::IndexAll);
          plan.borrow_mut().push(MatrixAccessRangeAll{}.compile(&fxn_input)?);
        },
        _ => unreachable!(),
      };
      let plan_brrw = plan.borrow();
      let mut new_fxn = &plan_brrw.last().unwrap();
      new_fxn.solve();
      let res = new_fxn.out();
      return Ok(res);
    },
    Subscript::Brace(x) => todo!(),
    _ => unreachable!(),
  }
}

fn structure(strct: &Structure, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match strct {
    Structure::Empty => Ok(Value::Empty),
    Structure::Record(x) => record(&x, plan.clone(), symbols.clone(), functions.clone()),
    Structure::Matrix(x) => matrix(&x, plan.clone(), symbols.clone(), functions.clone()),
    Structure::Table(x) => table(&x, plan.clone(), symbols.clone(), functions.clone()),
    Structure::Tuple(x) => tuple(&x, plan.clone(), symbols.clone(), functions.clone()),
    Structure::TupleStruct(x) => todo!(),
    Structure::Set(x) => set(&x, plan.clone(), symbols.clone(), functions.clone()),
    Structure::Map(x) => map(&x, plan.clone(), symbols.clone(), functions.clone()),
  }
}

fn tuple(tpl: &Tuple, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut elements = vec![];
  for el in &tpl.elements {
    let result = expression(el,plan.clone(),symbols.clone(), functions.clone())?;
    elements.push(Box::new(result));
  }
  let mech_tuple = MechTuple{elements};
  Ok(Value::Tuple(mech_tuple))
}

fn map(mp: &Map, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut m = IndexMap::new();
  for b in &mp.elements {
    let key = expression(&b.key, plan.clone(), symbols.clone(), functions.clone())?;
    let val = expression(&b.value, plan.clone(), symbols.clone(), functions.clone())?;
    m.insert(key,val);
  }
  Ok(Value::Map(MechMap{map: m}))
}

fn record(rcrd: &Record, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut m = IndexMap::new();
  for b in &rcrd.bindings {
    let name = b.name.hash();
    let kind = &b.kind;
    let val = expression(&b.value, plan.clone(), symbols.clone(), functions.clone())?;
    m.insert(Value::Id(name),val);
  }
  Ok(Value::Record(MechMap{map: m}))
}

fn set(m: &Set, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> { 
  let mut out = IndexSet::new();
  for el in &m.elements {
    let result = expression(el, plan.clone(), symbols.clone(), functions.clone())?;
    out.insert(result);
  }
  Ok(Value::Set(MechSet{set: out}))
}

macro_rules! handle_value_kind {
  ($value_kind:ident, $val:expr, $field_label:expr, $data_map:expr, $converter:ident) => {{
      let mut vals = Vec::new();
      for x in $val.as_vec().iter() {
        match x.$converter() {
          Some(u) => vals.push(u.to_value()),
          None => {return Err(MechError {tokens: vec![],msg: file!().to_string(),id: line!(),kind: MechErrorKind::WrongTableColumnKind,});}
        }
      }
      $data_map.insert($field_label.clone(), ($value_kind, Value::to_matrix(vals.clone(), vals.len(), 1)));
  }};
}

fn table(t: &Table, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> { 
  let mut rows = vec![];
  let (ids,col_kinds) = table_header(&t.header, functions.clone())?;
  let mut cols = 0;
  // Interpret the rows
  for row in &t.rows {
    let result = table_row(row, plan.clone(), symbols.clone(), functions.clone())?;
    cols = result.len();
    rows.push(result);
  }
  // Provision columns
  let mut data = Vec::new();
  for i in 0..cols {
    data.push(vec![])
  }
  // Populate columns with data from rows
  for row in rows {
    for (ix,el) in row.iter().enumerate() {
      data[ix].push(el.clone());
    }
  }
  // Build the table
  let mut data_map = IndexMap::new();
  for (field_label,(column,knd)) in ids.iter().zip(data.iter().zip(col_kinds)) {
    let val = Value::to_matrix(column.clone(),column.len(),1);
    match knd {
      ValueKind::I8   => handle_value_kind!(knd, val, field_label, data_map, as_i8),
      ValueKind::I16  => handle_value_kind!(knd, val, field_label, data_map, as_i16),
      ValueKind::I32  => handle_value_kind!(knd, val, field_label, data_map, as_i32),
      ValueKind::I64  => handle_value_kind!(knd, val, field_label, data_map, as_i64),
      ValueKind::I128 => handle_value_kind!(knd, val, field_label, data_map, as_i128),      
      ValueKind::U8   => handle_value_kind!(knd, val, field_label, data_map, as_u8),
      ValueKind::U16  => handle_value_kind!(knd, val, field_label, data_map, as_u16),
      ValueKind::U32  => handle_value_kind!(knd, val, field_label, data_map, as_u32),
      ValueKind::U64  => handle_value_kind!(knd, val, field_label, data_map, as_u64),
      ValueKind::U128 => handle_value_kind!(knd, val, field_label, data_map, as_u128),
      ValueKind::F32  => handle_value_kind!(knd, val, field_label, data_map, as_f32),
      ValueKind::F64  => handle_value_kind!(knd, val, field_label, data_map, as_f64),
      ValueKind::Bool => {
        let vals: Vec<Value> = val.as_vec().iter().map(|x| x.as_bool().unwrap().to_value()).collect::<Vec<Value>>();
        data_map.insert(field_label.clone(),(knd,Value::to_matrix(vals.clone(),vals.len(),1)));
      },
      _ => todo!(),
    };
  }
  let tbl = MechTable{rows: t.rows.len(), cols, data: data_map.clone()  };
  Ok(Value::Table(tbl))
}

fn table_header(fields: &Vec<Field>, functions: FunctionsRef) -> MResult<(Vec<Value>,Vec<ValueKind>)> {
  let mut ids: Vec<Value> = Vec::new();
  let mut kinds: Vec<ValueKind> = Vec::new();
  for f in fields {
    let id = f.name.hash();
    let kind = kind_annotation(&f.kind.kind, functions.clone())?;
    ids.push(Value::Id(id));
    kinds.push(kind.to_value_kind(functions.clone())?);
  }
  Ok((ids,kinds))
}

fn table_row(r: &TableRow, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Vec<Value>> {
  let mut row: Vec<Value> = Vec::new();
  for col in &r.columns {
    let result = table_column(col, plan.clone(), symbols.clone(), functions.clone())?;
    row.push(result);
  }
  Ok(row)
}

fn table_column(r: &TableColumn, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> { 
  expression(&r.element, plan.clone(), symbols.clone(), functions.clone())
}

fn matrix(m: &Mat, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut out = vec![];
  for row in &m.rows {
    let result = matrix_row(row, plan.clone(), symbols.clone(), functions.clone())?;
    out.push(result);
  }

  if out.is_empty() {
    return Ok(Value::MatrixF64(Matrix::<F64>::DMatrix(new_ref(DMatrix::from_vec(0, 0, vec![])))));
  }

  let shape = out[0].shape();
  let col_n = shape[1];
  let row_n = out.len();

  // Function to put element vector into column-major ordering so it can be reconstituted into a matrix
  fn to_column_major<T: Clone>(out: &[Value], row_n: usize, col_n: usize, extract_fn: impl Fn(&Value) -> Option<Vec<T>> + Clone) -> Vec<T> {
    (0..col_n).flat_map(|col| out.iter().map({let value = extract_fn.clone();move |row| value(row).unwrap()[col].clone()})).collect()
  }

  let mat = match &out[0] {
    Value::MatrixBool(_) => Value::MatrixBool(bool::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecbool()), row_n, col_n)),
    Value::MatrixU8(_)   => Value::MatrixU8(u8::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu8()), row_n, col_n)),
    Value::MatrixU16(_)  => Value::MatrixU16(u16::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu16()), row_n, col_n)),
    Value::MatrixU32(_)  => Value::MatrixU32(u32::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu32()), row_n, col_n)),
    Value::MatrixU64(_)  => Value::MatrixU64(u64::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu64()), row_n, col_n)),
    Value::MatrixU128(_) => Value::MatrixU128(u128::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu128()), row_n, col_n)),
    Value::MatrixI8(_)   => Value::MatrixI8(i8::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci8()), row_n, col_n)),
    Value::MatrixI16(_)  => Value::MatrixI16(i16::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci16()), row_n, col_n)),
    Value::MatrixI32(_)  => Value::MatrixI32(i32::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci32()), row_n, col_n)),
    Value::MatrixI64(_)  => Value::MatrixI64(i64::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci64()), row_n, col_n)),
    Value::MatrixI128(_) => Value::MatrixI128(i128::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci128()), row_n, col_n)),
    Value::MatrixF32(_)  => Value::MatrixF32(F32::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecf32()), row_n, col_n)),
    Value::MatrixF64(_)  => Value::MatrixF64(F64::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecf64()), row_n, col_n)),
    _ => todo!(),
  };

  Ok(mat)
}

fn matrix_row(r: &MatrixRow, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut row: Vec<Value> = Vec::new();
  for col in &r.columns {
    let result = matrix_column(col, plan.clone(), symbols.clone(), functions.clone())?;
    row.push(result);
  }
  let mat = match &row[0] {
    Value::Bool(_) => {Value::MatrixBool(bool::to_matrix(row.iter().map(|v| v.as_bool().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::U8(_)   => {Value::MatrixU8(u8::to_matrix(row.iter().map(|v| v.as_u8().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::U16(_)  => {Value::MatrixU16(u16::to_matrix(row.iter().map(|v| v.as_u16().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::U32(_)  => {Value::MatrixU32(u32::to_matrix(row.iter().map(|v| v.as_u32().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::U64(_)  => {Value::MatrixU64(u64::to_matrix(row.iter().map(|v| v.as_u64().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::U128(_) => {Value::MatrixU128(u128::to_matrix(row.iter().map(|v| v.as_u128().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I8(_)   => {Value::MatrixI8(i8::to_matrix(row.iter().map(|v| v.as_i8().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I16(_)  => {Value::MatrixI16(i16::to_matrix(row.iter().map(|v| v.as_i16().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I32(_)  => {Value::MatrixI32(i32::to_matrix(row.iter().map(|v| v.as_i32().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I64(_)  => {Value::MatrixI64(i64::to_matrix(row.iter().map(|v| v.as_i64().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I128(_) => {Value::MatrixI128(i128::to_matrix(row.iter().map(|v| v.as_i128().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::F32(_)  => {Value::MatrixF32(F32::to_matrix(row.iter().map(|v| v.as_f32().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::F64(_)  => {Value::MatrixF64(F64::to_matrix(row.iter().map(|v| v.as_f64().unwrap().borrow().clone()).collect(),1,row.len()))},
    _ => todo!(),
  };
  Ok(mat)
}

fn matrix_column(r: &MatrixColumn, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> { 
  expression(&r.element, plan.clone(), symbols.clone(), functions.clone())
}

fn var(v: &Var, symbols: SymbolTableRef) -> MResult<Value> {
  let id = v.name.hash();
  let symbols_brrw = symbols.borrow();
  match symbols_brrw.get(id) {
    Some(value) => {
      return Ok(Value::MutableReference(value.clone()))
    }
    None => {
      return Err(MechError{tokens: v.tokens(), msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedVariable(id)});
    }
  }
}

fn factor(fctr: &Factor, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match fctr {
    Factor::Term(trm) => {
      let result = term(trm, plan.clone(), symbols.clone(), functions.clone())?;
      Ok(result)
    },
    Factor::Expression(expr) => expression(expr, plan.clone(), symbols.clone(), functions.clone()),
    Factor::Negate(neg) => {
      let value = factor(neg, plan.clone(), symbols.clone(), functions.clone())?;
      let new_fxn = MathNegate{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      let mut plan_brrw = plan.borrow_mut();
      plan_brrw.push(new_fxn);
      Ok(out)
    },
    Factor::Not(neg) => {
      let value = factor(neg, plan.clone(), symbols.clone(), functions.clone())?;
      let new_fxn = LogicNot{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      let mut plan_brrw = plan.borrow_mut();
      plan_brrw.push(new_fxn);
      Ok(out)
    },
    Factor::Transpose(fctr) => {
      let value = factor(fctr, plan.clone(), symbols.clone(), functions.clone())?;
      let new_fxn = MatrixTranspose{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      let mut plan_brrw = plan.borrow_mut();
      plan_brrw.push(new_fxn);
      Ok(out)
    },
  }
}

fn term(trm: &Term, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut lhs = factor(&trm.lhs, plan.clone(), symbols.clone(), functions.clone())?;
  let mut term_plan: Vec<Box<dyn MechFunction>> = vec![];
  for (op,rhs) in &trm.rhs {
    let rhs = factor(&rhs, plan.clone(), symbols.clone(), functions.clone())?;
    let new_fxn = match op {
      FormulaOperator::AddSub(AddSubOp::Add) => MathAdd{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::AddSub(AddSubOp::Sub) => MathSub{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::MulDiv(MulDivOp::Mul) => MathMul{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::MulDiv(MulDivOp::Div) => MathDiv{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Exponent(ExponentOp::Exp) => MathExp{}.compile(&vec![lhs,rhs])?,

      FormulaOperator::Vec(VecOp::MatMul) => MatrixMatMul{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Vec(VecOp::Solve) => todo!(),
      FormulaOperator::Vec(VecOp::Cross) => todo!(),
      FormulaOperator::Vec(VecOp::Dot) => todo!(),

      FormulaOperator::Comparison(ComparisonOp::Equal) => CompareEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::NotEqual) => CompareNotEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::LessThanEqual) => CompareLessThanEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::GreaterThanEqual) => CompareGreaterThanEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::LessThan) => CompareLessThan{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::GreaterThan) => CompareGreaterThan{}.compile(&vec![lhs,rhs])?,

      FormulaOperator::Logic(LogicOp::And) => LogicAnd{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Logic(LogicOp::Or)  => LogicOr{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Logic(LogicOp::Not) => LogicNot{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Logic(LogicOp::Xor) => LogicXor{}.compile(&vec![lhs,rhs])?,
    };
    new_fxn.solve();
    let res = new_fxn.out();
    term_plan.push(new_fxn);
    lhs = res;
  }
  let mut plan_brrw = plan.borrow_mut();
  plan_brrw.append(&mut term_plan);
  return Ok(lhs);
}

fn literal(ltrl: &Literal, functions: FunctionsRef) -> MResult<Value> {
  match &ltrl {
    Literal::Empty(_) => Ok(empty()),
    Literal::Boolean(bln) => Ok(boolean(bln)),
    Literal::Number(num) => Ok(number(num)),
    Literal::String(strng) => Ok(string(strng)),
    Literal::Atom(atm) => Ok(atom(atm)),
    Literal::TypedLiteral((ltrl,kind)) => typed_literal(ltrl,kind,functions),
  }
}

fn typed_literal(ltrl: &Literal, knd_attn: &KindAnnotation, functions: FunctionsRef) -> MResult<Value> {
  let value = literal(ltrl,functions.clone())?;
  let kind = kind_annotation(&knd_attn.kind, functions.clone())?;
  match (&value,kind) {
    (Value::I64(num), Kind::Scalar(to_kind_id)) => {
      match functions.borrow().kinds.get(&to_kind_id) {
        Some(ValueKind::I8)   => Ok(Value::I8(new_ref(*num.borrow() as i8))),
        Some(ValueKind::I16)  => Ok(Value::I16(new_ref(*num.borrow() as i16))),
        Some(ValueKind::I32)  => Ok(Value::I32(new_ref(*num.borrow() as i32))),
        Some(ValueKind::I64)  => Ok(value),
        Some(ValueKind::I128) => Ok(Value::I128(new_ref(*num.borrow() as i128))),
        Some(ValueKind::U8)   => Ok(Value::U8(new_ref(*num.borrow() as u8))),
        Some(ValueKind::U16)  => Ok(Value::U16(new_ref(*num.borrow() as u16))),
        Some(ValueKind::U32)  => Ok(Value::U32(new_ref(*num.borrow() as u32))),
        Some(ValueKind::U64)  => Ok(Value::U64(new_ref(*num.borrow() as u64))),
        Some(ValueKind::U128) => Ok(Value::U128(new_ref(*num.borrow() as u128))),
        Some(ValueKind::F32)  => Ok(Value::F32(new_ref(F32::new(*num.borrow() as f32)))),
        Some(ValueKind::F64)  => Ok(Value::F64(new_ref(F64::new(*num.borrow() as f64)))),
        None => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedKind(to_kind_id)}),
        _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::CouldNotAssignKindToValue}),
      }
    }
    (Value::F64(num), Kind::Scalar(to_kind_id)) => {
      match functions.borrow().kinds.get(&to_kind_id) {
        Some(ValueKind::I8)   => Ok(Value::I8(new_ref((*num.borrow()).0 as i8))),
        Some(ValueKind::I16)  => Ok(Value::I16(new_ref((*num.borrow()).0 as i16))),
        Some(ValueKind::I32)  => Ok(Value::I32(new_ref((*num.borrow()).0 as i32))),
        Some(ValueKind::I64)  => Ok(Value::I64(new_ref((*num.borrow()).0 as i64))),
        Some(ValueKind::I128) => Ok(Value::I128(new_ref((*num.borrow()).0 as i128))),
        Some(ValueKind::U8)   => Ok(Value::U8(new_ref((*num.borrow()).0 as u8))),
        Some(ValueKind::U16)  => Ok(Value::U16(new_ref((*num.borrow()).0 as u16))),
        Some(ValueKind::U32)  => Ok(Value::U32(new_ref((*num.borrow()).0 as u32))),
        Some(ValueKind::U64)  => Ok(Value::U64(new_ref((*num.borrow()).0 as u64))),
        Some(ValueKind::U128) => Ok(Value::U128(new_ref((*num.borrow()).0 as u128))),
        Some(ValueKind::F32)  => Ok(Value::F32(new_ref(F32::new((*num.borrow()).0 as f32)))),
        Some(ValueKind::F64)  => Ok(value),
        None => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedKind(to_kind_id)}),
        _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::CouldNotAssignKindToValue}),
      }
    }
    _ => todo!(),
  }
}

fn atom(atm: &Atom) -> Value {
  let id = atm.name.hash();
  Value::Atom(id)
}

fn number(num: &Number) -> Value {
  match num {
    Number::Real(num) => real(num),
    Number::Imaginary(num) => todo!(),
  }
}

fn real(rl: &RealNumber) -> Value {
  match rl {
    RealNumber::Negated(num) => todo!(),
    RealNumber::Integer(num) => integer(num),
    RealNumber::Float(num) => float(num),
    RealNumber::Decimal(num) => dec(num),
    RealNumber::Hexadecimal(num) => hex(num),
    RealNumber::Octal(num) => oct(num),
    RealNumber::Binary(num) => binary(num),
    RealNumber::Scientific(num) => scientific(num),
    RealNumber::Rational(num) => todo!(),
  }
}

fn dec(bnry: &Token) -> Value {
  let binary_str: String = bnry.chars.iter().collect();
  let num = i64::from_str_radix(&binary_str, 10).unwrap();
  Value::I64(new_ref(num))
}

fn binary(bnry: &Token) -> Value {
  let binary_str: String = bnry.chars.iter().collect();
  let num = i64::from_str_radix(&binary_str, 2).unwrap();
  Value::I64(new_ref(num))
}

fn oct(octl: &Token) -> Value {
  let hex_str: String = octl.chars.iter().collect();
  let num = i64::from_str_radix(&hex_str, 8).unwrap();
  Value::I64(new_ref(num))
}

fn hex(hxdcml: &Token) -> Value {
  let hex_str: String = hxdcml.chars.iter().collect();
  let num = i64::from_str_radix(&hex_str, 16).unwrap();
  Value::I64(new_ref(num))
}

fn scientific(sci: &(Base,Exponent)) -> Value {
  let (base,exp): &(Base,Exponent) = sci;
  let (whole,part): &(Whole,Part) = base;
  let (sign,exp_whole, exp_part): &(Sign, Whole, Part) = exp;

  let a = whole.chars.iter().collect::<String>();
  let b = part.chars.iter().collect::<String>();
  let c = exp_whole.chars.iter().collect::<String>();
  let d = exp_part.chars.iter().collect::<String>();
  let num_f64: f64 = format!("{}.{}",a,b).parse::<f64>().unwrap();
  let mut exp_f64: f64 = format!("{}.{}",c,d).parse::<f64>().unwrap();
  if *sign {
    exp_f64 = -exp_f64;
  }

  let num = num_f64 * 10f64.powf(exp_f64);


  Value::F64(new_ref(F64(num)))
}

fn float(flt: &(Token,Token)) -> Value {
  let a = flt.0.chars.iter().collect::<String>();
  let b = flt.1.chars.iter().collect::<String>();
  let num: f64 = format!("{}.{}",a,b).parse::<f64>().unwrap();
  Value::F64(new_ref(F64(num)))
}

fn integer(int: &Token) -> Value {
  let num: f64 = int.chars.iter().collect::<String>().parse::<f64>().unwrap();
  Value::F64(new_ref(F64::new(num)))
}

fn string(tkn: &MechString) -> Value {
  let strng: String = tkn.text.chars.iter().collect::<String>();
  Value::String(strng)
}

fn empty() -> Value {
  Value::Empty
}

fn boolean(tkn: &Token) -> Value {
  let strng: String = tkn.chars.iter().collect::<String>();
  let val = match strng.as_str() {
    "true" => true,
    "false" => false,
    _ => unreachable!(),
  };
  Value::Bool(new_ref(val))
}