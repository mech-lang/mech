use mech_core::{Block, Constraint};
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

  pub fn format(&mut self, block: Block) -> String {

    for (text, steps) in block.constraints {
      println!("{:?} {:?}", text, steps);
    }

    String::new()
  }

}