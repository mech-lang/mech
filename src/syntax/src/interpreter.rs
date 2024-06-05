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
use std::hash::{Hash, Hasher};
use indexmap::set::IndexSet;

//-----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
  Number(i64),
  String(String),
  Bool(bool),
  Atom(u64),
  Matrix(Matrix),
  Set(MechSet),
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
      Value::Set(x) => (1,x.set.len()),
      Value::Empty => (0,0),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechSet {
  set: IndexSet<Value>,
}

impl Hash for MechSet {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for x in self.set.iter() {
        x.hash(state)
    }
  }
}



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
}

pub trait MechFunction {
  fn solve(&self) -> Value;
}

// The naming scheme will be OP LHS RHS
// The abbreviations are:
// Rv - row vector
// Cv - col vector
// MXY  - Matrix size X Y, or just X if it's square

struct AddRv3Rv3 {
  lhs: RowVector3<i64>,
  rhs: RowVector3<i64>,
}

impl MechFunction for AddRv3Rv3 {
  fn solve(&self) -> Value {
    let result = &self.lhs + &self.rhs;
    Value::Matrix(Matrix::RowVector3(result))
  }
}

struct AddM3M3 {
  lhs: Matrix3<i64>,
  rhs: Matrix3<i64>,
}

impl MechFunction for AddM3M3 {
  fn solve(&self) -> Value {
    let result = &self.lhs + &self.rhs;
    Value::Matrix(Matrix::Matrix3(result))
  }
}

struct MatMulM2M2 {
  lhs: Matrix2<i64>,
  rhs: Matrix2<i64>,
}

impl MechFunction for MatMulM2M2 {
  fn solve(&self) -> Value {
    let result = &self.lhs * &self.rhs;
    Value::Matrix(Matrix::Matrix2(result))
  }
}

struct TransposeM2 {
  mat: Matrix2<i64>,
}

impl MechFunction for TransposeM2 {
  fn solve(&self) -> Value {
    let result = self.mat.transpose();
    Value::Matrix(Matrix::Matrix2(result))
  }
}

struct NegateF64 {
  n: i64,
}

impl MechFunction for NegateF64 {
  fn solve(&self) -> Value {
    Value::Number(-self.n)
  }
}

struct NegateM2 {
  mat: Matrix2<i64>,
}

impl MechFunction for NegateM2 {
  fn solve(&self) -> Value {
    Value::Matrix(Matrix::Matrix2(-self.mat))
  }
}

//-----------------------------------------------------------------------------

pub struct Interpreter {
  pub symbols: HashMap<u64, Value>,
  pub functions: Vec<Rc<dyn MechFunction>>,
}

impl Interpreter {

  pub fn new() -> Interpreter {
    Interpreter {
      symbols: HashMap::new(),
      functions: Vec::new(),
    }
  }

  pub fn interpret(&mut self, tree: &Program) -> Result<Value,MechError> {
    self.program(tree)
  }
  
//-----------------------------------------------------------------------------

  fn program(&mut self, program: &Program) -> Result<Value,MechError> {
    self.body(&program.body)
  }

  fn body(&mut self, body: &Body) -> Result<Value,MechError> {
    let mut result = None;
    for sec in &body.sections {
      result = Some(self.section(&sec)?);
    }
    Ok(result.unwrap())
  }

  fn section(&mut self, section: &Section) -> Result<Value,MechError> {
    let mut result = None;
    for el in &section.elements {
      result = Some(self.section_element(&el)?);
      println!("Interpreter Result: {:?}", result);
    }
    Ok(result.unwrap())
  }

  fn section_element(&mut self, element: &SectionElement) -> Result<Value,MechError> {
    match element {
      SectionElement::MechCode(code) => {self.mech_code(&code)},
      _ => todo!(),
    }
  }

  fn mech_code(&mut self, code: &MechCode) -> Result<Value,MechError> {
    match &code {
      MechCode::Expression(expr) => self.expression(&expr),
      MechCode::Statement(stmt) => self.statement(&stmt),
      MechCode::FsmSpecification(_) => todo!(),
      MechCode::FsmImplementation(_) => todo!(),
      MechCode::FunctionDefine(_) => todo!(),
    }
  }

  fn statement(&mut self, stmt: &Statement) -> Result<Value,MechError> {
    match stmt {
      Statement::VariableDefine(var_def) => self.variable_define(&var_def),
      Statement::VariableAssign(_) => todo!(),
      Statement::KindDefine(_) => todo!(),
      Statement::EnumDefine(_) => todo!(),
      Statement::FsmDeclare(_) => todo!(),
      Statement::SplitTable => todo!(),
      Statement::FlattenTable => todo!(),
    }
  }

  fn variable_define(&mut self, var_def: &VariableDefine) -> Result<Value,MechError> {
    let id = var_def.var.name.hash();
    let result = self.expression(&var_def.expression)?;
    self.symbols.insert(id,result.clone());
    Ok(result)
  }

  fn expression(&mut self, expr: &Expression) -> Result<Value,MechError> {
    match &expr {
      Expression::Var(v) => self.var(&v),
      Expression::Slice(_) => todo!(),
      Expression::Formula(fctr) => self.factor(fctr),
      Expression::Structure(strct) => self.structure(strct),
      Expression::Literal(ltrl) => Ok(self.literal(&ltrl)),
      Expression::FunctionCall(_) => todo!(),
      Expression::FsmPipe(_) => todo!(),
    }
  }

  fn structure(&mut self, strct: &Structure) -> Result<Value,MechError> {
    match strct {
      Structure::Empty => todo!(),
      Structure::Record(x) => todo!(),
      Structure::Matrix(mat) => self.matrix(&mat),
      Structure::Table(x) => todo!(),
      Structure::Tuple(x) => todo!(),
      Structure::TupleStruct(x) => todo!(),
      Structure::Set(x) => todo!(),
      Structure::Map(x) => todo!(),
    }
  }


  fn matrix(&mut self, m: &Mat) -> Result<Value,MechError> { 
    let mut out = vec![];
    for row in &m.rows {
      let result = self.matrix_row(row)?;
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

  fn matrix_row(&mut self, r: &MatrixRow) -> Result<Value,MechError> {
    let mut row: Vec<i64> = Vec::new();
    for col in &r.columns {
      let result = self.matrix_column(col)?;
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

  fn matrix_column(&mut self, r: &MatrixColumn) -> Result<Value,MechError> { 
    self.expression(&r.element)
  }

  fn var(&mut self, v: &Var) -> Result<Value,MechError> {
    let id = v.name.hash();
    match self.symbols.get(&id) {
      Some(value) => {
        return Ok(value.clone())         
      }
      None => {
        return Err(MechError{tokens: vec![], msg: "interpreter.rs".to_string(), id: 314, kind: MechErrorKind::None});
      }
    }
  }

  fn factor(&mut self, fctr: &Factor) -> Result<Value,MechError> {
    match fctr {
      Factor::Term(trm) => self.term(trm),
      Factor::Expression(expr) => self.expression(expr),
      Factor::Negated(neg) => {
        match self.factor(neg)? {
          Value::Matrix(Matrix::Matrix2(mat)) => {
            let fxn = NegateM2{mat}; 
            let out: Value = fxn.solve();
            self.functions.push(Rc::new(fxn));
            return Ok(out);
          }
          Value::Number(n) => {
            let fxn = NegateF64{n}; 
            let out: Value = fxn.solve();
            self.functions.push(Rc::new(fxn));
            return Ok(out);
          }
          _ => todo!(),
        }  
        return Err(MechError{tokens: vec![], msg: "interpreter.rs".to_string(), id: 331, kind: MechErrorKind::None});
      },
      Factor::Transpose(fctr) => {
        if let Value::Matrix(Matrix::Matrix2(mat)) = self.factor(fctr)? {
          let fxn = TransposeM2{mat}; 
          let out: Value = fxn.solve();
          self.functions.push(Rc::new(fxn));
          return Ok(out);
        }
        return Err(MechError{tokens: vec![], msg: "interpreter.rs".to_string(), id: 331, kind: MechErrorKind::None});
      },
    }
  }

  fn term(&mut self, trm: &Term) -> Result<Value,MechError> {
    let mut lhs_result = self.factor(&trm.lhs)?;
    for (op,rhs) in &trm.rhs {
      let rhs_result = self.factor(&rhs)?;
      lhs_result = match (lhs_result, rhs_result, op) {
        (Value::Number(lhs_val), Value::Number(rhs_val), FormulaOperator::AddSub(AddSubOp::Add)) 
          => Value::Number(lhs_val + rhs_val),
        (Value::Number(lhs_val), Value::Number(rhs_val), FormulaOperator::AddSub(AddSubOp::Sub))
          => Value::Number(lhs_val - rhs_val),
        (Value::Matrix(Matrix::RowVector3(lhs)), Value::Matrix(Matrix::RowVector3(rhs)), FormulaOperator::AddSub(AddSubOp::Add)) => {
          let fxn = AddRv3Rv3{lhs,rhs}; 
          let out = fxn.solve();
          self.functions.push(Rc::new(fxn));
          return Ok(out);
        }
        (Value::Matrix(Matrix::Matrix3(lhs)), Value::Matrix(Matrix::Matrix3(rhs)), FormulaOperator::AddSub(AddSubOp::Add)) => {
          let fxn = AddM3M3{lhs,rhs}; 
          let out = fxn.solve();
          self.functions.push(Rc::new(fxn));
          return Ok(out);
        }
        (Value::Matrix(Matrix::Matrix2(lhs)), Value::Matrix(Matrix::Matrix2(rhs)), FormulaOperator::Vec(VecOp::MatMul)) => {
          let fxn = MatMulM2M2{lhs,rhs}; 
          let out = fxn.solve();
          self.functions.push(Rc::new(fxn));
          return Ok(out);
        }
        x => {
          return Err(MechError{tokens: trm.tokens(), msg: "interpreter.rs".to_string(), id: 239, kind: MechErrorKind::UnhandledFunctionArgumentKind});
        }
      }
    }
    Ok(lhs_result)
  }

  fn literal(&mut self, ltrl: &Literal) -> Value {
    match &ltrl {
      Literal::Empty(_) => self.empty(),
      Literal::Boolean(bln) => self.boolean(bln),
      Literal::Number(num) => self.number(num),
      Literal::String(strng) => self.string(strng),
      Literal::Atom(atm) => todo!(),
      Literal::TypedLiteral((ltrl,kind)) => {
        todo!();
      },
    }
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

  fn empty(&mut self) -> Value {
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