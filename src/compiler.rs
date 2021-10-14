// # Mech Syntax Compiler

// ## Preamble

use mech_core::{hash_string, Value, hash_chars, NumberLiteral, Block, Transformation, Table, TableId, TableIndex, NumberLiteralKind};

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
  static ref TABLE_HORIZONTAL__CONCATENATE: u64 = hash_string("table/horizontal-concatenate");
  static ref TABLE_VERTICAL__CONCATENATE: u64 = hash_string("table/vertical-concatenate");
  static ref TABLE_SET: u64 = hash_string("table/set");
  static ref TABLE_APPEND__ROW: u64 = hash_string("table/append-row");
  static ref TABLE_SPLIT: u64 = hash_string("table/split");
  static ref SET_ANY: u64 = hash_string("set/any");
}

fn get_blocks(nodes: &Vec<Node>) -> Vec<Node> {
  let mut blocks = Vec::new();
  for n in nodes {
    match n {
      Node::Statement{..} |
      Node::Block{..} => blocks.push(n.clone()),
      Node::Root{children} |
      Node::Body{children} |
      Node::Section{children,..} |
      Node::Program{children,..} |
      Node::Fragment{children} |
      Node::MechCodeBlock{children} => {
        blocks.append(&mut get_blocks(children));
      }
      _ => (), 
    }
  }
  blocks
}

pub struct Compiler {

}

impl Compiler {

  pub fn new() -> Compiler {
    Compiler{}
  }

  pub fn compile_blocks(&mut self, nodes: &Vec<Node>) -> Vec<Block> {
    let mut blocks = Vec::new();
    for b in get_blocks(nodes) {
      let mut block = Block::new();
      let tfms = self.compile_node(&b);
      for tfm in tfms {
        block.add_tfm(tfm);
      }
      blocks.push(block);
    }
    blocks
  }

  pub fn compile_nodes(&mut self, nodes: &Vec<Node>) -> Vec<Transformation> {
    let mut compiled = Vec::new();
    for node in nodes {
      let mut result = self.compile_node(node);
      compiled.append(&mut result);
    }
    compiled
  }

  pub fn compile_node(&mut self, node: &Node) -> Vec<Transformation> {
    let mut tfms = vec![];
    match node {
      Node::Identifier{name, id} => {
        tfms.push(Transformation::Identifier{name: name.to_vec(), id: *id});
      },
      Node::Empty => {
        let table_id = TableId::Local(hash_string("_"));
        tfms.push(Transformation::NewTable{table_id: table_id.clone(), rows: 1, columns: 1 });
        tfms.push(Transformation::Constant{table_id: table_id, value: Value::Empty});
      },
      Node::True => {
        let table_id = TableId::Local(hash_string("true"));
        tfms.push(Transformation::NewTable{table_id: table_id.clone(), rows: 1, columns: 1 });
        tfms.push(Transformation::Constant{table_id: table_id, value: Value::Bool(true)});
      },
      Node::False => {
        let table_id = TableId::Local(hash_string("false"));
        tfms.push(Transformation::NewTable{table_id: table_id.clone(), rows: 1, columns: 1 });
        tfms.push(Transformation::Constant{table_id: table_id, value: Value::Bool(false)});
      },
      Node::NumberLiteral{kind, bytes} => {
        let bytes_vec = bytes.to_vec();
        /*let kind = match bytes_vec.len() {
          1 => NumberLiteralKind::U8,
          2 => NumberLiteralKind::U16,
          3..=4 => NumberLiteralKind::U32,
          5..=8 => NumberLiteralKind::U64,
          9..=16 => NumberLiteralKind::U128,
          _ => NumberLiteralKind::U128, // TODO Error
        };*/
        let table_id = TableId::Local(hash_string(&format!("{:?}{:?}", kind, bytes_vec)));
        tfms.push(Transformation::NewTable{table_id: table_id, rows: 1, columns: 1 });
        tfms.push(Transformation::NumberLiteral{kind: *kind, bytes: bytes_vec});
      },
      Node::Table{name, id} => {
        //self.strings.insert(*id, name.to_string());
        tfms.push(Transformation::NewTable{table_id: TableId::Global(*id), rows: 1, columns: 1});
      }
      Node::TableDefine{children} => {
        let mut output = self.compile_node(&children[0]);
        // Get the output table id
        let output_table_id = match output[0] {
          Transformation::NewTable{table_id,..} => {
            Some(table_id)
          },
          _ => None,
        };

        tfms.append(&mut output);
        let mut input = self.compile_node(&children[1]);
        let (input_table_id, input_indices) = match &mut input[0] {
          Transformation::NewTable{table_id,..} => {
            Some((table_id.clone(),vec![(TableIndex::All, TableIndex::All)]))
          },
          Transformation::Select{table_id,ref indices,..} => {
            let table_id = table_id.clone();
            let indices = indices.clone();
            input.remove(0);
            Some((table_id,indices))
          },
          _ => None,
        }.unwrap();
        //tfms.push(Transformation::TableAlias{table_id: input_table_id.unwrap(), alias: variable_name});
        tfms.append(&mut input);
        tfms.push(Transformation::Select{
          table_id: input_table_id.clone(), 
          indices: input_indices, 
          out: output_table_id.unwrap()
        });
      }
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
        let mut input = self.compile_node(&children[1]);
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
          // get the argument identifier off the function binding. Default to 0 if there is no named arg
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
          let mut result = self.compile_node(&child);
          match &result[0] {
            Transformation::NewTable{table_id,..} => {
              args.push((arg, *table_id, TableIndex::All, TableIndex::All));
            },
            Transformation::Select{table_id, indices, out} => {
              let (row, column) = indices[0];
              args.push((arg, *table_id, row, column));
              result.remove(0);
            }
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
      Node::AnonymousTableDefine{children} => {
        let anon_table_id = hash_string(&format!("anonymous-table: {:?}",children));
        let mut table_children = children.clone();
        let mut header_tfms = Vec::new();
        let mut column_aliases = Vec::new();
        let mut body_tfms = Vec::new();
        let mut columns = 1;
        match &table_children[0] {
          Node::TableHeader{children} => {
            let mut result = self.compile_nodes(&children);
            columns = result.len();
            for (ix,tfm) in result.iter().enumerate() {
              match tfm {
                Transformation::Identifier{name,id} => {
                  let alias_tfm = move |x| {
                    Transformation::ColumnAlias{table_id: TableId::Local(x), column_ix: ix.clone(), column_alias: id.clone()}
                  };
                  column_aliases.push(alias_tfm);
                }
                _ => (),
              }
            }
            header_tfms.append(&mut result);
            table_children.remove(0);
          }
          _ => (),
        };
        if table_children.len() > 1  {
          let mut args: Vec<(u64, TableId, TableIndex, TableIndex)> = vec![];
          let mut result_tfms = vec![];
          for child in table_children {
            let mut result = self.compile_node(&child);
            match &result[0] {
              Transformation::NewTable{table_id,..} => {
                args.push((0,table_id.clone(),TableIndex::All, TableIndex::All));
              }
              Transformation::Select{table_id, indices, ..} => {
                let (row,col) = indices[0];
                args.push((0,table_id.clone(),row, col));
                result.remove(0);
              }
              _ => (),
            }  
            result_tfms.append(&mut result);       
          }
          header_tfms.insert(0,Transformation::NewTable{table_id: TableId::Local(anon_table_id), rows: 1, columns: args.len()});
          body_tfms.append(&mut result_tfms);
          body_tfms.push(Transformation::Function{
            name: *TABLE_VERTICAL__CONCATENATE,
            arguments: args,
            out: (TableId::Local(anon_table_id), TableIndex::All, TableIndex::All),
          });
          tfms.append(&mut header_tfms);
          tfms.append(&mut body_tfms);
        } else {
          let mut result = self.compile_nodes(children);
          tfms.append(&mut result);          
        }
      },
      Node::TableRow{children} => {
        if children.len() > 1 {
          let row_id = hash_string(&format!("horzcat:{:?}", children));
          let mut args: Vec<(u64, TableId, TableIndex, TableIndex)> = vec![];
          let mut result_tfms = vec![];
          for child in children {
            let mut result = self.compile_node(child);
            match &result[0] {
              Transformation::NewTable{table_id,..} => {
                args.push((0,table_id.clone(),TableIndex::All, TableIndex::All));
              }
              Transformation::Select{table_id, indices, ..} => {
                let (row,col) = indices[0];
                args.push((0,table_id.clone(),row, col));
                result.remove(0);
              }
              _ => (),
            }  
            result_tfms.append(&mut result);       
          }
          tfms.push(Transformation::NewTable{table_id: TableId::Local(row_id), rows: 1, columns: args.len()});
          tfms.append(&mut result_tfms);
          tfms.push(Transformation::Function{
            name: *TABLE_HORIZONTAL__CONCATENATE,
            arguments: args,
            out: (TableId::Local(row_id), TableIndex::All, TableIndex::All),
          });
        } else {
          let mut result = self.compile_nodes(children);
          tfms.append(&mut result);            
        }
      },
      Node::TableColumn{children} => {
        let mut result = self.compile_nodes(children);
        tfms.append(&mut result);
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
                          let mut result = self.compile_node(child);
                          match &result[1] {
                            Transformation::NumberLiteral{kind, bytes} => {
                              let value = NumberLiteral{kind: *kind, bytes: bytes.clone()};
                              if indices.len() == 2 && indices[0] == TableIndex::All {
                                indices[0] = TableIndex::Index(value.as_usize());
                              } else {
                                indices.push(TableIndex::Index(value.as_usize()));
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
                    let mut result = self.compile_node(child);
                    match &result[1] {
                      Transformation::NumberLiteral{kind, bytes} => {
                        let value = NumberLiteral{kind: *kind, bytes: bytes.clone()};
                        if indices.len() == 2 && indices[0] == TableIndex::All {
                          indices[0] = TableIndex::Index(value.as_usize());
                        } else {
                          indices.push(TableIndex::Index(value.as_usize()));
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
        println!("{:?}", all_indices);
        if all_indices.len() > 1 {
          tfms.push(Transformation::NewTable{table_id: TableId::Local(out_id), rows: 1, columns: 1});
          tfms.push(Transformation::Select{table_id: *id, indices: all_indices, out: TableId::Local(out_id)});
        } else {
          //tfms.push(Transformation::NewTable{table_id: TableId::Local(out_id), rows: 1, columns: 1});
          tfms.push(Transformation::Select{table_id: *id, indices: all_indices, out: TableId::Local(out_id)});
        }
        tfms.append(&mut local_tfms);
      }
      Node::Program{children, ..} |
      Node::Section{children, ..} |
      Node::Attribute{children} |
      Node::Transformation{children} |
      Node::Statement{children} |
      Node::Fragment{children} |
      Node::Block{children} |
      Node::MathExpression{children} |
      Node::Expression{children} |
      Node::TableRow{children} |
      Node::TableColumn{children} |
      Node::Root{children} => {
        let mut result = self.compile_nodes(children);
        tfms.append(&mut result);
      }
      Node::Null => (),
      x => println!("Unhandled Node {:?}", x),
    }
    tfms
  }
}