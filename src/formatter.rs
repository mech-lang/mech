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
  indent: usize,
}

impl Formatter {

  pub fn new() -> Formatter {
    Formatter {
      code: String::new(),
      identifiers: HashMap::new(),
      rows: 0,
      cols: 0,
      indent: 0,
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
          "*" | "+" | "/" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} {} {}", lhs, name, rhs);
          },
          "-" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            if lhs == "<span class=\"highlight-constant\">0.0</span>" {
              code = format!("{}{}", name, rhs);
            } else {
              code = format!("{} {} {}", lhs, name, rhs);
            }
          }
          _ => {
            node_type = "function";
            for (ix, child) in children.iter().enumerate() {
              let binding = self.write_node(&child);
              if ix == children.len() - 1 {
                code = format!("{}{}",code, binding);
              } else {
                code = format!("{}{}<span class=\"highlight-clear\">, </span>",code, binding);
              }
            }
            code = format!("{}({})", name, code);
          }
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
        let prefix = format!("{} = ", lhs);
        self.indent = prefix.len() + 1 - 31;
        let rhs = self.write_node(&children[1]);
        code = format!("<span class=\"highlight-bracket\">#</span>{}{}", prefix, rhs);
      },
      Node::SetData{children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        code = format!("{} := {}", lhs, rhs);
      },
      Node::VariableDefine{children} => {
        let lhs = self.write_node(&children[0]);
        let prefix = format!("{} = ", lhs);
        let rhs = self.write_node(&children[1]);
        code = format!("{}{}", prefix, rhs);
      },
      Node::String{text} => {
        node_type = "string";
        code = format!("\"{}\"", text);
      },
      Node::SelectData{name, id, children} => {
        for child in children {
          let written_child = self.write_node(child);
          code = format!("{}{}",code, written_child);
        }
        let formatted_name = match id {
          TableId::Local(..) => format!("{}", name),
          TableId::Global(..) => format!("<span class=\"highlight-bracket\">#</span>{}", name),
        };
        code = format!("{}{}",formatted_name, code);
      }
      Node::SubscriptIndex{children} => {
        for (ix, child) in children.iter().enumerate() {
          let written_child = self.write_node(child);
          if ix == children.len() - 1 {
            code = format!("{}{}",code, written_child);
          } else {
            code = format!("{}{}<span class=\"highlight-clear\">, </span>",code, written_child);
          }
        }
        code = format!("<span class=\"highlight-bracket\">{{{}}}</span>", code);
      }
      Node::AnonymousTableDefine{children} => {
        self.rows = 0;
        self.cols = 0;
        for child in children {
          let written_child = self.write_node(&child);
          code = format!("{}{}", code, written_child);
        }
        if self.rows == 1 && self.cols == 1 {
          code = format!("{}", code);
        } else {
          code = format!("<span class=\"highlight-bracket\">[</span>{}<span class=\"highlight-bracket\">]</span>", code);
        }
      }
      Node::SelectAll => {
        code = ":".to_string();
      }
      Node::InlineTable{children} => {
        for (ix, child) in children.iter().enumerate() {
          let binding = self.write_node(&child);
          if ix == children.len() - 1 {
            code = format!("{}{}",code, binding);
          } else {
            code = format!("{}{}<span class=\"highlight-clear\">, </span>",code, binding);
          }
        }
        code = format!("<span class=\"highlight-bracket\">[</span>{}<span class=\"highlight-bracket\">]</span>", code);
      }
      Node::Binding{children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        code = format!("<span class=\"highlight-parameter\">{}:</span> <span class=\"highlight-clear\">{}</span>", lhs, rhs);
      }
      Node::DataWatch{children} => {
        let table = self.write_node(&children[0]);
        code = format!("~ {}", table);
      }
      Node::TableHeader{children} => {
        self.rows += 1;
        node_type = "parameter";
        for child in children {
          let written_child = self.write_node(child);
          code = format!("{}{} ",code, written_child);
        }
        code = format!("|{}|\n",code);
      }
      Node::TableRow{children} => {
        self.rows += 1;
        self.cols = 0;
        for child in children {
          let written_child = self.write_node(child);
          code = format!("{}{} ", code, written_child)
        }
        let indent = if self.rows != 1 {
          repeat_char(" ", self.indent)
        } else {
          "".to_string()
        };
        code = format!("{}{}\n", indent, code)
      }
      Node::Column{children} => {
        self.cols += 1;
        for child in children {
          code = self.write_node(child);
        }
      }
      Node::Attribute{children} |
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

fn repeat_char(to_print: &str, n: usize) -> String {
  let mut result = "".to_string();
  for _ in 0..n {
    result = format!("{}{}", result, to_print);
  }
  result
}