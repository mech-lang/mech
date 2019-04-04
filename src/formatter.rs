use mech_core::{Block, Constraint};
use mech_core::{Function, Comparator, Logic, Parameter, Quantity, ToQuantity, QuantityMath, make_quantity};
use super::compiler::Node;
use hashbrown::hash_map::{HashMap, Entry};

// # Formatter

// Formats a block as text syntax

#[derive(Debug, Clone, PartialEq)]
pub struct Formatter{
  code: String,
  identifiers: HashMap<u64, String>,
}

impl Formatter {

  pub fn new() -> Formatter {
    Formatter {
      code: String::new(),
      identifiers: HashMap::new(),
    }
  }

  pub fn format(&mut self, block_ast: &Node) -> String {
    let code = self.write_node(block_ast);
    code
  }

  pub fn write_node(&mut self, node: &Node) -> String {
    let mut code = String::new();
    match node {
      Node::Constant{value} => {
        code = format!("{:?}", value.to_float());
      },
      Node::Function{name, children} => {
        match name.as_ref() {
          "*" | "+" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} {} {}", lhs, name, rhs);
          },
          _ => (),
        }
      },
      Node::Table{name, id} => {
        code = name.clone();
      },
      Node::TableDefine{children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        code = format!("#{} = {}", lhs, rhs);
      },
      Node::MathExpression{children} |
      Node::Expression{children} |
      Node::Statement{children} |
      Node::Constraint{children, ..} | 
      Node::Block{children, ..} => { 
        for child in children {
          code = self.write_node(child);
        }
      },
      _ => (),
    }
    code
  }

}