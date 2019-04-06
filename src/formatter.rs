use mech_core::{Block, Constraint, TableId};
use mech_core::{Function, Comparator, Logic, Parameter, Quantity, ToQuantity, QuantityMath, make_quantity};
use super::compiler::Node;
use hashbrown::hash_map::{HashMap, Entry};

// # Formatter

// Formats a block as text syntax

#[derive(Debug, Clone, PartialEq)]
pub struct Formatter{
  code: String,
  identifiers: HashMap<u64, String>,
  rows: usize,
  cols: usize,
}

impl Formatter {

  pub fn new() -> Formatter {
    Formatter {
      code: String::new(),
      identifiers: HashMap::new(),
      rows: 0,
      cols: 0,
    }
  }

  pub fn format(&mut self, block_ast: &Node) -> String {
    let code = self.write_node(block_ast);
    code
  }

  pub fn write_node(&mut self, node: &Node) -> String {
    let mut code = String::new();
    let mut node_type = "";
    match node {
      Node::Constant{value} => {
        node_type = "constant";
        code = format!("{:?}", value.to_float());
      },
      Node::LogicExpression{operator, children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        code = format!("{} {:?} {}", lhs, operator, rhs);
      },
      Node::FilterExpression{comparator, children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        code = format!("{} {:?} {}", lhs, comparator, rhs);
      },
      Node::Function{name, children} => {
        match name.as_ref() {
          "*" | "+" | "-" | "/" => {
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
      Node::Identifier{name, id} => {
        code = name.clone();
      },
      Node::TableDefine{children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        code = format!("#{} = {}", lhs, rhs);
      },
      Node::SetData{children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        code = format!("{} := {}", lhs, rhs);
      },
      Node::VariableDefine{children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        code = format!("{} = {}", lhs, rhs);
      },
      Node::SelectData{name, id, children} => {
        for child in children {
          let written_child = self.write_node(child);
          code = format!("{}{}",code, written_child);
        }
        let formatted_name = match id {
          TableId::Local(..) => format!("{}", name),
          TableId::Global(..) => format!("#{}", name),
        };
        code = format!("{}{}",formatted_name, code);
      }
      Node::SubscriptIndex{children} => {
        for (ix, child) in children.iter().enumerate() {
          let written_child = self.write_node(child);
          if ix == children.len() - 1 {
            code = format!("{}{}",code, written_child);
          } else {
            code = format!("{}{}, ",code, written_child);
          }
        }
        code = format!("{{{}}}", code);
      }
      Node::AnonymousTableDefine{children} => {
        let table_contents = self.write_node(&children[0]);
        if self.rows == 1 && self.cols == 1 {
          code = format!("{}", table_contents);
        } else {
          code = format!("[{}]", table_contents);
        }
      }
      Node::InlineTable{children} => {
        for child in children {
          let binding = self.write_node(&child);
          code = format!("{}{} ", code, binding);
        }
        code = format!("[{}]", code);
      }
      Node::Binding{children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        code = format!("{}: {}", lhs, rhs);
      }
      Node::DataWatch{children} => {
        let table = self.write_node(&children[0]);
        code = format!("~ {}", table);
      }
      Node::TableRow{children} => {
        self.rows += 1;
        self.cols = 0;
        for child in children {
          code = self.write_node(child);
        }
      }
      Node::Column{children} => {
        self.cols += 1;
        for child in children {
          code = self.write_node(child);
        }
      }
      Node::SubscriptIndex{children} |
      Node::MathExpression{children} |
      Node::Expression{children} |
      Node::Statement{children} |
      Node::Constraint{children, ..} => { 
        for child in children {
          code = self.write_node(child);
        }
      },
      Node::Block{children, ..} => { 
        for child in children {
          let constraint = self.write_node(child);
          code = format!("{}{}\n", code, constraint);
        }
      },
      _ => (),
    }
    code = format!("<span class=\"highlight-{}\">{}</span>", node_type, code);
    code
  }

}