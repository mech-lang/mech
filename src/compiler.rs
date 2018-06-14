use mech::runtime::{Block, Constraint};
use mech::operations::{Function, Plan, Comparator};
use parser::Node;
use mech::indexes::Hasher;




pub struct Compiler {
  pub blocks: Vec<Block>,
  pub constraints: Vec<Constraint>,
}

impl Compiler {

  pub fn new() -> Compiler {
    Compiler {
      blocks: Vec::new(),
      constraints: Vec::new(),
    }
  }

  pub fn compile(&mut self, roots: Vec<Node>) -> Vec<Constraint> {
    
    let mut constraints = Vec::new();
    for root in roots {
      self.walk_tree(&root,0);
      match root {
        // SELECT
        Node::Select{children} => {
          let table = &children[0];
          let id = get_id(table).unwrap();
          let columns = get_children(table).unwrap();
          for column in columns {
            let column_ix = get_value(column).unwrap();
            constraints.push(Constraint::Scan{table: id, column: *column_ix, input: 0});
            constraints.push(Constraint::Identity{source: 0, sink: 0});
          }
        },
        Node::ColumnDefine{parts} => {
          //let sink = &parts[0].clone();
          //let left = &parts[1].clone();
          //let right = &parts[2].clone();
          //constraints.append(&mut self.compile(parts));
          //constraints.push(Constraint::Function {operation: Function::Add, parameters: vec![0, 0], output: 0}); 
        },
        Node::MathExpression{operation, arguments} => {

        },
        _ => (),
      }
    }
    self.constraints = constraints.clone();
    constraints
  }

  pub fn walk_tree(&mut self, node: &Node, depth: usize) {
    space(depth + 1);
    println!("{:?}", node);
    match node {
      Node::Table{id, token, children} => {
        for child in children {
          self.walk_tree(child, depth + 1)
        }
      },
      Node::Select{children} => {
        for child in children {
          self.walk_tree(child, depth + 1)
        }
      },
      Node::ColumnDefine{parts} => {
        for child in parts {
          self.walk_tree(child, depth + 1)
        }
      },
      _ => (),
    }
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

fn space(n: usize) {
  for _ in 0..n {
    print!(" ");
  }
}