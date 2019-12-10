// # Mech Syntax Compiler

// ## Preamble

use mech_core::{Block, BlockState, Constraint, Index, TableId};
use mech_core::{Function, Comparator, Logic, Parameter, Quantity, ToQuantity, QuantityMath, make_quantity};
use mech_core::Hasher;
use mech_core::ErrorType;
use parser;
use parser::Parser;
use lexer::Token;
#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use hashbrown::hash_set::{HashSet};
use super::formatter::Formatter;

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
  FilterExpression{ comparator: Comparator, children: Vec<Node> },
  LogicExpression{ operator: Logic, children: Vec<Node> },
  SelectExpression{ children: Vec<Node> },
  Data{ children: Vec<Node> },
  DataWatch{ children: Vec<Node> },
  SelectData{name: String, id: TableId, children: Vec<Node> },
  SetData{ children: Vec<Node> },
  SplitData{ children: Vec<Node> },
  Column{ children: Vec<Node> },
  Binding{ children: Vec<Node> },
  Function{ name: String, children: Vec<Node> },
  Define { name: String, id: u64},
  DotIndex { children: Vec<Node>},
  SubscriptIndex { children: Vec<Node> },
  Range { children: Vec<Node> },
  VariableDefine {children: Vec<Node> },
  TableDefine {children: Vec<Node> },
  AnonymousTableDefine {children: Vec<Node> },
  InlineTable {children: Vec<Node> },
  TableHeader {children: Vec<Node> },
  Attribute {children: Vec<Node> },
  TableRow {children: Vec<Node> },
  Comment {children: Vec<Node> },
  AddRow {children: Vec<Node> },
  Constraint{ children: Vec<Node> },
  Identifier{ name: String, id: u64 },
  Table{ name: String, id: u64 },
  Constant {value: Quantity, unit: Option<String>},
  String{ text: String },
  Token{ token: Token, byte: u8 },
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
    print_recurse(self, 0);
    Ok(())
  }
}

pub fn print_recurse(node: &Node, level: usize) {
  spacer(level);
  let children: Option<&Vec<Node>> = match node {
    Node::Root{children} => {print!("Root\n"); Some(children)},
    Node::Fragment{children, ..} => {print!("Fragment\n"); Some(children)},
    Node::Program{title, children} => {print!("Program({:?})\n", title); Some(children)},
    Node::Head{children} => {print!("Head\n"); Some(children)},
    Node::Body{children} => {print!("Body\n"); Some(children)},
    Node::VariableDefine{children} => {print!("VariableDefine\n"); Some(children)},
    Node::Column{children} => {print!("Column\n"); Some(children)},
    Node::Binding{children} => {print!("Binding\n"); Some(children)},
    Node::TableDefine{children} => {print!("TableDefine\n"); Some(children)},
    Node::AnonymousTableDefine{children} => {print!("AnonymousTableDefine\n"); Some(children)},
    Node::InlineTable{children} => {print!("InlineTable\n"); Some(children)},
    Node::TableHeader{children} => {print!("TableHeader\n"); Some(children)},
    Node::Attribute{children} => {print!("Attribute\n"); Some(children)},
    Node::TableRow{children} => {print!("TableRow\n"); Some(children)},
    Node::AddRow{children} => {print!("AddRow\n"); Some(children)},
    Node::Section{title, children} => {print!("Section({:?})\n", title); Some(children)},
    Node::Block{children, ..} => {print!("Block\n"); Some(children)},
    Node::Statement{children} => {print!("Statement\n"); Some(children)},
    Node::SetData{children} => {print!("SetData\n"); Some(children)},
    Node::SplitData{children} => {print!("SplitData\n"); Some(children)},
    Node::Data{children} => {print!("Data\n"); Some(children)},
    Node::DataWatch{children} => {print!("DataWatch\n"); Some(children)},
    Node::SelectData{name, id, children} => {print!("SelectData({:?}))\n", id); Some(children)},
    Node::DotIndex{children} => {print!("DotIndex\n"); Some(children)},
    Node::SubscriptIndex{children} => {print!("SubscriptIndex\n"); Some(children)},
    Node::Range{children} => {print!("Range\n"); Some(children)},
    Node::Expression{children} => {print!("Expression\n"); Some(children)},
    Node::Function{name, children} => {print!("Function({:?})\n", name); Some(children)},
    Node::MathExpression{children} => {print!("MathExpression\n"); Some(children)},
    Node::Comment{children} => {print!("Comment\n"); Some(children)},
    Node::SelectExpression{children} => {print!("SelectExpression\n"); Some(children)},
    Node::FilterExpression{comparator, children} => {print!("FilterExpression({:?})\n", comparator); Some(children)},
    Node::LogicExpression{operator, children} => {print!("LogicExpression({:?})\n", operator); Some(children)},
    Node::Constraint{children, ..} => {print!("Constraint\n"); Some(children)},
    Node::Identifier{name, id} => {print!("Identifier({}({:#x}))\n", name, id); None},
    Node::String{text} => {print!("String({:?})\n", text); None},
    Node::Constant{value, unit} => {print!("Constant({}{:?})\n", value.to_float(), unit); None},
    Node::Table{name,id} => {print!("Table(#{}({:#x}))\n", name, id); None},
    Node::Define{name,id} => {print!("Define #{}({:?})\n", name, id); None},
    Node::Token{token, byte} => {print!("Token({:?})\n", token); None},
    Node::SelectAll => {print!("SelectAll\n"); None},
    Node::LessThan => {print!("LessThan\n"); None},
    Node::GreaterThan => {print!("GreaterThan\n"); None},
    Node::GreaterThanEqual => {print!("GreaterThanEqual\n"); None},
    Node::LessThanEqual => {print!("LessThanEqual\n"); None},
    Node::Equal => {print!("Equal\n"); None},
    Node::NotEqual => {print!("NotEqual\n"); None},
    Node::Empty => {print!("Empty\n"); None},
    Node::Null => {print!("Null\n"); None},
    // Markdown Nodes
    Node::Title{text} => {print!("Title({:?})\n", text); None},
    Node::ParagraphText{text} => {print!("ParagraphText({:?})\n", text); None},
    Node::UnorderedList{children} => {print!("UnorderedList\n"); Some(children)},
    Node::ListItem{children} => {print!("ListItem\n"); Some(children)},
    Node::Paragraph{children} => {print!("Paragraph\n"); Some(children)},
    Node::InlineCode{children} => {print!("InlineCode\n"); Some(children)},
    Node::CodeBlock{children} => {print!("CodeBlock\n"); Some(children)},
    // Extended Mechdown
    Node::InlineMechCode{children} => {print!("InlineMechCode\n"); Some(children)},
    Node::MechCodeBlock{children} => {print!("MechCodeBlock\n"); Some(children)},
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
    write!(f, "Program: {}\n", self.title.clone().unwrap_or("".to_string()));
    for section in &self.sections {
      write!(f, "  {:?}\n", section);
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
    write!(f, "Section: {}\n", self.title.clone().unwrap_or("".to_string()));
    for element in &self.elements {
      write!(f, "    {:?}\n", element);
    }
    Ok(())
  }
}

#[derive(Clone, PartialEq)]
pub enum Element {
  Block((usize, Node)),
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
  pub constraints: Vec<Constraint>,
  depth: usize,
  row: usize,
  column: usize,
  element: usize,
  table: u64,
  expression: usize,
  pub text: String,
  pub parse_tree: parser::Node,
  pub syntax_tree: Node,
  pub node_stack: Vec<Node>, 
  pub section: usize,
  pub program: usize,
  pub block: usize,
  pub current_char: usize,
  pub current_line: usize,
  pub current_col: usize,
  pub errors: Vec<u64>,
  pub unparsed: String,
}

impl Compiler {

  pub fn new() -> Compiler {
    Compiler {
      blocks: Vec::new(),
      programs: Vec::new(),
      constraints: Vec::new(),
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
      unparsed: String::new(),
      text: String::new(),
      parse_tree: parser::Node::Root{ children: Vec::new() },
      syntax_tree: Node::Root{ children: Vec::new() },
      errors: Vec::new(),
    }
  }

  pub fn clear(&mut self) {
    self.blocks.clear();
    self.programs.clear();
    self.constraints.clear();
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
              let name = format!("mech/inline/{}", Hasher::hash_string(name.clone()));
              let id = Hasher::hash_string(name.clone());
              let block_tree = Node::Block{children: vec![
                            Node::Constraint{children: vec![
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
              "pending" => self.blocks.last_mut().unwrap().state = BlockState::Pending,
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

  pub fn compile_block(&mut self, node: Node) -> Option<(usize, Node)> {
    let block = match node.clone() {
      Node::Fragment{children} |
      Node::Block{children} => {
        let mut block = Block::new();
        let mut formatter = Formatter::new();
        block.text = formatter.format(&node, false);
        block.name = format!("{:?},{:?},{:?}", self.program, self.section, self.block);
        block.id = Hasher::hash_string(block.name.clone()) as usize;
        self.block += 1;
        let mut constraints = Vec::new();
        let mut plan: Vec<(String, HashSet<u64>, HashSet<u64>, Vec<Constraint>)> = Vec::new();
        let mut unsatisfied_constraints: Vec<(String, HashSet<u64>, HashSet<u64>, Vec<Constraint>)> = Vec::new();
        let mut block_produced: HashSet<u64> = HashSet::new();
        let mut block_consumed: HashSet<u64> = HashSet::new();
        for constraint_node in children {
          let constraint_text = formatter.format(&constraint_node, false);
          let mut result = self.compile_constraint(&constraint_node);
          // ----------------------------------------------------------------------------------------------------------
          // Planner
          // ----------------------------------------------------------------------------------------------------------
          // This is the start of a new planner. This will evolve into its own thing I imagine. It's messy and rough now
          let mut produces: HashSet<u64> = HashSet::new();
          let mut consumes: HashSet<u64> = HashSet::new();
          let this_one = result.clone();
          for constraint in result {
            match &constraint {
              Constraint::AliasTable{table, alias} => {
                produces.insert(*alias);
              },
              Constraint::NewTable{id, ..} => {
                match id {
                  TableId::Local(id) => {
                    block_produced.insert(*id);
                    produces.insert(*id)
                  },
                  _ => false,
                };
              },
              Constraint::Append{from_table, to_table} => {
                match from_table {
                  TableId::Local(id) => consumes.insert(*id),
                  _ => false,
                };
              },
              Constraint::Scan{table, indices, output} => {
                match table {
                  TableId::Local(id) => consumes.insert(*id),
                  TableId::Global(id) => false, // TODO handle global
                };
                match output {
                  TableId::Local(id) => produces.insert(*id),
                  _ => false,
                };
              },
              Constraint::Insert{from: (from_table, ..), to: (to_table, to_ixes, ..)} => {
                // TODO Handle other cases of from and parameters
                let to_rows = to_ixes[0];
                match to_rows {
                  Some(Parameter::TableId(TableId::Local(id))) => consumes.insert(id),
                  _ => false,
                };
                match to_table {
                  TableId::Global(id) => produces.insert(*id),
                  _ => false,
                };
              },
              _ => (),
            }
            constraints.push(constraint.clone());
          }
          // If the constraint doesn't consume anything, put it on the top of the plan. It can run any time.
          if consumes.len() == 0 {
            block_produced = block_produced.union(&produces).cloned().collect();
            plan.insert(0, (constraint_text, produces, consumes, this_one));
          // Otherwise, the constraint consumes something, and we have to see if it's satisfied
          } else {
            let mut satisfied = false;
            //let (step_node, step_produces, step_consumes, step_constraints) = step;
            //let intersection: HashSet<u64> = block_produces.intersection(&consumes).cloned().collect();
            let unsatisfied: HashSet<u64> = consumes.difference(&block_produced).cloned().collect();
            if unsatisfied.is_empty() {
              block_produced = block_produced.union(&produces).cloned().collect();
              plan.push((constraint_text, produces, consumes, this_one));
            } else {
              unsatisfied_constraints.push((constraint_text, produces, consumes, this_one));
            }
          }
          // Check if any of the unsatisfied constraints have been met yet. If they have, put them on the plan.
          let mut now_satisfied = unsatisfied_constraints.drain_filter(|unsatisfied_constraint| {
            let (_, unsatisfied_produces, unsatisfied_consumes, _) = unsatisfied_constraint;
            let unsatisfied: HashSet<u64> = unsatisfied_consumes.difference(&block_produced).cloned().collect();
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
        let mut now_satisfied = unsatisfied_constraints.drain_filter(|unsatisfied_constraint| {
          let (_, unsatisfied_produces, unsatisfied_consumes, _) = unsatisfied_constraint;
          let unsatisfied: HashSet<u64> = unsatisfied_consumes.difference(&block_produced).cloned().collect();
          match unsatisfied.is_empty() {
            true => {
              block_produced = block_produced.union(&unsatisfied_produces).cloned().collect();
              true
            }
            false => false
          }
        }).collect::<Vec<_>>();
        plan.append(&mut now_satisfied);
        // ----------------------------------------------------------------------------------------------------------
        for step in plan {
          let (constraint_text, _, _, step_constraints) = step;
          block.add_constraints((constraint_text, step_constraints));
        }
        self.blocks.push(block.clone());
        Some((block.id, node))
      },
      _ => None,
    };
    block
  }

  pub fn compile_constraint(&mut self, node: &Node) -> Vec<Constraint> {
    let mut constraints: Vec<Constraint> = Vec::new();
    match node {
      Node::SetData{children} => {
        let mut result1 = self.compile_constraint(&children[0]);
        result1.remove(0);
        let scan = result1.remove(0);
        let (to, to_ixes) = match scan {
          Constraint::Scan{table, indices, ..} => {
            (table, indices.clone())
          },
          _ => (TableId::Global(0), vec![None, None]), 
        };
        let mut result2 = self.compile_constraint(&children[1]);
        let (from, from_ixes) = match &result2[0] {
          Constraint::NewTable{id, ..} => (id.clone(), vec![None, None]),
          Constraint::Scan{table, indices, output} => (table.clone(), indices.clone()),
          _ => (TableId::Local(0), vec![None, None]), 
        };
        constraints.push(Constraint::Insert{from: (from, from_ixes), to: (to, to_ixes)});
        constraints.append(&mut result1);
        constraints.append(&mut result2);
      },
      Node::SplitData{children} => {
        let mut result = self.compile_constraints(children);
        if result.len() > 2 {
          match result[2] {
            Constraint::Reference{..} => {
              result.remove(3);
              result.remove(2);
              result.remove(1);
            }
            _ => (),
          };
          let alias: u64 = match result[0] {
            Constraint::Identifier{id, ..} => id,
            _ => 0,
          };
          let table = match &result[1] {
            Constraint::NewTable{id, rows, columns} => id.clone(),
            Constraint::AliasTable{table, alias} => table.clone(),
            _ => TableId::Local(0),
          };
          
          let intermediate_table = Hasher::hash_string(format!("tablesplit{:?},{:?}-{:?}-{:?}", self.section, self.block, self.expression, self.table));
          constraints.push(Constraint::AliasTable{table: TableId::Local(intermediate_table), alias});
          constraints.push(Constraint::NewTable{id: TableId::Local(intermediate_table), rows: 1, columns: 1});
          constraints.push(Constraint::Function{operation: Function::TableSplit, fnstring: "table_split".to_string(), parameters: vec![(table, None, None)], output: vec![TableId::Local(intermediate_table)]});
        } else {
          // TODO error if there are no children
        }
        constraints.append(&mut result);
      },
      Node::DataWatch{children} => {
        let mut result = self.compile_constraints(&children);
        match &result[1] {
          Constraint::Scan{table, indices, output} => constraints.push(Constraint::ChangeScan{table: table.clone(), column: indices.clone()}),
          Constraint::Filter{comparator, lhs, rhs, output} => {
            let intermediate_table = Hasher::hash_string(format!("inlinescan{:?},{:?}-{:?}-{:?}", self.section, self.block, self.expression, self.table));
            let column = Hasher::hash_str("column");
            self.table += 1;
            for x in &result {
              match x {
                Constraint::Scan{table: TableId::Global(x), ..} => {

                  constraints.push(Constraint::ChangeScan{table: TableId::Global(x.clone()), column: vec![None, None]});
                },
                _ => (),
              };
            }
            constraints.push(Constraint::ChangeScan{table: TableId::Local(intermediate_table), column: vec![None, None]});
            constraints.push(Constraint::NewTable{id: TableId::Local(intermediate_table), rows: 1, columns: 1});
            constraints.push(Constraint::Function{operation: Function::SetAny, fnstring: "set_any".to_string(), parameters: vec![(TableId::Local(column), None, None), (output.clone(), None, None)], output: vec![TableId::Local(intermediate_table)]});
            constraints.push(Constraint::Identifier{id: column, text: "column".to_string()});
            constraints.append(&mut result);
          },
          _ => (),
        }
      },
      Node::AddRow{children} => {
        //let mut result = self.compile_constraints(&children);
        let mut to_table_constraints = self.compile_constraint(&children[0]);
        let mut from_table_constraints = self.compile_constraint(&children[1]);
        match from_table_constraints[1] {
          Constraint::Reference{..} => {
            from_table_constraints.remove(2);
            from_table_constraints.remove(1);
            from_table_constraints.remove(0);
          }
          _ => (),
        };
        let to_table = match to_table_constraints[0].clone() {
          Constraint::Identifier{id, ..} => TableId::Global(id),
          _ => TableId::Global(0),
        };
        let from_table = match from_table_constraints[0].clone() {
          Constraint::NewTable{id, ..} => id,
          _ => TableId::Global(0),
        };
        constraints.push(Constraint::Append{from_table, to_table});
        constraints.append(&mut from_table_constraints);
        constraints.append(&mut to_table_constraints);
      },
      Node::Statement{children} => {
        constraints.append(&mut self.compile_constraints(children));
      },
      Node::Constraint{children, ..} => {
        self.row = 0;
        self.column = 0;
        constraints.append(&mut self.compile_constraints(children));
      },
      Node::Expression{children} => {
        self.expression += 1;
        let mut result = self.compile_constraints(children);
        constraints.append(&mut result);
      }, 
      Node::VariableDefine{children} => {
        let mut result = self.compile_constraints(children);
        if result.len() > 2 {
          match result[2] {
            Constraint::Reference{..} => {
              result.remove(3);
              result.remove(2);
              result.remove(1);
            }
            _ => (),
          };
          let alias: u64 = match result[0] {
            Constraint::Identifier{id, ..} => id,
            _ => 0,
          };
          let table = match &result[1] {
            Constraint::NewTable{id, rows, columns} => id.clone(),
            Constraint::AliasTable{table, alias} => table.clone(),
            _ => TableId::Local(0),
          };
          constraints.push(Constraint::AliasTable{table, alias});
        } else {
          // TODO error if there are no children
        }
        constraints.append(&mut result);
      },
      Node::TableDefine{children} => {
        let mut result = self.compile_constraints(children);
        if result.len() > 2 {
          match result[2] {
            Constraint::Reference{..} => {
              result.remove(3);
              result.remove(2);
              result.remove(1);
            }
            _ => (),
          };
        }
        if result.len() > 2 {
          let to_table: u64 = match result[0] {
            Constraint::Identifier{id, ..} => {
              id
            },
            _ => 0,
          };
          let from_table: u64 = match &result[1] {
            Constraint::NewTable{id, rows, columns} => {
              *id.unwrap()
            },
            Constraint::AliasTable{table, alias} => {
              match table {
                TableId::Local(id) => *id,
                _ => 0,
              }
            },
            _ => 0,
          };
          constraints.push(Constraint::CopyTable{from_table, to_table});
        } else {
          // TODO error if there are no children
        }
        constraints.append(&mut result);
      },
      Node::InlineTable{children} => {
        let store_table = self.table;
        let store_column = self.column;
        let store_row = self.row;
        let store_expression = self.expression;
        self.row = 1;
        self.expression += 1;
        self.table = Hasher::hash_string(format!("InlineTable{:?},{:?}-{:?}", self.section, self.block, self.expression));
        let mut i = 0;
        let mut column_names = vec![];
        let mut parameters: Vec<(TableId, Option<Parameter>, Option<Parameter>)> = vec![]; 
        let mut compiled = vec![];
        for (ix, child) in children.iter().enumerate() {
          let mut result = self.compile_constraint(child);
          match result[0] {
            Constraint::Identifier{id, ..} => {
              column_names.push(Constraint::TableColumn{table: self.table, column_ix: ix as u64 + 1, column_alias: id});
            }
            _ => (),
          }
          if result.len() > 1 {
            match &result[1] {
              Constraint::NewTable{id, rows, columns} => {
                parameters.push((id.clone(), None, None));
              }
              Constraint::Identifier{id, ..} => {
                parameters.push((TableId::Local(id.clone()),None, None));
              }
              _ => (),
            }
          }
          compiled.append(&mut result);
        }
        let table_reference = Hasher::hash_string(format!("Reference-{:?}", self.table));
        constraints.push(Constraint::NewTable{id: TableId::Local(table_reference), rows: 1, columns: 1});
        constraints.push(Constraint::Reference{table: TableId::Local(self.table), destination: table_reference});
        constraints.push(Constraint::CopyTable{from_table: self.table, to_table: self.table });
        constraints.push(Constraint::NewTable{id: TableId::Local(self.table), rows: self.row as u64, columns: 1});
        constraints.append(&mut column_names);
        constraints.push(Constraint::Function{operation: Function::HorizontalConcatenate, fnstring: "table_horizontal_concatenate".to_string(), parameters, output: vec![TableId::Local(self.table)]});
        constraints.append(&mut compiled);
        self.row = store_row;
        self.column = store_column;
        self.table = store_table;
        self.expression = store_expression;
      }
      Node::Binding{children} => {
        let mut result = self.compile_constraints(children);
        constraints.append(&mut result);
      }
      Node::AnonymousTableDefine{children} => {
        let store_table = self.table;
        let anon_table_rows = 0;
        let anon_table_cols = 0;
        self.table = Hasher::hash_string(format!("AnonymousTable{:?},{:?}-{:?}", self.section, self.block, self.expression));
        let mut parameters: Vec<(TableId, Option<Parameter>, Option<Parameter>)> = vec![]; 
        let mut compiled = vec![];
        let mut alt_id = 0;
        for child in children {
          let mut result = self.compile_constraint(child);
          match &result[0] {
            Constraint::NewTable{id, rows, columns} => {
              parameters.push((id.clone(), None, None));
              match id {
                TableId::Local(id) => alt_id = *id,
                TableId::Global(id) => alt_id = *id,
              };
            },
            Constraint::Scan{table, ..} => {
              match table {
                TableId::Local(id) => alt_id = *id,
                TableId::Global(id) => alt_id = *id,
              };
            }
            _ => (),
          }
          compiled.append(&mut result);
        }
        let table_reference = Hasher::hash_string(format!("Reference-{:?}", self.table));
        if parameters.len() > 1 {
          constraints.push(Constraint::NewTable{id: TableId::Local(table_reference), rows: 1, columns: 1});
          constraints.push(Constraint::Reference{table: TableId::Local(self.table), destination: table_reference});
          constraints.push(Constraint::CopyTable{from_table: self.table, to_table: self.table });
          constraints.push(Constraint::NewTable{id: TableId::Local(self.table), rows: self.row as u64, columns: 1});
          constraints.push(Constraint::Function{operation: Function::VerticalConcatenate, fnstring: "table_vertical_concatenate".to_string(), parameters, output: vec![TableId::Local(self.table)]});
        } else if alt_id != 0 {
          constraints.push(Constraint::NewTable{id: TableId::Local(table_reference), rows: 1, columns: 1});
          constraints.push(Constraint::Reference{table: TableId::Local(self.table), destination: table_reference});
          constraints.push(Constraint::CopyTable{from_table: alt_id, to_table: self.table });
          constraints.push(Constraint::AliasTable{table: TableId::Local(alt_id), alias: self.table});
          constraints.push(Constraint::NewTable{id: TableId::Local(alt_id), rows: 1, columns: 1});
        } else {
          constraints.push(Constraint::NewTable{id: TableId::Local(self.table), rows: 0, columns: 0});
        }
        constraints.append(&mut compiled);
        self.table = store_table;
      },
      Node::TableHeader{children} => {
        let result = self.compile_constraints(children);
        let mut i = 0;
        for constraint in result {
          i += 1;
          match constraint {
            Constraint::Identifier{id, ..} => {
              constraints.push(Constraint::TableColumn{table: self.table, column_ix: i, column_alias: id});
              constraints.push(constraint);             
            }
            _ => (),
          }
        }
      },
      Node::FilterExpression{comparator, children} => {
        self.expression += 1;
        self.table = Hasher::hash_string(format!("FilterExpression{:?},{:?}-{:?}", self.section, self.block, self.expression));
        let mut output = TableId::Local(self.table);
        let mut parameters: Vec<Vec<Constraint>> = vec![];
        for child in children {
          self.column += 1;
          parameters.push(self.compile_constraint(child));
        }
        let mut parameter_registers: Vec<(TableId, Option<Parameter>, Option<Parameter>)> = vec![];
        for parameter in &parameters {
          match &parameter[0] {
            Constraint::NewTable{id, rows, columns} => {
              parameter_registers.push((id.clone(), None, None));
            },
            Constraint::Scan{table, indices, output} => {
              parameter_registers.push((table.clone(), indices[0].clone(), indices[1].clone()));
            },
            Constraint::Function{operation, fnstring, parameters, output} => {
              for o in output {
                parameter_registers.push((o.clone(), None, None));
              }
            },
            _ => (),
          };
        }
        constraints.push(Constraint::NewTable{id: output.clone(), rows: 0, columns: 0});
        constraints.push(Constraint::Filter{comparator: comparator.clone(), lhs: parameter_registers[0].clone(), rhs: parameter_registers[1].clone(), output: output.clone()});
        for mut p in &parameters {
          constraints.append(&mut p.clone());
        }  
      },
      Node::LogicExpression{operator, children} => {
        self.expression += 1;
        self.table = Hasher::hash_string(format!("LogicExpression{:?},{:?}-{:?}", self.section, self.block, self.expression));
        let mut output = TableId::Local(self.table);
        let mut parameters: Vec<Vec<Constraint>> = vec![];
        for child in children {
          self.column += 1;
          parameters.push(self.compile_constraint(child));
        }
        let mut parameter_registers: Vec<(TableId, Option<Parameter>, Option<Parameter>)> = vec![];
        for parameter in &parameters {
          match &parameter[0] {
            Constraint::NewTable{id, rows, columns} => {
              parameter_registers.push((id.clone(), None, None));
            },
            Constraint::Scan{table, indices, output} => {
              parameter_registers.push((table.clone(), indices[0].clone(), indices[1].clone()));
            },
            Constraint::Function{operation, fnstring, parameters, output} => {
              for o in output {
                parameter_registers.push((o.clone(), None, None));
              }
            },
            _ => (),
          };
        }
        constraints.push(Constraint::NewTable{id: output.clone(), rows: 0, columns: 0});
        constraints.push(Constraint::Logic{logic: operator.clone(), lhs: parameter_registers[0].clone(), rhs: parameter_registers[1].clone(), output: output.clone()});
        for mut p in &parameters {
          constraints.append(&mut p.clone());
        }  
      },      
      Node::Range{children} => {        
        let table_id = TableId::Local(Hasher::hash_string(format!("RangeExpression{:?},{:?}-{:?}", self.section, self.block, self.expression)));
        let mut arguments = vec![];
        let mut compiled = vec![];
        for child in children {
          let mut result = self.compile_constraint(child);
          match &result[0] {
            Constraint::NewTable{id, rows, columns} => arguments.push(id.clone()),
            _ => (),
          };
          compiled.append(&mut result);
        }
        constraints.push(Constraint::NewTable{id: table_id.clone(), rows: 0, columns: 0});
        if arguments.len() == 2 {
          constraints.push(Constraint::Range{table: table_id.clone(), start: arguments[0].clone(), end: arguments[1].clone()});
        }
        constraints.append(&mut compiled);
      },
      Node::MathExpression{children} => {
        let store_row = self.row;
        let store_col = self.column;
        let store_table = self.table;
        self.row = 1;
        self.column = 1;
        self.expression += 1;
        self.table = Hasher::hash_string(format!("MathExpression{:?},{:?}-{:?}", self.section, self.block, self.expression));
        let mut result = self.compile_constraints(children);
        // If the math expression is just a constant, we don't need a new internal table for it.
        //constraints.push(Constraint::Reference{table: self.table, rows: vec![0], columns: vec![1], destination: (store_table, store_row as u64, store_col as u64)});
        constraints.append(&mut result);
        self.row = store_row;
        self.column = store_col;
        self.table = store_table;
      },
      Node::Function{name, children} => {
        self.expression += 1;
        self.table = Hasher::hash_string(format!("Function{:?},{:?}-{:?}", self.section, self.block, self.expression));
        constraints.push(Constraint::NewTable{id: TableId::Local(self.table), rows: 0, columns: 0});        
        let operation = match name.as_ref() {
          "+" => Function::Add,
          "-" => Function::Subtract,
          "*" => Function::Multiply,
          "/" => Function::Divide,
          "^" => Function::Power,
          "math/round" => Function::MathRound,
          "math/floor" => Function::MathFloor,
          "math/sin" => Function::MathSin,
          "math/cos" => Function::MathCos,
          "stat/sum" => Function::StatSum,
          "set/any" => Function::SetAny,
          "table/split" => Function::TableSplit,
          _ => Function::Undefined,
        };
        let mut output: Vec<TableId> = vec![TableId::Local(self.table)];
        let mut parameters: Vec<Vec<Constraint>> = vec![];
        for child in children {
          self.column += 1;
          parameters.push(self.compile_constraint(child));
        }
        let mut parameter_registers: Vec<(TableId, Option<Parameter>, Option<Parameter>)> = vec![];
        for parameter in &parameters {
          match &parameter[0] {
            /*Constraint::Constant{table, row, column, value} => {
              parameter_registers.push((*table, *row, *column));
            },*/
            Constraint::Identifier{id, ..} => {
              parameter_registers.push((TableId::Local(*id),None, None));
              match &parameter[1] {
                Constraint::NewTable{id, rows, columns} => {
                  parameter_registers.push((id.clone(), None, None));
                },
                _ => (),
              }
            },
            Constraint::NewTable{id, rows, columns} => {
              parameter_registers.push((id.clone(), None, None));
            },
            Constraint::Scan{table, indices, output} => {
              parameter_registers.push((table.clone(), indices[0].clone(), indices[1].clone()));
            },
            Constraint::Function{operation, fnstring, parameters, output} => {
              for o in output {
                parameter_registers.push((o.clone(), None, None));
              }
            },
            _ => (),
          };
        }
        constraints.push(Constraint::Function{operation, fnstring: name.to_string(), parameters: parameter_registers, output});
        for mut p in &parameters {
          constraints.append(&mut p.clone());
        }
      },
      Node::Table{name, id} => {
        self.table = Hasher::hash_string(format!("Table{:?},{:?}-{:?}", self.section, self.block, name));
        constraints.push(Constraint::Identifier{id: *id, text: name.clone()});
      },
      Node::SelectData{name, id, children} => {
        let mut compiled = vec![];
        let mut indices: Vec<Option<Parameter>> = vec![];
        let mut scan_id = id.clone();
        for child in children {
          match child {
            Node::DotIndex{children} |
            Node::SubscriptIndex{children} => {
              for child in children {
                let mut result = self.compile_constraint(child);
                match &result[0] {
                  Constraint::NewTable{ref id, rows, columns} => indices.push(Some(Parameter::TableId(id.clone()))),
                  Constraint::Null => indices.push(None),
                  Constraint::Scan{table, ..} => indices.push(Some(Parameter::TableId(table.clone()))),
                  Constraint::Identifier{id, ..} => indices.push(Some(Parameter::Index(Index::Alias(id.clone())))),
                  _ => (),
                };
                compiled.append(&mut result);
              }
            }
            Node::Null => indices.push(None),
            _ => (),
          }
          if indices.len() == 2 {
            compiled.reverse();
            constraints.append(&mut compiled);
            let scan_output = Hasher::hash_string(format!("ScanTable{:?},{:?}-{:?}-{:?}", self.section, self.block, scan_id, indices));
            constraints.push(Constraint::Scan{table: scan_id.clone(), indices: indices.clone(), output: TableId::Local(scan_output)});
            constraints.push(Constraint::NewTable{id: TableId::Local(scan_output), rows: 0, columns: 0});
            scan_id = TableId::Local(scan_output);
            indices.clear();
            compiled.clear();
          }
        }
        // example: #x{1}
        if indices.len() == 1 {
          compiled.reverse();
          constraints.append(&mut compiled);
          let scan_output = Hasher::hash_string(format!("ScanTable{:?},{:?}-{:?}-{:?}", self.section, self.block, scan_id, indices));
          constraints.push(Constraint::Scan{table: scan_id.clone(), indices: indices.clone(), output: TableId::Local(scan_output)});
          constraints.push(Constraint::NewTable{id: TableId::Local(scan_output), rows: 0, columns: 0});
          scan_id = TableId::Local(scan_output);
          indices.clear();
          compiled.clear();
        }
        constraints.reverse();
      },
      Node::SubscriptIndex{children} => {
        constraints.append(&mut self.compile_constraints(children));
      },
      Node::SelectAll => {
        constraints.push(Constraint::Null);
      },
      Node::Attribute{children} => {
        self.column += 1;
        constraints.append(&mut self.compile_constraints(children));
      },
      Node::TableRow{children} => {
        self.row += 1;
        self.column = 0;
        let mut parameter_registers: Vec<(TableId, Option<Parameter>, Option<Parameter>)> = vec![]; 
        let mut compiled = vec![];
        let table = Hasher::hash_string(format!("TableRow{:?},{:?}", self.table, self.row));
        for child in children {
          let mut result = self.compile_constraint(child);
          match &result[0] {
            Constraint::Identifier{id, ..} => {
              parameter_registers.push((TableId::Local(id.clone()), None, None));
            },
            Constraint::NewTable{id, rows, columns} => {
              parameter_registers.push((id.clone(), None, None));
            },
            Constraint::Scan{table, indices, output} => {
              parameter_registers.push((table.clone(), indices[0].clone(), indices[1].clone()));
            },
            _ => (),
          }
          compiled.append(&mut result);
        }
        if parameter_registers.len() > 1 {
          constraints.push(Constraint::NewTable{id: TableId::Local(table), rows: 0, columns: 0});
          constraints.push(Constraint::Function{operation: Function::HorizontalConcatenate, fnstring: "table_horizontal_concatenate".to_string(), parameters: parameter_registers, output: vec![TableId::Local(table)]});
        }
        constraints.append(&mut compiled);
      },
      Node::Column{children} => {
        self.column += 1;       
        for child in children {
          let mut result = self.compile_constraint(child);
          constraints.append(&mut result);
        }
      },
      Node::Empty => {
        let table = Hasher::hash_str("Empty");
        constraints.push(Constraint::NewTable{id: TableId::Local(table), rows: 1, columns: 1});
        constraints.push(Constraint::Empty{table: TableId::Local(table), row: Index::Index(1), column: Index::Index(1)});
      },
      Node::Identifier{name, id} => {
        constraints.push(Constraint::Identifier{id: *id, text: name.clone()});
      },
      Node::Constant{value, unit} => {
        let table = Hasher::hash_string(format!("Constant-{:?}", value.to_float()));
        constraints.push(Constraint::NewTable{id: TableId::Local(table), rows: 1, columns: 1});
        constraints.push(Constraint::Constant{table: TableId::Local(table), row: Index::Index(1), column: Index::Index(1), value: *value, unit: unit.clone()});
      },
      Node::String{text} => {
        let table = Hasher::hash_string(format!("String-{:?}", *text));
        constraints.push(Constraint::NewTable{id: TableId::Local(table), rows: 1, columns: 1});
        constraints.push(Constraint::String{table: TableId::Local(table), row: Index::Index(1), column: Index::Index(1), value: text.clone()});
      },
      Node::Null => constraints.push(Constraint::Null),
      _ => ()
    }
    constraints
  }

  pub fn compile_constraints(&mut self, nodes: &Vec<Node>) -> Vec<Constraint> {
    let mut compiled = Vec::new();
    for node in nodes {
      let mut result = self.compile_constraint(node);
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
                select_data_children = vec![Node::Null; 2];
              }
              select_data_children.reverse();
              compiled.push(Node::SelectData{name, id: TableId::Global(id), children: select_data_children.clone()});
            }, 
            Node::Identifier{name, id} => {
              if select_data_children.is_empty() {
                select_data_children = vec![Node::Null; 2];
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
      parser::Node::DataWatch{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(Node::DataWatch{children});
      },
      parser::Node::SelectAll => {
        compiled.push(Node::SelectAll);
      },
      parser::Node::Range{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::Range{children: result});
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
        compiled.push(Node::Column{children});
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
      parser::Node::Constraint{children} => {
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
          compiled.push(Node::Constraint{children});
        }
      },
      parser::Node::SelectExpression{children} => {
        let result = self.compile_nodes(children);
        compiled.push(Node::SelectExpression{children: result});
      },
      parser::Node::FilterExpression{children} => {
        let result = self.compile_nodes(children);
        let mut comparator = Comparator::Undefined;
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::LessThan => comparator = Comparator::LessThan,
            Node::GreaterThan => comparator = Comparator::GreaterThan,
            Node::GreaterThanEqual => comparator = Comparator::GreaterThanEqual,
            Node::LessThanEqual => comparator = Comparator::LessThanEqual,
            Node::Equal => comparator = Comparator::Equal,
            Node::NotEqual => comparator = Comparator::NotEqual,
            _ => children.push(node),
          }
        }
        compiled.push(Node::FilterExpression{comparator, children});
      },
      parser::Node::LogicExpression{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        let mut operator = Logic::Undefined;
        for node in result {
          match node {
            Node::And => operator = Logic::And,
            Node::Or => operator = Logic::Or,
            _ => children.push(node),
          }
        }
        compiled.push(Node::LogicExpression{operator, children});
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
            Node::Constant{..} => {
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
                    children: vec![Node::Column{
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
                    children: vec![Node::Column{
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
        result.reverse();
        compiled.push(Node::DotIndex{children: result});
      },
      parser::Node::SubscriptIndex{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<Node> = Vec::new();
        for node in result {
          match node {
            Node::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(Node::SubscriptIndex{children});
      },
      parser::Node::Table{children} => {
        let result = self.compile_nodes(children);
        match &result[0] {
          Node::Identifier{name, id} => compiled.push(Node::Table{name: name.to_string(), id: *id}),
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
            Node::Constant{value, unit} => {
              quantity = quantity.add(value).unwrap();
            },
            Node::Identifier{name: word, id} => unit = Some(word),
            _ => (),
          }
        }
        compiled.push(Node::Constant{value: quantity, unit});
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
            Node::Constant{value, unit} => quantities.push(value),
            _ => (),
          }
        }
        let mut quantity = make_quantity(value as i64,0,0);
        for q in quantities {
          quantity = quantity.add(q).unwrap();
        }
        compiled.push(Node::Constant{value: quantity, unit: None});
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
        let quantity = make_quantity(value as i64,(1 - place as i64),0);
        compiled.push(Node::Constant{value: quantity, unit: None});
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
            Node::Constant{value, unit} => text_node.push_str(&format!("{}", value.to_float())),
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
            Node::Constant{value, unit} => word.push_str(&format!("{}", value.to_float())),
            _ => compiled.push(node),
          }
        }
        let id = Hasher::hash_string(word.clone());
        compiled.push(Node::Identifier{name: word, id});
      },
      // Math
      parser::Node::L1{children} |
      parser::Node::L2{children} |
      parser::Node::L3{children} |
      parser::Node::L4{children} => {
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
      parser::Node::L1Infix{children} |
      parser::Node::L2Infix{children} |
      parser::Node::L3Infix{children} => {
        let result = self.compile_nodes(children);
        let operator = &result[0].clone();
        let input = &result[1].clone();
        let name: String = match operator {
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
        let mut input = vec![Node::Constant{value: 0, unit: None}];
        input.push(result[0].clone());
        compiled.push(Node::Function{ name: "-".to_string(), children: input });
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
      parser::Node::ParentheticalExpression{children} => {
        let mut result = self.compile_nodes(children);
        compiled.push(result[0].clone());
      },
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
      parser::Node::StateMachine{children} |
      parser::Node::Transition{children} |
      parser::Node::Transitions{children} |
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
      parser::Node::Constant{children} |
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
      _ => println!("Unhandled Node: {:?}", node),
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
      println!("Unhandled Byte {:?}", byte);
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