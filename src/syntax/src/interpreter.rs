use mech_core::{MechError, MechErrorKind, hash_str, ValueKind, nodes::*};
use crate::parser2::*;
use serde_derive::*;
use std::any::Any;
use hashbrown::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
  Number(i64),
  String(String),
  Bool(bool),
  Atom(u64),
  Table(Vec<Value>),
  Empty
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
      Expression::Structure(_) => todo!(),
      Expression::Literal(ltrl) => Ok(self.literal(&ltrl)),
      Expression::FunctionCall(_) => todo!(),
      Expression::FsmPipe(_) => todo!(),
    }
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
        (Value::String(lhs_val), Value::Number(rhs_val), FormulaOperator::AddSub(AddSubOp::Sub)) => {
          return Err(MechError{tokens: trm.tokens(), msg: "interpreter.rs".to_string(), id: 135, kind: MechErrorKind::UnhandledFunctionArgumentKind(ValueKind::String,ValueKind::NumberLiteral)});
        }
        x => {
          return Err(MechError{tokens: trm.tokens(), msg: "interpreter.rs".to_string(), id: 135, kind: MechErrorKind::UnhandledFunctionArgumentKind(ValueKind::NumberLiteral,ValueKind::NumberLiteral)});
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

  fn empty(&mut self, ) -> Value {
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