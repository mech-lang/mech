use mech::{Block, Constraint};
use mech::{Function, Plan, Comparator};
use mech::Hasher;
use parser;
use lexer::Token;
use alloc::{String, Vec, fmt};

#[derive(Clone, PartialEq)]
pub enum Node {
  Root{ children: Vec<Node> },
  Program{ children: Vec<Node> },
  Head{ children: Vec<Node> },
  Body{ children: Vec<Node> },
  Section{ children: Vec<Node> },
  Block{ children: Vec<Node> },
  LHS{ children: Vec<Node> },
  RHS{ children: Vec<Node> },
  Define { name: String, id: u64},
  ColumnDefine {children: Vec<Node> },
  Constraint{ children: Vec<Node> },
  Title{ text: String },
  Table{ name: String, id: u64 },
  Paragraph{ text: String },
  Constant {value: u64},
  String{ text: String },
  Token{ token: Token, byte: u8 },
  Null,
}

impl fmt::Debug for Node {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    print_recurse(self, 0);
    Ok(())
  }
}

pub fn print_recurse(node: &Node, level: usize) {
  spacer(level);
  let children: Option<&Vec<Node>> = match node {
    Node::Root{children} => {print!("Root\n"); Some(children)},
    Node::Program{children} => {print!("Program\n"); Some(children)},
    Node::Head{children} => {print!("Head\n"); Some(children)},
    Node::Body{children} => {print!("Body\n"); Some(children)},
    Node::LHS{children} => {print!("LHS\n"); Some(children)},
    Node::RHS{children} => {print!("RHS\n"); Some(children)},
    Node::ColumnDefine{children} => {print!("ColumnDefine\n"); Some(children)},
    Node::Section{children} => {print!("Section\n"); Some(children)},
    Node::Block{children} => {print!("Block\n"); Some(children)},
    Node::Constraint{children} => {print!("Constraint\n"); Some(children)},
    Node::String{text} => {print!("String({:?})\n", text); None},
    Node::Title{text} => {print!("Title({:?})\n", text); None},
    Node::Constant{value} => {print!("Constant({:?})\n", value); None},
    Node::Paragraph{text} => {print!("Paragraph({:?})\n", text); None},
    Node::Table{name,id} => {print!("#{}({:?})\n", name, id); None},
    Node::Define{name,id} => {print!("Define #{}({:?})\n", name, id); None},
    Node::Token{token, byte} => {print!("Token({:?})\n", token); None},
    Node::Null => {print!("Null\n"); None},
    _ => {print!("Unhandled Node"); None},
  };  
  match children {
    Some(childs) => {
      for child in childs {
        print_recurse(child, level + 1)
      }
    },
    _ => (),
  }    
}

pub fn spacer(width: usize) {
  let limit = if width > 0 {
    width - 1
  } else {
    width
  };
  for _ in 0..limit {
    print!("│");
  }
  print!("├");
}


pub struct Compiler {
  pub blocks: Vec<Block>,
  pub constraints: Vec<Constraint>,
  pub depth: usize,
  pub input_registers: usize,
  pub intermediate_registers: usize,
  pub output_registers: usize,
  pub syntax_tree: Node,
  pub node_stack: Vec<Node>, 
  pub section: usize,
  pub block: usize,
}

impl Compiler {

  pub fn new() -> Compiler {
    Compiler {
      blocks: Vec::new(),
      constraints: Vec::new(),
      node_stack: Vec::new(),
      depth: 0,
      section: 1,
      block: 1,
      input_registers: 1,
      intermediate_registers: 1,
      output_registers: 1,
      syntax_tree: Node::Root{ children: Vec::new() },
    }
  }

  pub fn compile_block(&mut self, node: Node) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    match node {
      Node::Block{children} => {
        let mut block = Block::new();
        block.name = format!("{:?},{:?}", self.section, self.block);
        block.id = Hasher::hash_string(block.name.clone()) as usize;
        self.block += 1;
        let constraints = self.compile_constraints(children);
        block.add_constraints(constraints);
        block.plan();
        blocks.push(block);
      },
      Node::Root{children} => {
        let result = self.compile_blocks(children);
        self.blocks = result;
      },
      Node::Program{children} => {blocks.append(&mut self.compile_blocks(children));},
      Node::Body{children} => {blocks.append(&mut self.compile_blocks(children));},
      Node::Section{children} => {
        blocks.append(&mut self.compile_blocks(children));
        self.section += 1;
        self.block = 1;
      },
      _ => (),
    }
    blocks
  }

  pub fn compile_blocks(&mut self, nodes: Vec<Node>) -> Vec<Block> {
    let mut compiled = Vec::new();
    for node in nodes {
      compiled.append(&mut self.compile_block(node));
    }
    compiled
  }

  pub fn compile_constraint(&mut self, node: Node) -> Vec<Constraint> {
    let mut constraints: Vec<Constraint> = Vec::new();
    match node {
      Node::ColumnDefine{children} => {constraints.append(&mut self.compile_constraints(children));},
      Node::Constant{value} => {
        constraints.push(Constraint::Constant{value: value as i64, input: self.intermediate_registers as u64});
        self.intermediate_registers += 1;
      },
      _ => (),
    }
    constraints
  }

  pub fn compile_constraints(&mut self, nodes: Vec<Node>) -> Vec<Constraint> {
    let mut compiled = Vec::new();
    for node in nodes {
      compiled.append(&mut self.compile_constraint(node));
    }
    compiled
  }

  pub fn build_syntax_tree(&mut self, node: parser::Node) -> Vec<Node> {
    let mut compiled = Vec::new();
    self.depth += 1;
    match node {
      parser::Node::Root{children} => {
        let result = self.compile_nodes(children);
        self.syntax_tree = Node::Root{children: result};        
      },
      parser::Node::Program{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Program{children: result});
      },
      parser::Node::Head{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Head{children: result});
      },
      parser::Node::Body{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Body{children: result});
      },
      parser::Node::Section{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Section{children: result});
      },
      parser::Node::Block{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Block{children: result});
      },
      parser::Node::LHS{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::LHS{children: result});
      },
      parser::Node::RHS{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::RHS{children: result});
      },
      parser::Node::Constraint{children} => {
        let result = self.compile_nodes(children);
        println!("Constraint: {:?}", result);
        //let constraint = result[2].clone();
        //match constraint {
        //  Node::Token{..} => (),
        //  _ => compiled.push(constraint),
        // }
      },
      parser::Node::ProseOrCode{children} => {
        compiled.append(&mut self.compile_nodes(children));
      },
      parser::Node::StatementOrExpression{children} => {
        compiled.append(&mut self.compile_nodes(children));
      },
      parser::Node::Statement{children} => {
        compiled.append(&mut self.compile_nodes(children));
      },
      parser::Node::ColumnDefine{children} => {
        let result = self.compile_nodes(children);
        println!("{:?}", result);
        for node in result {
          match node {
            Node::LHS{children} => {
              for n in children {
                match n {
                  Node::Table{name, id} => {
                    println!("#{} {:?}", name, id);
                    self.constraints.push(Constraint::Insert{table: id, column: 1, output: 1})
                  },
                  _ => println!("{:?}",n),
                }  
              }
              
            },
            Node::RHS{children} => {
              for n in children {
                match n {
                  _ => println!("{:?}",n),
                }  
              }
            },
            _ => (),
          }
        }
      },
      parser::Node::Data{children} => {
        compiled.append(&mut self.compile_nodes(children));
      },
      parser::Node::Table{children} => {
        let result = self.compile_nodes(children);
        let table_name = match &result[1] {
          Node::String{text} => text.clone(),
          _ => String::from(""),
        };
        let id = Hasher::hash_string(table_name.clone());
        compiled.push(Node::Table{name: table_name, id});
      },
      parser::Node::Expression{children} => {
        compiled.append(&mut self.compile_nodes(children));
      },
      parser::Node::Data{children} => {
        compiled.append(&mut self.compile_nodes(children));
      },
      parser::Node::Constant{children} => {
        let mut value = 0;
        let mut result = self.compile_nodes(children);
        for node in result {
          match node {
            Node::Token{token, byte} => {
              let digit = byte_to_digit(byte).unwrap();
              value += digit;
            },
            _ => (),
          }
        }
        compiled.push(Node::Constant{value});
      },
      parser::Node::Paragraph{children} => {
        let mut result = self.compile_nodes(children);
        let node = match &result[0] {
          Node::String{text} => Node::Paragraph{text: text.clone()},
          _ => Node::Null,
        };
        compiled.push(node);
      },
      parser::Node::Title{children} => {
        let mut result = self.compile_nodes(children);
        let node = match &result[2] {
          Node::String{text} => Node::Title{text: text.clone()},
          _ => Node::Null,
        };
        compiled.push(node);
      },
      parser::Node::Subtitle{children} => {
        let mut result = self.compile_nodes(children);
        let node = match &result[3] {
          Node::String{text} => Node::Title{text: text.clone()},
          _ => Node::Null,
        };
        compiled.push(node);
      },
      parser::Node::Text{children} => {
        let mut result = self.compile_nodes(children);
        let mut text_node = String::new();
        for node in result {
          match node {
            Node::String{text} => text_node.push_str(&text),
            Node::Token{token: Token::Space, ..} => text_node.push(' '),
            _ => (),
          }
        }
        compiled.push(Node::String{text: text_node});
      },
      parser::Node::Word{children} => {
        let mut word = String::new();
        let mut result = self.compile_nodes(children);
        for node in result {
          match node {
            Node::Token{token, byte} => {
              let character = byte_to_alpha(byte).unwrap();
              word.push(character);
            },
            _ => (),
          }
        }
        compiled.push(Node::String{text: word});
      },
      parser::Node::Identifier{children} => {
        let mut word = String::new();
        let mut result = self.compile_nodes(children);
        for node in result {
          match node {
            Node::Token{token, byte} => {
              let character = byte_to_alpha(byte).unwrap();
              word.push(character);
            },
            _ => (),
          }
        }
        compiled.push(Node::String{text: word});
      },
      parser::Node::Repeat{children} => {
        compiled.append(&mut self.compile_nodes(children));
      },
      parser::Node::Alphanumeric{children} => {
        compiled.append(&mut self.compile_nodes(children));
      },
      parser::Node::Token{token, byte} => {
        compiled.push(Node::Token{token, byte});
      },
      _ => (),
    }
    
    //self.constraints = constraints.clone();
    compiled
  }

  pub fn compile_nodes(&mut self, nodes: Vec<parser::Node>) -> Vec<Node> {
    let mut compiled = Vec::new();
    for node in nodes {
      compiled.append(&mut self.build_syntax_tree(node));
    }
    compiled
  }

}

fn get_destination_register(constraint: &Constraint) -> Option<usize> {
  match constraint {
    Constraint::Identity{source, sink} => Some(*sink as usize),
    _ => None,
  }
}


fn byte_to_digit(byte: u8) -> Option<u64> {
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

fn byte_to_alpha(byte: u8) -> Option<char> {
  match byte {
    97 => Some('a'),
    98 => Some('b'),
    99 => Some('c'),
    100 => Some('d'),
    101 => Some('e'),
    102 => Some('f'),
    103 => Some('g'),
    104 => Some('h'),
    105 => Some('i'),
    106 => Some('j'),
    107 => Some('k'),
    108 => Some('l'),
    109 => Some('m'),
    110 => Some('n'),
    111 => Some('o'),
    112 => Some('p'),
    113 => Some('q'),
    114 => Some('r'),
    115 => Some('s'),
    116 => Some('t'),
    117 => Some('u'),
    118 => Some('v'),
    119 => Some('w'),
    120 => Some('x'),
    121 => Some('y'),    
    122 => Some('z'),
    65 => Some('A'),
    66 => Some('B'),
    67 => Some('C'),
    68 => Some('D'),
    69 => Some('E'),
    70 => Some('F'),
    71 => Some('G'),
    72 => Some('H'),
    73 => Some('I'),
    74 => Some('J'),
    75 => Some('K'),
    76 => Some('L'),
    77 => Some('M'),
    78 => Some('N'),
    79 => Some('O'),
    80 => Some('P'),
    81 => Some('Q'),
    82 => Some('R'),
    83 => Some('S'),
    84 => Some('T'),
    85 => Some('U'),
    86 => Some('V'),
    87 => Some('W'),
    88 => Some('X'),
    89 => Some('Y'),
    90 => Some('Z'),
    _ => None,
  }
}

/*

# Core

As the name suggests, Mech Core is the innermost element of the language. It's where all the computation in the mech langeuage occurs, and defines the semantics of the language. Let's take a look at the current design of Mech Core.

## Tables

Core stores data in a set of tables, which can be thought of comprising a database. Each table is a 2D array of cells, which can be Empty, or hold a Value. Values can be one of Number, String, Bool, or Table (so tables can hold tables).

Additionally, tables have a name, which is a global identifier used to reference the specific table. For example, a table of data about people might be called `#people`. Then, you have a global address to any piece of data in the system: `#people[2,5]` refers to row 2, column 5 in the `#people` table. This takes away some of the friction with naming; if you don't know what to name a variable, at least you always have an address for it.

[img]

If you would like to use names instead of addresses, Tables handle that as well. You can map any column or row to a name, and use that in code. Going back to the people example, if a column is supposed to represent an age of a person, you can label that column `age`. If a row is supposed to represent a specific person, you can label thqat row with their name. For example: `#people[corey, age]`. Labels for rows, columns, and tables follow [identifier naming rules]().

We can also slice and dice tables, by using indexing notation to select row and column ranges.

## Transactions and Changes

To add/remove data to/from a table, we bundle all the changes we want to make in a "transaction", in the database sense. We use a transaction model because evetually we would like to answer questions like "Who made this change?" or "When was this change made?" We would also like to be able to roll back changes we don't like, and avoid inconsistent states by rolling back partially applied transactions. 

Changes can apply to a specific cell, a range of cells, or to a whole table.

## Watchers


*/