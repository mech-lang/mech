use mech_core::{MechError, MechErrorKind, hash_str, nodes::*};
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
    Expression::Formula(fctr) => factor(fctr),
    Expression::Structure(_) => todo!(),
    Expression::Literal(ltrl) => Ok(literal(&ltrl)),
    Expression::FunctionCall(_) => todo!(),
    Expression::FsmPipe(_) => todo!(),
  }
}

fn factor(fctr: &Factor) -> Result<Value,MechError> {
  match fctr {
    Factor::Term(trm) => term(trm),
    Factor::Expression(expr) => expression(expr),
    Factor::Negated(_) => todo!(),
    Factor::Transpose(_) => todo!(),
  }
}

fn term(trm: &Term) -> Result<Value,MechError> {
  let mut lhs_result = factor(&trm.lhs)?;
  for (op,rhs) in &trm.rhs {
    let rhs_result = factor(&rhs)?;
    lhs_result = match (lhs_result, rhs_result, op) {
      (Value::Number(lhs_val), Value::Number(rhs_val), FormulaOperator::AddSub(AddSubOp::Add)) 
        => Value::Number(lhs_val + rhs_val),
      (Value::Number(lhs_val), Value::Number(rhs_val), FormulaOperator::AddSub(AddSubOp::Sub)) 
        => Value::Number(lhs_val - rhs_val),
      x => {
        println!("{:?}", trm);
        return Err(MechError{msg: "interpreter.rs".to_string(), id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});
      }
    }
  }
  Ok(lhs_result)
}

fn literal(ltrl: &Literal) -> Value {
  match &ltrl {
    Literal::Empty(_) => empty(),
    Literal::Boolean(bln) => boolean(bln),
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

fn boolean(tkn: &Token) -> Value {
  let strng: String = tkn.chars.iter().collect::<String>();
  let val = match strng.as_str() {
    "true" => true,
    "false" => false,
    _ => unreachable!(),
  };
  Value::Bool(val)
}

/*fn atom(tkn: &Atom) -> Value {

}*/

