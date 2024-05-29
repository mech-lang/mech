// The purpose of the analyzer is to walk the program tree and
// - Assign a kind to each node. This is the output of that node.
// - Assign a size to each node.
// - Report any errors on the node
use mech_core::{MechError, hash_str, nodes::*};
use crate::parser2::*;
use serde_derive::*;
use std::any::Any;

#[derive(Clone, Debug)]
pub struct Annotation {
  pub kind: u64,
  pub size: Size,
}

#[derive(Clone, Debug)]
pub struct Size {
  dimensions: usize,
  sizes: Vec<usize>
}

pub fn analyze(tree: &Program) -> Result<(),MechError> {
  let resut = program(tree)?;
  Ok(())
}

fn program(program: &Program) -> Result<(),MechError> {
  let result = body(&program.body)?;
  Ok(())
}

fn body(body: &Body) -> Result<(),MechError> {
  for sec in &body.sections {
    let result = section(&sec)?;
  }
  Ok(())
}

fn section(section: &Section) -> Result<(),MechError> {
  for el in &section.elements {
    let result = section_element(&el)?;
  }
  Ok(())
}

fn section_element(element: &SectionElement) -> Result<(),MechError> {
  match element {
    SectionElement::MechCode(code) => {mech_code(&code);},
    _ => unimplemented!(),
  }
  Ok(())
}

fn mech_code(code: &MechCode) -> Result<(),MechError> {
  match &code {
    MechCode::Expression(expr) => {expression(&expr)?;},
    MechCode::Statement(_) => (),
    MechCode::FsmSpecification(_) => (),
    MechCode::FsmImplementation(_) => (),
    MechCode::FunctionDefine(_) => (),
  }
  Ok(())
}

fn expression(expr: &Expression) -> Result<(),MechError> {
  match &expr {
    Expression::Var(_) => (),
    Expression::Slice(_) => (),
    Expression::Formula(_) => (),
    Expression::Structure(_) => (),
    Expression::Literal(ltrl) => {literal(&ltrl)?;},
    Expression::Transpose(_) => (),
    Expression::FunctionCall(_) => (),
    Expression::FsmPipe(_) => (),
  }
  Ok(())
}

fn literal(ltrl: &Literal) -> Result<(),MechError> {
  /*match &ltrl {
    Literal::Empty(_) => (),
    Literal::Boolean(bln) => boolean(&bln),
    Literal::Number(num) => number(&num),
    Literal::String(strng) => string(&num),
    Literal::Atom(_) => (),
    Literal::TypedLiteral(_) => {

    },
  }*/
  Ok(())
}

fn boolean(tkn: Token) -> Annotation {
  Annotation {
      kind: hash_str("boolean"),
      size: Size{dimensions: 1, sizes: vec![1]},
  }
}

/*fn empty(ltrl: &Empty) -> Annotation {
  Annotation {
    kind: hash_str("_"),
    size: Size{dimensions: 1, sizes: vec![1]},
  }
}

fn string(ltrl: &MechString) -> Annotation {
  Annotation {
    kind: hash_str("string"),
    size: Size{dimensions: 1, sizes: vec![1]},
  }
}*/

/*fn number(ltrl: &Number) -> Annotation {
  Annotation {
    kind: hash_str("number"),
    size: Size{dimensions: 1, sizes: vec![1]},
    token: 
  }
}*/