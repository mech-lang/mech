use mech::runtime::{Block, Constraint};
use mech::operations::{Function, Plan, Comparator};
use parser::Node;
use lexer::Token;
use mech::indexes::Hasher;

pub struct Compiler {
  pub blocks: Vec<Block>,
  pub constraints: Vec<Constraint>,
  pub depth: usize,
  pub input_registers: usize,
  pub intermediate_registers: usize,
  pub output_registers: usize,
}

impl Compiler {

  pub fn new() -> Compiler {
    Compiler {
      blocks: Vec::new(),
      constraints: Vec::new(),
      depth: 0,
      input_registers: 1,
      intermediate_registers: 1,
      output_registers: 1,
    }
  }

  pub fn compile(&mut self, roots: Vec<Node>) -> Vec<Constraint> {
    
    let mut constraints = Vec::new();
    self.depth += 1;
    for root in roots {
      match root {
        // SELECT
        Node::Select{children} => {
          let table = &children[0];
          let id = get_id(table).unwrap();
          let columns = get_children(table).unwrap();
          for column in columns {
            let input = self.input_registers as u64;
            let intermediate = self.intermediate_registers as u64;
            let column_ix = byte_to_digit(*get_value(column).unwrap() as u8).unwrap();
            constraints.push(Constraint::Scan{table: id, column: column_ix as u64, input: input});
            constraints.push(Constraint::Identity{source: input, sink: intermediate});
            self.input_registers += 1;
            self.intermediate_registers += 1;
          }
        },
        // COLUMN
        Node::ColumnDefine{parts} => {
          let sink = &parts[0].clone();
          constraints.append(&mut self.compile(parts));
        },
        // MATH
        Node::MathExpression{parameters} => {
          let mut new_constraints = self.compile(parameters);
          let lhs = get_destination_register(&new_constraints[1]).unwrap() as u64;
          let rhs = get_destination_register(&new_constraints[4]).unwrap() as u64;
          for constraint in new_constraints {
            match constraint {
              Constraint::Function{operation, parameters, output} => {
                let modified_constraint = Constraint::Function{
                  operation, 
                  parameters: vec![lhs, rhs], 
                  output}
                ;
                constraints.push(modified_constraint);
              },
              _ => constraints.push(constraint),
            }
          }          
        },
        // INFIX
        Node::InfixOperation{token} => {
          let op: Function = match token {
            Token::Plus => Some(Function::Add),
            Token::Dash => Some(Function::Subtract),
            Token::Asterisk => Some(Function::Multiply),
            Token::Backslash => Some(Function::Divide),
            _ => None,
          }.unwrap();
          let intermediate = self.intermediate_registers as u64;
          constraints.push(Constraint::Function {operation: op, parameters: vec![0, 0], output: intermediate});
          self.intermediate_registers += 1;
        }
        _ => (),
      }
    }
    self.constraints = constraints.clone();
    constraints
  }

}

fn get_destination_register(constraint: &Constraint) -> Option<usize> {
  match constraint {
    Constraint::Identity{source, sink} => Some(*sink as usize),
    _ => None,
  }
}


fn byte_to_digit(byte: u8) -> Option<usize> {
  match byte {
    48 => Some(0),
    49 => Some(1),
    50 => Some(2),
    51 => Some(3),
    52 => Some(4),
    53 => Some(5),
    54 => Some(6),
    55 => Some(7),
    56 => Some(8),
    57 => Some(9),
    _ => None,
  }
}

fn get_number_from_select(node: &Node) -> u8 {
  *get_value(&get_first_child(get_first_child(node).unwrap()).unwrap()).unwrap() as u8
}

fn get_first_child(node: &Node) -> Option<&Node> {
  match node {
    Node::Table{id, token, children} => Some(&children[0]),
    Node::Select{children} => Some(&children[0]),
    _ => None,
  }
}

fn get_children(node: &Node) -> Option<&Vec<Node>> {
  match node {
    Node::Table{id, token, children} => Some(children),
    _ => None,
  }
}

fn get_id(node: &Node) -> Option<u64> {
  match node {
    Node::Table{id, token, children} => Some(*id),
    _ => None,
  }
}

fn get_value(node: &Node) -> Option<&u64> {
  match node {
    Node::Number{value, token} => Some(value),
    _ => None,
  }
}
