use mech_core::{MechError, MechErrorKind, hash_str, nodes::Kind as NodeKind, nodes::*, humanize};
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
use std::fmt;
use tabled::{
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};
use tabled::{settings::style::LineText};
use std::fmt::Debug;

// Value ----------------------------------------------------------------------

#[derive(PartialEq, Debug)]
pub struct F64(f64);
impl F64 {
  pub fn new(val: f64) -> F64 {
    F64(val)
  }
}
impl Eq for F64 {}
impl Hash for F64 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.to_bits().hash(state);
  }
}

#[derive(PartialEq, Debug)]
pub struct F32(f32);
impl F32 {
  pub fn new(val: f32) -> F32 {
    F32(val)
  }
}
impl Eq for F32 {}
impl Hash for F32 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.to_bits().hash(state);
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ValueKind {
  U8, U16, U32, U64, U128, I8, I16, I32, I64, I128, F32, F64, Str, Bool
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
  U8(Rc<RefCell<u8>>),
  U16(Rc<RefCell<u16>>),
  U32(Rc<RefCell<u32>>),
  U64(Rc<RefCell<u64>>),
  U128(Rc<RefCell<u128>>),
  I8(Rc<RefCell<i8>>),
  I16(Rc<RefCell<i16>>),
  I32(Rc<RefCell<i32>>),
  I64(Rc<RefCell<i64>>),
  I128(Rc<RefCell<i128>>),
  F32(Rc<RefCell<F32>>),
  F64(Rc<RefCell<F64>>),
  String(String),
  Bool(Rc<RefCell<bool>>),
  Atom(u64),
  Matrix(Matrix),
  Set(MechSet),
  Map(MechMap),
  Record(MechMap),
  Table(MechTable),
  Tuple(MechTuple),
  Id(u64),
  MutableReference(MutableReference),
  Empty
}

impl Hash for Value {
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      Value::U8(x) => x.borrow().hash(state),
      Value::U16(x) => x.borrow().hash(state),
      Value::U32(x) => x.borrow().hash(state),
      Value::U64(x) => x.borrow().hash(state),
      Value::U128(x) => x.borrow().hash(state),
      Value::I8(x) => x.borrow().hash(state),
      Value::I16(x) => x.borrow().hash(state),
      Value::I32(x) => x.borrow().hash(state),
      Value::I64(x) => x.borrow().hash(state),
      Value::I128(x) => x.borrow().hash(state),
      Value::F32(x) => x.borrow().hash(state),
      Value::F64(x) => x.borrow().hash(state),
      Value::String(x) => x.hash(state),
      Value::Bool(x) => x.borrow().hash(state),
      Value::Atom(x) => x.hash(state),
      Value::Matrix(x) => x.hash(state),
      Value::Set(x) => x.hash(state),
      Value::Map(x) => x.hash(state),
      Value::Record(x) => x.hash(state),
      Value::Table(x) => x.hash(state),
      Value::Tuple(x) => x.hash(state),
      Value::Id(x) => x.hash(state),
      Value::MutableReference(x) => x.borrow().hash(state),
      Value::Empty => Value::Empty.hash(state),
    }
  }
}

impl Value {
  pub fn shape(&self) -> (usize,usize) {
    match self {
      Value::U8(x) => (1,1),
      Value::U16(x) => (1,1),
      Value::U32(x) => (1,1),
      Value::U64(x) => (1,1),
      Value::U128(x) => (1,1),
      Value::I8(x) => (1,1),
      Value::I16(x) => (1,1),
      Value::I32(x) => (1,1),
      Value::I64(x) => (1,1),
      Value::I128(x) => (1,1),
      Value::F32(x) => (1,1),
      Value::F64(x) => (1,1),
      Value::String(x) => (1,1),
      Value::Bool(x) => (1,1),
      Value::Atom(x) => (1,1),
      Value::Matrix(x) => x.shape(),
      Value::Table(x) => x.shape(),
      Value::Set(x) => (1,x.set.len()),
      Value::Map(x) => (1,x.map.len()),
      Value::Record(x) => (1,x.map.len()),
      Value::Tuple(x) => (1,x.size()),
      Value::MutableReference(x) => (1,1),
      Value::Empty => (0,0),
      Value::Id(x) => (0,0),
    }
  }
}

trait ToValue {
  fn to_value(&self) -> Value;
}

impl ToValue for Rc<RefCell<u8>> {
  fn to_value(&self) -> Value {
    Value::U8(self.clone())
  }
}

impl ToValue for Rc<RefCell<u16>> {
  fn to_value(&self) -> Value {
    Value::U16(self.clone())
  }
}

impl ToValue for Rc<RefCell<u32>> {
  fn to_value(&self) -> Value {
    Value::U32(self.clone())
  }
}

impl ToValue for Rc<RefCell<u64>> {
  fn to_value(&self) -> Value {
    Value::U64(self.clone())
  }
}

impl ToValue for Rc<RefCell<u128>> {
  fn to_value(&self) -> Value {
    Value::U128(self.clone())
  }
}

impl ToValue for Rc<RefCell<i8>> {
  fn to_value(&self) -> Value {
    Value::I8(self.clone())
  }
}

impl ToValue for Rc<RefCell<i16>> {
  fn to_value(&self) -> Value {
    Value::I16(self.clone())
  }
}

impl ToValue for Rc<RefCell<i32>> {
  fn to_value(&self) -> Value {
    Value::I32(self.clone())
  }
}

impl ToValue for Rc<RefCell<i64>> {
  fn to_value(&self) -> Value {
    Value::I64(self.clone())
  }
}

impl ToValue for Rc<RefCell<i128>> {
  fn to_value(&self) -> Value {
    Value::I128(self.clone())
  }
}

/*
impl ToValue for Rc<RefCell<f32>> {
  fn to_value(&self) -> Value {
    Value::F32(F32::new(self.clone()))
  }
}

impl ToValue for Rc<RefCell<f64>> {
  fn to_value(&self) -> Value {
    Value::F64(F64::new(self.clone()))
  }
}*/

// Kind -----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
  Scalar(u64),
  Tuple,
  Bracket,
  Brace,
  Map,
  Atom,
  Function,
  Fsm,
  Empty,
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

pub struct Functions {
  pub functions: HashMap<u64,FunctionDefinition>,
  pub function_compilers: HashMap<u64, Box<dyn NativeFunctionCompiler>>,
  pub kinds: HashMap<u64,ValueKind>,
}

impl Functions {
  pub fn new() -> Self {
    Self {functions: HashMap::new(), function_compilers: HashMap::new(), kinds: HashMap::new()}
  }
}


type FunctionsRef = Rc<RefCell<Functions>>;
type Plan = Rc<RefCell<Vec<Box<dyn MechFunction>>>>;
type MutableReference = Rc<RefCell<Value>>;
type SymbolTableRef= Rc<RefCell<SymbolTable>>;
type ValRef = Rc<RefCell<Value>>;

#[derive(Clone, Debug)]
pub struct SymbolTable {
  pub symbols: HashMap<u64,ValRef>,
  pub reverse_lookup: HashMap<*const RefCell<Value>, u64>,
}

impl SymbolTable {

  pub fn new() -> SymbolTable {
    Self {
      symbols: HashMap::new(),
      reverse_lookup: HashMap::new(),
    }
  }

  pub fn get(&self, key: u64) -> Option<ValRef> {
    self.symbols.get(&key).cloned()
  }

  pub fn insert(&mut self, key: u64, value: Value) -> ValRef {
    let cell = Rc::new(RefCell::new(value));
    self.reverse_lookup.insert(Rc::as_ptr(&cell), key);
    self.symbols.insert(key,cell.clone());
    cell.clone()
  }
}


#[derive(Clone)]
pub struct FunctionDefinition {
  pub code: FunctionDefine,
  pub id: u64,
  pub name: String,
  pub input: IndexMap<u64, KindAnnotation>,
  pub output: IndexMap<u64, KindAnnotation>,
  pub symbols: SymbolTableRef,
  pub out: Rc<RefCell<Value>>,
  pub plan: Plan,
}

impl fmt::Debug for FunctionDefinition {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let input_str = format!("{:#?}", self.input);
    let output_str = format!("{:#?}", self.output);
    let symbols_str = format!("{:#?}", self.symbols);
    let mut plan_str = "".to_string();
    for step in self.plan.borrow().iter() {
      plan_str = format!("{}  - {}\n",plan_str,step.to_string());
    }
    let data = vec!["📥 Input", &input_str, 
                    "📤 Output", &output_str, 
                    "🔣 Symbols",   &symbols_str,
                    "📋 Plan", &plan_str];
    let mut table = tabled::Table::new(data);
    table
        .with(Style::modern())
        .with(Panel::header(format!("📈 UserFxn::{}\n({})", self.name, humanize(&self.id))))
        .with(Alignment::left());
    println!("{table}");
    Ok(())
  }
}

impl FunctionDefinition {

  pub fn new(id: u64, name: String, code: FunctionDefine) -> Self {
    Self {
      id,
      name,
      code,
      input: IndexMap::new(),
      output: IndexMap::new(),
      out: Rc::new(RefCell::new(Value::Empty)),
      symbols: Rc::new(RefCell::new(SymbolTable::new())),
      plan: Rc::new(RefCell::new(Vec::new())),
    }
  }

  pub fn recompile(&self, functions: FunctionsRef) -> Result<FunctionDefinition,MechError> {
    function_define(&self.code, functions.clone())
  }

  pub fn solve(&self) -> ValRef {
    let plan_brrw = self.plan.borrow();
    for step in plan_brrw.iter() {
      let result = step.solve();
    }
    self.out.clone()
  }

  pub fn out(&self) -> ValRef {
    self.out.clone()
  }


}


// Matrix ---------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Matrix {
  RowDVector(Rc<RefCell<RowDVector<i64>>>),
  RowVector2(RowVector2<i64>),
  RowVector3(Rc<RefCell<RowVector3<i64>>>),
  RowVector4(RowVector4<i64>),
  Matrix1(Matrix1<i64>),
  Matrix2(Rc<RefCell<Matrix2<i64>>>),
  Matrix3(Rc<RefCell<Matrix3<i64>>>),
  Matrix4(Matrix4<i64>),
  Matrix2x3(Matrix2x3<i64>),
  DMatrix(DMatrix<i64>),
}

impl Hash for Matrix {
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      Matrix::RowDVector(x) => x.borrow().hash(state),
      Matrix::RowVector2(x) => x.hash(state),
      Matrix::RowVector3(x) => x.borrow().hash(state),
      Matrix::RowVector4(x) => x.hash(state),
      Matrix::Matrix1(x) => x.hash(state),
      Matrix::Matrix2(x) => x.borrow().hash(state),
      Matrix::Matrix3(x) => x.borrow().hash(state),
      Matrix::Matrix4(x) => x.hash(state),
      Matrix::Matrix2x3(x) => x.hash(state),
      Matrix::DMatrix(x) => x.hash(state),
    }
  }
}

impl Matrix {
  pub fn shape(&self) -> (usize,usize) {
    match self {
      Matrix::RowDVector(x) => x.borrow().shape(),
      Matrix::RowVector2(x) => x.shape(),
      Matrix::RowVector3(x) => x.borrow().shape(),
      Matrix::RowVector4(x) => x.shape(),
      Matrix::Matrix1(x) => x.shape(),
      Matrix::Matrix2(x) => x.borrow().shape(),
      Matrix::Matrix3(x) => x.borrow().shape(),
      Matrix::Matrix4(x) => x.shape(),
      Matrix::Matrix2x3(x) => x.shape(),
      Matrix::DMatrix(x) => x.shape(),
    }
  }

  pub fn index1d(&self, ix: usize) -> i64 {
    match self {
      Matrix::RowDVector(x) => *x.borrow().index(ix-1),
      Matrix::RowVector2(x) => todo!(),
      Matrix::RowVector3(x) => *x.borrow().index(ix-1),
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
      Matrix::RowDVector(x) => *x.borrow().index((row-1,col-1)),
      Matrix::RowVector2(x) => todo!(),
      Matrix::RowVector3(x) => *x.borrow().index((row-1,col-1)),
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
  fn solve(&self);
  fn out(&self) -> Value;
  fn to_string(&self) -> String;
}

// User Function --------------------------------------------------------------

#[derive(Debug)]
struct UserFunction {
  fxn: FunctionDefinition,
}

impl MechFunction for UserFunction {
  fn solve(&self) {
    self.fxn.solve();
  }
  fn out(&self) -> Value {
    Value::MutableReference(self.fxn.out.clone())
  }
  fn to_string(&self) -> String { format!("UserFxn::{:?}", self.fxn.name)}
}

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

    Interpreter {
      symbols: Rc::new(RefCell::new(SymbolTable::new())),
      plan: Rc::new(RefCell::new(Vec::new())),
      functions: Rc::new(RefCell::new(fxns)),
    }
  }

  pub fn interpret(&mut self, tree: &Program) -> Result<Value,MechError> {
    program(tree, self.plan.clone(), self.symbols.clone(), self.functions.clone())
  }
}

pub trait NativeFunctionCompiler {
  fn compile(&self, arguments: &Vec<Value>) -> std::result::Result<Box<dyn MechFunction>,MechError>;
}

//-----------------------------------------------------------------------------

fn program(program: &Program, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
  body(&program.body, plan.clone(), symbols.clone(), functions.clone())
}

fn body(body: &Body, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
  let mut result = None;
  for sec in &body.sections {
    result = Some(section(&sec, plan.clone(), symbols.clone(), functions.clone())?);
  }
  Ok(result.unwrap())
}

fn section(section: &Section, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
  let mut result = None;
  for el in &section.elements {
    result = Some(section_element(&el, plan.clone(), symbols.clone(), functions.clone())?);
  }
  Ok(result.unwrap())
}

fn section_element(element: &SectionElement, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
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

fn mech_code(code: &MechCode, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
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


fn function_define(fxn_def: &FunctionDefine, functions: FunctionsRef) -> Result<FunctionDefinition,MechError> {
  let fxn_name_id = fxn_def.name.hash();
  let mut new_fxn = FunctionDefinition::new(fxn_name_id,fxn_def.name.to_string(), fxn_def.clone());
  for input_arg in &fxn_def.input {
    let arg_id = input_arg.name.hash();
    new_fxn.input.insert(arg_id,input_arg.kind.clone());
    let in_arg = Value::I64(Rc::new(RefCell::new(0)));
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

fn statement(stmt: &Statement, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
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

fn variable_define(var_def: &VariableDefine, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
  let id = var_def.var.name.hash();
  let result = expression(&var_def.expression, plan.clone(), symbols.clone(), functions.clone())?;
  let mut symbols_brrw = symbols.borrow_mut();
  symbols_brrw.insert(id,result.clone());
  Ok(result)
}

fn expression(expr: &Expression, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
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

fn function_call(fxn_call: &FunctionCall, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
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

fn range(rng: &RangeExpression, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
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

fn slice(slc: &Slice, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
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
  unreachable!() // subscript should have through an error if we can't access an element
}

fn subscript(sbscrpt: &Subscript, val: &Value, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
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
        Value::Matrix(mat) => {
          let result = match &resolved_subs[..] {
            [Value::I64(ix)] => mat.index1d(*ix.borrow() as usize),
            [Value::I64(row_ix),Value::I64(col_ix)] => mat.index2d(*row_ix.borrow() as usize,*col_ix.borrow() as usize),
            _ => todo!(),
          };
          return Ok(Value::I64(Rc::new(RefCell::new(result))));
        }
        Value::MutableReference(x) => match &*x.borrow() {
          Value::Matrix(mat) => {
            let result = match &resolved_subs[..] {
              [Value::I64(ix)] => mat.index1d(*ix.borrow() as usize),
              [Value::I64(row_ix),Value::I64(col_ix)] => mat.index2d(*row_ix.borrow() as usize,*col_ix.borrow() as usize),
              _ => todo!(),
            };
            return Ok(Value::I64(Rc::new(RefCell::new(result))));
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

fn structure(strct: &Structure, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
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

fn tuple(tpl: &Tuple, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
  let mut elements = vec![];
  for el in &tpl.elements {
    let result = expression(el,plan.clone(),symbols.clone(), functions.clone())?;
    elements.push(Box::new(result));
  }
  let mech_tuple = MechTuple{elements};
  Ok(Value::Tuple(mech_tuple))
}

fn map(mp: &Map, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
  let mut m = IndexMap::new();
  for b in &mp.elements {
    let key = expression(&b.key, plan.clone(), symbols.clone(), functions.clone())?;
    let val = expression(&b.value, plan.clone(), symbols.clone(), functions.clone())?;
    m.insert(key,val);
  }
  Ok(Value::Map(MechMap{map: m}))
}

fn record(rcrd: &Record, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
  let mut m = IndexMap::new();
  for b in &rcrd.bindings {
    let name = b.name.hash();
    let kind = &b.kind;
    let val = expression(&b.value, plan.clone(), symbols.clone(), functions.clone())?;
    m.insert(Value::Id(name),val);
  }
  Ok(Value::Record(MechMap{map: m}))
}

fn set(m: &Set, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> { 
  let mut out = IndexSet::new();
  for el in &m.elements {
    let result = expression(el, plan.clone(), symbols.clone(), functions.clone())?;
    out.insert(result);
  }
  Ok(Value::Set(MechSet{set: out}))
}

fn table(t: &Table, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> { 
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

fn table_header(fields: &Vec<Field>) -> Result<Vec<Value>,MechError> {
  let mut row: Vec<Value> = Vec::new();
  for f in fields {
    let id = f.name.hash();
    let kind = &f.kind;
    row.push(Value::Id(id));
  }
  Ok(row)
}

fn table_row(r: &TableRow, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Vec<Value>,MechError> {
  let mut row: Vec<Value> = Vec::new();
  for col in &r.columns {
    let result = table_column(col, plan.clone(), symbols.clone(), functions.clone())?;
    row.push(result);
  }
  Ok(row)
}

fn table_column(r: &TableColumn, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> { 
  expression(&r.element, plan.clone(), symbols.clone(), functions.clone())
}

fn matrix(m: &Mat, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> { 
  let mut out = vec![];
  for row in &m.rows {
    let result = matrix_row(row, plan.clone(), symbols.clone(), functions.clone())?;
    out.push(result);
  }
  if out.is_empty() {
    return Ok(Value::Matrix(Matrix::DMatrix(DMatrix::from_vec(0,0,vec![]))));
  }
  let (_,col_n) = out[0].shape();
  let out_vec = match (out.len(),col_n) {
    (1,_) => out[0].clone(),
    (2,2) => {
      let mut rows: Vec<RowVector2<i64>> = vec![];
      for o in &out {if let Value::Matrix(Matrix::RowVector2(v)) = &o {rows.push(v.clone());}}
      Value::Matrix(Matrix::Matrix2(Rc::new(RefCell::new(Matrix2::from_rows(&[rows[0].clone(), rows[1].clone()])))))
    }
    (2,3) => {
      let mut rows: Vec<RowVector3<i64>> = vec![];
      for o in &out {if let Value::Matrix(Matrix::RowVector3(v)) = &o {rows.push(v.borrow().clone());}}
      Value::Matrix(Matrix::Matrix2x3(Matrix2x3::from_rows(&[rows[0].clone(), rows[1].clone()])))
    }
    (3,3) => {
      let mut rows: Vec<RowVector3<i64>> = vec![];
      for o in &out {if let Value::Matrix(Matrix::RowVector3(v)) = &o {rows.push(v.borrow().clone());}}
      Value::Matrix(Matrix::Matrix3(Rc::new(RefCell::new(Matrix3::from_rows(&[rows[0].clone(), rows[1].clone(), rows[2].clone()])))))
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

fn matrix_row(r: &MatrixRow, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
  let mut row: Vec<i64> = Vec::new();
  for col in &r.columns {
    let result = matrix_column(col, plan.clone(), symbols.clone(), functions.clone())?;
    match result {
      Value::I64(n) => row.push(n.borrow().clone()),
      _ => todo!(),
    }
  }
  let out_vec = match row.len() {
    1 => Matrix::Matrix1(Matrix1::from_element(row[0].clone())),
    2 => Matrix::RowVector2(RowVector2::from_vec(row.clone())),
    3 => Matrix::RowVector3(Rc::new(RefCell::new(RowVector3::from_vec(row.clone())))),
    4 => Matrix::RowVector4(RowVector4::from_vec(row.clone())),
    n => Matrix::RowDVector(Rc::new(RefCell::new(RowDVector::from_vec(row.clone())))),
  };
  Ok(Value::Matrix(out_vec))
}

fn matrix_column(r: &MatrixColumn, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> { 
  expression(&r.element, plan.clone(), symbols.clone(), functions.clone())
}

fn var(v: &Var, symbols: SymbolTableRef) -> Result<Value,MechError> {
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

fn factor(fctr: &Factor, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
  match fctr {
    Factor::Term(trm) => {
      let result = term(trm, plan.clone(), symbols.clone(), functions.clone())?;
      Ok(result)
    },
    Factor::Expression(expr) => expression(expr, plan.clone(), symbols.clone(), functions.clone()),
    Factor::Negated(neg) => {
      let value = factor(neg, plan.clone(), symbols.clone(), functions.clone())?;
      let new_fxn = MathNegate{}.compile(&vec![value])?;
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

fn term(trm: &Term, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> Result<Value,MechError> {
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
      FormulaOperator::Vec(VecOp::MatMul) => MatrixMul{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::LessThan) => CompareLessThan{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::GreaterThan) => CompareGreaterThan{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Logic(LogicOp::And) => LogicAnd{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Logic(LogicOp::Or) => LogicOr{}.compile(&vec![lhs,rhs])?,
      x => todo!(),
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

fn literal(ltrl: &Literal, functions: FunctionsRef) -> Result<Value,MechError> {
  match &ltrl {
    Literal::Empty(_) => Ok(empty()),
    Literal::Boolean(bln) => Ok(boolean(bln)),
    Literal::Number(num) => Ok(number(num)),
    Literal::String(strng) => Ok(string(strng)),
    Literal::Atom(atm) => Ok(atom(atm)),
    Literal::TypedLiteral((ltrl,kind)) => typed_literal(ltrl,kind,functions),
  }
}

fn typed_literal(ltrl: &Literal, knd_attn: &KindAnnotation, functions: FunctionsRef) -> Result<Value,MechError> {
  let value = literal(ltrl,functions.clone())?;
  let kind = kind_annotation(knd_attn);
  match (&value,kind) {
    (Value::I64(num), Kind::Scalar(to_kind_id)) => {
      match functions.borrow().kinds.get(&to_kind_id) {
        Some(ValueKind::I8) => Ok(Value::I8(Rc::new(RefCell::new(*num.borrow() as i8)))),
        Some(ValueKind::I16) => Ok(Value::I16(Rc::new(RefCell::new(*num.borrow() as i16)))),
        Some(ValueKind::I32) => Ok(Value::I32(Rc::new(RefCell::new(*num.borrow() as i32)))),
        Some(ValueKind::I64) => Ok(value),
        Some(ValueKind::I128) => Ok(Value::I128(Rc::new(RefCell::new(*num.borrow() as i128)))),
        Some(ValueKind::U8) => Ok(Value::U8(Rc::new(RefCell::new(*num.borrow() as u8)))),
        Some(ValueKind::U16) => Ok(Value::U16(Rc::new(RefCell::new(*num.borrow() as u16)))),
        Some(ValueKind::U32) => Ok(Value::U32(Rc::new(RefCell::new(*num.borrow() as u32)))),
        Some(ValueKind::U64) => Ok(Value::U64(Rc::new(RefCell::new(*num.borrow() as u64)))),
        Some(ValueKind::U128) => Ok(Value::U128(Rc::new(RefCell::new(*num.borrow() as u128)))),
        None => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedKind(to_kind_id)}),
        _ => Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::CouldNotAssignKindToValue}),
      }
    }
    _ => todo!(),
  }
}

fn kind_annotation(knd_attn: &KindAnnotation) -> Kind {
  match &knd_attn.kind {
    NodeKind::Scalar(id) => {
      let kind_id = id.hash();
      Kind::Scalar(kind_id)
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
  Value::F64(Rc::new(RefCell::new(F64(num))))
}

fn integer(int: &Token) -> Value {
  let num: i64 = int.chars.iter().collect::<String>().parse::<i64>().unwrap();
  Value::I64(Rc::new(RefCell::new(num)))
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
  Value::Bool(Rc::new(RefCell::new(val)))
}

// ----------------------------------------------------------------------------
// Math Library
// ----------------------------------------------------------------------------

// Sin ------------------------------------------------------------------------

use libm::sin;

#[derive(Debug)]
pub struct MathSinScalar {
  val: Rc<RefCell<F64>>,
  out: Rc<RefCell<F64>>,
}

impl MechFunction for MathSinScalar {
  fn solve(&self) {
    let val_ptr = self.val.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe{(*out_ptr).0 = sin((*val_ptr).0);}
  }
  fn out(&self) -> Value { Value::F64(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct MathSin {}

impl NativeFunctionCompiler for MathSin {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 1 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match &arguments[0] {
      Value::F64(val) =>
        Ok(Box::new(MathSinScalar{val: val.clone(), out: Rc::new(RefCell::new(F64(0.0)))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Add ------------------------------------------------------------------------

#[derive(Debug)] 
struct AddScalar<T> {
  lhs: Rc<RefCell<T>>,
  rhs: Rc<RefCell<T>>,
  out: Rc<RefCell<T>>,
}

impl<T> MechFunction for AddScalar<T>
where T: Copy + Debug + Clone + Sync + Send + Add<Output = T>,
      Rc<RefCell<T>>: ToValue
{
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr + *rhs_ptr;}
  }
  fn out(&self) -> Value {
    self.out.to_value()
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct AddRv3Rv3 {
  lhs: Rc<RefCell<RowVector3<i64>>>,
  rhs: Rc<RefCell<RowVector3<i64>>>,
  out: Rc<RefCell<RowVector3<i64>>>,
}

impl MechFunction for AddRv3Rv3 {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr + *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::Matrix(Matrix::RowVector3(self.out.clone()))
  }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

#[derive(Debug)]
struct AddM3M3 {
  lhs: Rc<RefCell<Matrix3<i64>>>,
  rhs: Rc<RefCell<Matrix3<i64>>>,
  out: Rc<RefCell<Matrix3<i64>>>,
}

impl MechFunction for AddM3M3 {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr + *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::Matrix(Matrix::Matrix3(self.out.clone()))
  }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

pub struct MathAdd {}

impl NativeFunctionCompiler for MathAdd {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(lhs), Value::I64(rhs)) =>
          Ok(Box::new(AddScalar{lhs, rhs, out: Rc::new(RefCell::new(0))})),
      (Value::U64(lhs), Value::U64(rhs)) =>
          Ok(Box::new(AddScalar{lhs, rhs, out: Rc::new(RefCell::new(0))})),
      (Value::MutableReference(lhs), Value::I64(rhs)) => match *lhs.borrow() {
        Value::I64(ref lhs) =>
          Ok(Box::new(AddScalar{lhs: lhs.clone(), rhs, out: Rc::new(RefCell::new(0))})),
        _ => todo!(),
      }
      (Value::I64(lhs), Value::MutableReference(rhs)) => match *rhs.borrow() {
        Value::I64(ref rhs) =>
          Ok(Box::new(AddScalar{lhs, rhs: rhs.clone(), out: Rc::new(RefCell::new(0))})),
        _ => todo!(),
      }
      (Value::MutableReference(lhs), Value::MutableReference(rhs)) => match (&*lhs.borrow(),&*rhs.borrow()) { 
        (Value::I64(ref lhs),Value::I64(ref rhs)) =>
          Ok(Box::new(AddScalar{lhs: lhs.clone(), rhs: rhs.clone(), out: Rc::new(RefCell::new(0))})),
        (Value::Matrix(Matrix::RowVector3(lhs)),Value::Matrix(Matrix::RowVector3(rhs))) =>
          Ok(Box::new(AddRv3Rv3{lhs: lhs.clone(), rhs: rhs.clone(), out: Rc::new(RefCell::new(RowVector3::from_element(0)))})),
        _ => todo!(),
      }
      (Value::Matrix(Matrix::RowVector3(lhs)), Value::Matrix(Matrix::RowVector3(rhs))) =>
        Ok(Box::new(AddRv3Rv3{lhs, rhs, out: Rc::new(RefCell::new(RowVector3::from_element(0)))})),
      (Value::Matrix(Matrix::Matrix3(lhs)), Value::Matrix(Matrix::Matrix3(rhs))) =>
        Ok(Box::new(AddM3M3{lhs, rhs, out: Rc::new(RefCell::new(Matrix3::from_element(0)))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Sub ------------------------------------------------------------------------

#[derive(Debug)] 
struct SubScalar {
  lhs: Rc<RefCell<i64>>,
  rhs: Rc<RefCell<i64>>,
  out: Rc<RefCell<i64>>,
}

impl MechFunction for SubScalar {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr - *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::I64(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct SubRv3Rv3 {
  lhs: Rc<RefCell<RowVector3<i64>>>,
  rhs: Rc<RefCell<RowVector3<i64>>>,
  out: Rc<RefCell<RowVector3<i64>>>,
}

impl MechFunction for SubRv3Rv3 {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr - *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::Matrix(Matrix::RowVector3(self.out.clone()))
  }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

pub struct MathSub {}

impl NativeFunctionCompiler for MathSub {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(lhs), Value::I64(rhs)) =>
        Ok(Box::new(SubScalar{lhs, rhs, out: Rc::new(RefCell::new(0))})),
      (Value::Matrix(Matrix::RowVector3(lhs)), Value::Matrix(Matrix::RowVector3(rhs))) =>
        Ok(Box::new(SubRv3Rv3{lhs,rhs,out: Rc::new(RefCell::new(RowVector3::from_element(0)))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}


// Mul ------------------------------------------------------------------------

#[derive(Debug)] 
struct MulScalar {
  lhs: Rc<RefCell<i64>>,
  rhs: Rc<RefCell<i64>>,
  out: Rc<RefCell<i64>>,
}

impl MechFunction for MulScalar {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr * *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::I64(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct MathMul {}

impl NativeFunctionCompiler for MathMul {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(lhs), Value::I64(rhs)) =>
        Ok(Box::new(MulScalar{lhs, rhs, out: Rc::new(RefCell::new(0))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Div ------------------------------------------------------------------------

#[derive(Debug)] 
struct DivScalar {
  lhs: Rc<RefCell<i64>>,
  rhs: Rc<RefCell<i64>>,
  out: Rc<RefCell<i64>>,
}

impl MechFunction for DivScalar {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr / *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::I64(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct MathDiv {}

impl NativeFunctionCompiler for MathDiv {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(lhs), Value::I64(rhs)) =>
        Ok(Box::new(DivScalar{lhs, rhs, out: Rc::new(RefCell::new(0))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Exp ------------------------------------------------------------------------

#[derive(Debug)] 
struct ExpScalar {
  lhs: Rc<RefCell<i64>>,
  rhs: Rc<RefCell<i64>>,
  out: Rc<RefCell<i64>>,
}

impl MechFunction for ExpScalar {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = (*lhs_ptr).pow(*rhs_ptr as u32);}
  }
  fn out(&self) -> Value {
    Value::I64(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct MathExp {}

impl NativeFunctionCompiler for MathExp {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(lhs), Value::I64(rhs)) =>
        Ok(Box::new(ExpScalar{lhs, rhs, out: Rc::new(RefCell::new(0))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Negate ---------------------------------------------------------------------

#[derive(Debug)]
struct NegateScalar {
  input: Rc<RefCell<i64>>,
  output: Rc<RefCell<i64>>,
}

impl MechFunction for NegateScalar {
  fn solve(&self) {
    let input_ptr = self.input.as_ptr();
    let output_ptr = self.output.as_ptr();

    unsafe {
      *output_ptr = -*input_ptr;
    }
  }
  fn out(&self) -> Value {
    Value::I64(self.output.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self)}
}


#[derive(Debug)]
struct NegateM2 {
  mat: Rc<RefCell<Matrix2<i64>>>,
  out: Rc<RefCell<Matrix2<i64>>>,
}

impl MechFunction for NegateM2 {
  fn solve(&self) {
    let mat_ptr = self.mat.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = -*mat_ptr;}
  }
  fn out(&self) -> Value {
    Value::Matrix(Matrix::Matrix2(self.out.clone()))
  }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

pub struct MathNegate {}

impl NativeFunctionCompiler for MathNegate {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 1 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone()) {
      Value::Matrix(Matrix::Matrix2(mat)) =>
        Ok(Box::new(NegateM2{mat, out: Rc::new(RefCell::new(Matrix2::from_element(0)))})),
      Value::I64(n) =>
        Ok(Box::new(NegateScalar{input: n, output: Rc::new(RefCell::new(0))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// ----------------------------------------------------------------------------
// Logic Library
// ----------------------------------------------------------------------------

// And ------------------------------------------------------------------------

#[derive(Debug)]
struct AndScalar {
  lhs: Rc<RefCell<bool>>,
  rhs: Rc<RefCell<bool>>,
  out: Rc<RefCell<bool>>,
}

impl MechFunction for AndScalar {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr && *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::Bool(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct LogicAnd {}

impl NativeFunctionCompiler for LogicAnd {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::Bool(lhs), Value::Bool(rhs)) =>
        Ok(Box::new(AndScalar{lhs, rhs, out: Rc::new(RefCell::new(false))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Or ------------------------------------------------------------------------

#[derive(Debug)]
struct OrScalar {
  lhs: Rc<RefCell<bool>>,
  rhs: Rc<RefCell<bool>>,
  out: Rc<RefCell<bool>>,
}

impl MechFunction for OrScalar {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr || *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::Bool(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct LogicOr {}

impl NativeFunctionCompiler for LogicOr {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::Bool(lhs), Value::Bool(rhs)) =>
        Ok(Box::new(OrScalar{lhs, rhs, out: Rc::new(RefCell::new(false))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// ----------------------------------------------------------------------------
// Compare Library
// ----------------------------------------------------------------------------

// Greater Than ---------------------------------------------------------------

#[derive(Debug)]
struct GTScalar {
  lhs: Rc<RefCell<i64>>,
  rhs: Rc<RefCell<i64>>,
  out: Rc<RefCell<bool>>,
}

impl MechFunction for GTScalar {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr > *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::Bool(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct CompareGreaterThan {}

impl NativeFunctionCompiler for CompareGreaterThan {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(lhs), Value::I64(rhs)) =>
        Ok(Box::new(GTScalar{lhs, rhs, out: Rc::new(RefCell::new(false))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Less Than ------------------------------------------------------------------

#[derive(Debug)]
struct LTScalar<T> {
  lhs: Rc<RefCell<T>>,
  rhs: Rc<RefCell<T>>,
  out: Rc<RefCell<bool>>,
}

impl<T> MechFunction for LTScalar<T> 
where T: Copy + Debug + Clone + Sync + Send + PartialOrd
{
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {*out_ptr = *lhs_ptr < *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::Bool(self.out.clone())
  }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct CompareLessThan {}

impl NativeFunctionCompiler for CompareLessThan {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(lhs), Value::I64(rhs)) =>
        Ok(Box::new(LTScalar{lhs, rhs, out: Rc::new(RefCell::new(false))})),
      (Value::U64(lhs), Value::U64(rhs)) =>
        Ok(Box::new(LTScalar{lhs, rhs, out: Rc::new(RefCell::new(false))})),        
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// ----------------------------------------------------------------------------
// Matrix Library
// ----------------------------------------------------------------------------

// MatMul ---------------------------------------------------------------------

#[derive(Debug)]
struct MatMulM2M2 {
  lhs: Rc<RefCell<Matrix2<i64>>>,
  rhs: Rc<RefCell<Matrix2<i64>>>,
  out: Rc<RefCell<Matrix2<i64>>>,
}

impl MechFunction for MatMulM2M2 {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe{ *out_ptr = *lhs_ptr * *rhs_ptr;}
  }
  fn out(&self) -> Value {
    Value::Matrix(Matrix::Matrix2(self.out.clone()))
  }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

pub struct MatrixMul {}

impl NativeFunctionCompiler for MatrixMul {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::Matrix(Matrix::Matrix2(lhs)), Value::Matrix(Matrix::Matrix2(rhs))) => 
        Ok(Box::new(MatMulM2M2{lhs,rhs,out: Rc::new(RefCell::new(Matrix2::from_element(0)))})),
      (Value::MutableReference(lhs), Value::MutableReference(rhs)) => match (&*lhs.borrow(),&*rhs.borrow()) {
        (Value::Matrix(Matrix::Matrix2(lhs)), Value::Matrix(Matrix::Matrix2(rhs))) =>
          Ok(Box::new(MatMulM2M2{lhs: lhs.clone(), rhs: rhs.clone(), out: Rc::new(RefCell::new(Matrix2::from_element(0)))})),
        _ => todo!(),
      } 
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// Transpose ------------------------------------------------------------------

#[derive(Debug)]
struct TransposeM2 {
  mat: Rc<RefCell<Matrix2<i64>>>,
  out: Rc<RefCell<Matrix2<i64>>>,
}

impl MechFunction for TransposeM2 {
  fn solve(&self) {
    let input_ptr = self.mat.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe{*out_ptr = (*input_ptr).transpose();}
  }
  fn out(&self) -> Value {
    Value::Matrix(Matrix::Matrix2(self.out.clone()))
  }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

pub struct MatrixTranspose {}

impl NativeFunctionCompiler for MatrixTranspose {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 1 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone()) {
      Value::Matrix(Matrix::Matrix2(mat)) =>
        Ok(Box::new(TransposeM2{mat, out: Rc::new(RefCell::new(Matrix2::from_element(0)))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}

// ----------------------------------------------------------------------------
// Range Library
// ----------------------------------------------------------------------------

// Exclusive ------------------------------------------------------------------

#[derive(Debug)]
struct RangeExclusiveScalar {
  max: Rc<RefCell<i64>>,
  min: Rc<RefCell<i64>>,
  out: Rc<RefCell<RowDVector<i64>>>,
}

impl MechFunction for RangeExclusiveScalar {
  fn solve(&self) {
    let max_ptr = self.max.as_ptr();
    let min_ptr = self.min.as_ptr();
    let out_ptr = self.out.as_ptr();
    
    unsafe {
        let rng = (*min_ptr..*max_ptr).collect::<Vec<i64>>();
        *out_ptr = RowDVector::from_vec(rng);
    }
  }
  fn out(&self) -> Value {
    Value::Matrix(Matrix::RowDVector(self.out.clone()))
  }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

pub struct RangeExclusive {}

impl NativeFunctionCompiler for RangeExclusive {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(min), Value::I64(max)) =>
        Ok(Box::new(RangeExclusiveScalar{max,min, out: Rc::new(RefCell::new(RowDVector::from_element(1,0)))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}


// Inclusive ------------------------------------------------------------------

#[derive(Debug)]
struct RangeInclusiveScalar {
  max: Rc<RefCell<i64>>,
  min: Rc<RefCell<i64>>,
  out: Rc<RefCell<RowDVector<i64>>>,
}

impl MechFunction for RangeInclusiveScalar {
  fn solve(&self) {
    let max_ptr = self.max.as_ptr();
    let min_ptr = self.min.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {
      let rng = (*min_ptr..=*max_ptr).collect::<Vec<i64>>();
      *out_ptr = RowDVector::from_vec(rng);
    }
  }
  fn out(&self) -> Value {
    Value::Matrix(Matrix::RowDVector(self.out.clone()))
  }
  fn to_string(&self) -> String { format!("{:?}", self)}
}

pub struct RangeInclusive {}

impl NativeFunctionCompiler for RangeInclusive {
  fn compile(&self, arguments: &Vec<Value>) -> Result<Box<dyn MechFunction>,MechError> {
    if arguments.len() != 2 {
      return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    match (arguments[0].clone(), arguments[1].clone()) {
      (Value::I64(min), Value::I64(max)) =>
        Ok(Box::new(RangeInclusiveScalar{max,min, out: Rc::new(RefCell::new(RowDVector::from_element(1,0)))})),
      x => 
        Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
    }
  }
}
