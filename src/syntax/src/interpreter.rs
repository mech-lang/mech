use crate::matrix::{Matrix, ToMatrix};
use crate::kind::Kind;
use crate::stdlib::math::*;
use crate::stdlib::logic::*;
use crate::stdlib::compare::*;
use crate::stdlib::matrix::*;
use crate::stdlib::range::*;
use crate::*;

use mech_core::{MechError, MechErrorKind, hash_str, nodes::Kind as NodeKind, nodes::*};
use mech_core::nodes::Matrix as Mat;
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
    SectionElement::Section(sctn) => todo!(),
    SectionElement::Comment(cmmnt) => Value::Empty,
    SectionElement::Paragraph(p) => Value::Empty,
    SectionElement::MechCode(code) => todo!(),
    SectionElement::UnorderedList(ul) => todo!(),
    SectionElement::CodeBlock => todo!(),
    SectionElement::OrderedList => todo!(),
    SectionElement::BlockQuote => todo!(),
    SectionElement::ThematicBreak => todo!(),
    SectionElement::Image => todo!(),
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
    let in_arg = Value::I64(new_ref(0));
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
    Statement::VariableAssign(_) => todo!(),
    Statement::KindDefine(_) => todo!(),
    Statement::EnumDefine(_) => todo!(),
    Statement::FsmDeclare(_) => todo!(),
    Statement::SplitTable => todo!(),
    Statement::FlattenTable => todo!(),
  }
}

fn variable_define(var_def: &VariableDefine, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let id = var_def.var.name.hash();
  let mut result = expression(&var_def.expression, plan.clone(), symbols.clone(), functions.clone())?;
  if let Some(knd_atn) =  &var_def.var.kind {
    let knd = kind_annotation(&knd_atn.kind,functions.clone())?;
    let target_knd = knd.to_value_kind(functions.clone())?;
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
          _ => todo!(),
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

fn subscript(sbscrpt: &Subscript, val: &Value, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match sbscrpt {
    Subscript::Dot(x) => {
      let key = x.hash();
      match val {
        Value::Record(rcrd) => {
          match rcrd.map.get(&Value::Id(key)) {
            Some(value) => return Ok(value.clone()),
            None => { return Err(MechError{tokens: x.tokens(), msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedField(key)});}
          }
        }
        Value::MutableReference(r) => match &*r.borrow() {
          Value::Record(rcrd) => {
            match rcrd.map.get(&Value::Id(key)) {
              Some(value) => return Ok(value.clone()),
              None => { return Err(MechError{tokens: x.tokens(), msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedField(key)});}
            }
          }
          _ => todo!(),
        }
        _ => todo!(),
      }
    },
    Subscript::Range(x) => todo!(),
    Subscript::Swizzle(x) => todo!(),
    Subscript::Formula(fctr) => {return factor(fctr,plan.clone(), symbols.clone(), functions.clone());},
    Subscript::Bracket(subs) => {
      let mut resolved_subs = vec![];
      for s in subs {
        let result = subscript(&s, val, plan.clone(), symbols.clone(), functions.clone())?;
        resolved_subs.push(result);
      }
      match val {
        Value::MatrixI64(mat) => {
          let result = match &resolved_subs[..] {
            [Value::I64(ix)] => mat.index1d(*ix.borrow() as usize),
            [Value::I64(row_ix),Value::I64(col_ix)] => mat.index2d(*row_ix.borrow() as usize,*col_ix.borrow() as usize),
            _ => todo!(),
          };
          return Ok(Value::I64(new_ref(result)));
        }
        Value::MutableReference(x) => match &*x.borrow() {
          Value::MatrixI64(mat) => {
            let result = match &resolved_subs[..] {
              [Value::I64(ix)] => mat.index1d(*ix.borrow() as usize),
              [Value::I64(row_ix),Value::I64(col_ix)] => mat.index2d(*row_ix.borrow() as usize,*col_ix.borrow() as usize),
              _ => todo!(),
            };
            return Ok(Value::I64(new_ref(result)));
          }
          _ => todo!(),
        }
        x => {
          println!("{:?}",x);
          todo!()
        },
      }
    },
    Subscript::Brace(x) => todo!(),
    Subscript::All => todo!(),
  }
  return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::None});
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

fn table(t: &Table, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> { 
  let mut rows = vec![];
  let header = table_header(&t.header)?;
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
  for (field_label,column) in header.iter().zip(data.iter()) {
    data_map.insert(field_label.clone(),column.clone());
  }
  let tbl = MechTable{rows: t.rows.len(), cols, data: data_map.clone()  };
  Ok(Value::Table(tbl))
}

fn table_header(fields: &Vec<Field>) -> MResult<Vec<Value>> {
  let mut row: Vec<Value> = Vec::new();
  for f in fields {
    let id = f.name.hash();
    let kind = &f.kind;
    row.push(Value::Id(id));
  }
  Ok(row)
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
    Value::MatrixU8(_) => Value::MatrixU8(u8::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu8()), row_n, col_n)),
    Value::MatrixU16(_) => Value::MatrixU16(u16::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu16()), row_n, col_n)),
    Value::MatrixU32(_) => Value::MatrixU32(u32::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu32()), row_n, col_n)),
    Value::MatrixU64(_) => Value::MatrixU64(u64::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu64()), row_n, col_n)),
    Value::MatrixU128(_) => Value::MatrixU128(u128::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecu128()), row_n, col_n)),
    Value::MatrixI8(_) => Value::MatrixI8(i8::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci8()), row_n, col_n)),
    Value::MatrixI16(_) => Value::MatrixI16(i16::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci16()), row_n, col_n)),
    Value::MatrixI32(_) => Value::MatrixI32(i32::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci32()), row_n, col_n)),
    Value::MatrixI64(_) => Value::MatrixI64(i64::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci64()), row_n, col_n)),
    Value::MatrixI128(_) => Value::MatrixI128(i128::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_veci128()), row_n, col_n)),
    Value::MatrixF32(_) => Value::MatrixF32(F32::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecf32()), row_n, col_n)),
    Value::MatrixF64(_) => Value::MatrixF64(F64::to_matrix(to_column_major(&out, row_n, col_n, |v| v.as_vecf64()), row_n, col_n)),
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
    Value::U8(_) => {Value::MatrixU8(u8::to_matrix(row.iter().map(|v| v.as_u8().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::U16(_) => {Value::MatrixU16(u16::to_matrix(row.iter().map(|v| v.as_u16().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::U32(_) => {Value::MatrixU32(u32::to_matrix(row.iter().map(|v| v.as_u32().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::U64(_) => {Value::MatrixU64(u64::to_matrix(row.iter().map(|v| v.as_u64().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::U128(_) => {Value::MatrixU128(u128::to_matrix(row.iter().map(|v| v.as_u128().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I8(_) => {Value::MatrixI8(i8::to_matrix(row.iter().map(|v| v.as_i8().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I16(_) => {Value::MatrixI16(i16::to_matrix(row.iter().map(|v| v.as_i16().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I32(_) => {Value::MatrixI32(i32::to_matrix(row.iter().map(|v| v.as_i32().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I64(_) => {Value::MatrixI64(i64::to_matrix(row.iter().map(|v| v.as_i64().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::I128(_) => {Value::MatrixI128(i128::to_matrix(row.iter().map(|v| v.as_i128().unwrap().borrow().clone()).collect(),1,row.len()))},
    Value::F32(_) => {Value::MatrixF32(F32::to_matrix(row.iter().map(|v| F32::new(v.as_f32().unwrap().borrow().clone())).collect(),1,row.len()))},
    Value::F64(_) => {Value::MatrixF64(F64::to_matrix(row.iter().map(|v| F64::new(v.as_f64().unwrap().borrow().clone())).collect(),1,row.len()))},
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
      FormulaOperator::Logic(LogicOp::Or) => LogicOr{}.compile(&vec![lhs,rhs])?,
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
        Some(ValueKind::I8) => Ok(Value::I8(new_ref(*num.borrow() as i8))),
        Some(ValueKind::I16) => Ok(Value::I16(new_ref(*num.borrow() as i16))),
        Some(ValueKind::I32) => Ok(Value::I32(new_ref(*num.borrow() as i32))),
        Some(ValueKind::I64) => Ok(value),
        Some(ValueKind::I128) => Ok(Value::I128(new_ref(*num.borrow() as i128))),
        Some(ValueKind::U8) => Ok(Value::U8(new_ref(*num.borrow() as u8))),
        Some(ValueKind::U16) => Ok(Value::U16(new_ref(*num.borrow() as u16))),
        Some(ValueKind::U32) => Ok(Value::U32(new_ref(*num.borrow() as u32))),
        Some(ValueKind::U64) => Ok(Value::U64(new_ref(*num.borrow() as u64))),
        Some(ValueKind::U128) => Ok(Value::U128(new_ref(*num.borrow() as u128))),
        Some(ValueKind::F32) => Ok(Value::F32(new_ref(F32::new(*num.borrow() as f32)))),
        Some(ValueKind::F64) => Ok(Value::F64(new_ref(F64::new(*num.borrow() as f64)))),
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
    RealNumber::Decimal(num) => todo!(),
    RealNumber::Hexadecimal(num) => todo!(),
    RealNumber::Octal(num) => todo!(),
    RealNumber::Binary(num) => todo!(),
    RealNumber::Scientific(num) => todo!(),
    RealNumber::Rational(num) => todo!(),
  }
}

fn float(flt: &(Token,Token)) -> Value {
  let a = flt.0.chars.iter().collect::<String>();
  let b = flt.1.chars.iter().collect::<String>();
  let num: f64 = format!("{}.{}",a,b).parse::<f64>().unwrap();
  Value::F64(new_ref(F64(num)))
}

fn integer(int: &Token) -> Value {
  let num: i64 = int.chars.iter().collect::<String>().parse::<i64>().unwrap();
  Value::I64(new_ref(num))
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