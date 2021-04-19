// # Mech Syntax Compiler

// ## Preamble

use mech_core::{Value, Block, BlockState, ValueMethods, Transformation, TableIndex, TableId, Register, NumberLiteral, NumberLiteralKind};
use mech_core::{Quantity, QuantityMath, make_quantity};
use mech_core::hash_string;
use mech_core::{Error, ErrorType};
use parser;
use parser::Parser;
use lexer::Token;
#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use hashbrown::hash_set::{HashSet};
use hashbrown::hash_map::{HashMap};
use super::formatter::Formatter;
use std::sync::Arc;

lazy_static! {
  static ref TABLE_COPY: u64 = hash_string("table/copy");
  static ref TABLE_HORZCAT: u64 = hash_string("table/horizontal-concatenate");
  static ref TABLE_VERTCAT: u64 = hash_string("table/vertical-concatenate");
  static ref TABLE_SET: u64 = hash_string("table/set");
  static ref TABLE_ADD_ROW: u64 = hash_string("table/add-row");
  static ref TABLE_SPLIT: u64 = hash_string("table/split");
  static ref SET_ANY: u64 = hash_string("set/any");
}

// ## Compiler Nodes

#[derive(Clone, PartialEq)]
pub enum Node {
  Root{ children: Vec<Node> },
  Fragment{ children: Vec<Node> },
  Program{title: Option<String>, children: Vec<Node> },
  Head{ children: Vec<Node> },
  Body{ children: Vec<Node> },
  Section{title: Option<String>, children: Vec<Node> },
  Block{ children: Vec<Node> },
  Statement{ children: Vec<Node> },
  Expression{ children: Vec<Node> },
  MathExpression{ children: Vec<Node> },
  SelectExpression{ children: Vec<Node> },
  Data{ children: Vec<Node> },
  Whenever{ children: Vec<Node> },
  WheneverIndex{ children: Vec<Node> },
  Wait{ children: Vec<Node> },
  Until{ children: Vec<Node> },
  SelectData{name: String, id: TableId, children: Vec<Node> },
  SetData{ children: Vec<Node> },
  SplitData{ children: Vec<Node> },
  TableColumn{ children: Vec<Node> },
  Binding{ children: Vec<Node> },
  FunctionBinding{ children: Vec<Node> },
  Function{ name: String, children: Vec<Node> },
  Define { name: String, id: u64},
  DotIndex { children: Vec<Node>},
  SubscriptIndex { children: Vec<Node> },
  Range,
  VariableDefine {children: Vec<Node> },
  TableDefine {children: Vec<Node> },
  AnonymousTableDefine {children: Vec<Node> },
  InlineTable {children: Vec<Node> },
  TableHeader {children: Vec<Node> },
  Attribute {children: Vec<Node> },
  TableRow {children: Vec<Node> },
  Comment {children: Vec<Node> },
  AddRow {children: Vec<Node> },
  Transformation{ children: Vec<Node> },
  Identifier{ name: String, id: u64 },
  Table{ name: String, id: u64 },
  Quantity {value: Quantity, unit: Option<String>},
  String{ text: String },
  Token{ token: Token, byte: u8 },
  Add,
  Subtract,
  Multiply,
  Divide,
  Exponent,
  LessThan,
  GreaterThan,
  GreaterThanEqual,
  LessThanEqual,
  Equal,
  NotEqual,
  And,
  Or,
  SelectAll,
  Empty,
  True,
  False,
  NumberLiteral{kind: NumberLiteralKind, bytes: Vec<u8> },
  RationalNumber{children: Vec<Node> },
  // Markdown
  SectionTitle{ text: String },
  Title{ text: String },
  ParagraphText{ text: String },
  Paragraph{ children: Vec<Node> },
  UnorderedList{ children: Vec<Node> },
  ListItem{ children: Vec<Node> },
  InlineCode{ children: Vec<Node> },
  CodeBlock{ children: Vec<Node> },
  // Mechdown
  InlineMechCode{ children: Vec<Node> },
  MechCodeBlock{ children: Vec<Node> },
  Null,
}

impl fmt::Debug for Node {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    print_recurse(self, 0, f);
    Ok(())
  }
}

pub fn print_recurse(node: &Node, level: usize, f: &mut fmt::Formatter) {
  spacer(level,f);
  let children: Option<&Vec<Node>> = match node {
    Node::Root{children} => {write!(f,"Root\n").ok(); Some(children)},
    Node::Fragment{children, ..} => {write!(f,"Fragment\n").ok(); Some(children)},
    Node::Program{title, children} => {write!(f,"Program({:?})\n", title).ok(); Some(children)},
    Node::Head{children} => {write!(f,"Head\n").ok(); Some(children)},
    Node::Body{children} => {write!(f,"Body\n").ok(); Some(children)},
    Node::VariableDefine{children} => {write!(f,"VariableDefine\n").ok(); Some(children)},
    Node::TableColumn{children} => {write!(f,"TableColumn\n").ok(); Some(children)},
    Node::Binding{children} => {write!(f,"Binding\n").ok(); Some(children)},
    Node::FunctionBinding{children} => {write!(f,"FunctionBinding\n").ok(); Some(children)},
    Node::TableDefine{children} => {write!(f,"TableDefine\n").ok(); Some(children)},
    Node::AnonymousTableDefine{children} => {write!(f,"AnonymousTableDefine\n").ok(); Some(children)},
    Node::InlineTable{children} => {write!(f,"InlineTable\n").ok(); Some(children)},
    Node::TableHeader{children} => {write!(f,"TableHeader\n").ok(); Some(children)},
    Node::Attribute{children} => {write!(f,"Attribute\n").ok(); Some(children)},
    Node::TableRow{children} => {write!(f,"TableRow\n").ok(); Some(children)},
    Node::AddRow{children} => {write!(f,"AddRow\n").ok(); Some(children)},
    Node::Section{title, children} => {write!(f,"Section({:?})\n", title).ok(); Some(children)},
    Node::Block{children, ..} => {write!(f,"Block\n").ok(); Some(children)},
    Node::Statement{children} => {write!(f,"Statement\n").ok(); Some(children)},
    Node::SetData{children} => {write!(f,"SetData\n").ok(); Some(children)},
    Node::SplitData{children} => {write!(f,"SplitData\n").ok(); Some(children)},
    Node::Data{children} => {write!(f,"Data\n").ok(); Some(children)},
    Node::Whenever{children} => {write!(f,"Whenever\n").ok(); Some(children)},
    Node::WheneverIndex{children} => {write!(f,"WheneverIndex\n").ok(); Some(children)},
    Node::Wait{children} => {write!(f,"Wait\n").ok(); Some(children)},
    Node::Until{children} => {write!(f,"Until\n").ok(); Some(children)},
    Node::SelectData{name, id, children} => {write!(f,"SelectData({:?} {:?}))\n", name, id).ok(); Some(children)},
    Node::DotIndex{children} => {write!(f,"DotIndex\n").ok(); Some(children)},
    Node::SubscriptIndex{children} => {write!(f,"SubscriptIndex\n").ok(); Some(children)},
    Node::Range => {write!(f,"Range\n").ok(); None},
    Node::Expression{children} => {write!(f,"Expression\n").ok(); Some(children)},
    Node::Function{name, children} => {write!(f,"Function({:?})\n", name).ok(); Some(children)},
    Node::MathExpression{children} => {write!(f,"MathExpression\n").ok(); Some(children)},
    Node::Comment{children} => {write!(f,"Comment\n").ok(); Some(children)},
    Node::SelectExpression{children} => {write!(f,"SelectExpression\n").ok(); Some(children)},
    Node::Transformation{children, ..} => {write!(f,"Transformation\n").ok(); Some(children)},
    Node::Identifier{name, id} => {write!(f,"Identifier({}({:#x}))\n", name, id).ok(); None},
    Node::String{text} => {write!(f,"String({:?})\n", text).ok(); None},
    Node::RationalNumber{children} => {write!(f,"RationalNumber\n").ok(); Some(children)},
    Node::NumberLiteral{kind, bytes} => {write!(f,"NumberLiteral({:?})\n", bytes).ok(); None},
    Node::Quantity{value, unit} => {write!(f,"Quantity({}{:?})\n", value.to_float(), unit).ok(); None},
    Node::Table{name,id} => {write!(f,"Table(#{}({:#x}))\n", name, id).ok(); None},
    Node::Define{name,id} => {write!(f,"Define #{}({:?})\n", name, id).ok(); None},
    Node::Token{token, byte} => {write!(f,"Token({:?})\n", token).ok(); None},
    Node::SelectAll => {write!(f,"SelectAll\n").ok(); None},
    Node::LessThan => {write!(f,"LessThan\n").ok(); None},
    Node::GreaterThan => {write!(f,"GreaterThan\n").ok(); None},
    Node::GreaterThanEqual => {write!(f,"GreaterThanEqual\n").ok(); None},
    Node::LessThanEqual => {write!(f,"LessThanEqual\n").ok(); None},
    Node::Equal => {write!(f,"Equal\n").ok(); None},
    Node::NotEqual => {write!(f,"NotEqual\n").ok(); None},
    Node::Empty => {write!(f,"Empty\n").ok(); None},
    Node::True => {write!(f,"True\n").ok(); None},
    Node::False => {write!(f,"False\n").ok(); None},
    Node::Null => {write!(f,"Null\n").ok(); None},
    Node::Add => {write!(f,"Add\n").ok(); None},
    Node::Subtract => {write!(f,"Subtract\n").ok(); None},
    Node::Multiply => {write!(f,"Multiply\n").ok(); None},
    Node::Divide => {write!(f,"Divide\n").ok(); None},
    Node::Exponent => {write!(f,"Exponent\n").ok(); None},
    // Markdown Nodes
    Node::Title{text} => {write!(f,"Title({:?})\n", text).ok(); None},
    Node::ParagraphText{text} => {write!(f,"ParagraphText({:?})\n", text).ok(); None},
    Node::UnorderedList{children} => {write!(f,"UnorderedList\n").ok(); Some(children)},
    Node::ListItem{children} => {write!(f,"ListItem\n").ok(); Some(children)},
    Node::Paragraph{children} => {write!(f,"Paragraph\n").ok(); Some(children)},
    Node::InlineCode{children} => {write!(f,"InlineCode\n").ok(); Some(children)},
    Node::CodeBlock{children} => {write!(f,"CodeBlock\n").ok(); Some(children)},
    // Extended Mechdown
    Node::InlineMechCode{children} => {write!(f,"InlineMechCode\n").ok(); Some(children)},
    Node::MechCodeBlock{children} => {write!(f,"MechCodeBlock\n").ok(); Some(children)},
    _ => {write!(f,"Unhandled Compiler Node").ok(); None},
  };
  match children {
    Some(childs) => {
      for child in childs {
        print_recurse(child, level + 1,f)
      }
    },
    _ => (),
  }
}

pub fn spacer(width: usize, f: &mut fmt::Formatter) {
  let limit = if width > 0 {
    width - 1
  } else {
    width
  };
  for _ in 0..limit {
    write!(f,"│").ok();
  }
  write!(f,"├").ok();
}

// ## Program

// Define a program struct that has everything we need to render a mech program.

#[derive(Clone, PartialEq)]
pub struct Program {
  pub title: Option<String>,
  pub sections: Vec<Section>,
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

// ## Compiler

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

  pub fn compile_string(&mut self, input: String) -> Vec<Program> {
    self.text = input.clone();
    let mut parser = Parser::new();
    parser.parse(&input);
    self.unparsed = parser.unparsed;
    self.parse_tree = parser.parse_tree.clone();
    self.build_syntax_tree(parser.parse_tree);
    let ast = self.syntax_tree.clone();
    let programs = self.compile(ast);
    self.programs = programs.clone();
    programs
  }

  pub fn compile_block_string(&mut self, input: String) -> Node {
    self.text = input.clone();
    let mut parser = Parser::new();
    parser.parse_block(&input);
    self.unparsed = parser.unparsed;
    self.parse_tree = parser.parse_tree.clone();
    let ast = self.build_syntax_tree(parser.parse_tree);
    ast[0].clone()
  }

  pub fn compile_fragment_string(&mut self, input: String) -> Node {
    self.text = input.clone();
    let mut parser = Parser::new();
    parser.parse_fragment(&input);
    self.unparsed = parser.unparsed;
    self.parse_tree = parser.parse_tree.clone();
    let ast = self.build_syntax_tree(parser.parse_tree);
    ast[0].clone()
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
    let block = self.compile_block(input).unwrap();
    let program = Program{title: None, sections: vec![
      Section {title: None, elements: vec![Element::Block(block)]}
    ]};
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
                                  children[0].clone()]}]}]}]};
              let block = self.compile_block(block_tree);
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
        let program = Program{title, sections};
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
      Node::Block{..} => Some(Element::Block(self.compile_block(input).unwrap())),
      Node::CodeBlock{..} => Some(Element::CodeBlock(input)),
      Node::MechCodeBlock{ref children} => {
        let (block_id, node) = self.compile_block(children[1].clone()).unwrap();
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

  pub fn compile_block(&mut self, node: Node) -> Option<(u64, Node)> {
    let block = match node.clone() {
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
        // ----------------------------------------------------------------------------------------------------------
        // Planner
        // ----------------------------------------------------------------------------------------------------------
        // This is the start of a new planner. This will evolve into its own thing I imagine. It's messy and rough now
        for transformation_node in children {
          let constraint_text = formatter.format(&transformation_node, false);
          let mut compiled_tfm = self.compile_transformation(&transformation_node);
          let mut produces: HashSet<u64> = HashSet::new();
          let mut consumes: HashSet<u64> = HashSet::new();
          let this_one = compiled_tfm.clone();
          for transformation in compiled_tfm {
            match &transformation {
              Transformation::TableAlias{table_id, alias} => {
                produces.insert(*alias);
                match aliases.insert(*alias) {
                  true => (),
                  false => {
                    // This alias has already been marked as produced, so it is a duplicate
                    block.errors.push(Error {
                      block: block.id,
                      step_text: constraint_text.clone(),
                      error_type: ErrorType::DuplicateAlias(*alias),
                    });
                  },
                }
              }
              Transformation::TableReference{table_id, reference} => {
                match table_id {
                  TableId::Local(id) => {
                    produces.insert(*id);
                  },
                  _ => (),
                };
              }
              Transformation::Whenever{table_id, ..} => {
                produces.insert(hash_string("~"));
              }
              Transformation::Constant{table_id, ..} => {
                match table_id {
                  TableId::Local(id) => {
                    produces.insert(*id);
                  },
                  _ => (),
                };
              }
              Transformation::NewTable{table_id, ..} => {
                match table_id {
                  TableId::Local(id) => {
                    produces.insert(*id);
                  },
                  _ => (),
                };
              },
              Transformation::Select{table_id, row, column, indices, out} => {
                match table_id {
                  TableId::Local(id) => {consumes.insert(*id);},
                  _ => (),
                }
                for (row, column) in indices {
                  match row {
                    TableIndex::Table(TableId::Local(id)) => {consumes.insert(*id);},
                    _ => (),
                  }
                  match column {
                    TableIndex::Table(TableId::Local(id)) => {consumes.insert(*id);},
                    _ => (),
                  }
                }
                match out {
                  TableId::Local(id) => {
                    produces.insert(*id);
                  },
                  _ => (),
                };
              },
              Transformation::Function{name, arguments, out} => {
                for (_, table_id, row, column) in arguments {
                  match row {
                    TableIndex::Table(TableId::Local(id)) => {consumes.insert(*id);},
                    _ => (),
                  }
                  match table_id {
                    TableId::Local(id) => {consumes.insert(*id);},
                    _ => (),
                  }
                }
                let (out_id, row, column) = out;
                match out_id {
                  TableId::Local(id) => {produces.insert(*id);},
                  _ => (),
                }
                match row {
                  TableIndex::Table(TableId::Local(id)) => {
                    consumes.insert(*id);
                  }
                  _ => (),
                }
                match column {
                  TableIndex::Table(TableId::Local(id)) => {
                    consumes.insert(*id);
                  }
                  _ => (),
                }
              }
              _ => (),
            }
            transformations.push(transformation.clone());
          }
          //transformations.append(&mut functions);
          // If the constraint doesn't consume anything, put it on the top of the plan. It can run any time.
          if consumes.len() == 0 || consumes.difference(&produces).cloned().collect::<HashSet<u64>>().len() == 0 {
            block_produced = block_produced.union(&produces).cloned().collect();
            plan.insert(0, (constraint_text, produces, consumes, this_one));
          // Otherwise, the constraint consumes something, and we have to see if it's satisfied
          } else {
            let mut satisfied = false;
            //let (step_node, step_produces, step_consumes, step_constraints) = step;
            let union: HashSet<u64> = block_produced.union(&produces).cloned().collect();
            let unsatisfied: HashSet<u64> = consumes.difference(&union).cloned().collect();
            if unsatisfied.is_empty() {
              block_produced = block_produced.union(&produces).cloned().collect();
              plan.push((constraint_text, produces, consumes, this_one));
            } else {
              unsatisfied_transformations.push((constraint_text, produces, consumes, this_one));
            }
          }


          // Check if any of the unsatisfied constraints have been met yet. If they have, put them on the plan.
          let mut now_satisfied = unsatisfied_transformations.drain_filter(|unsatisfied_transformation| {
            let (text, unsatisfied_produces, unsatisfied_consumes, _) = unsatisfied_transformation;
            let union: HashSet<u64> = block_produced.union(&unsatisfied_produces).cloned().collect();
            let unsatisfied: HashSet<u64> = unsatisfied_consumes.difference(&union).cloned().collect();
            match unsatisfied.is_empty() {
              true => {
                block_produced = block_produced.union(&unsatisfied_produces).cloned().collect();
                true
              }
              false => false
            }
          }).collect::<Vec<_>>();
          plan.append(&mut now_satisfied);
        }
        // Do a final check on unsatisfied constraints that are now satisfied
         let mut now_satisfied = unsatisfied_transformations.drain_filter(|unsatisfied_transformation| {
          let (_, unsatisfied_produces, unsatisfied_consumes, _) = unsatisfied_transformation;
          let union: HashSet<u64> = block_produced.union(&unsatisfied_produces).cloned().collect();
          let unsatisfied: HashSet<u64> = unsatisfied_consumes.difference(&union).cloned().collect();
          match unsatisfied.is_empty() {
            true => {
              block_produced = block_produced.union(&unsatisfied_produces).cloned().collect();
              true
            }
            false => false
          }
        }).collect::<Vec<_>>();

        plan.append(&mut now_satisfied);
        
        let mut global_out = vec![];
        let mut block_transformations = vec![];
        for step in plan {
          let (step_text, _, _, step_transformations) = step;
          let mut rtfms = step_transformations.clone();
          rtfms.reverse();
          for tfm in rtfms {
            match tfm {
              Transformation::TableReference{table_id, reference} => {
                block.plan.push(Transformation::Function{
                  name: *TABLE_COPY,
                  arguments: vec![(0,TableId::Local(reference.as_reference().unwrap()), TableIndex::All, TableIndex::All)],
                  out: (TableId::Global(reference.as_reference().unwrap()), TableIndex::All, TableIndex::All),
                });
              }
              Transformation::Whenever{..} => {
                block.plan.push(tfm.clone());
              }
              Transformation::Select{..} => {
                block.plan.push(tfm.clone());
              }
              Transformation::Function{name, ref arguments, out} => {
                let (out_id, row, column) = out;
                match out_id {
                  TableId::Local(id) => block.plan.push(tfm.clone()),
                  _ => {
                    global_out.push(tfm.clone());
                  },
                }
              }
              _ => (),
            }
          }
          block_transformations.push((step_text, step_transformations));
        }
        block.plan.append(&mut global_out);

        // Here we try to optimize the plan a little bit. The compiler will generate chains of concatenating
        // tables sometimes where there is no actual work to be done. If we remove these moot intermediate steps,
        // we can save time. We do this by comparing the input and outputs of consecutive steps. If the two steps
        // can be condensed into one step, we do this.
        let mut defunct_tables = vec![];
        if block.plan.len() > 1 {
          let mut new_plan = vec![];
          let mut step_ix = 0;
          loop {
            if step_ix >= block.plan.len() - 1 {
              if step_ix == block.plan.len() - 1 {
                new_plan.push(block.plan[step_ix].clone());
              }
              break;
            }
            let this = &block.plan[step_ix];
            let next = &block.plan[step_ix + 1];
            match (this, next) {
              (Transformation::Function{name, arguments, out}, Transformation::Function{name: name2, arguments: arguments2, out: out2}) => {
                if (*name2 == *TABLE_HORZCAT || *name2 == *TABLE_VERTCAT) && arguments2.len() == 1 {
                  defunct_tables.append(&mut arguments2.clone());
                  let (_, out_table2, out_row2, out_column2) = arguments2[0];
                  if *out == (out_table2, out_row2, out_column2) {
                    let new_step = Transformation::Function{name: *name, arguments: arguments.clone(), out: *out2};
                    new_plan.push(new_step);
                    step_ix += 2;
                    continue;
                  }
                }
                new_plan.push(this.clone());
              }
              (Transformation::Select{table_id, row, column, indices, out}, _) => {
                if indices.len() == 1 {
                  defunct_tables.push((0, *table_id, TableIndex::None, TableIndex::None));
                } else {
                  new_plan.push(this.clone());
                }
              }
              _ => new_plan.push(this.clone()),
            }
            step_ix += 1;
          }
          block.plan = new_plan;
        }
        /*// Remove all defunct tables from the transformation list. These would be tables that were written to by
        // some function that was removed from the previous optimization pass
        let defunct_table_ids = defunct_tables.iter().map(|(_, table_id, _, _)| table_id).collect::<HashSet<&TableId>>();
        let mut new_transformations = vec![];
        for (tfm_text, steps) in &block_transformations {
          let mut new_steps = vec![];
          for step in steps {
            match step {
              Transformation::NewTable{table_id, ..} => {
                match defunct_table_ids.contains(&table_id) {
                  true => continue,
                  false => new_steps.push(step.clone()),
                }
              }
              _ => new_steps.push(step.clone()),
            }
          }
          new_transformations.push((tfm_text.clone(), new_steps));
        }
        block_transformations = new_transformations;*/
        // End Planner ----------------------------------------------------------------------------------------------------------

        for transformation in block_transformations {
          block.register_transformations(transformation);
        }

        for (step_text, _, unsatisfied_consumes, step_transformations) in unsatisfied_transformations {
          block.errors.push(Error {
            block: block.id,
            step_text: step_text,
            error_type: ErrorType::UnsatisfiedConstraint(
              unsatisfied_consumes.iter().map(|x| x.clone()).collect::<Vec<u64>>(),
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
          block.errors.push(err);
        }
        self.variable_names.clear();
        self.blocks.push(block.clone());
        Some((block.id, node))
      },
      _ => None,
    };
    block
  }

  pub fn compile_transformation(&mut self, node: &Node) -> Vec<Transformation> {
    let mut transformations: Vec<Transformation> = Vec::new();
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
          println!("{:?}", result);
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
        //let mut output = self.compile_transformation(&children[0]);
        let output_table_id = match &children[0] {
          Node::Identifier{name,..} => {
            let name_hash = hash_string(name);
            self.strings.insert(name_hash, name.to_string());
            transformations.push(Transformation::NewTable{table_id: TableId::Local(name_hash), rows: 1, columns: 1});
            TableId::Local(name_hash)
          }
          Node::Table{name, ..} => {
            let name_hash = hash_string(name);
            self.strings.insert(name_hash, name.to_string());
            transformations.push(Transformation::NewTable{table_id: TableId::Global(name_hash), rows: 1, columns: 1});
            TableId::Global(name_hash)
          },
          _ => TableId::Local(0),
        };

        let mut input = self.compile_transformation(&children[1]);

        let input_table_id = match input[0] {
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
        let input_table_id = match input[0] {
          Transformation::NewTable{table_id,..} => {
            Some(table_id)
          }
          Transformation::TableReference{table_id, reference} => {
            Some(TableId::Local(reference.as_reference().unwrap()))
          }
          _ => None,
        };

        let (output_table_id, output_row, output_col) = output_tup.unwrap();

        let fxn = Transformation::Function{
          name: *TABLE_ADD_ROW,
          arguments: vec![
            (0, input_table_id.unwrap(), TableIndex::All, TableIndex::All)
          ],
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
            Some((table_id,row,column))
          }
          _ => None,
        };
        let mut output_tup2 = if output.len() > 1 {
          match &output[1] {
            Transformation::Select{table_id, row, column, indices, out} => {
              let (row, column) = indices[0];
              let tfm = Transformation::Set{table_id: *table_id, row: row, column: column};
              transformations.push(tfm);
              Some((table_id,row,column))
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
            (table,row2,col)
          },
          (Some(a),_) => a,
          _ => (&TableId::Global(0),TableIndex::All,TableIndex::All),
        };

        let fxn = Transformation::Function{
          name: *TABLE_SET,
          arguments: vec![
            (0, input_table_id, row_select, column_select)
          ],
          out: (*output_table_id, output_row, output_col),
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
        let value = Value::from_string(text.to_string());
        self.strings.insert(value, text.to_string());
        transformations.push(Transformation::Constant{table_id: TableId::Local(table), value, unit: 0});
      }
      Node::NumberLiteral{kind, bytes} => {
        let table = hash_string(&format!("NumberLiteral-{:?}{:?}", kind, bytes));
        transformations.push(Transformation::NewTable{table_id: TableId::Local(table), rows: 1, columns: 1});
        let value = Value::from_byte_vector(bytes);
        self.number_literals.insert(value, NumberLiteral{kind: *kind, bytes: bytes.clone()} );
        transformations.push(Transformation::Constant{table_id: TableId::Local(table), value, unit: 0});
      }
      Node::Empty => {
        let value = Value::empty();
        let table = hash_string(&format!("Empty-{:?}", value.to_float()));
        transformations.push(Transformation::NewTable{table_id: TableId::Local(table), rows: 1, columns: 1});
        transformations.push(Transformation::Constant{table_id: TableId::Local(table), value: value, unit: 0});
      }
      Node::Quantity{value, unit} => {
        let table = hash_string(&format!("Quantity-{:?}{:?}", value.to_float(), unit));

        let unit = match unit {
          Some(unit_string) => hash_string(unit_string),
          None => 0,
        };
        transformations.push(Transformation::NewTable{table_id: TableId::Local(table), rows: 1, columns: 1});
        transformations.push(Transformation::Constant{table_id: TableId::Local(table), value: *value, unit: unit.clone()});
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
    }
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

  pub fn build_syntax_tree(&mut self, node: parser::Node) -> Vec<Node> {
    let mut compiled = Vec::new();
    self.depth += 1;
    match node {
      parser::Node::Root{children} => {
        let result = self.compile_nodes(children);
        self.syntax_tree = Node::Root{children: result};
      },
      parser::Node::Fragment{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Fragment{children: result});
      },
      parser::Node::Program{children} => {
        let result = self.compile_nodes(children);
        let mut children = vec![];
        let mut title = None;
        for node in result {
          match node {
            Node::Title{text} => title = Some(text),
            _ => children.push(node),
          }
        }
        compiled.push(Node::Program{title, children});
      },
      parser::Node::Head{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Head{children: result});
      },
      parser::Node::Section{children} => {
        let result = self.compile_nodes(children);
        let mut children = vec![];
        let mut title = None;
        for node in result {
          match node {
            Node::Title{text} => title = Some(text),
            _ => children.push(node),
          }
        }
        compiled.push(Node::Section{title, children});
      },
      parser::Node::Block{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Block{children: result});
      },
      parser::Node::Data{children} => {
        let result = self.compile_nodes(children);
        let mut reversed = result.clone();
        reversed.reverse();
        let mut select_data_children: Vec<Node> = vec![];

        for node in reversed {
          match node {
            Node::Table{name, id} => {
              if select_data_children.is_empty() {
                select_data_children = vec![Node::Null; 1];
              }
              select_data_children.reverse();
              compiled.push(Node::SelectData{name, id: TableId::Global(id), children: select_data_children.clone()});
            },
            Node::Identifier{name, id} => {
              if select_data_children.is_empty() {
                select_data_children = vec![Node::Null; 1];
              }
              //select_data_children.reverse();
              compiled.push(Node::SelectData{name, id: TableId::Local(id), children: select_data_children.clone()});
            },
            Node::DotIndex{children} => {
              let mut reversed = children.clone();
              if children.len() == 1 {
                reversed.push(Node::Null);
                reversed.reverse();
              }
              select_data_children.push(Node::DotIndex{children: reversed});
            },
            Node::SubscriptIndex{..} => {
              /*let mut reversed = children.clone();
              reversed.reverse();*/
              select_data_children.push(node.clone());
            }
            _ => (),
          }
        }
      },
      parser::Node::Statement{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Statement{children: result});
      },
      parser::Node::Expression{children} => {
        let result = self.compile_nodes(children);
        for node in result {
          match node {
            // If the node is a naked expression, modify the graph
            // TODO this is hacky... maybe change the parser?
            Node::SelectData{..} => {
              compiled.push(node);
            },
            _ => compiled.push(Node::Expression{children: vec![node]}),
          }
        }
      },
      parser::Node::Attribute{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(Node::Attribute{children});
      },
      parser::Node::Whenever{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Whenever{children: result});
      },
      parser::Node::Wait{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Wait{children: result});
      },
      parser::Node::Until{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Until{children: result});
      },
      parser::Node::SelectAll => {
        compiled.push(Node::SelectAll);
      },
      parser::Node::SetData{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(Node::SetData{children});
      },
      parser::Node::SplitData{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(Node::SplitData{children});
      },
      parser::Node::Column{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(Node::TableColumn{children});
      },
      parser::Node::Empty => {
        compiled.push(Node::Empty);
      },
      parser::Node::Binding{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(Node::Binding{children});
      },
      parser::Node::FunctionBinding{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(Node::FunctionBinding{children});
      },
      parser::Node::Transformation{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            // Ignore irrelevant nodes like spaces and operators
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        if !children.is_empty() {
          compiled.push(Node::Transformation{children});
        }
      },
      parser::Node::SelectExpression{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::SelectExpression{children: result});
      },
      parser::Node::InlineTable{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(Node::InlineTable{children});
      },
      parser::Node::AnonymousTable{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(Node::AnonymousTableDefine{children});
      },
      parser::Node::TableHeader{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(Node::TableHeader{children});
      },
      parser::Node::TableRow{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(Node::TableRow{children});
      },
      parser::Node::MathExpression{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        let mut new_node = false;
        for node in result {
          match node {
            // Ignore irrelevant nodes like spaces and operators
            Node::Token{..} => (),
            Node::Function{..} => {
              new_node = true;
              children.push(node);
            },
            Node::Quantity{..} => {
              new_node = false;
              children.push(node);
            }
            _ => children.push(node),
          }
        }
        // TODO I might be able to simplify this now. This doesn't seem to be necessary
        if new_node {
          compiled.push(Node::MathExpression{children});
        } else {
          compiled.append(&mut children);
        }
      },
      parser::Node::Infix{children} => {
        let result = self.compile_nodes(children);
        let operator = &result[0];
        let name: String = match operator {
          Node::Token{token, byte} => byte_to_char(*byte).unwrap().to_string(),
          _ => String::from(""),
        };
        compiled.push(Node::Function{name, children: vec![]});
      },
      parser::Node::VariableDefine{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          // If the node is a naked expression, modify the
          // graph to put it into an anonymous table
          match node {
            Node::Token{..} => (),
            Node::SelectData{..} => {
              children.push(Node::Expression{
                children: vec![Node::AnonymousTableDefine{
                  children: vec![Node::TableRow{
                    children: vec![Node::TableColumn{
                      children: vec![node]}]}]}]});
            },
            _ => children.push(node),
          }
        }
        compiled.push(Node::VariableDefine{children});
      },
      parser::Node::TableDefine{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            Node::SelectData{..} => {
              children.push(Node::Expression{
                children: vec![Node::AnonymousTableDefine{
                  children: vec![Node::TableRow{
                    children: vec![Node::TableColumn{
                      children: vec![node]}]}]}]});
            },
            _ => children.push(node),
          }
        }
        compiled.push(Node::TableDefine{children});
      },
      parser::Node::AddRow{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(Node::AddRow{children});
      },
      parser::Node::Index{children} => {
        compiled.append(&mut self.compile_nodes(children));
      },
      parser::Node::DotIndex{children} => {
        let mut result = self.compile_nodes(children);
        compiled.push(Node::DotIndex{children: result});
      },
      parser::Node::SubscriptIndex{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{token, byte} => {
              match token {
                Token::Tilde => {
                  children.push(Node::WheneverIndex{children: vec![Node::Null]});
                }
                _ => (),
              }
            },
            _ => children.push(node),
          }
        }
        compiled.push(Node::SubscriptIndex{children});
      },
      parser::Node::Table{children} => {
        let result = self.compile_nodes(children);
        match &result[0] {
          Node::Identifier{name, id} => {
            compiled.push(Node::Table{name: name.to_string(), id: *id});
          },
          _ => (),
        };
      },
      // Quantities
      parser::Node::Quantity{children} => {
        let mut result = self.compile_nodes(children);
        let mut quantity = make_quantity(0,0,0);
        let mut unit = None;
        for node in result {
          match node {
            Node::Quantity{value, unit} => {
              quantity = quantity.add(value).unwrap();
            },
            Node::Identifier{name: word, id} => unit = Some(word),
            _ => (),
          }
        }
        compiled.push(Node::Quantity{value: quantity, unit});
      },
      parser::Node::Number{children} => {
        let mut value: u64 = 0;
        let mut result = self.compile_nodes(children);
        result.reverse();
        let mut place = 1;
        let mut quantities: Vec<Quantity> = vec![];
        for node in result {
          match node {
            Node::Token{token: Token::Comma, byte} => (),
            Node::Token{token, byte} => {
              let digit = byte_to_digit(byte).unwrap();
              let q = digit * magnitude(place);
              place += 1;
              value += q;
            },
            Node::Quantity{value, unit} => quantities.push(value),
            _ => (),
          }
        }
        let mut quantity = make_quantity(value as i64,0,0);
        for q in quantities {
          quantity = quantity.add(q).unwrap();
        }
        compiled.push(Node::Quantity{value: quantity, unit: None});
      },
      parser::Node::FloatingPoint{children} => {
        let mut value: u64 = 0;
        let mut result = self.compile_nodes(children);
        result.reverse();
        let mut place = 1;
        for node in result {
          match node {
            Node::Token{token: Token::Period, byte} => (),
            Node::Token{token, byte} => {
              let digit = byte_to_digit(byte).unwrap();
              let q = digit * magnitude(place);
              place += 1;
              value += q;
            },
            _ => (),
          }
        }
        let quantity = make_quantity(value as i64,1 - place as i64,0);
        compiled.push(Node::Quantity{value: quantity, unit: None});
      },
      // String-like nodes
      parser::Node::ParagraphText{children} => {
        let mut result = self.compile_nodes(children);
        let mut paragraph = "".to_string();
        for node in result {
          match &node {
            Node::String{text} => {paragraph = paragraph + text;},
            _ => (),
          };
        }

        let node = Node::ParagraphText{text: paragraph.clone()};
        compiled.push(node);
      },
      parser::Node::InlineCode{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::InlineCode{children: result});
      },
      parser::Node::CodeBlock{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::CodeBlock{children: result});
      },
      parser::Node::MechCodeBlock{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::MechCodeBlock{children: result});
      },
      parser::Node::Comment{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Comment{children: result});
      },
      parser::Node::InlineMechCode{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::InlineMechCode{children: result});
      },
      parser::Node::Paragraph{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Paragraph{children: result});
      },
      parser::Node::UnorderedList{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::UnorderedList{children: result});
      },
      parser::Node::ListItem{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::ListItem{children: result});
      },
      parser::Node::Title{children} => {
        let mut result = self.compile_nodes(children);
        let node = match &result[0] {
          Node::String{text} => Node::Title{text: text.clone()},
          _ => Node::Null,
        };
        compiled.push(node);
      },
      parser::Node::Subtitle{children} => {
        let mut result = self.compile_nodes(children);
        let node = match &result[0] {
          Node::String{text} => Node::Title{text: text.clone()},
          _ => Node::Null,
        };
        compiled.push(node);
      },
      parser::Node::SectionTitle{children} => {
        let mut result = self.compile_nodes(children);
        let node = match &result[0] {
          Node::String{text} => Node::SectionTitle{text: text.clone()},
          _ => Node::Null,
        };
        compiled.push(node);
      },
      parser::Node::FormattedText{children} |
      parser::Node::Text{children} => {
        let mut result = self.compile_nodes(children);
        let mut text_node = String::new();
        for node in result {
          match node {
            Node::String{text} => text_node.push_str(&text),
            Node::Token{token, byte} => text_node.push_str(&format!("{}",byte_to_char(byte).unwrap())),
            Node::Quantity{value, unit} => text_node.push_str(&format!("{}", value.to_float())),
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
              let character = byte_to_char(byte).unwrap();
              word.push(character);
            },
            _ => (),
          }
        }
        compiled.push(Node::String{text: word});
      },
      parser::Node::TableIdentifier{children} |
      parser::Node::Identifier{children} => {
        let mut word = String::new();
        let mut result = self.compile_nodes(children);
        for node in result {
          match node {
            Node::Token{token, byte} => {
              let character = byte_to_char(byte).unwrap();
              word.push(character);
            },
            Node::String{text} => word.push_str(&text),
            Node::Quantity{value, unit} => word.push_str(&format!("{}", value.to_float())),
            _ => compiled.push(node),
          }
        }
        let id = hash_string(&word);
        compiled.push(Node::Identifier{name: word, id});
      },
      // Math
      parser::Node::L0{children} |
      parser::Node::L1{children} |
      parser::Node::L2{children} |
      parser::Node::L3{children} |
      parser::Node::L4{children} |
      parser::Node::L5{children} |
      parser::Node::L6{children} => {
        let result = self.compile_nodes(children);
        let mut last = Node::Null;
        for node in result {
          match last {
            Node::Null => last = node,
            _ => {
              let (name, mut children) = match node {
                Node::Function{name, mut children} => (name.clone(), children.clone()),
                _ => (String::from(""), vec![]),
              };
              children.push(last);
              children.reverse();
              last = Node::Function{name, children};
            },
          };
        }
        compiled.push(last);
      },
      parser::Node::L0Infix{children} |
      parser::Node::L1Infix{children} |
      parser::Node::L2Infix{children} |
      parser::Node::L3Infix{children} |
      parser::Node::L4Infix{children} |
      parser::Node::L5Infix{children} => {
        let result = self.compile_nodes(children);
        let operator = &result[0].clone();
        let input = &result[1].clone();
        let name: String = match operator {
          Node::Add => "math/add".to_string(),
          Node::Subtract => "math/subtract".to_string(),
          Node::Multiply => "math/multiply".to_string(),
          Node::Divide => "math/divide".to_string(),
          Node::Exponent => "math/exponent".to_string(),
          Node::GreaterThan => "compare/greater-than".to_string(),
          Node::GreaterThanEqual => "compare/greater-than-equal".to_string(),
          Node::LessThanEqual => "compare/less-than-equal".to_string(),
          Node::LessThan => "compare/less-than".to_string(),
          Node::Equal => "compare/equal".to_string(),
          Node::NotEqual => "compare/not-equal".to_string(),
          Node::Range => "table/range".to_string(),
          Node::And => "logic/and".to_string(),
          Node::Or => "logic/or".to_string(),
          Node::Token{token, byte} => byte_to_char(*byte).unwrap().to_string(),
          _ => String::from(""),
        };
        compiled.push(Node::Function{name, children: vec![input.clone()]});
      },
      parser::Node::Function{children} => {
        let mut result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        let mut function_name: String = "".to_string();
        for node in result {
          match node {
            Node::Token{..} => (),
            Node::Identifier{name, id} => function_name = name,
            _ => children.push(node),
          }
        }
        compiled.push(Node::Function{name: function_name, children: children.clone()});
      },
      parser::Node::Negation{children} => {
        let mut result = self.compile_nodes(children);
        let mut input = vec![Node::Quantity{value: 0, unit: None}];
        input.push(result[0].clone());
        compiled.push(Node::Function{ name: "math/subtract".to_string(), children: input });
      },
      parser::Node::String{children} => {
        let mut result = self.compile_nodes(children);
        let string = if result.len() > 0 {
          result[0].clone()
        } else {
          Node::String{text: String::new()}
        };
        compiled.push(string);
      },
      parser::Node::NumberLiteral{children} => {
        let mut result = self.compile_nodes(children);
        compiled.push(result[0].clone());
      },
      parser::Node::RationalNumber{children} => {
        let mut result = self.compile_nodes(children);
        compiled.push(Node::RationalNumber{children: result});
      },
      parser::Node::DecimalLiteral{bytes} => {
        let dec_bytes: Vec<u8> = bytes.iter().map(|b| {
          match b {
            48 => 0,
            49 => 1,
            50 => 2,
            51 => 3,
            52 => 4,
            53 => 5,
            54 => 6,
            55 => 7,
            56 => 8,
            57 => 9,
            _ => 0,        // TODO: ERROR
          }
        }).collect::<Vec<u8>>();
        compiled.push(Node::NumberLiteral{kind: NumberLiteralKind::Decimal, bytes: dec_bytes});
      },
      parser::Node::BinaryLiteral{bytes} => {
        let bin_bytes: Vec<u8> = bytes.iter().map(|b| {
          match b {
            48 => 0,
            49 => 1,
            _ => 0,        // TODO: ERROR
          }
        }).collect::<Vec<u8>>();
        compiled.push(Node::NumberLiteral{kind: NumberLiteralKind::Binary, bytes: bin_bytes});
      }
      parser::Node::OctalLiteral{bytes} => {
        let oct_bytes: Vec<u8> = bytes.iter().map(|b| {
          match b {
            48 => 0,
            49 => 1,
            50 => 2,
            51 => 3,
            52 => 4,
            53 => 5,
            54 => 6,
            55 => 7,
            _ => 0,        // TODO: ERROR
          }
        }).collect::<Vec<u8>>();
        compiled.push(Node::NumberLiteral{kind: NumberLiteralKind::Octal, bytes: oct_bytes});
      }
      parser::Node::HexadecimalLiteral{bytes} => {
        let hex_bytes: Vec<u8> = bytes.iter().map(|b| {
          match b {
            48 => 0,
            49 => 1,
            50 => 2,
            51 => 3,
            52 => 4,
            53 => 5,
            54 => 6,
            55 => 7,
            56 => 8,
            57 => 9,
            65 | 97 => 10, // A
            66 | 98 => 11, // B
            67 | 99 => 12, // C
            68 | 100 => 13,// D
            69 | 101 => 14,// E
            70 | 102 => 15,// F
            _ => 0,        // TODO: ERROR
          }
        }).collect::<Vec<u8>>();
        compiled.push(Node::NumberLiteral{kind: NumberLiteralKind::Hexadecimal, bytes: hex_bytes});
      },
      parser::Node::True => {
        compiled.push(Node::True);
      },
      parser::Node::Null => {
      },
      parser::Node::False => {
        compiled.push(Node::False);
      },
      parser::Node::ParentheticalExpression{children} => {
        let mut result = self.compile_nodes(children);
        compiled.push(result[0].clone());
      },
      parser::Node::GreaterThan => compiled.push(Node::GreaterThan),
      parser::Node::LessThan => compiled.push(Node::LessThan),
      parser::Node::GreaterThanEqual => compiled.push(Node::GreaterThanEqual),
      parser::Node::LessThanEqual => compiled.push(Node::LessThanEqual),
      parser::Node::Equal => compiled.push(Node::Equal),
      parser::Node::NotEqual => compiled.push(Node::NotEqual),
      parser::Node::Add => compiled.push(Node::Add),
      parser::Node::Range => compiled.push(Node::Range),
      parser::Node::Subtract => compiled.push(Node::Subtract),
      parser::Node::Multiply => compiled.push(Node::Multiply),
      parser::Node::Divide => compiled.push(Node::Divide),
      parser::Node::Exponent => compiled.push(Node::Exponent),
      parser::Node::And => compiled.push(Node::And),
      parser::Node::Or => compiled.push(Node::Or),
      parser::Node::Comparator{children} => {
        match children[0] {
          parser::Node::LessThan => compiled.push(Node::LessThan),
          parser::Node::LessThanEqual => compiled.push(Node::LessThanEqual),
          parser::Node::GreaterThanEqual => compiled.push(Node::GreaterThanEqual),
          parser::Node::Equal => compiled.push(Node::Equal),
          parser::Node::NotEqual => compiled.push(Node::NotEqual),
          parser::Node::GreaterThan => compiled.push(Node::GreaterThan),
          _ => (),
        }
      },
      parser::Node::LogicOperator{children} => {
        match children[0] {
          parser::Node::And => compiled.push(Node::And),
          parser::Node::Or => compiled.push(Node::Or),
          _ => (),
        }
      },
      // Pass through nodes. These will just be omitted
      parser::Node::Constant{children} |
      parser::Node::StateMachine{children} |
      parser::Node::StateTransition{children} |
      parser::Node::Body{children} |
      parser::Node::Punctuation{children} |
      parser::Node::DigitOrComma{children} |
      parser::Node::Comment{children} |
      parser::Node::Any{children} |
      parser::Node::Symbol{children} |
      parser::Node::AddOperator{children} |
      parser::Node::LogicOperator{children} |
      parser::Node::Subscript{children} |
      parser::Node::DataOrConstant{children} |
      parser::Node::SpaceOrTab{children} |
      parser::Node::Whitespace{children} |
      parser::Node::NewLine{children} |
      parser::Node::Attribute{children} |
      parser::Node::Comparator{children} |
      parser::Node::IdentifierOrConstant{children} |
      parser::Node::ProseOrCode{children}|
      parser::Node::StatementOrExpression{children} |
      parser::Node::WatchOperator{children} |
      parser::Node::Quantity{children} |
      parser::Node::SetOperator{children} |
      parser::Node::Repeat{children} |
      parser::Node::Alphanumeric{children} |
      parser::Node::IdentifierCharacter{children} => {
        compiled.append(&mut self.compile_nodes(children));
      },
      parser::Node::Token{token, byte} => {
        match token {
          Token::Newline => {
            self.current_line += 1;
            self.current_col = 1;
            self.current_char += 1;
          },
          Token::EndOfStream => (),
          _ => {
            self.current_char += 1;
            self.current_col += 1;
          }
        }
        compiled.push(Node::Token{token, byte});
      },
      _ => println!("Unhandled Parser Node in Compiler: {:?}", node),
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

// ## Appendix

// ### Encodings

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

fn byte_to_char(byte: u8) -> Option<char> {
  match byte {
    10 => Some('\n'),
    13 => Some('\r'),
    32 => Some(' '),
    33 => Some('!'),
    34 => Some('"'),
    35 => Some('#'),
    36 => Some('$'),
    37 => Some('%'),
    38 => Some('&'),
    39 => Some('\''),
    40 => Some('('),
    41 => Some(')'),
    42 => Some('*'),
    43 => Some('+'),
    44 => Some(','),
    45 => Some('-'),
    46 => Some('.'),
    47 => Some('/'),
    48 => Some('0'),
    49 => Some('1'),
    50 => Some('2'),
    51 => Some('3'),
    52 => Some('4'),
    53 => Some('5'),
    54 => Some('6'),
    55 => Some('7'),
    56 => Some('8'),
    57 => Some('9'),
    58 => Some(':'),
    59 => Some(';'),
    60 => Some('<'),
    61 => Some('='),
    62 => Some('>'),
    63 => Some('?'),
    64 => Some('@'),
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
    91 => Some('['),
    92 => Some('\\'),
    93 => Some(']'),
    94 => Some('^'),
    95 => Some('_'),
    96 => Some('`'),
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
    123 => Some('{'),
    124 => Some('|'),
    125 => Some('}'),
    126 => Some('~'),
    _ => {
      //println!("Unhandled Byte {:?}", byte);
      None
    },
  }
}

// ### Utility Functions

fn magnitude(n: usize) -> u64 {
  let mut m = 1;
  for i in 1 .. n {
    m = m * 10;
  }
  m
}
