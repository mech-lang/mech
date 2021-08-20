// # Mech Syntax Compiler

// ## Preamble

/*use mech_core::{Value, Block, BlockState, ValueMethods, Transformation, TableIndex, TableId, Register, NumberLiteral, NumberLiteralKind};
use mech_core::{Quantity, QuantityMath};

use mech_core::{Error, ErrorType};*/
use mech_core::hash_string;

use crate::ast::Ast;
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
                    block.errors.insert(Error {
                      block_id: block.id,
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
        let mut to_copy: HashMap<TableId, Vec<Transformation>> = HashMap::new();
        let mut new_steps = vec![];
        for step in plan {
          let (step_text, _, _, mut step_transformations) = step;
          let mut rtfms = step_transformations.clone();
          rtfms.reverse();
          for tfm in rtfms {
            match tfm {
              Transformation::TableReference{table_id, reference} => {
                let referenced_table_id = reference.as_reference().unwrap();
                block.plan.push(Transformation::Function{
                  name: *TABLE_COPY,
                  arguments: vec![(0,TableId::Local(referenced_table_id), TableIndex::All, TableIndex::All)],
                  out: (TableId::Global(referenced_table_id), TableIndex::All, TableIndex::All),
                });
                match to_copy.get(&TableId::Local(referenced_table_id)) {
                  Some(aliases) => {
                    for alias in aliases {
                      match alias {
                        Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
                          new_steps.push(Transformation::ColumnAlias{table_id: TableId::Global(*table_id.unwrap()), column_ix: *column_ix, column_alias: *column_alias});                          
                        }
                        _ => (),
                      }
                    }
                  }
                  _ => (),
                }
              }
              Transformation::Whenever{..} => {
                block.plan.push(tfm.clone());
              }
              Transformation::Select{..} => {
                block.plan.push(tfm.clone());
              }
              Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
                let aliases = to_copy.entry(table_id).or_insert(Vec::new());
                aliases.push(tfm.clone());
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
          step_transformations.append(&mut new_steps);
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

          // Combine steps with set, e.g.:
          // 1. math/add(#ball{:,:}, moe-veg-cut-six{:,:}) -> yel-ohi-sod-fil{:,:}
          // 2. table/set(yel-ohi-sod-fil{:,:}) -> #ball{:,:}
          // Becomes
          // 1. math/add(#ball{:,:}, moe-veg-cut-six{:,:}) -> #ball{:,:}
          let mut include = vec![];
          let mut exclude: HashSet<Transformation> = HashSet::new();
          'step_loop: for step in &block.plan {
            match step {
              Transformation::Function{name, arguments, out} => {
                for step2 in &block.plan {
                  match step2 {
                    Transformation::Function{name: name2, arguments: arguments2, out: out2} => {
                      if *name2 == *TABLE_SET  && arguments2.len() == 1 {
                        let (_, table, row, column) = arguments2[0];
                        if (table,row,column) == *out && row == TableIndex::All && column == TableIndex::All {
                          include.push(Transformation::Function{name: *name, arguments: arguments.clone(), out: *out2});
                          exclude.insert(step2.clone());
                          exclude.insert(step.clone());
                          continue 'step_loop;
                        }
                      }
                    }
                    _ => (),
                  }
                }
                match exclude.contains(&step) {
                  false => include.push(step.clone()),
                  _ => (),
                }
              }
              _ => {
                match exclude.contains(&step) {
                  false => include.push(step.clone()),
                  _ => (),
                }
              },
            }
          }
          //block.plan = include;
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
      Node::Quantity{value, unit} => {
        let table = hash_string(&format!("Quantity-{:?}{:?}", value.to_f32(), unit));

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

  pub fn compile_nodes(&mut self, nodes: Vec<parser::Node>) -> Vec<Node> {
    let mut compiled = Vec::new();
    for node in nodes {
      compiled.append(&mut self.build_syntax_tree(node));
    }
    compiled
  }

}*/