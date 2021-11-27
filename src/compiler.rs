// # Mech Syntax Compiler

// ## Preamble

use mech_core::*;

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
  static ref TABLE_APPEND: u64 = hash_str("table/append");
  static ref TABLE_HORIZONTAL__CONCATENATE: u64 = hash_str("table/horizontal-concatenate");
  static ref TABLE_VERTICAL__CONCATENATE: u64 = hash_str("table/vertical-concatenate");
}

fn get_blocks(nodes: &Vec<Node>) -> Vec<Node> {
  let mut blocks = Vec::new();
  for n in nodes {
    match n {
      Node::Statement{..} |
      Node::Block{..} => blocks.push(n.clone()),
      Node::MechCodeBlock{children} => {
        // Do something with the block state string.
        // ```mech: disabled
        match &children[0] {
          Node::String{text} => {
            let block_state = text.iter().collect::<String>();
            if block_state != "disabled".to_string() {
              blocks.append(&mut get_blocks(children));
            }
          }
          _ => (),
        }
      }
      Node::Root{children} |
      Node::Body{children} |
      Node::Section{children,..} |
      Node::Program{children,..} |
      Node::Fragment{children} => {
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

  pub fn compile_blocks(&mut self, nodes: &Vec<Node>) -> Result<Vec<Block>,MechError> {
    let mut blocks = Vec::new();
    for b in get_blocks(nodes) {
      let mut block = Block::new();
      let mut tfms = self.compile_node(&b)?;
      let tfms_before = tfms.clone();
      tfms.sort();
      for tfm in tfms {
        block.add_tfm(tfm);
      }
      blocks.push(block);
    }
    Ok(blocks)
  }

  pub fn compile_nodes(&mut self, nodes: &Vec<Node>) -> Result<Vec<Transformation>,MechError> {
    let mut compiled = Vec::new();
    for node in nodes {
      let mut result = self.compile_node(node)?;
      compiled.append(&mut result);
    }
    Ok(compiled)
  }

  pub fn compile_node(&mut self, node: &Node) -> Result<Vec<Transformation>,MechError> {
    let mut tfms = vec![];
    match node {
      Node::Identifier{name, id} => {
        tfms.push(Transformation::Identifier{name: name.to_vec(), id: *id});
      },
      Node::Empty => {
        let table_id = TableId::Local(hash_str("_"));
        tfms.push(Transformation::NewTable{table_id: table_id.clone(), rows: 1, columns: 1 });
        tfms.push(Transformation::Constant{table_id: table_id, value: Value::Empty});
      },
      Node::True => {
        let table_id = TableId::Local(hash_str("true"));
        tfms.push(Transformation::NewTable{table_id: table_id.clone(), rows: 1, columns: 1 });
        tfms.push(Transformation::Constant{table_id: table_id, value: Value::Bool(true)});
      },
      Node::False => {
        let table_id = TableId::Local(hash_str("false"));
        tfms.push(Transformation::NewTable{table_id: table_id.clone(), rows: 1, columns: 1 });
        tfms.push(Transformation::Constant{table_id: table_id, value: Value::Bool(false)});
      },
      Node::String{text} => {
        let table_id = TableId::Local(hash_str(&format!("string: {:?}", text)));
        tfms.push(Transformation::NewTable{table_id: table_id.clone(), rows: 1, columns: 1 });
        tfms.push(Transformation::Constant{table_id: table_id, value: Value::String(text.to_vec())});
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
        let table_id = TableId::Local(hash_str(&format!("{:?}{:?}", kind, bytes_vec)));
        tfms.push(Transformation::NewTable{table_id: table_id, rows: 1, columns: 1 });
        tfms.push(Transformation::NumberLiteral{kind: *kind, bytes: bytes_vec});
      },
      Node::Table{name, id} => {
        //self.strings.insert(*id, name.to_string());
        tfms.push(Transformation::NewTable{table_id: TableId::Global(*id), rows: 1, columns: 1});
      }
      Node::SetData{children} => {

        let mut src = self.compile_node(&children[1])?;
        let mut dest = self.compile_node(&children[0])?;

        let (src_table_id, src_indices) = match &mut src[0] {
          Transformation::NewTable{table_id,..} => {
            Some((table_id.clone(),vec![(TableIndex::All, TableIndex::All)]))
          },
          Transformation::Select{table_id,ref indices} => {
            let table_id = table_id.clone();
            let indices = indices.clone();
            src.remove(0);
            Some((table_id,indices))
          },
          Transformation::TableReference{table_id, reference: Value::Reference(id)} => {
            let table_id = id.clone();
            src.remove(0);
            src.remove(0);
            Some((table_id.clone(),vec![(TableIndex::All, TableIndex::All)]))
          },
          _ => None,
        }.unwrap();     

        match &mut dest[0] {
          Transformation::Select{table_id, indices} => {
            let dest_id = table_id.clone();
            let (dest_row, dest_col) = indices[0];
            dest.remove(0);
            let (src_row,src_col) = src_indices[0];
            tfms.push(Transformation::Set{src_id: src_table_id, src_row, src_col, dest_id, dest_row, dest_col});
          }
          _ => (),
        }

        tfms.append(&mut dest);
        tfms.append(&mut src);
      }
      Node::TableDefine{children} => {
        let mut output = self.compile_node(&children[0])?;
        // Get the output table id
        let output_table_id = match output[0] {
          Transformation::NewTable{table_id,..} => {
            Some(table_id)
          },
          _ => None,
        };

        tfms.append(&mut output);
        let mut input = self.compile_node(&children[1])?;
        let mut rhs = vec![];
        if input.len() > 0 {
          loop { 
            match &mut input[0] {
              Transformation::NewTable{table_id,..} => {
                rhs.push((table_id.clone(),vec![(TableIndex::All, TableIndex::All)]));
                break;
              },
              Transformation::Select{table_id,ref indices} => {
                let table_id = table_id.clone();
                let indices = indices.clone();
                input.remove(0);
                rhs.push((table_id,indices));
                break;
              },
              Transformation::TableReference{table_id, reference: Value::Reference(id)} => {
                input.remove(0);
                input.remove(0);
                continue;
              },
              _ => break,
            }
          }
          let (input_table_id, input_indices) = &rhs[0];
          //tfms.push(Transformation::TableAlias{table_id: input_table_id.unwrap(), alias: variable_name});
          tfms.append(&mut input);
          tfms.push(Transformation::TableDefine{
            table_id: input_table_id.clone(), 
            indices: input_indices.clone(), 
            out: output_table_id.unwrap()
          });
        }
      }
      Node::VariableDefine{children} => {
        let mut output = self.compile_node(&children[0])?;
        // Get the output table id
        let output_table_id = match &output[0] {
          Transformation::Identifier{name, id} => {
            let name_hash = hash_chars(name);
            Some(TableId::Local(name_hash))
          },
          _ => None,
        }.unwrap();

        tfms.append(&mut output);
        let mut input = self.compile_node(&children[1])?;
        let mut rhs = vec![];
        if input.len() > 0 {
          loop { 
            match &mut input[0] {
              Transformation::NewTable{table_id,..} => {
                rhs.push((table_id.clone(),vec![(TableIndex::All, TableIndex::All)]));
                break;
              },
              Transformation::Select{table_id,ref indices} => {
                let table_id = table_id.clone();
                let indices = indices.clone();
                input.remove(0);
                rhs.push((table_id,indices));
                break;
              },
              Transformation::TableReference{table_id, reference: Value::Reference(id)} => {
                input.remove(0);
                input.remove(0);
                continue;
              },
              _ => break,
            }
          }
          let (input_table_id, input_indices) = &rhs[0];
          match input_indices[0] {
            (TableIndex::All,TableIndex::All) => {
              tfms.push(Transformation::TableAlias{table_id: *input_table_id, alias: *output_table_id.unwrap()});
              tfms.append(&mut input);
            }
            _ => {
              tfms.push(Transformation::NewTable{table_id: output_table_id, rows: 1, columns: 1});
              tfms.append(&mut input);
              tfms.push(Transformation::TableDefine{
                table_id: input_table_id.clone(), 
                indices: input_indices.clone(), 
                out: output_table_id
              });
            }
          }
        }
      }
      Node::Function{name, children} => {
        let mut args: Vec<Argument>  = vec![];
        let mut arg_tfms = vec![];
        for child in children {
          // get the argument identifier off the function binding. Default to 0 if there is no named arg
          let mut result = self.compile_node(&child)?;

          let arg: u64 = match &result[0] {
            Transformation::Identifier{name, id} => {
              let arg_id = id.clone();
              result.remove(0);
              arg_id
            },
            _ => 0,
          };
          match &result[0] {
            Transformation::NewTable{table_id,..} => {
              args.push((arg, *table_id, vec![(TableIndex::All, TableIndex::All)]));
            },
            Transformation::Select{table_id, indices} => {
              args.push((arg, *table_id, indices.to_vec()));
              result.remove(0);
            }
            Transformation::TableReference{table_id, reference: Value::Reference(id)} => {
              let table_id = id.clone();
              result.remove(0);
              result.remove(0);
              args.push((arg, table_id, vec![(TableIndex::All, TableIndex::All)]));
            },
            _ => (),
          }
          arg_tfms.append(&mut result);
        }
        let name_hash = hash_chars(name);
        let id = hash_str(&format!("{:?}{:?}", name, arg_tfms));
        tfms.push(Transformation::NewTable{table_id: TableId::Local(id), rows: 1, columns: 1});
        tfms.append(&mut arg_tfms);
        tfms.push(Transformation::Function{
          name: name_hash,
          arguments: args,
          out: (TableId::Local(id), TableIndex::All, TableIndex::All),
        });
      },
      Node::InlineTable{children} => {
        let columns = children.len();
        let mut table_row_children = vec![];
        let mut aliases = vec![];
        // Compile bindings
        for (ix, binding) in children.iter().enumerate() {
          match binding {
            Node::Binding{children} => {
              let mut identifier = self.compile_node(&children[0])?;
              match &identifier[0] {
                Transformation::Identifier{name,id} => {
                  let column_alias = id.clone();
                  let column_ix = ix.clone();
                  let alias_tfm = move |x| Transformation::ColumnAlias{table_id: x, column_ix, column_alias};
                  aliases.push(alias_tfm);
                }
                _ => (),
              }
              table_row_children.push(children[1].clone());
            }
            _ => (),
          }
        }
        let table_row = Node::TableRow{children: table_row_children};
        let mut compiled_row_tfms = self.compile_node(&table_row)?;
        let mut a_tfms = vec![];
        loop {
          match &compiled_row_tfms[0] {
            Transformation::NewTable{table_id,..} => {
              let mut alias_tfms = aliases.iter().map(|a| a(*table_id)).collect();
              a_tfms.append(&mut alias_tfms);
              break;
            }
            Transformation::TableReference{..} => {
              compiled_row_tfms.remove(0);
              compiled_row_tfms.remove(0);
            },
            _ => break,
          }
        }
        tfms.append(&mut compiled_row_tfms);
        tfms.append(&mut a_tfms);
      }
      Node::EmptyTable{children} => {
        let anon_table_id = hash_str(&format!("anonymous-table: {:?}",children));
        let mut table_children = children.clone();
        let mut column_aliases = Vec::new();
        let mut header_tfms = Vec::new();
        let mut columns = 1;
        match table_children.first() {
          Some(Node::TableHeader{children}) => {
            let mut result = self.compile_nodes(&children)?;
            columns = result.len();
            for (ix,tfm) in result.iter().enumerate() {
              match tfm {
                Transformation::Identifier{name,id} => {
                  let alias_tfm = Transformation::ColumnAlias{table_id: TableId::Local(anon_table_id), column_ix: ix.clone(), column_alias: id.clone()};
                  column_aliases.push(alias_tfm);
                }
                _ => (),
              }
            }

            header_tfms.append(&mut result);
            header_tfms.append(&mut column_aliases);
            table_children.remove(0);
          }
          _ => (),
        };
        header_tfms.insert(0,Transformation::NewTable{table_id: TableId::Local(anon_table_id), rows: 1, columns: columns});
        tfms.append(&mut header_tfms);
      }
      Node::AnonymousTableDefine{children} => {
        let anon_table_id = hash_str(&format!("anonymous-table: {:?}",children));
        let mut table_children = children.clone();
        let mut header_tfms = Vec::new();
        let mut column_aliases = Vec::new();
        let mut body_tfms = Vec::new();
        let mut columns = 1;
        match table_children.first() {
          Some(Node::TableHeader{children}) => {
            let mut result = self.compile_nodes(&children)?;
            columns = result.len();
            for (ix,tfm) in result.iter().enumerate() {
              match tfm {
                Transformation::Identifier{name,id} => {
                  let alias_tfm = Transformation::ColumnAlias{table_id: TableId::Local(anon_table_id), column_ix: ix.clone(), column_alias: id.clone()};
                  column_aliases.push(alias_tfm);
                }
                _ => (),
              }
            }

            header_tfms.append(&mut result);
            header_tfms.append(&mut column_aliases);
            table_children.remove(0);
          }
          _ => (),
        };
        if header_tfms.len() > 1 || table_children.len() > 1  {
          let mut args: Vec<Argument> = vec![];
          let mut result_tfms = vec![];
          for child in table_children {
            let mut result = self.compile_node(&child)?;
            match &result[0] {
              Transformation::NewTable{table_id,..} => {
                args.push((0,table_id.clone(),vec![(TableIndex::All, TableIndex::All)]));
              }
              Transformation::Select{table_id, indices} => {
                args.push((0,table_id.clone(),indices.to_vec()));
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
          let mut result = self.compile_nodes(&table_children)?;
          tfms.append(&mut result);          
        }
        match &tfms[0] {
          Transformation::NewTable{table_id,..} |
          Transformation::Select{table_id, ..} => {
            let reference_table_id = TableId::Local(hash_str(&format!("reference:{:?}", tfms[0])));
            let value = Value::Reference(*table_id);
            tfms.insert(0,Transformation::NewTable{table_id: reference_table_id, rows: 1, columns: 1});
            tfms.insert(0,Transformation::TableReference{table_id: reference_table_id, reference: value});
          }
          _ => (),
        }  
      },
      Node::TableColumn{children} => {
        let mut result = self.compile_nodes(children)?;
        tfms.append(&mut result);
      },
      Node::TableRow{children} => {
        if children.len() > 1 {
          let row_id = hash_str(&format!("horzcat:{:?}", children));
          let mut args: Vec<Argument> = vec![];
          let mut result_tfms = vec![];
          for child in children {
            let mut result = self.compile_node(child)?;
            match &result[0] {
              Transformation::NewTable{table_id,..} => {
                args.push((0,table_id.clone(),vec![(TableIndex::All, TableIndex::All)]));
              }
              Transformation::Select{table_id, indices} => {
                args.push((0,table_id.clone(),indices.to_vec()));
                result.remove(0);
              }
              Transformation::TableReference{table_id,..} => {
                let table_id = table_id.clone();
                args.push((0,table_id.clone(),vec![(TableIndex::All, TableIndex::All)]));
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
          let mut result = self.compile_nodes(children)?;
          tfms.append(&mut result);            
        }
      },
      Node::AddRow{children} => {
        let mut result_tfms = Vec::new();
        let mut args: Vec<Argument> = Vec::new();

        let mut result = self.compile_node(&children[0])?;
        match &result[0] {
          Transformation::NewTable{table_id,..} => {
            args.push((0,table_id.clone(),vec![(TableIndex::All, TableIndex::All)]));
            result.remove(0);
          }
          Transformation::Select{table_id, indices} => {
            args.push((0,table_id.clone(),indices.to_vec()));
            result.remove(0);
          }
          _ => (),
        }
        result_tfms.append(&mut result); 

        let mut result = self.compile_node(&children[1])?;
        loop {
          match &result[0] {
            Transformation::NewTable{table_id,..} => {
              args.push((0,table_id.clone(),vec![(TableIndex::All, TableIndex::All)]));
              break;
            }
            Transformation::Select{table_id, indices} => {
              args.push((0,table_id.clone(),indices.to_vec()));
              result.remove(0);
              break;
            }
            Transformation::TableReference{table_id,..} => {
              result.remove(0);
              result.remove(0);
              continue;
            }
            _ => (),
          }
        }
        result_tfms.append(&mut result); 

        let (_,o,oi) = &args[0];
        let (or,oc) = oi[0];
        tfms.append(&mut result_tfms);
        tfms.push(Transformation::Function{
          name: *TABLE_APPEND,
          arguments: vec![args[1].clone()],
          out: (*o,or,oc),
        });
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
                          let id = hash_str("~");
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
                          let mut result = self.compile_node(child)?;
                          match &result[1] {
                            Transformation::NewTable{table_id, ..} => {
                              indices.push(TableIndex::Table(*table_id));
                            }
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
                    let id = hash_str("~");
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
                    let mut result = self.compile_node(child)?;
                    match &result[1] {
                      Transformation::NewTable{table_id, ..} => {
                        indices.push(TableIndex::Table(*table_id));
                      }
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
        tfms.push(Transformation::Select{table_id: *id, indices: all_indices});
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
      Node::Binding{children} |
      Node::FunctionBinding{children} |
      Node::Root{children} => {
        let mut result = self.compile_nodes(children)?;
        tfms.append(&mut result);
      }
      Node::Null => (),
      x => println!("Unhandled Node {:?}", x),
    }
    Ok(tfms)
  }
}