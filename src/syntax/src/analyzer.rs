// The purpose of the analyzer is to walk the program tree and
// - Assign a kind to each node. This is the output of that node.
// - Assign a size to each node.
// - Report any errors on the node
use mech_core::{MechError, hash_str, nodes::*};
use crate::parser2::*;
use serde_derive::*;
use std::any::Any;

#[derive(Clone, Debug)]
pub struct NodeAnnotation {
  pub kind_id: u64,
  pub kind_annotation: Option<KindAnnotation>,
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
    println!("Section Result: {:?}", result);
  }
  Ok(())
}

fn section_element(element: &SectionElement) -> Result<NodeAnnotation,MechError> {
  match element {
    SectionElement::MechCode(code) => {mech_code(&code)},
    _ => todo!(),
  }
}

fn mech_code(code: &MechCode) -> Result<NodeAnnotation,MechError> {
  match &code {
    MechCode::Expression(expr) => {expression(&expr)},
    MechCode::Statement(_) => todo!(),
    MechCode::FsmSpecification(_) => todo!(),
    MechCode::FsmImplementation(_) => todo!(),
    MechCode::FunctionDefine(_) => todo!(),
  }
}

fn expression(expr: &Expression) -> Result<NodeAnnotation,MechError> {
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

fn literal(ltrl: &Literal) -> NodeAnnotation {
  match &ltrl {
    Literal::Empty(_) => empty(),
    Literal::Boolean(bln) => boolean(bln),
    Literal::Number(num) => number(num),
    Literal::String(strng) => string(strng),
    Literal::Atom(atm) => atom(atm),
    Literal::TypedLiteral((ltrl,kind)) => {
      let kind_id = kind.hash();
      NodeAnnotation {
        kind_id,
        kind_annotation: Some(kind.clone()),
        size: Size{dimensions: 1, sizes: vec![1]},
      }
    },
  }
}

fn boolean(tkn: &Token) -> NodeAnnotation {
  NodeAnnotation {
      kind_id: hash_str("boolean"),
      kind_annotation: None,
      size: Size{dimensions: 1, sizes: vec![1]},
  }
}

fn number(tkn: &Number) -> NodeAnnotation {
  NodeAnnotation {
      kind_id: hash_str("number"),
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

fn string(tkn: &MechString) -> NodeAnnotation {
  NodeAnnotation {
      kind_id: hash_str("string"),
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
}

