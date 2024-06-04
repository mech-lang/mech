use mech_core::{MechError, MechErrorKind, hash_str, ValueKind, nodes::*};
use crate::parser2::*;
use serde_derive::*;
use std::any::Any;
use hashbrown::HashMap;
use na::{Vector3, DVector, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix6, Matrix2};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
  Number(i64),
  String(String),
  Bool(bool),
  Atom(u64),
  Table(Vec<Value>),
  RowDVector(RowDVector<Value>),
  RowVector2(Box<RowVector2<Value>>),
  RowVector3(Box<RowVector3<Value>>),
  RowVector4(Box<RowVector4<Value>>),
  Matrix1(Box<Matrix1<Value>>),
  Matrix2(Box<Matrix2<Value>>),
  Matrix3(Box<Matrix3<Value>>),
  Matrix4(Box<Matrix4<Value>>),
  Matrix2x3(Box<Matrix2x3<Value>>),
  DMatrix(DMatrix<Value>),
  Empty
}

impl Value {

  pub fn shape(&self) -> (usize,usize) {
    match self {
      Value::Number(x) => (1,1),
      Value::String(x) => (1,1),
      Value::Bool(x) => (1,1),
      Value::Atom(x) => (1,1),
      Value::Table(x) => (1,1),
      Value::RowDVector(x) => x.shape(),
      Value::RowVector2(x) => x.shape(),
      Value::RowVector3(x) => x.shape(),
      Value::RowVector4(x) => x.shape(),
      Value::Matrix1(x) => x.shape(),
      Value::Matrix2(x) => x.shape(),
      Value::Matrix3(x) => x.shape(),
      Value::Matrix4(x) => x.shape(),
      Value::Matrix2x3(x) => x.shape(),
      Value::DMatrix(x) => x.shape(),
      Value::Empty => (0,0),
    }
  }

}

#[derive(Clone, Debug)]
pub struct Interpreter {
  pub symbols: HashMap<u64, Value>,
}


impl Interpreter {

  pub fn new() -> Interpreter {
    Interpreter {
      symbols: HashMap::new(),
    }
  }

  pub fn interpret(&mut self, tree: &Program) -> Result<Value,MechError> {
    self.program(tree)
  }

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


  fn matrix(&mut self, m: &Matrix) -> Result<Value,MechError> { 
    let mut out = vec![];
    for row in &m.rows {
      let result = self.matrix_row(row)?;
      out.push(result);
    }
    let (_,col_n) = out[0].shape();
    let out_vec = match (out.len(),col_n) {
      (1,_) => out[0].clone(),
      (2,2) => {
        let mut rows = vec![];
        for o in &out {if let Value::RowVector2(v) = &o {rows.push(v.clone());}}
        Value::Matrix2(Box::new(Matrix2::from_rows(&[*rows[0].clone(), *rows[1].clone()])))
      }
      (2,3) => {
        let mut rows = vec![];
        for o in &out {if let Value::RowVector3(v) = &o {rows.push(v.clone());}}
        Value::Matrix2x3(Box::new(Matrix2x3::from_rows(&[*rows[0].clone(), *rows[1].clone()])))
      }
      (3,3) => {
        let mut rows = vec![];
        for o in &out {if let Value::RowVector3(v) = &o {rows.push(v.clone());}}
        Value::Matrix3(Box::new(Matrix3::from_rows(&[*rows[0].clone(), *rows[1].clone(), *rows[2].clone()])))
      }
      (4,4) => {
        let mut rows = vec![];
        for o in &out {if let Value::RowVector4(v) = &o {rows.push(v.clone());}}
        Value::Matrix4(Box::new(Matrix4::from_rows(&[*rows[0].clone(), *rows[1].clone(), *rows[2].clone(), *rows[3].clone()])))
      }
      _ => todo!(),
    };
    Ok(out_vec)
  }

  fn matrix_row(&mut self, r: &MatrixRow) -> Result<Value,MechError> {
    let mut row = Vec::new();
    for col in &r.columns {
      let result = self.matrix_column(col)?;
      row.push(result);
    }
    let out_vec = match row.len() {
      1 => Value::Matrix1(Box::new(Matrix1::from_element(row[0].clone()))),
      2 => Value::RowVector2(Box::new(RowVector2::from_vec(row.clone()))),
      3 => Value::RowVector3(Box::new(RowVector3::from_vec(row.clone()))),
      4 => Value::RowVector4(Box::new(RowVector4::from_vec(row.clone()))),
      n => Value::RowDVector(RowDVector::from_vec(row)),
    };
    Ok(out_vec)
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
        return Err(MechError{tokens: vec![], msg: "interpreter.rs".to_string(), id: 110, kind: MechErrorKind::None});
      }
    }
  }

  fn factor(&mut self, fctr: &Factor) -> Result<Value,MechError> {
    match fctr {
      Factor::Term(trm) => self.term(trm),
      Factor::Expression(expr) => self.expression(expr),
      Factor::Negated(_) => todo!(),
      Factor::Transpose(_) => todo!(),
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
        x => {
          return Err(MechError{tokens: trm.tokens(), msg: "interpreter.rs".to_string(), id: 135, kind: MechErrorKind::UnhandledFunctionArgumentKind});
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