// # Mech Syntax Compiler

// ## Preamble

/*use mech_core::{Value, Block, BlockState, ValueMethods, Transformation, TableIndex, TableId, Register, NumberLiteral, NumberLiteralKind};
use mech_core::{Quantity, QuantityMath};

use mech_core::{Error, ErrorType};*/
use mech_core::{hash_string, hash_chars, Block, Transformation, Table, TableId, TableIndex};

use crate::ast::{Ast, Node};
use crate::parser::Parser;
use crate::lexer::Token;
//use super::formatter::Formatter;

#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use hashbrown::hash_set::{HashSet};
use hashbrown::hash_map::{HashMap};
use std::sync::Arc;
use std::mem;

lazy_static! {
  static ref TABLE_COPY: u64 = hash_string("table/copy");
  static ref TABLE_HORZCAT: u64 = hash_string("table/horizontal-concatenate");
  static ref TABLE_VERTCAT: u64 = hash_string("table/vertical-concatenate");
  static ref TABLE_SET: u64 = hash_string("table/set");
  static ref TABLE_APPEND__ROW: u64 = hash_string("table/append-row");
  static ref TABLE_SPLIT: u64 = hash_string("table/split");
  static ref SET_ANY: u64 = hash_string("set/any");
}

pub struct Compiler {


}

impl Compiler {

  pub fn new() -> Compiler {
    Compiler {

    }
  }

  pub fn compile_transformations(&mut self, nodes: &Vec<Node>) -> Vec<Transformation> {
    let mut compiled = Vec::new();
    for node in nodes {
      let mut result = self.compile_transformation(node);
      compiled.append(&mut result);
    }
    compiled
  }

  pub fn compile_transformation(&mut self, input: &Node) -> Vec<Transformation> {
    let mut tfms = vec![];
    match input {
      Node::Identifier{name, id} => {
        tfms.push(Transformation::Identifier{name: name.to_vec(), id: *id});
      },
      Node::NumberLiteral{kind, bytes} => {
        let table_id = hash_string(&format!("{:?}", input));
        tfms.push(Transformation::NewTable{table_id: TableId::Local(table_id), rows: 1, columns: 1 });
        tfms.push(Transformation::NumberLiteral{kind: *kind, bytes: bytes.to_vec()});
      },
      Node::VariableDefine{children} => {
        let variable_name = match &children[0] {
          Node::Identifier{name,..} => {
            let name_hash = hash_chars(name);
            //self.strings.insert(name_hash, name.to_string());
            name_hash
          }
          _ => 0, // TODO Error
        };
        // Compile input of the variable define
        let mut input = self.compile_transformation(&children[1]);
        // If the first element is a reference, remove it
        match input[0] {
          Transformation::TableReference{..} => {
            input.remove(0);
          }
          _ => (),
        }
        let input_table_id = match input[0] {
          Transformation::NewTable{table_id,..} => {
            Some(table_id)
          }
          _ => None,
        };
        tfms.push(Transformation::TableAlias{table_id: input_table_id.unwrap(), alias: variable_name});
        tfms.append(&mut input);
      },
      Node::Function{name, children} => {
        let mut args: Vec<(u64, TableId, TableIndex, TableIndex)>  = vec![];
        let mut arg_tfms = vec![];
        for child in children {
          let arg: u64 = match child {
            Node::FunctionBinding{children} => {
              match &children[0] {
                Node::Identifier{name, id} => {
                  *id
                },
                _ => 0,
              }
            }
            _ => 0,
          };
          let mut result = self.compile_transformation(&child);
          match &result[0] {
            Transformation::NewTable{table_id,..} => {
              args.push((arg, *table_id, TableIndex::All, TableIndex::All));
            },
            Transformation::Select{table_id, indices, out} => {
              let (row, column) = indices[0];
              args.push((arg, *table_id, row, column));
            }
            /*Transformation::TableReference{table_id, reference} => {
              args.push((arg, TableId::Local(reference.as_reference().unwrap()), TableIndex::All, TableIndex::All));
            }*/
            _ => (),
          }
          arg_tfms.append(&mut result);
        }
        let name_hash = hash_chars(name);
        let id = hash_string(&format!("{:?}{:?}", name, arg_tfms));
        tfms.push(Transformation::NewTable{table_id: TableId::Local(id), rows: 1, columns: 1});
        tfms.push(Transformation::Function{
          name: name_hash,
          arguments: args,
          out: (TableId::Local(id), TableIndex::All, TableIndex::All),
        });
        tfms.append(&mut arg_tfms);
      },
      Node::SelectData{name, id, children} => {
        let mut indices = vec![];
        let mut all_indices = vec![];
        let mut local_tfms = vec![];
        for child in children {
          match child {
            Node::DotIndex{children} => {
              for child in children {
                match child {
                  Node::Null => {
                    indices.push(TableIndex::All);
                  }
                  Node::Identifier{name, id} => {
                    indices.push(TableIndex::Alias(*id));
                  }
                  Node::SubscriptIndex{children} => {
                    for child in children {
                      match child {
                        Node::SelectAll => {
                          indices.push(TableIndex::All);
                        }
                        Node::WheneverIndex{..} => {
                          let id = hash_string("~");
                          if indices.len() == 2 && indices[0] == TableIndex::All {
                            indices[0] = TableIndex::Table(TableId::Local(id));
                          } else {
                            indices.push(TableIndex::Table(TableId::Local(id)));
                          }
                        }
                        Node::SelectData{name, id, children} => {
                          if indices.len() == 2 && indices[0] == TableIndex::All {
                            indices[0] = TableIndex::Table(*id);
                          } else {
                            indices.push(TableIndex::Table(*id));
                          }
                        }
                        Node::Expression{..} => {
                          let mut result = self.compile_transformation(child);
                          match &result[1] {
                            /*Transformation::Constant{table_id, value, unit} => {
                              if indices.len() == 2 && indices[0] == TableIndex::All {
                                indices[0] = TableIndex::Index(value.as_u64().unwrap() as usize);
                              } else {
                                indices.push(TableIndex::Index(value.as_u64().unwrap() as usize));
                              }
                            }*/
                            Transformation::Function{name, arguments, out} => {
                              let (output_table_id, output_row, output_col) = out;
                              if indices.len() == 2 && indices[0] == TableIndex::All {
                                indices[0] = TableIndex::Table(*output_table_id);
                              } else {
                                indices.push(TableIndex::Table(*output_table_id));
                              }
                            }
                            _ => (),
                          }
                          local_tfms.append(&mut result);
                        }
                        _ => (),
                      }
                    }
                  }
                  _ => (),
                }
              }
            }
            Node::SubscriptIndex{children} => {
              for child in children {
                match child {
                  Node::SelectAll => {
                    indices.push(TableIndex::All);
                  }
                  Node::WheneverIndex{..} => {
                    let id = hash_string("~");
                    if indices.len() == 2 && indices[0] == TableIndex::All {
                      indices[0] = TableIndex::Table(TableId::Local(id));
                    } else {
                      indices.push(TableIndex::Table(TableId::Local(id)));
                    }
                  }
                  Node::SelectData{name, id, children} => {
                    if indices.len() == 2 && indices[0] == TableIndex::All {
                      indices[0] = TableIndex::Table(*id);
                    } else {
                      indices.push(TableIndex::Table(*id));
                    }
                  }
                  Node::Expression{..} => {
                    let mut result = self.compile_transformation(child);
                    match &result[1] {
                      /*Transformation::Constant{table_id, value, unit} => {
                        if indices.len() == 2 && indices[0] == TableIndex::All {
                          indices[0] = TableIndex::Index(value.as_u64().unwrap() as usize);
                        } else {
                          indices.push(TableIndex::Index(value.as_u64().unwrap() as usize));
                        }
                      }*/
                      Transformation::Function{name, arguments, out} => {
                        let (output_table_id, output_row, output_col) = out;
                        if indices.len() == 2 && indices[0] == TableIndex::All {
                          indices[0] = TableIndex::Table(*output_table_id);
                        } else {
                          indices.push(TableIndex::Table(*output_table_id));
                        }
                      }
                      _ => (),
                    }
                    local_tfms.append(&mut result);
                  }
                  _ => (),
                }
              }
              if children.len() == 1 {
                indices.push(TableIndex::None);
              }
            }
            _ => {
              indices.push(TableIndex::All);
              indices.push(TableIndex::All);
            },
          }
          if indices.len() == 2 {
            all_indices.push((indices[0],indices[1]));
            indices.clear();
          }
        }
        //all_indices.reverse();
        let out_id = hash_string(&format!("{:?}{:?}", *id, all_indices));
        if all_indices.len() > 1 {
          tfms.push(Transformation::NewTable{table_id: TableId::Local(out_id), rows: 1, columns: 1});
          tfms.push(Transformation::Select{table_id: *id, indices: all_indices, out: TableId::Local(out_id)});
        } else {
          tfms.push(Transformation::Select{table_id: *id, indices: all_indices, out: TableId::Local(out_id)});
        }
        tfms.append(&mut local_tfms);
      }
      Node::Program{children, ..} |
      Node::Section{children, ..} |
      Node::Transformation{children} |
      Node::Statement{children} |
      Node::Block{children} |
      Node::MathExpression{children} |
      Node::Expression{children} |
      Node::Root{children} => {
        let mut result = self.compile_transformations(children);
        tfms.append(&mut result);
      }
      _ => (),
    }
    tfms
  }



}


/*
// ## Program

// Define a program struct that has everything we need to render a mech program.

#[derive(Clone)]
pub struct Program {
  pub title: Option<String>,
  pub sections: Vec<Section>,
  pub blocks: Vec<Block>,
}

impl fmt::Debug for Program {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Program: {}\n", self.title.clone().unwrap_or("".to_string())).ok();
    for section in &self.sections {
      write!(f, "  {:?}\n", section).ok();
    }
    Ok(())
  }
}

#[derive(Clone, PartialEq)]
pub struct Section {
  pub title: Option<String>,
  pub elements: Vec<Element>,
}

impl fmt::Debug for Section {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Section: {}\n", self.title.clone().unwrap_or("".to_string())).ok();
    for element in &self.elements {
      write!(f, "    {:?}\n", element).ok();
    }
    Ok(())
  }
}

#[derive(Clone, PartialEq)]
pub enum Element {
  Block((u64, Node)),
  List(Node),
  CodeBlock(Node),
  Paragraph(Node),
}

impl fmt::Debug for Element {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Element::Paragraph(node) => write!(f, "Paragraph: {:?}", node),
      Element::List(node) => write!(f, "List: {:?}", node),
      Element::CodeBlock(node) => write!(f, "CodeBlock: {:?}", node),
      Element::Block((block_id, node)) => write!(f, "  Block({:#x})", block_id),
    };
    Ok(())
  }
}
*/
// ## Compiler
/*
#[derive(Debug)]
pub struct Compiler {
  pub blocks: Vec<Block>,
  pub programs: Vec<Program>,
  pub transformations: Vec<Transformation>,
  depth: usize,
  row: usize,
  column: usize,
  element: usize,
  table: u64,
  expression: usize,
  pub text: String,
  pub strings: HashMap<u64, String>,
  pub number_literals: HashMap<u64, NumberLiteral>,
  pub variable_names: HashSet<u64>,
  pub parse_tree: parser::Node,
  pub syntax_tree: Node,
  pub node_stack: Vec<Node>,
  pub section: usize,
  pub program: usize,
  pub block: usize,
  pub current_char: usize,
  pub current_line: usize,
  pub current_col: usize,
  pub errors: Vec<Error>,
  pub unparsed: String,
}

impl Compiler {

  pub fn new() -> Compiler {
    Compiler {
      blocks: Vec::new(),
      programs: Vec::new(),
      transformations: Vec::new(),
      node_stack: Vec::new(),
      depth: 0,
      expression: 0,
      column: 0,
      row: 0,
      table: 0,
      element: 0,
      section: 1,
      program: 1,
      block: 1,
      current_char: 0,
      current_line: 1,
      current_col: 1,
      strings: HashMap::new(),
      number_literals: HashMap::new(),
      unparsed: String::new(),
      text: String::new(),
      variable_names: HashSet::new(),
      parse_tree: parser::Node::Root{ children: Vec::new() },
      syntax_tree: Node::Root{ children: Vec::new() },
      errors: Vec::new(),
    }
  }

  pub fn clear(&mut self) {
    self.blocks.clear();
    self.programs.clear();
    self.transformations.clear();
    self.node_stack.clear();
    self.depth = 0;
    self.expression = 0;
    self.column = 0;
    self.row = 0;
    self.element = 0;
    self.table = 0;
    self.section = 1;
    self.program = 1;
    self.block = 1;
    self.strings.clear();
    self.current_char = 0;
    self.current_line = 1;
    self.current_col = 1;
    self.text = String::new();
    self.parse_tree = parser::Node::Root{ children: Vec::new() };
    self.syntax_tree = Node::Root{ children: Vec::new() };
    self.errors.clear();
  }

  pub fn compile_string(&mut self, input: &String) -> Vec<Program> {
    let mut parser = Parser::new();
    let mut ast = Ast::new();
    parser.parse(input);
    self.unparsed = parser.unparsed;
    self.parse_tree = parser.parse_tree.clone();
    let syntax_tree = ast.build_syntax_tree(&parser.parse_tree);
    self.syntax_tree = syntax_tree.clone();
    let programs = self.compile(syntax_tree);
    self.programs = programs.clone();
    programs
  }

  pub fn compile_block_string(&mut self, input: &String) -> Node {
    let mut parser = Parser::new();
    let mut ast = Ast::new();
    parser.parse_block(input);
    self.unparsed = parser.unparsed;
    self.parse_tree = parser.parse_tree.clone();
    let syntax_tree = ast.build_syntax_tree(&parser.parse_tree);
    self.syntax_tree = syntax_tree[0];
    self.syntax_tree.clone()
  }

  pub fn compile_fragment_string(&mut self, input: &String) -> Node {
    let mut parser = Parser::new();
    let mut ast = Ast::new();
    parser.parse_fragment(input);
    self.unparsed = parser.unparsed;
    self.parse_tree = parser.parse_tree.clone();
    let syntax_tree = ast.build_syntax_tree(&parser.parse_tree);
    self.syntax_tree = syntax_tree;
    self.syntax_tree.clone()
  }

  pub fn compile(&mut self, input: Node) -> Vec<Program> {
    let mut programs = Vec::new();
    match input {
      Node::Root{children} => {
        for child in children {
          match child {
            Node::Program{..} => programs.push(self.compile_program(child).unwrap()),
            Node::Fragment{..} => programs.push(self.compile_fragment(child).unwrap()),
            _ => (),
          };
        }
      },
      _ => (),
    };
    programs
  }

  pub fn compile_fragment(&mut self, input: Node) -> Option<Program> {
    let block = self.compile_block(&input).unwrap();
    let program = Program{title: None, sections: vec![
      Section {title: None, elements: vec![Element::Block(block)]}
    ], blocks: self.blocks.clone()};
    self.blocks.clear();
    self.program += 1;
    self.section = 1;
    Some(program)
  }

  pub fn compile_paragraph(&mut self, input: Node) -> Option<Node> {
    let result = Some(input.clone());
    match input {
      Node::Paragraph{children}  => {
        for child in &children {
          match child {
            Node::InlineMechCode{children} => {
              self.element += 1;
              self.expression += 1;
              let mut formatter = Formatter::new();
              let name = formatter.format(&children[0], false);
              let name = format!("mech/inline/{}", hash_string(&name));
              let id = hash_string(&name);
              let block_tree = Node::Block{children: vec![
                            Node::Transformation{children: vec![
                              Node::Statement{children: vec![
                                Node::TableDefine{children: vec![
                                  Node::Table{name, id},
                                  Node::Expression{children: vec![
                                    Node::AnonymousTableDefine{children: vec![
                                      Node::TableRow{children: vec![
                                        Node::TableColumn{children: vec![
                                          children[0].clone()]}]}]}]}]}]}]}]};
              let block = self.compile_block(&block_tree);
            }
            _ => (),
          }
        }
      }
      _ => (),
    }
    result
  }

  pub fn compile_unordered_list(&mut self, input: Node) -> Option<Node> {
    let result = Some(input.clone());
    match input {
      Node::UnorderedList{children}  => {
        for child in &children {
          match child {
            Node::ListItem{children} => {
              self.compile_paragraph(children[0].clone());
            }
            _ => (),
          }
        }
      }
      _ => (),
    }
    result
  }

  pub fn compile_program(&mut self, input: Node) -> Option<Program> {
    let program = match input {
      Node::Program{title, children} => {
        let mut sections = vec![];
        for child in children {
          match self.compile_section(child) {
            Some(section) => sections.push(section),
            _ => (),
          };
        }
        let program = Program{title, sections, blocks: self.blocks.clone()};
        self.blocks.clear();
        Some(program)
      },
      _ => None,
    };
    self.program += 1;
    self.section = 1;
    program
  }

  pub fn compile_section(&mut self, input: Node) -> Option<Section> {
    let section = match input {
      Node::Section{title, children} => {
        let mut elements = vec![];
        for child in children {
          match self.compile_element(child) {
            Some(element) => elements.push(element),
            _ => (),
          };
        }
        let section = Section{title, elements};
        Some(section)
      },
      _ => None,
    };
    self.section += 1;
    self.block = 1;
    section
  }

  pub fn compile_element(&mut self, input: Node) -> Option<Element> {
    self.element += 1;
    let element = match input {
      Node::Paragraph{..} => Some(Element::Paragraph(self.compile_paragraph(input).unwrap())),
      Node::UnorderedList{..} => Some(Element::List(self.compile_unordered_list(input).unwrap())),
      Node::Block{..} => Some(Element::Block(self.compile_block(&input).unwrap())),
      Node::CodeBlock{..} => Some(Element::CodeBlock(input)),
      Node::MechCodeBlock{ref children} => {
        let (block_id, node) = self.compile_block(&children[1]).unwrap();
        // set the block's state based on the provided flag
        match children[0] {
          Node::String{ref text} => {
            match text.as_ref() {
              "disabled" => self.blocks.last_mut().unwrap().state = BlockState::Disabled,
              _ => (),
            }
          }
          _ => (),
        }
        Some(Element::Block((block_id, node)))
      },
      _ => None,
    };
    element
  }

  pub fn compile_block(&mut self, node: &Node) -> Option<(u64, Node)> {
    match node {
      Node::Fragment{children} |
      Node::Block{children} => {

        let mut block = Block::new(100);
        let mut formatter = Formatter::new();
        block.text = formatter.format(&node, false);
        block.id = hash_string(&block.text);
        block.name = format!("{:?},{:?},{:?}", self.program, self.section, self.block);
        self.block += 1;
        let mut transformations: Vec<Transformation> = Vec::new();
        let mut plan: Vec<(String, HashSet<u64>, HashSet<u64>, Vec<Transformation>)> = Vec::new();
        let mut unsatisfied_transformations: Vec<(String, HashSet<u64>, HashSet<u64>, Vec<Transformation>)> = Vec::new();
        let mut block_produced: HashSet<u64> = HashSet::new();
        let mut block_consumed: HashSet<u64> = HashSet::new();
        let mut aliases: HashSet<u64> = HashSet::new();
        
        for transformation in block_transformations {
          block.register_transformations(transformation);
        }

        for (step_text, unsatisfied_produces, unsatisfied_consumes, step_transformations) in unsatisfied_transformations {
          let union: HashSet<u64> = block_produced.union(&unsatisfied_produces).cloned().collect();
          let unsatisfied: HashSet<u64> = unsatisfied_consumes.difference(&union).cloned().collect();
          block.errors.insert(Error {
            block_id: block.id,
            step_text: step_text,
            error_type: ErrorType::UnsatisfiedTransformation(
              unsatisfied.iter().map(|x| x.clone()).collect::<Vec<u64>>(),
            ),
          });
        }
        //block.id = block.gen_block_id();
        for (k,v) in self.strings.drain() {
          let store = unsafe{&mut *Arc::get_mut_unchecked(&mut block.store)};
          store.strings.insert(k,v.to_string());
        }
        for (k,v) in self.number_literals.drain() {
          let store = unsafe{&mut *Arc::get_mut_unchecked(&mut block.store)};
          store.number_literals.insert(k,v.clone());
        }
        for err in self.errors.drain(..) {
          block.errors.insert(err);
        }
        self.variable_names.clear();
        self.blocks.push(block.clone());
        Some((block.id, node.clone()))
      },
      _ => None,
    }
  }

  pub fn compile_transformation(&mut self, node: &Node) -> Vec<Transformation> {
    let mut transformations: Vec<Transformation> = Vec::new();
    /*
    match node {
      // An inline table is like x = [a: 1, b: 2, c :3]
      Node::InlineTable{children} => {
        let table = self.table;
        let column = self.column;
        self.column = 1;
        self.table = hash_string(&format!("{:?}",children));
        let mut args = vec![];
        let mut tfms = vec![];
        let table_reference_id = TableId::Local(hash_string(&format!("InlineTable{:?}", children)));
        let new_table_id = TableId::Local(hash_string(&format!("InlineTableHorzcatResult{:?}", children)));
        self.table = *new_table_id.unwrap();
        for child in children {
          let mut result = self.compile_transformation(child);
          match &result[0] {
            Transformation::TableReference{table_id,..} => {
              args.push((0, *table_id, TableIndex::All, TableIndex::All));
            }
            Transformation::NewTable{table_id,..} => {
              args.push((0, *table_id, TableIndex::All, TableIndex::All)); 
            }
            Transformation::Select{table_id, row, column, indices, out} => {
              let (row_index, column_index) = &indices[0];
              args.push((0, *table_id, *row_index, *column_index)); 
            }
            _ => (),
          }
          tfms.append(&mut result);
        }
        // Join all of the columns together using table/horizontal-concatenate.
        let fxn = Transformation::Function {
          name: *TABLE_HORZCAT,
          arguments: args,
          out: (new_table_id, TableIndex::All, TableIndex::All),
        };
        // Push a reference so any upstream nodes know what to do
        transformations.push(Transformation::TableReference{table_id: table_reference_id, reference: Value::from_id(*new_table_id.unwrap())});
        // Push the horzcat function
        transformations.push(Transformation::NewTable{table_id: new_table_id, rows: 1, columns: 1});
        transformations.push(fxn);
        // Push the transformations derived from the children
        transformations.append(&mut tfms);
        self.table = table;
        self.column = column;
      }
      Node::AnonymousTableDefine{children} => {
        let mut args = vec![];
        let mut tfms = vec![];
        let table = self.table;
        let column = self.column;
        self.column = 0;
        let table_reference_id = TableId::Local(hash_string(&format!("AnonymousTable{:?}", children)));
        let new_table_id = TableId::Local(hash_string(&format!("AnonymousTableVertcatResult{:?}", children)));
        self.table = *new_table_id.unwrap();
        // Compile each row of the table
        for child in children {
          let mut result = self.compile_transformation(child);
          // The first result is the table to which vertcat writes
          match &result[0] {
            Transformation::TableReference{table_id,..} => {
              args.push((0, *table_id, TableIndex::All, TableIndex::All));
            }
            Transformation::NewTable{table_id,..} => {
              args.push((0, *table_id, TableIndex::All, TableIndex::All));
            }
            Transformation::Select{table_id, row, column, indices, out} => {
              let (row_index, column_index) = &indices[0];
              args.push((0, *table_id, *row_index, *column_index)); 
            }
            _ => (),
          }
          tfms.append(&mut result);
        }
        // Join all of the rows together using table/vertical-concatenate.
        if args.len() > 0 {        
          let fxn = Transformation::Function {
            name: *TABLE_VERTCAT,
            arguments: args,
            out: (new_table_id, TableIndex::All, TableIndex::All),
          };
          // Push a reference so any upstream nodes know what to do
          transformations.push(Transformation::TableReference{table_id: table_reference_id, reference: Value::from_id(*new_table_id.unwrap())});
          // Push the vertcat function
          transformations.push(Transformation::NewTable{table_id: new_table_id, rows: 1, columns: 1});
          transformations.push(fxn);
        } else {
          // Push a new table with 0 rows if there are no arguments for the fxn
          transformations.push(Transformation::NewTable{table_id: new_table_id, rows: 0, columns: self.column});
        }
        // Push the rest of the transformations
        transformations.append(&mut tfms);
        self.table = table;
        self.column = column;
      }
      Node::TableRow{children} => {
        let mut args = vec![];
        let mut tfms = vec![];
        self.row += 1;
        let new_table_id = TableId::Local(hash_string(&format!("{:?}{:?}", self.row, children)));
        // Compile each column of the table
        for child in children {
          let mut result = self.compile_transformation(child);
          match &result[0] {
            Transformation::TableReference{table_id,..} => {
              args.push((0, *table_id, TableIndex::All, TableIndex::All));
            }
            Transformation::NewTable{table_id,..} => {
              args.push((0, *table_id, TableIndex::All, TableIndex::All));
            }
            Transformation::Select{table_id, row, column, indices, out} => {
              let (row_index, column_index) = &indices[0];
              args.push((0, *table_id, *row_index, *column_index)); 
            }
            _ => (),
          }
          tfms.append(&mut result);
        }
        // Join all of the columns together using table/horizontal-concatenate.
        if args.len() > 1 {
          let fxn = Transformation::Function {
            name: *TABLE_HORZCAT,
            arguments: args,
            out: (new_table_id, TableIndex::All, TableIndex::All),
          };
          transformations.push(Transformation::NewTable{table_id: new_table_id, rows: 1, columns: 1});
          transformations.push(fxn);
        }
        transformations.append(&mut tfms);
      }
      Node::TableColumn{children} => {
        let mut result = self.compile_transformations(children);
        transformations.append(&mut result);
      }
      Node::TableHeader{children} => {
        for child in children {
          self.column += 1;
          let mut result = self.compile_transformation(child);
          transformations.append(&mut result);
        }
      }
      Node::Attribute{children} => {
        for child in children {
          match child {
            Node::Identifier{name, id} => {
              transformations.push(Transformation::ColumnAlias{table_id: TableId::Local(self.table), column_ix: self.column, column_alias: *id});
            }
            _ => (),
          }
        }
        let mut result = self.compile_transformations(children);
        transformations.append(&mut result);
      }
      Node::Binding{children} => {
        let mut tfms = vec![];
        match &children[0] {
          Node::Identifier{name, id} => {
            self.strings.insert(hash_string(&name.to_string()), name.to_string());
            tfms.push(Transformation::ColumnAlias{table_id: TableId::Local(self.table), column_ix: self.column, column_alias: hash_string(&name.to_string())});
            self.column += 1;
          }
          _ => (),
        }
        let mut result = self.compile_transformation(&children[1]);
        transformations.append(&mut result);
        transformations.append(&mut tfms);
      }
      Node::Identifier{name, id} => {
        self.strings.insert(hash_string(&name.to_string()), name.to_string());
      }
      Node::Transformation{children} => {
        let mut result = self.compile_transformations(children);
        transformations.append(&mut result);
      }
      Node::SplitData{children} => {
        let (output_table_tfm, output_table_id) = match &children[0] {
          Node::Identifier{name,..} => {
            let name_hash = hash_string(name);
            self.strings.insert(name_hash, name.to_string());
            (Transformation::NewTable{table_id: TableId::Local(name_hash), rows: 1, columns: 1},
            TableId::Local(name_hash))
          }
          Node::Table{name, ..} => {
            let name_hash = hash_string(name);
            self.strings.insert(name_hash, name.to_string());
            (Transformation::NewTable{table_id: TableId::Global(name_hash), rows: 1, columns: 1},
            TableId::Global(name_hash))
          },
          _ => (Transformation::NewTable{table_id: TableId::Local(0), rows: 1, columns: 1},TableId::Local(0)),
        };

        let mut input = self.compile_transformation(&children[1]);

        let input_table_id = match input[0] {
          Transformation::TableReference{table_id,reference} => {
            Some(TableId::Local(reference.as_reference().unwrap()))
          }
          Transformation::NewTable{table_id,..} => {
            Some(table_id)
          }
          Transformation::Select{table_id,..} => {
            Some(table_id)
          }
          _ => None,
        };

        let fxn = Transformation::Function{
          name: *TABLE_SPLIT,
          arguments: vec![
            (0, input_table_id.unwrap(), TableIndex::All, TableIndex::All)
          ],
          out: (output_table_id, TableIndex::All, TableIndex::All),
        };
        // Push a reference so any upstream nodes know what to do
        transformations.push(output_table_tfm);
        transformations.push(fxn);
        transformations.append(&mut input);
      }
      Node::AddRow{children} => {
        let mut output = self.compile_transformation(&children[0]);

        let mut output_tup = match output[0] {
          Transformation::NewTable{table_id, ..} => {
            let tfm = Transformation::Set{table_id, row: TableIndex::All, column: TableIndex::All};
            transformations.push(tfm);
            Some((table_id,TableIndex::All,TableIndex::All))
          }
          _ => None,
        };

        let mut input = self.compile_transformation(&children[1]);
        let mut args = vec![];
        match &input[0] {
          Transformation::Select{table_id, row, column, indices, out} => {
            let (row,column) = indices[0];
            args.push((0, *table_id, row, column));
          }
          Transformation::NewTable{table_id,..} => {
            args.push((0, *table_id, TableIndex::All, TableIndex::All));
          }
          Transformation::TableReference{table_id, reference} => {
            args.push((0, TableId::Local(reference.as_reference().unwrap()),TableIndex::All, TableIndex::All));
          }
          _ => (),
        }

        let (output_table_id, output_row, output_col) = output_tup.unwrap();

        let fxn = Transformation::Function{
          name: *TABLE_APPEND__ROW,
          arguments: args,
          out: (output_table_id, output_row, output_col),
        };
        transformations.push(fxn);
        transformations.append(&mut input);
      }
      Node::Whenever{children} => {
        let mut result = self.compile_transformations(children);
        match &result[0] {
          Transformation::Select{table_id, row, column, indices, out} => {
            let (row, column) = indices[0];
            let register = Register{table_id: *table_id, row: row, column: column};
            transformations.push(
              Transformation::Whenever{table_id: *table_id, row: row, column: column, registers: vec![register]},
            );
          }
          Transformation::NewTable{table_id, ..} => {
            let mut registers: Vec<Register> = vec![];
            for r in &result {
              match r {
                Transformation::Select{table_id,row,column,indices, out} => {
                  let (row, column) = indices[0];
                  let register = Register{table_id: *table_id, row: row, column: column};
                  registers.push(register);
                }
                _ => (),
              }
            }
            transformations.push(Transformation::Whenever{table_id: *table_id, row: TableIndex::All, column: TableIndex::All, registers});
          }
          _ => (),
        }
        transformations.append(&mut result);
      }
      Node::Statement{children} => {
        let mut result = self.compile_transformations(children);
        transformations.append(&mut result);
      }
      Node::SetData{children} => {
        let mut output = self.compile_transformation(&children[0]);
        let mut output_tup = match &output[0] {
          Transformation::Select{table_id, row, column, indices, out} => {
            let (row, column) = indices[0];
            let tfm = Transformation::Set{table_id: *table_id, row: row, column: column};
            transformations.push(tfm);
            Some((table_id.clone(),row.clone(),column.clone()))
          }
          _ => None,
        };
        let mut output_tup2 = if output.len() > 1 {
          match &output[1] {
            Transformation::Select{table_id, row, column, indices, out} => {
              let (row, column) = indices[0];
              let tfm = Transformation::Set{table_id: *table_id, row: row, column: column};
              transformations.push(tfm);
              Some((table_id.clone(),row.clone(),column.clone()))
            }
            _ => None,
          }
        } else {
          None
        };

        let mut input = self.compile_transformation(&children[1]);

        let (input_table_id, row_select, column_select) = match &input[0] {
          Transformation::Select{table_id, row, column, indices, out} => {
            let (row, column) = indices[0];
            (*table_id, row, column)
          }
          Transformation::NewTable{table_id,..} => {
            (*table_id, TableIndex::All, TableIndex::All)
          }
          Transformation::TableReference{table_id, reference} => {
            (TableId::Local(reference.as_reference().unwrap()), TableIndex::All, TableIndex::All)
          }
          _ => (TableId::Local(0), TableIndex::All, TableIndex::All) // TODO This is an error really
        };

        let (output_table_id, output_row, output_col) = match (output_tup, output_tup2) {
          (Some((table,row,col)), Some((_,row2,col2))) => {
            output.remove(0);
            output.remove(0);
            (table,row2,col)
          },
          (Some(a),_) => {
            output.remove(0);
            a
          },
          _ => (TableId::Global(0),TableIndex::All,TableIndex::All),
        };

        let fxn = Transformation::Function{
          name: *TABLE_SET,
          arguments: vec![
            (0, input_table_id, row_select, column_select)
          ],
          out: (output_table_id, output_row, output_col),
        };
        transformations.push(fxn);
        transformations.append(&mut input);
        transformations.append(&mut output);
      }
      Node::SelectData{name, id, children} => {
        self.strings.insert(*id.unwrap(), name.to_string());
        let mut indices = vec![];
        let mut all_indices = vec![];
        let mut tfms = vec![];
        for child in children {
          match child {
            Node::DotIndex{children} => {
              for child in children {
                match child {
                  Node::Null => {
                    indices.push(TableIndex::All);
                  }
                  Node::Identifier{name, id} => {
                    self.strings.insert(hash_string(&name.to_string()), name.to_string());
                    indices.push(TableIndex::Alias(*id));
                  }
                  Node::SubscriptIndex{children} => {
                    for child in children {
                      match child {
                        Node::SelectAll => {
                          indices.push(TableIndex::All);
                        }
                        Node::WheneverIndex{..} => {
                          let id = hash_string("~");
                          self.strings.insert(id, "~".to_string());
                          if indices.len() == 2 && indices[0] == TableIndex::All {
                            indices[0] = TableIndex::Table(TableId::Local(id));
                          } else {
                            indices.push(TableIndex::Table(TableId::Local(id)));
                          }
                        }
                        Node::SelectData{name, id, children} => {
                          self.strings.insert(*id.unwrap(), name.to_string());
                          if indices.len() == 2 && indices[0] == TableIndex::All {
                            indices[0] = TableIndex::Table(*id);
                          } else {
                            indices.push(TableIndex::Table(*id));
                          }
                        }
                        Node::Expression{..} => {
                          let mut result = self.compile_transformation(child);
                          match &result[1] {
                            Transformation::Constant{table_id, value, unit} => {
                              if indices.len() == 2 && indices[0] == TableIndex::All {
                                indices[0] = TableIndex::Index(value.as_u64().unwrap() as usize);
                              } else {
                                indices.push(TableIndex::Index(value.as_u64().unwrap() as usize));
                              }
                            }
                            Transformation::Function{name, arguments, out} => {
                              let (output_table_id, output_row, output_col) = out;
                              if indices.len() == 2 && indices[0] == TableIndex::All {
                                indices[0] = TableIndex::Table(*output_table_id);
                              } else {
                                indices.push(TableIndex::Table(*output_table_id));
                              }
                            }
                            _ => (),
                          }
                          tfms.append(&mut result);
                        }
                        _ => (),
                      }
                    }
                  }
                  _ => (),
                }
              }
            }
            Node::SubscriptIndex{children} => {
              for child in children {
                match child {
                  Node::SelectAll => {
                    indices.push(TableIndex::All);
                  }
                  Node::WheneverIndex{..} => {
                    let id = hash_string("~");
                    self.strings.insert(id, "~".to_string());
                    if indices.len() == 2 && indices[0] == TableIndex::All {
                      indices[0] = TableIndex::Table(TableId::Local(id));
                    } else {
                      indices.push(TableIndex::Table(TableId::Local(id)));
                    }
                  }
                  Node::SelectData{name, id, children} => {
                    self.strings.insert(*id.unwrap(), name.to_string());
                    if indices.len() == 2 && indices[0] == TableIndex::All {
                      indices[0] = TableIndex::Table(*id);
                    } else {
                      indices.push(TableIndex::Table(*id));
                    }
                  }
                  Node::Expression{..} => {
                    let mut result = self.compile_transformation(child);
                    match &result[1] {
                      Transformation::Constant{table_id, value, unit} => {
                        if indices.len() == 2 && indices[0] == TableIndex::All {
                          indices[0] = TableIndex::Index(value.as_u64().unwrap() as usize);
                        } else {
                          indices.push(TableIndex::Index(value.as_u64().unwrap() as usize));
                        }
                      }
                      Transformation::Function{name, arguments, out} => {
                        let (output_table_id, output_row, output_col) = out;
                        if indices.len() == 2 && indices[0] == TableIndex::All {
                          indices[0] = TableIndex::Table(*output_table_id);
                        } else {
                          indices.push(TableIndex::Table(*output_table_id));
                        }
                      }
                      _ => (),
                    }
                    tfms.append(&mut result);
                  }
                  _ => (),
                }
              }
              if children.len() == 1 {
                indices.push(TableIndex::None);
              }
            }
            _ => {
              indices.push(TableIndex::All);
              indices.push(TableIndex::All);
            },
          }
          if indices.len() == 2 {
            all_indices.push((indices[0],indices[1]));
            indices.clear();
          }
        }
        //all_indices.reverse();
        let out_id = hash_string(&format!("{:?}{:?}", *id, all_indices));
        if all_indices.len() > 1 {
          transformations.push(Transformation::NewTable{table_id: TableId::Local(out_id), rows: 1, columns: 1});
          transformations.push(Transformation::Select{table_id: *id, row: TableIndex::None, column: TableIndex::None, indices: all_indices, out: TableId::Local(out_id)});
        } else {
          transformations.push(Transformation::Select{table_id: *id, row: TableIndex::None, column: TableIndex::None, indices: all_indices, out: TableId::Local(out_id)});
        }
        transformations.append(&mut tfms);
      }
      Node::VariableDefine{children} => {
        let variable_name = match &children[0] {
          Node::Identifier{name,..} => {
            let name_hash = hash_string(name);
            self.strings.insert(name_hash, name.to_string());
            name_hash
          }
          _ => 0, // TODO Error
        };
        // Compile input of the variable define
        let mut input = self.compile_transformation(&children[1]);
        // If the first element is a reference, remove it
        match input[0] {
          Transformation::TableReference{..} => {
            input.remove(0);
          }
          _ => (),
        }
        let input_table_id = match input[0] {
          Transformation::NewTable{table_id,..} => {
            Some(table_id)
          }
          _ => None,
        };
        transformations.push(Transformation::TableAlias{table_id: input_table_id.unwrap(), alias: variable_name});
        transformations.append(&mut input);
      }
      Node::TableDefine{children} => {
        let mut output = self.compile_transformation(&children[0]);

        let mut nt_rows = 1;
        let mut nt_columns = 1;

        // Get the output table id
        let output_table_id = match output[0] {
          Transformation::NewTable{table_id,..} => {
            Some(table_id)
          },
          _ => None,
        };

        // Rewrite input rows
        let mut input = self.compile_transformation(&children[1]);

        // If the first element is a reference, remove it
        match input[0] {
          Transformation::TableReference{..} => {
            input.remove(0);
          }
          _ => (),
        }
        let input_table_id = match input[0] {
          Transformation::NewTable{table_id,..} => {
            Some(table_id)
          },
          _ => None,
        };

        let mut input_tfms = vec![];

        // Transform all the inputs
        for tfm in input {
          match tfm {
            Transformation::NewTable{table_id,rows,columns} => {
              if table_id == input_table_id.unwrap() {
                input_tfms.push(Transformation::NewTable{table_id: output_table_id.unwrap(), rows, columns});
              } else {
                input_tfms.push(tfm);
              }
            }
            Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
              if table_id == input_table_id.unwrap() {
                input_tfms.push(Transformation::ColumnAlias{table_id: output_table_id.unwrap(), column_ix, column_alias});
              } else {
                input_tfms.push(tfm);
              }
            }
            Transformation::Function{name, ref arguments, out} => {
              let (out_table, out_rows, out_columns) = out;
              if out_table == input_table_id.unwrap() {
                input_tfms.push(Transformation::Function{name, arguments: arguments.clone(), out: (output_table_id.unwrap(), out_rows, out_columns)});
              } else {
                input_tfms.push(tfm);
              }
            }
            Transformation::Constant{table_id, value, unit} => {
              if table_id == input_table_id.unwrap() {
                input_tfms.push(Transformation::Constant{table_id: output_table_id.unwrap(), value, unit});
              } else {
                input_tfms.push(tfm);
              }
            }
            _ => input_tfms.push(tfm),
          };
        }
        transformations.append(&mut input_tfms);
      }
      Node::Table{name, id} => {
        self.strings.insert(*id, name.to_string());
        transformations.push(Transformation::NewTable{table_id: TableId::Global(*id), rows: 1, columns: 1});
      }
      Node::Expression{children} => {
        let mut result = self.compile_transformations(children);
        transformations.append(&mut result);
      }
      Node::RationalNumber{children} => {
        let mut result = self.compile_transformations(children);
        transformations.append(&mut result);
      }
      Node::MathExpression{children} => {
        let mut result = self.compile_transformations(children);
        transformations.append(&mut result);
      }
      Node::FunctionBinding{children} => {
        let mut result = self.compile_transformations(children);
        transformations.append(&mut result);
      }
      Node::Function{name, children} => {
        let mut args: Vec<(u64, TableId, TableIndex, TableIndex)>  = vec![];
        let mut arg_tfms = vec![];
        for child in children {
          let arg: u64 = match child {
            Node::FunctionBinding{children} => {
              match &children[0] {
                Node::Identifier{name, id} => {
                  self.strings.insert(*id, name.to_string());
                  *id
                },
                _ => 0,
              }
            }
            _ => 0,
          };
          let mut result = self.compile_transformation(&child);
          match &result[0] {
            Transformation::NewTable{table_id,..} => {
              args.push((arg, *table_id, TableIndex::All, TableIndex::All));
            },
            Transformation::Select{table_id, row, column, indices, out} => {
              let (row, column) = indices[0];
              args.push((arg, *table_id, row, column));
            }
            Transformation::TableReference{table_id, reference} => {
              args.push((arg, TableId::Local(reference.as_reference().unwrap()), TableIndex::All, TableIndex::All));
            }
            _ => (),
          }
          arg_tfms.append(&mut result);
        }
        let name_hash = hash_string(name);
        self.strings.insert(name_hash,name.to_string());
        let id = hash_string(&format!("{:?}{:?}", name, arg_tfms));
        transformations.push(Transformation::NewTable{table_id: TableId::Local(id), rows: 1, columns: 1});
        transformations.push(Transformation::Function{
          name: name_hash,
          arguments: args,
          out: (TableId::Local(id), TableIndex::All, TableIndex::All),
        });
        transformations.append(&mut arg_tfms);
      }
      Node::String{text} => {
        let table = hash_string(&format!("String-{:?}", text));
        transformations.push(Transformation::NewTable{table_id: TableId::Local(table), rows: 1, columns: 1});
        let value = Value::from_string(&text.to_string());
        self.strings.insert(value, text.to_string());
        transformations.push(Transformation::Constant{table_id: TableId::Local(table), value, unit: 0});
      }
      Node::NumberLiteral{kind, bytes} => {
        let table = hash_string(&format!("NumberLiteral-{:?}{:?}", kind, bytes));
        transformations.push(Transformation::NewTable{table_id: TableId::Local(table), rows: 1, columns: 1});
        let number_literal = match kind {
          NumberLiteralKind::Hexadecimal => {
            let mut new_bytes = vec![];
            if bytes.len() == 1 {
              new_bytes = bytes.to_vec();
            } else {
              for i in (0..=bytes.len()).step_by(2) {
                let b1 = if i == bytes.len() {
                  break;
                } else {
                  bytes[i]
                };
                let b2 = if i + 1 == bytes.len() {
                  0
                } else {
                  bytes[i + 1]
                };
                new_bytes.push(b1 << 4 | b2);
              }
            }
            NumberLiteral{kind: *kind, bytes: new_bytes}
          }
          _ => NumberLiteral{kind: *kind, bytes: bytes.clone()}, // TODO Do the right byte representation for the rest of the number literal kinds
        };
        let value = Value::from_number_literal(&number_literal);
        if number_literal.bytes.len() > 7 {
          self.number_literals.insert(value, number_literal);
        } 
        transformations.push(Transformation::Constant{table_id: TableId::Local(table), value, unit: 0});
      }
      Node::Empty => {
        let value = Value::empty();
        let table = hash_string(&format!("Empty-{:?}", value.to_f32()));
        transformations.push(Transformation::NewTable{table_id: TableId::Local(table), rows: 1, columns: 1});
        transformations.push(Transformation::Constant{table_id: TableId::Local(table), value: value, unit: 0});
      }
      Node::Quantity{children} => {
       /* let table = hash_string(&format!("Quantity-{:?}{:?}", value.to_f32(), unit));

        let unit = match unit {
          Some(unit_string) => hash_string(unit_string),
          None => 0,
        };
        transformations.push(Transformation::NewTable{table_id: TableId::Local(table), rows: 1, columns: 1});
        transformations.push(Transformation::Constant{table_id: TableId::Local(table), value: *value, unit: unit.clone()});*/
      }
      Node::True => {
        let table = hash_string(&format!("True"));
        transformations.push(Transformation::NewTable{table_id: TableId::Local(table), rows: 1, columns: 1});
        transformations.push(Transformation::Constant{table_id: TableId::Local(table), value: 0x4000000000000001, unit: 0});
      }
      Node::False => {
        let table = hash_string(&format!("False"));
        transformations.push(Transformation::NewTable{table_id: TableId::Local(table), rows: 1, columns: 1});
        transformations.push(Transformation::Constant{table_id: TableId::Local(table), value: 0x4000000000000000, unit: 0});
      }
      _ => (),
    }*/
    transformations
  }

  pub fn compile_transformations(&mut self, nodes: &Vec<Node>) -> Vec<Transformation> {
    let mut compiled = Vec::new();
    for node in nodes {
      let mut result = self.compile_transformation(node);
      compiled.append(&mut result);
    }
    compiled
  }

}
*/