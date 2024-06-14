use mech_core::{MechError, MechErrorKind, hash_str, ValueKind, nodes::*};
use crate::parser2::*;
use mech_core::nodes::Matrix as Mat;
use serde_derive::*;
use std::any::Any;
use hashbrown::{HashMap, HashSet};
use na::{Vector3, DVector, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix6, Matrix2};
use std::ops::AddAssign;
use std::ops::Add;
use std::rc::Rc;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use indexmap::set::IndexSet;
use indexmap::map::IndexMap;

// Value-----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
  Number(i64),
  String(String),
  Bool(bool),
  Atom(u64),
  Matrix(Matrix),
  Set(MechSet),
  Map(MechMap),
  Record(MechMap),
  Table(MechTable),
  Tuple(MechTuple),
  Id(u64),
  Empty
}

impl Hash for Value {
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      Value::Number(x) => (*x as i64).hash(state),
      Value::String(x) => x.hash(state),
      Value::Bool(x) => x.hash(state),
      Value::Atom(x) => x.hash(state),
      Value::Matrix(x) => x.hash(state),
      Value::Set(x) => x.hash(state),
      Value::Map(x) => x.hash(state),
      Value::Record(x) => x.hash(state),
      Value::Table(x) => x.hash(state),
      Value::Tuple(x) => x.hash(state),
      Value::Id(x) => x.hash(state),
      Value::Empty => Value::Empty.hash(state),
    }
  }
}

impl Value {
  pub fn shape(&self) -> (usize,usize) {
    match self {
      Value::Number(x) => (1,1),
      Value::String(x) => (1,1),
      Value::Bool(x) => (1,1),
      Value::Atom(x) => (1,1),
      Value::Matrix(x) => x.shape(),
      Value::Table(x) => x.shape(),
      Value::Set(x) => (1,x.set.len()),
      Value::Map(x) => (1,x.map.len()),
      Value::Record(x) => (1,x.map.len()),
      Value::Tuple(x) => (1,x.size()),
      Value::Empty => (0,0),
      Value::Id(x) => (0,0),
    }
  }
}

impl Add for Value {
  type Output = Value;

  fn add(self, rhs: Value) -> Self::Output {
    match (self,rhs) {
      (Value::Number(lhs), Value::Number(rhs)) => Value::Number(lhs + rhs),
      _ => Value::Empty,
    }
  }
}

impl AddAssign for Value {
  fn add_assign(&mut self, rhs: Value) {
    match (self.clone(),rhs) {
      (Value::Number(mut lhs),Value::Number(rhs)) => *self = Value::Number(lhs + rhs),
      _ => *self = Value::Empty,
    }
  }
}

//-----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechSet {
  set: IndexSet<Value>,
}

impl MechSet {
  pub fn from_vec(vec: Vec<Value>) -> MechSet {
    let mut set = IndexSet::new();
    for v in vec {
      set.insert(v);
    }
    MechSet{set}
  }
}

impl Hash for MechSet {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for x in self.set.iter() {
        x.hash(state)
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechMap {
  map: IndexMap<Value,Value>,
}

impl MechMap {
  pub fn from_vec(vec: Vec<(Value,Value)>) -> MechMap {
    let mut map = IndexMap::new();
    for (k,v) in vec {
      map.insert(k,v);
    }
    MechMap{map}
  }
}

impl Hash for MechMap {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for x in self.map.iter() {
        x.hash(state)
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechTable {
  rows: usize,
  cols: usize,
  data: IndexMap<Value,Vec<Value>>,
}

impl MechTable {
  pub fn shape(&self) -> (usize,usize) {
    (self.rows,self.cols)
  }
}

impl Hash for MechTable {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for (k,v) in self.data.iter() {
        k.hash(state);
        v.hash(state);
    }
  }
}

type Plan = Rc<RefCell<Vec<Box<dyn MechFunction>>>>;
type SymbolTable = Rc<RefCell<HashMap<u64,Value>>>;

pub struct FunctionDefinition {
  pub id: u64,
  pub input: HashMap<u64, KindAnnotation>,
  pub output: HashMap<u64, KindAnnotation>,
  pub symbols: SymbolTable,
  pub plan: Plan,
}

impl FunctionDefinition {

  pub fn new(id: u64) -> Self {
    Self {
      id,
      input: HashMap::new(),
      output: HashMap::new(),
      symbols: Rc::new(RefCell::new(HashMap::new())),
      plan: Rc::new(RefCell::new(Vec::new())),
    }
  }

}


// Matrix ---------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Matrix {
  RowDVector(RowDVector<i64>),
  RowVector2(RowVector2<i64>),
  RowVector3(RowVector3<i64>),
  RowVector4(RowVector4<i64>),
  Matrix1(Matrix1<i64>),
  Matrix2(Matrix2<i64>),
  Matrix3(Matrix3<i64>),
  Matrix4(Matrix4<i64>),
  Matrix2x3(Matrix2x3<i64>),
  DMatrix(DMatrix<i64>),
}

impl Hash for Matrix {
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      Matrix::RowDVector(x) => x.hash(state),
      Matrix::RowVector2(x) => x.hash(state),
      Matrix::RowVector3(x) => x.hash(state),
      Matrix::RowVector4(x) => x.hash(state),
      Matrix::Matrix1(x) => x.hash(state),
      Matrix::Matrix2(x) => x.hash(state),
      Matrix::Matrix3(x) => x.hash(state),
      Matrix::Matrix4(x) => x.hash(state),
      Matrix::Matrix2x3(x) => x.hash(state),
      Matrix::DMatrix(x) => x.hash(state),
    }
  }
}

impl Matrix {
  pub fn shape(&self) -> (usize,usize) {
    match self {
      Matrix::RowDVector(x) => x.shape(),
      Matrix::RowVector2(x) => x.shape(),
      Matrix::RowVector3(x) => x.shape(),
      Matrix::RowVector4(x) => x.shape(),
      Matrix::Matrix1(x) => x.shape(),
      Matrix::Matrix2(x) => x.shape(),
      Matrix::Matrix3(x) => x.shape(),
      Matrix::Matrix4(x) => x.shape(),
      Matrix::Matrix2x3(x) => x.shape(),
      Matrix::DMatrix(x) => x.shape(),
    }
  }

  pub fn index1d(&self, ix: usize) -> i64 {
    match self {
      Matrix::RowDVector(x) => *x.index(ix-1),
      Matrix::RowVector2(x) => todo!(),
      Matrix::RowVector3(x) => *x.index(ix-1),
      Matrix::RowVector4(x) => todo!(),
      Matrix::Matrix1(x) => todo!(),
      Matrix::Matrix2(x) => todo!(),
      Matrix::Matrix3(x) => todo!(),
      Matrix::Matrix4(x) => todo!(),
      Matrix::Matrix2x3(x) => todo!(),
      Matrix::DMatrix(x) => todo!(),
    }
  }

  pub fn index2d(&self, row: usize, col: usize) -> i64 {
    match self {
      Matrix::RowDVector(x) => *x.index((row-1,col-1)),
      Matrix::RowVector2(x) => todo!(),
      Matrix::RowVector3(x) => *x.index((row-1,col-1)),
      Matrix::RowVector4(x) => todo!(),
      Matrix::Matrix1(x) => todo!(),
      Matrix::Matrix2(x) => todo!(),
      Matrix::Matrix3(x) => todo!(),
      Matrix::Matrix4(x) => todo!(),
      Matrix::Matrix2x3(x) => todo!(),
      Matrix::DMatrix(x) => todo!(),
    }
  }
}

// ------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechTuple {
  elements: Vec<Box<Value>>
}

impl MechTuple {

  pub fn from_vec(elements: Vec<Value>) -> Self {
    MechTuple{elements: elements.iter().map(|m| Box::new(m.clone())).collect::<Vec<Box<Value>>>()}
  }

  pub fn size(&self) -> usize {
    self.elements.len()
  }

}

impl Hash for MechTuple {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for x in self.elements.iter() {
        x.hash(state)
    }
  }
}


// Functions
// ------------------------------------------------------------------------

// The naming scheme will be OP LHS RHS
// The abbreviations are:
// Rv - row vector
// Cv - col vector
// MXY  - Matrix size X Y, or just X if it's square

pub trait MechFunction {
  fn solve(&self) -> Value;
  fn to_string(&self) -> String;
}

// Define ---------------------------------------------------------------------

#[derive(Debug)]
struct VarDef {
  name: u64,
  source: u64,
}

impl MechFunction for VarDef {
  fn solve(&self) -> Value {
    Value::Empty
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Greater Than ---------------------------------------------------------------

#[derive(Debug)]
struct GTScalar {
  lhs: i64,
  rhs: i64,
}

impl MechFunction for GTScalar {
  fn solve(&self) -> Value {
    Value::Bool(self.lhs > self.rhs)
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Less Than ---------------------------------------------------------------

#[derive(Debug)]
struct LTScalar {
  lhs: i64,
  rhs: i64,
}

impl MechFunction for LTScalar {
  fn solve(&self) -> Value {
    Value::Bool(self.lhs < self.rhs)
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// And ------------------------------------------------------------------------

#[derive(Debug)]
struct AndScalar {
  lhs: bool,
  rhs: bool,
}

impl MechFunction for AndScalar {
  fn solve(&self) -> Value {
    Value::Bool(self.lhs && self.rhs)
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Or ------------------------------------------------------------------------

#[derive(Debug)]
struct OrScalar {
  lhs: bool,
  rhs: bool,
}

impl MechFunction for OrScalar {
  fn solve(&self) -> Value {
    Value::Bool(self.lhs || self.rhs)
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Add ------------------------------------------------------------------------

#[derive(Debug)]
struct AddEmpty {
  term: Term
}

impl MechFunction for AddEmpty {
  fn solve(&self) -> Value {
    Value::Empty
  }
  fn to_string(&self) -> String { format!("AddEmpty")}
}

#[derive(Debug)]
struct AddScalar {
  lhs: i64,
  rhs: i64,
}

impl MechFunction for AddScalar {
  fn solve(&self) -> Value {
    Value::Number(self.lhs + self.rhs)
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
struct AddRv3Rv3 {
  lhs: RowVector3<i64>,
  rhs: RowVector3<i64>,
}

impl MechFunction for AddRv3Rv3 {
  fn solve(&self) -> Value {
    let result = &self.lhs + &self.rhs;
    Value::Matrix(Matrix::RowVector3(result))
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
struct AddM3M3 {
  lhs: Matrix3<i64>,
  rhs: Matrix3<i64>,
}

impl MechFunction for AddM3M3 {
  fn solve(&self) -> Value {
    let result = &self.lhs + &self.rhs;
    Value::Matrix(Matrix::Matrix3(result))
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Sub ------------------------------------------------------------------------

#[derive(Debug)]
struct SubScalar {
  lhs: i64,
  rhs: i64,
}

impl MechFunction for SubScalar {
  fn solve(&self) -> Value {
    Value::Number(self.lhs - self.rhs)
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
struct SubRv3Rv3 {
  lhs: RowVector3<i64>,
  rhs: RowVector3<i64>,
}

impl MechFunction for SubRv3Rv3 {
  fn solve(&self) -> Value {
    let result = &self.lhs - &self.rhs;
    Value::Matrix(Matrix::RowVector3(result))
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Mul ------------------------------------------------------------------------

#[derive(Debug)]
struct MulScalar {
  lhs: i64,
  rhs: i64,
}

impl MechFunction for MulScalar {
  fn solve(&self) -> Value {
    Value::Number(self.lhs * self.rhs)
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Div ------------------------------------------------------------------------

#[derive(Debug)]
struct DivScalar {
  lhs: i64,
  rhs: i64,
}

impl MechFunction for DivScalar {
  fn solve(&self) -> Value {
    Value::Number(self.lhs / self.rhs)
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// MatMul ---------------------------------------------------------------------

#[derive(Debug)]
struct MatMulM2M2 {
  lhs: Matrix2<i64>,
  rhs: Matrix2<i64>,
}

impl MechFunction for MatMulM2M2 {
  fn solve(&self) -> Value {
    let result = &self.lhs * &self.rhs;
    Value::Matrix(Matrix::Matrix2(result))
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Transpose ------------------------------------------------------------------

#[derive(Debug)]
struct TransposeM2 {
  mat: Matrix2<i64>,
}

impl MechFunction for TransposeM2 {
  fn solve(&self) -> Value {
    let result = self.mat.transpose();
    Value::Matrix(Matrix::Matrix2(result))
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Negate ---------------------------------------------------------------------

#[derive(Debug)]
struct NegateF64 {
  n: i64,
}

impl MechFunction for NegateF64 {
  fn solve(&self) -> Value {
    Value::Number(-self.n)
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
struct NegateM2 {
  mat: Matrix2<i64>,
}

impl MechFunction for NegateM2 {
  fn solve(&self) -> Value {
    Value::Matrix(Matrix::Matrix2(-self.mat))
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

// Interpreter 
// ----------------------------------------------------------------------------

pub struct Interpreter {
  pub symbols: SymbolTable,
  pub plan: Plan,
  pub functions: HashMap<u64,FunctionDefinition>,
  pub storage: HashMap<u64,Rc<RefCell<Matrix>>>,
}

impl Interpreter {

  pub fn new() -> Interpreter {
    Interpreter {
      symbols: Rc::new(RefCell::new(HashMap::new())),
      plan: Rc::new(RefCell::new(Vec::new())),
      functions: HashMap::new(),
      storage: HashMap::new(),
    }
  }

  pub fn interpret(&mut self, tree: &Program) -> Result<Value,MechError> {
    self.program(tree, self.plan.clone(), self.symbols.clone())
  }
  
//-----------------------------------------------------------------------------

  fn program(&mut self, program: &Program, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    self.body(&program.body, plan.clone(), symbols.clone())
  }

  fn body(&mut self, body: &Body, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    let mut result = None;
    for sec in &body.sections {
      result = Some(self.section(&sec, plan.clone(), symbols.clone())?);
    }
    Ok(result.unwrap())
  }

  fn section(&mut self, section: &Section, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    let mut result = None;
    for el in &section.elements {
      result = Some(self.section_element(&el, plan.clone(), symbols.clone())?);
      println!("Interpreter Result: {:?}", result);
    }
    Ok(result.unwrap())
  }

  fn section_element(&mut self, element: &SectionElement, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    match element {
      SectionElement::MechCode(code) => {self.mech_code(&code, plan.clone(), symbols.clone())},
      _ => todo!(),
    }
  }

  fn mech_code(&mut self, code: &MechCode, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    match &code {
      MechCode::Expression(expr) => self.expression(&expr, plan.clone(), symbols.clone()),
      MechCode::Statement(stmt) => self.statement(&stmt, plan.clone(), symbols.clone()),
      MechCode::FsmSpecification(_) => todo!(),
      MechCode::FsmImplementation(_) => todo!(),
      MechCode::FunctionDefine(fxn_def) => self.function_define(&fxn_def, plan.clone(), symbols.clone()),
    }
  }

  fn function_define(&mut self, fxn_def: &FunctionDefine, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    let name_id = fxn_def.name.hash();
    let mut new_fxn = FunctionDefinition::new(name_id);
    for input_arg in &fxn_def.input {
      let arg_id = input_arg.name.hash();
      new_fxn.input.insert(arg_id,input_arg.kind.clone());
      new_fxn.symbols.borrow_mut().insert(arg_id,Value::Empty);
    }
    for output_arg in &fxn_def.output {
      let arg_id = output_arg.name.hash();
      new_fxn.output.insert(arg_id,output_arg.kind.clone());
    }
    for stmnt in &fxn_def.statements {
      self.statement(stmnt, new_fxn.plan.clone(), new_fxn.symbols.clone());
    }
    
    println!("!!!{:?}", new_fxn.symbols);
    let plan = new_fxn.plan.borrow();
    for fxn in plan.iter() {
      println!("!!!{:?}", fxn.to_string());
    }
    
    //self.functions.insert(name_id,new_fxn);
    Ok(Value::Empty)
  }

  fn statement(&mut self, stmt: &Statement, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    match stmt {
      Statement::VariableDefine(var_def) => self.variable_define(&var_def, plan.clone(), symbols.clone()),
      Statement::VariableAssign(_) => todo!(),
      Statement::KindDefine(_) => todo!(),
      Statement::EnumDefine(_) => todo!(),
      Statement::FsmDeclare(_) => todo!(),
      Statement::SplitTable => todo!(),
      Statement::FlattenTable => todo!(),
    }
  }

  fn variable_define(&mut self, var_def: &VariableDefine, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    let id = var_def.var.name.hash();
    let result = self.expression(&var_def.expression, plan.clone(), symbols.clone())?;
    let mut symbols_brrw = symbols.borrow_mut();
    symbols_brrw.insert(id,result.clone());
    let var_def = VarDef{name: id, source: 0};
    let mut plan_brrw = plan.borrow_mut();
    plan_brrw.push(Box::new(var_def));
    Ok(result)
  }

  fn expression(&mut self, expr: &Expression, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    match &expr {
      Expression::Var(v) => self.var(&v, symbols.clone()),
      Expression::Slice(slc) => self.slice(&slc, plan.clone(), symbols.clone()),
      Expression::Formula(fctr) => self.factor(fctr, plan.clone(), symbols.clone()),
      Expression::Structure(strct) => self.structure(strct, plan.clone(), symbols.clone()),
      Expression::Literal(ltrl) => Ok(self.literal(&ltrl)),
      Expression::FunctionCall(_) => todo!(),
      Expression::FsmPipe(_) => todo!(),
    }
  }

  fn slice(&mut self, slc: &Slice, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    let name = &slc.name.hash();
    let symbols_brrw = symbols.borrow();
    let val: Value = match symbols_brrw.get(name) {
      Some(val) => val.clone(),
      None => {return Err(MechError{tokens: slc.name.tokens(), msg: "interpreter.rs".to_string(), id: 440, kind: MechErrorKind::UndefinedVariable(*name)});}
    };
    for s in &slc.subscript {
      let s_result = self.subscript(&s, &val, plan.clone(), symbols.clone())?;
      return Ok(s_result);
    }
    unreachable!() // subscript should have through an error if we can't access an element
  }

  fn subscript(&mut self, sbscrpt: &Subscript, val: &Value, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    match sbscrpt {
      Subscript::Dot(x) => {
        let key = x.hash();
        match val {
          Value::Record(rcrd) => {
            match rcrd.map.get(&Value::Id(key)) {
              Some(value) => return Ok(value.clone()),
              None => { return Err(MechError{tokens: x.tokens(), msg: "interpreter.rs".to_string(), id: 434, kind: MechErrorKind::UndefinedField(key)});}
            }
          }
          _ => todo!(),
        }
      },
      Subscript::Swizzle(x) => todo!(),
      Subscript::Formula(fctr) => {return self.factor(fctr,plan.clone(), symbols.clone());},
      Subscript::Bracket(subs) => {
        let mut resolved_subs = vec![];
        for s in subs {
          let result = self.subscript(&s, val, plan.clone(), symbols.clone())?;
          resolved_subs.push(result);
        }
        match val {
          Value::Matrix(mat) => {
            let result = match resolved_subs[..] {
              [Value::Number(ix)] => mat.index1d(ix as usize),
              [Value::Number(row_ix),Value::Number(col_ix)] => mat.index2d(row_ix as usize,col_ix as usize),
              _ => todo!(),
            };
            return Ok(Value::Number(result));
          }
          _ => todo!(),
        }
      },
      Subscript::Brace(x) => todo!(),
      Subscript::All => todo!(),
    }
    return Err(MechError{tokens: vec![], msg: "interpreter.rs".to_string(), id: 580, kind: MechErrorKind::None});
  }

  fn structure(&mut self, strct: &Structure, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    match strct {
      Structure::Empty => Ok(Value::Empty),
      Structure::Record(x) => self.record(&x, plan.clone(), symbols.clone()),
      Structure::Matrix(x) => self.matrix(&x, plan.clone(), symbols.clone()),
      Structure::Table(x) => self.table(&x, plan.clone(), symbols.clone()),
      Structure::Tuple(x) => self.tuple(&x, plan.clone(), symbols.clone()),
      Structure::TupleStruct(x) => todo!(),
      Structure::Set(x) => self.set(&x, plan.clone(), symbols.clone()),
      Structure::Map(x) => self.map(&x, plan.clone(), symbols.clone()),
    }
  }

  fn tuple(&mut self, tpl: &Tuple, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    let mut elements = vec![];
    for el in &tpl.elements {
      let result = self.expression(el,plan.clone(),symbols.clone())?;
      elements.push(Box::new(result));
    }
    let mech_tuple = MechTuple{elements};
    Ok(Value::Tuple(mech_tuple))
  }

  fn map(&mut self, mp: &Map, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    let mut m = IndexMap::new();
    for b in &mp.elements {
      let key = self.expression(&b.key, plan.clone(), symbols.clone())?;
      let val = self.expression(&b.value, plan.clone(), symbols.clone())?;
      m.insert(key,val);
    }
    Ok(Value::Map(MechMap{map: m}))
  }

  fn record(&mut self, rcrd: &Record, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    let mut m = IndexMap::new();
    for b in &rcrd.bindings {
      let name = b.name.hash();
      let kind = &b.kind;
      let val = self.expression(&b.value, plan.clone(), symbols.clone())?;
      m.insert(Value::Id(name),val);
    }
    Ok(Value::Record(MechMap{map: m}))
  }

  fn set(&mut self, m: &Set, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> { 
    let mut out = IndexSet::new();
    for el in &m.elements {
      let result = self.expression(el, plan.clone(), symbols.clone())?;
      out.insert(result);
    }
    Ok(Value::Set(MechSet{set: out}))
  }

  fn table(&mut self, t: &Table, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> { 
    let mut rows = vec![];
    let header = self.table_header(&t.header)?;
    let mut cols = 0;
    // Interpret the rows
    for row in &t.rows {
      let result = self.table_row(row, plan.clone(), symbols.clone())?;
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

  fn table_header(&mut self, fields: &Vec<Field>) -> Result<Vec<Value>,MechError> {
    let mut row: Vec<Value> = Vec::new();
    for f in fields {
      let id = f.name.hash();
      let kind = &f.kind;
      row.push(Value::Id(id));
    }
    Ok(row)
  }

  fn table_row(&mut self, r: &TableRow, plan: Plan, symbols: SymbolTable) -> Result<Vec<Value>,MechError> {
    let mut row: Vec<Value> = Vec::new();
    for col in &r.columns {
      let result = self.table_column(col, plan.clone(), symbols.clone())?;
      row.push(result);
   }
    Ok(row)
  }

  fn table_column(&mut self, r: &TableColumn, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> { 
    self.expression(&r.element, plan.clone(), symbols.clone())
  }

  fn matrix(&mut self, m: &Mat, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> { 
    let mut out = vec![];
    for row in &m.rows {
      let result = self.matrix_row(row, plan.clone(), symbols.clone())?;
      out.push(result);
    }
    let (_,col_n) = out[0].shape();
    let out_vec = match (out.len(),col_n) {
      (1,_) => out[0].clone(),
      (2,2) => {
        let mut rows: Vec<RowVector2<i64>> = vec![];
        for o in &out {if let Value::Matrix(Matrix::RowVector2(v)) = &o {rows.push(v.clone());}}
        Value::Matrix(Matrix::Matrix2(Matrix2::from_rows(&[rows[0].clone(), rows[1].clone()])))
      }
      (2,3) => {
        let mut rows: Vec<RowVector3<i64>> = vec![];
        for o in &out {if let Value::Matrix(Matrix::RowVector3(v)) = &o {rows.push(v.clone());}}
        Value::Matrix(Matrix::Matrix2x3(Matrix2x3::from_rows(&[rows[0].clone(), rows[1].clone()])))
      }
      (3,3) => {
        let mut rows: Vec<RowVector3<i64>> = vec![];
        for o in &out {if let Value::Matrix(Matrix::RowVector3(v)) = &o {rows.push(v.clone());}}
        Value::Matrix(Matrix::Matrix3(Matrix3::from_rows(&[rows[0].clone(), rows[1].clone(), rows[2].clone()])))
      }
      (4,4) => {
        let mut rows: Vec<RowVector4<i64>> = vec![];
        for o in &out {if let Value::Matrix(Matrix::RowVector4(v)) = &o {rows.push(v.clone());}}
        Value::Matrix(Matrix::Matrix4(Matrix4::from_rows(&[rows[0].clone(), rows[1].clone(), rows[2].clone(), rows[3].clone()])))
      }
      _ => todo!(),
    };
    Ok(out_vec)
  }

  fn matrix_row(&mut self, r: &MatrixRow, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    let mut row: Vec<i64> = Vec::new();
    for col in &r.columns {
      let result = self.matrix_column(col, plan.clone(), symbols.clone())?;
      match result {
        Value::Number(n) => row.push(n),
        _ => todo!(),
      }
    }
    let out_vec = match row.len() {
      1 => Matrix::Matrix1(Matrix1::from_element(row[0].clone())),
      2 => Matrix::RowVector2(RowVector2::from_vec(row.clone())),
      3 => Matrix::RowVector3(RowVector3::from_vec(row.clone())),
      4 => Matrix::RowVector4(RowVector4::from_vec(row.clone())),
      n => Matrix::RowDVector(RowDVector::from_vec(row.clone())),
    };
    Ok(Value::Matrix(out_vec))
  }

  fn matrix_column(&mut self, r: &MatrixColumn, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> { 
    self.expression(&r.element, plan.clone(), symbols.clone())
  }

  fn var(&mut self, v: &Var, symbols: SymbolTable) -> Result<Value,MechError> {
    let id = v.name.hash();
    let symbols_brrw = symbols.borrow();
    match symbols_brrw.get(&id) {
      Some(value) => {
        return Ok(value.clone())         
      }
      None => {
        return Err(MechError{tokens: v.tokens(), msg: "interpreter.rs".to_string(), id: 618, kind: MechErrorKind::UndefinedVariable(id)});
      }
    }
  }

  fn factor(&mut self, fctr: &Factor, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    match fctr {
      Factor::Term(trm) => {
        let result = self.term(trm, plan.clone(), symbols.clone())?;
        Ok(result)
      },
      Factor::Expression(expr) => self.expression(expr, plan.clone(), symbols.clone()),
      Factor::Negated(neg) => {
        match self.factor(neg, plan.clone(), symbols.clone())? {
          Value::Matrix(Matrix::Matrix2(mat)) => {
            let fxn = NegateM2{mat}; 
            let out: Value = fxn.solve();
            let mut plan_brrw = plan.borrow_mut();
            plan_brrw.push(Box::new(fxn));
            return Ok(out);
          }
          Value::Number(n) => {
            let fxn = NegateF64{n}; 
            let out: Value = fxn.solve();
            let mut plan_brrw = plan.borrow_mut();
            plan_brrw.push(Box::new(fxn));
            return Ok(out);
          }
          _ => todo!(),
        }  
        return Err(MechError{tokens: vec![], msg: "interpreter.rs".to_string(), id: 643, kind: MechErrorKind::None});
      },
      Factor::Transpose(fctr) => {
        if let Value::Matrix(Matrix::Matrix2(mat)) = self.factor(fctr, plan.clone(), symbols.clone())? {
          let fxn = TransposeM2{mat}; 
          let out: Value = fxn.solve();
          let mut plan_brrw = plan.borrow_mut();
          plan_brrw.push(Box::new(fxn));
          return Ok(out);
        }
        return Err(MechError{tokens: vec![], msg: "interpreter.rs".to_string(), id: 652, kind: MechErrorKind::None});
      },
    }
  }

  fn term(&mut self, trm: &Term, plan: Plan, symbols: SymbolTable) -> Result<Value,MechError> {
    let mut lhs_result = self.factor(&trm.lhs, plan.clone(), symbols.clone())?;
    let mut term_plan: Vec<Box<dyn MechFunction>> = vec![];
    for (op,rhs) in &trm.rhs {
      let rhs_result = self.factor(&rhs, plan.clone(), symbols.clone())?;
      match (lhs_result, rhs_result, op) {
        // Compare
        (Value::Number(lhs), Value::Number(rhs), FormulaOperator::Comparison(ComparisonOp::LessThan)) =>
          term_plan.push(Box::new(LTScalar{lhs,rhs})),          
        (Value::Number(lhs), Value::Number(rhs), FormulaOperator::Comparison(ComparisonOp::GreaterThan)) =>
          term_plan.push(Box::new(GTScalar{lhs,rhs})),
        // And
        (Value::Bool(lhs), Value::Bool(rhs), FormulaOperator::Logic(LogicOp::And)) =>
          term_plan.push(Box::new(AndScalar{lhs,rhs})),
        // Or
        (Value::Bool(lhs), Value::Bool(rhs), FormulaOperator::Logic(LogicOp::Or)) =>
          term_plan.push(Box::new(OrScalar{lhs,rhs})),
        // Add
        (Value::Empty, Value::Empty, FormulaOperator::AddSub(AddSubOp::Add)) =>
          term_plan.push(Box::new(AddEmpty{term: trm.clone()})),
        (Value::Number(lhs), Value::Number(rhs), FormulaOperator::AddSub(AddSubOp::Add)) =>
          term_plan.push(Box::new(AddScalar{lhs,rhs})),
        (Value::Matrix(Matrix::RowVector3(lhs)), Value::Matrix(Matrix::RowVector3(rhs)), FormulaOperator::AddSub(AddSubOp::Add)) =>
          term_plan.push(Box::new(AddRv3Rv3{lhs,rhs})),
        (Value::Matrix(Matrix::Matrix3(lhs)), Value::Matrix(Matrix::Matrix3(rhs)), FormulaOperator::AddSub(AddSubOp::Add)) =>
          term_plan.push(Box::new(AddM3M3{lhs,rhs})),
        // Sub
        (Value::Number(lhs), Value::Number(rhs), FormulaOperator::AddSub(AddSubOp::Sub)) =>
          term_plan.push(Box::new(SubScalar{lhs,rhs})),
        (Value::Matrix(Matrix::RowVector3(lhs)), Value::Matrix(Matrix::RowVector3(rhs)), FormulaOperator::AddSub(AddSubOp::Sub)) =>
          term_plan.push(Box::new(SubRv3Rv3{lhs,rhs})),
        // Mul
        (Value::Number(lhs), Value::Number(rhs), FormulaOperator::MulDiv(MulDivOp::Mul)) =>
          term_plan.push(Box::new(MulScalar{lhs,rhs})),
        // Div
        (Value::Number(lhs), Value::Number(rhs), FormulaOperator::MulDiv(MulDivOp::Div)) =>
          term_plan.push(Box::new(DivScalar{lhs,rhs})),
        // Mat Mul
        (Value::Matrix(Matrix::Matrix2(lhs)), Value::Matrix(Matrix::Matrix2(rhs)), FormulaOperator::Vec(VecOp::MatMul)) => 
          term_plan.push(Box::new(MatMulM2M2{lhs,rhs})),
        x => {
          return Err(MechError{tokens: trm.tokens(), msg: "interpreter.rs".to_string(), id: 685, kind: MechErrorKind::UnhandledFunctionArgumentKind});
        }
      };
      let res = term_plan.last().unwrap().solve();
      lhs_result = res;
    }
    let mut plan_brrw = plan.borrow_mut();
    plan_brrw.append(&mut term_plan);
    return Ok(lhs_result);
  }

  fn literal(&mut self, ltrl: &Literal) -> Value {
    match &ltrl {
      Literal::Empty(_) => self.empty(),
      Literal::Boolean(bln) => self.boolean(bln),
      Literal::Number(num) => self.number(num),
      Literal::String(strng) => self.string(strng),
      Literal::Atom(atm) => self.atom(atm),
      Literal::TypedLiteral((ltrl,kind)) => {
        todo!();
      },
    }
  }

  fn atom(&mut self, atm: &Atom) -> Value {
    let id = atm.name.hash();
    Value::Atom(id)
  }

  fn number(&mut self, num: &Number) -> Value {
    match num {
      Number::Real(num) => self.real(num),
      Number::Imaginary(num) => todo!(),
    }
  }

  fn real(&mut self, rl: &RealNumber) -> Value {
    match rl {
      RealNumber::Negated(num) => todo!(),
      RealNumber::Integer(num) => self.integer(num),
      RealNumber::Float(num) => todo!(),
      RealNumber::Decimal(num) => todo!(),
      RealNumber::Hexadecimal(num) => todo!(),
      RealNumber::Octal(num) => todo!(),
      RealNumber::Binary(num) => todo!(),
      RealNumber::Scientific(num) => todo!(),
      RealNumber::Rational(num) => todo!(),
    }
  }

  fn integer(&mut self, int: &Token) -> Value {
    let num: i64 = int.chars.iter().collect::<String>().parse::<i64>().unwrap();
    Value::Number(num)
  }

  fn string(&mut self, tkn: &MechString) -> Value {
    let strng: String = tkn.text.chars.iter().collect::<String>();
    Value::String(strng)
  }

  fn empty(&self) -> Value {
    Value::Empty
  }

  fn boolean(&mut self, tkn: &Token) -> Value {
    let strng: String = tkn.chars.iter().collect::<String>();
    let val = match strng.as_str() {
      "true" => true,
      "false" => false,
      _ => unreachable!(),
    };
    Value::Bool(val)
  }

}