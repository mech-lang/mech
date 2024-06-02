use mech_core::{MechError, hash_str, nodes::*};
use crate::parser2::*;
use serde_derive::*;
use std::any::Any;

#[derive(Clone, Debug)]
pub enum Value {
  Number(i64),
  String(String),
  Bool(bool),
  Atom(u64),
  Empty
}
pub fn interpret(tree: &Program) -> Result<Value,MechError> {
  program(tree)
}

fn program(program: &Program) -> Result<Value,MechError> {
  body(&program.body)
}

fn body(body: &Body) -> Result<Value,MechError> {
  let mut result = None;
  for sec in &body.sections {
    result = Some(section(&sec)?);
  }
  Ok(result.unwrap())
}

fn section(section: &Section) -> Result<Value,MechError> {
  let mut result = None;
  for el in &section.elements {
    result = Some(section_element(&el)?);
    println!("Interpreter Result: {:?}", result);
  }
  Ok(result.unwrap())
}

fn section_element(element: &SectionElement) -> Result<Value,MechError> {
  match element {
    SectionElement::MechCode(code) => {mech_code(&code)},
    _ => todo!(),
  }
}

fn mech_code(code: &MechCode) -> Result<Value,MechError> {
  match &code {
    MechCode::Expression(expr) => {expression(&expr)},
    MechCode::Statement(_) => todo!(),
    MechCode::FsmSpecification(_) => todo!(),
    MechCode::FsmImplementation(_) => todo!(),
    MechCode::FunctionDefine(_) => todo!(),
  }
}

fn expression(expr: &Expression) -> Result<Value,MechError> {
  match &expr {
    Expression::Var(_) => todo!(),
    Expression::Slice(_) => todo!(),
    Expression::Formula(_) => todo!(),
    Expression::Structure(_) => todo!(),
    Expression::Literal(ltrl) => Ok(literal(&ltrl)),
    Expression::FunctionCall(_) => todo!(),
    Expression::FsmPipe(_) => todo!(),
  }
}

fn literal(ltrl: &Literal) -> Value {
  match &ltrl {
    Literal::Empty(_) => empty(),
    Literal::Boolean(bln) => todo!(),
    Literal::Number(num) => number(num),
    Literal::String(strng) => string(strng),
    Literal::Atom(atm) => todo!(),
    Literal::TypedLiteral((ltrl,kind)) => {
      todo!();
    },
  }
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
    RealNumber::Float(num) => todo!(),
    RealNumber::Decimal(num) => todo!(),
    RealNumber::Hexadecimal(num) => todo!(),
    RealNumber::Octal(num) => todo!(),
    RealNumber::Binary(num) => todo!(),
    RealNumber::Scientific(num) => todo!(),
    RealNumber::Rational(num) => todo!(),
  }
}

fn integer(int: &Token) -> Value {
  let num: i64 = int.chars.iter().collect::<String>().parse::<i64>().unwrap();
  Value::Number(num)
}

fn string(tkn: &MechString) -> Value {
  let strng: String = tkn.text.chars.iter().collect::<String>();
  Value::String(strng)
}

fn empty() -> Value {
  Value::Empty
}

/*fn boolean(tkn: &Token) -> NodeAnnotation {
  NodeAnnotation {
      kind_id: hash_str("boolean"),
      kind_annotation: None,
      size: Size{dimensions: 1, sizes: vec![1]},
  }
}

fn empty() -> NodeAnnotation {
  NodeAnnotation {
      kind_id: hash_str("empty"),
      kind_annotation: None,
      size: Size{dimensions: 1, sizes: vec![1]},
  }
}

fn atom(tkn: &Atom) -> NodeAnnotation {
  NodeAnnotation {
      kind_id: hash_str("atom"),
      kind_annotation: None,
      size: Size{dimensions: 1, sizes: vec![1]},
  }
}*/

