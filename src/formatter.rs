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
  html: bool,
}

impl Formatter {

  pub fn new() -> Formatter {
    Formatter {
      code: String::new(),
      identifiers: HashMap::new(),
      rows: 0,
      cols: 0,
      indent: 0,
      html: false,
    }
  }

  pub fn format(&mut self, block_ast: &Node, html: bool) -> String {
    self.html = html;
    let code = self.write_node(block_ast);
    code
  }

  pub fn write_node(&mut self, node: &Node) -> String {
    let mut code = String::new();
    let mut node_type = "";
    match node {
      Node::Constant{value} => {
        node_type = "constant";
        code = format!("{}", value.format());
      },
      Node::Empty => {
        node_type = "empty";
        code = "_".to_string();
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
            if lhs == "<span class=\"highlight-constant\" id=\"constant\">0</span>" || lhs == "0" {
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
                if self.html {
                  code = format!("{}{}<span class=\"highlight-clear\">, </span>",code, binding);
                } else {
                  code = format!("{}{}, ",code, binding);
                }
                
              }
            }
            code = format!("{}({})", name, code);
          }
        }
      },
      Node::Table{name, id} => {
        code = name.clone();
        if self.html {
          code = format!("<span class=\"highlight-bracket\">#</span>{}", code)
        }
        else {
          code = format!("#{}", code)
        }
      },
      Node::Identifier{name, id} => {
        code = name.clone();
      },
      Node::TableDefine{children} => {
        let lhs = self.write_node(&children[0]);
        self.indent = if self.html {
          lhs.len() + 3 - 37
        } else {
          lhs.len() + 3
        };
        let rhs = self.write_node(&children[1]);
        let lhs = if self.html {
          format!("<span class=\"highlight-variable\">{}</span>", lhs)
        }
        else {
          format!("{}", lhs)
        };
        code = format!("{} = {}", lhs, rhs)
      },
      Node::SetData{children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        code = format!("{} := {}", lhs, rhs);
      },
      Node::AddRow{children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        code = format!("{} += {}", lhs, rhs);
      },
      Node::VariableDefine{children} => {
        let lhs = self.write_node(&children[0]);
        self.indent = if self.html {
          lhs.len() + 4
        } else {
          lhs.len() + 2
        };
        let rhs = self.write_node(&children[1]);
        let lhs = if self.html {
          format!("<span class=\"highlight-variable\">{}</span>", lhs)
        }
        else {
          format!("{}", lhs)
        };
        code = format!("{} = {}", lhs, rhs);
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
          TableId::Local(..) => {
            if self.html {
              format!("<span class=\"highlight-variable\">{}</span>", name)
            }
            else {
              format!("{}", name)
            }
          },
          TableId::Global(..) => {
            if self.html {
              format!("<span class=\"highlight-bracket\">#</span><span class=\"highlight-variable\">{}</span>", name)
            }
            else {
              format!("#{}", name)
            }
          },
        };
        code = format!("{}{}",formatted_name, code);
      }
      Node::SubscriptIndex{children} => {
        for (ix, child) in children.iter().enumerate() {
          let written_child = self.write_node(child);
          if ix == children.len() - 1 {
            code = format!("{}{}",code, written_child);
          } else {
            if self.html {
              code = format!("{}{}<span class=\"highlight-clear\">, </span>",code, written_child);
            } else {
              code = format!("{}{}, ",code, written_child);
            }
          }
        }
        if self.html {
          code = format!("<span class=\"highlight-bracket\">{{</span>{}<span class=\"highlight-bracket\">}}</span>", code);
        } else {
          code = format!("{{{}}}", code);
        }
        
      }
      Node::DotIndex{children} => {
        let mut reversed = children.clone();
        reversed.reverse();
        for child in reversed {
          let written_child = self.write_node(&child);
          code = format!("{}{}", code, written_child);
        }
        code = format!(".{}", code);
      }
      Node::AnonymousTableDefine{children} => {
        let rows = self.rows;
        let cols = self.cols;
        self.rows = 0;
        self.cols = 0;
        for (ix, child) in children.iter().enumerate() {
          let mut newline = "";
          let written_child = self.write_node(&child);
          if ix != children.len() - 1 {
            newline = "\n";
          }
          code = format!("{}{}{}", code, written_child, newline);
        }
        if self.rows == 1 && self.cols == 1 {
          code = format!("{}", code);
        } else {
          if self.html {
            code = format!("<span class=\"highlight-bracket\">[</span>{}<span class=\"highlight-bracket\">]</span>", code);
          } else {
            code = format!("[{}]", code);
          }
        }
        self.rows = rows;
        self.cols = cols;
      }
      Node::SelectAll => {
        node_type = "function";
        code = ":".to_string();
      }
      Node::InlineTable{children} => {
        for (ix, child) in children.iter().enumerate() {
          let binding = self.write_node(&child);
          if ix == children.len() - 1 {
            code = format!("{}{}",code, binding);
          } else {
            if self.html {
              code = format!("{}{}<span class=\"highlight-clear\">, </span>",code, binding);
            } else {
              code = format!("{}{}, ",code, binding);
            }
          }
        }
        if self.html {
          code = format!("<span class=\"highlight-bracket\">[</span>{}<span class=\"highlight-bracket\">]</span>", code);
        } else {
          code = format!("[{}]", code);
        };
      }
      Node::Binding{children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        if self.html {
          code = format!("<span class=\"highlight-parameter\">{}:</span> <span class=\"highlight-variable\">{}</span>", lhs, rhs);
        } else {
          code = format!("{}: {}", lhs, rhs);
        };
      }
      Node::DataWatch{children} => {
        let table = self.write_node(&children[0]);
        if self.html {
          code = format!("<span class=\"highlight-watch\">~</span> {}", table);
        } else {
          code = format!("~ {}", table);
        };
      }
      Node::TableHeader{children} => {
        self.rows += 1;
        node_type = "parameter";
        for child in children {
          let written_child = self.write_node(child);
          code = format!("{}{} ",code, written_child);
        }
        code = format!("|{}|",code);
      }
      Node::TableRow{children} => {
        self.rows += 1;
        self.cols = 0;
        for (ix, child) in children.iter().enumerate() {
          let mut space = "";
          let written_child = self.write_node(child);
          if ix != children.len() - 1 {
            space = " ";
          }
          code = format!("{}{}{}", code, written_child, space)
        }
        let indent = if self.rows != 1 {
          repeat_char(" ", self.indent)
        } else {
          "".to_string()
        };
        code = format!("{}{}", indent, code)
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
    if self.html && node_type != "" {
      code = format!("<span class=\"highlight-{}\" id=\"{}\">{}</span>", node_type, node_type, code);
    }
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