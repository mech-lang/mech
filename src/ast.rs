// # AST

// Takes a parse tree, produces an abstract syntax tree.

// ## Prelude

use crate::parser;

#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;

use mech_core::*;
use mech_core::node::*;

lazy_static! {
  pub static ref U16: u64 = hash_str("u16");
  pub static ref HEX: u64 = hash_str("hex");
  pub static ref OCT: u64 = hash_str("oct");
  pub static ref DEC: u64 = hash_str("dec");
  pub static ref BIN: u64 = hash_str("bin");
}

pub struct Ast {
  current_char: usize,
  current_line: usize,
  current_col: usize,
  depth: usize,
  pub syntax_tree: AstNode,
}

impl Ast {

  pub fn new() -> Ast {
    Ast {
      current_char: 0,
      current_line: 0,
      current_col: 0,
      depth: 0,
      syntax_tree: AstNode::Null,
    }
  }

  pub fn compile_nodes(&mut self, nodes: &Vec<ParserNode>) -> Vec<AstNode> {
    let mut compiled = Vec::new();
    for node in nodes {
      compiled.append(&mut self.build_syntax_tree(node));
    }
    compiled
  }

  pub fn build_syntax_tree(&mut self, node: &ParserNode) -> Vec<AstNode> {
    let mut compiled = Vec::new();
    self.depth += 1;
    match node {
      ParserNode::Root{children} => self.syntax_tree = AstNode::Root{children: self.compile_nodes(children)},
      ParserNode::Fragment{children} => self.syntax_tree = AstNode::Root{children: self.compile_nodes(children)},
      ParserNode::Program{children} => {
        let result = self.compile_nodes(children);
        let mut children = vec![];
        let mut title = None;
        for node in result {
          match node {
            AstNode::Title{text} => title = Some(text),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::Program{title, children});
      },
      ParserNode::Head{children} => compiled.push(AstNode::Head{children: self.compile_nodes(children)}),
      ParserNode::Section{children} => {
        let result = self.compile_nodes(children);
        let mut children = vec![];
        let mut title = None;
        for node in result {
          match node {
            AstNode::Title{text} => {
              if !children.is_empty() {
                compiled.push(AstNode::Section{title: title.clone(), children: children.clone()});
                children.clear();
              }
              title = Some(text);
            },
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::Section{title: title.clone(), children: children.clone()});
      },
      ParserNode::Block{children} => compiled.push(AstNode::Block{children: self.compile_nodes(children)}),
      ParserNode::Data{children} => {
        let result = self.compile_nodes(children);
        let mut reversed = result.clone();
        reversed.reverse();
        let mut select_data_children: Vec<AstNode> = vec![];

        for node in reversed {
          match node {
            AstNode::Table{name, id} => {
              if select_data_children.is_empty() {
                select_data_children = vec![AstNode::Null; 1];
              }
              select_data_children.reverse();
              compiled.push(AstNode::SelectData{name, id: TableId::Global(id), children: select_data_children.clone()});
            },
            AstNode::Identifier{name, id} => {
              if select_data_children.is_empty() {
                select_data_children = vec![AstNode::Null; 1];
              }
              select_data_children.reverse();
              compiled.push(AstNode::SelectData{name, id: TableId::Local(id), children: select_data_children.clone()});
            },
            AstNode::DotIndex{children} => {
              let mut reversed = children.clone();
              if children.len() == 1 {
                reversed.push(AstNode::Null);
                reversed.reverse();
              }
              select_data_children.push(AstNode::DotIndex{children: reversed});
            },
            AstNode::Swizzle{..} => {
              select_data_children.push(node.clone());
            },
            AstNode::SubscriptIndex{..} => {
              select_data_children.push(node.clone());
            }
            AstNode::ReshapeColumn => {
              select_data_children.push(AstNode::ReshapeColumn);
            }
            _ => (),
          }
        }
      },
      ParserNode::Statement{children} => compiled.push(AstNode::Statement{children: self.compile_nodes(children)}),
      ParserNode::Expression{children} => {
        let result = self.compile_nodes(children);
        for node in result {
          match node {
            // If the node is a naked expression, modify the graph
            // TODO this is hacky... maybe change the parser?
            AstNode::SelectData{..} => {
              compiled.push(node);
            },
            _ => compiled.push(AstNode::Expression{children: vec![node]}),
          }
        }
      },
      ParserNode::Attribute{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::Attribute{children});
      },
      ParserNode::Whenever{children} => compiled.push(AstNode::Whenever{children: self.compile_nodes(children)}),
      ParserNode::Wait{children} => compiled.push(AstNode::Wait{children: self.compile_nodes(children)}),
      ParserNode::Until{children} => compiled.push(AstNode::Until{children: self.compile_nodes(children)}),
      ParserNode::SelectAll => compiled.push(AstNode::SelectAll),
      ParserNode::SetData{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::SetData{children});
      },
      ParserNode::UpdateData{children} => {
        let result = self.compile_nodes(children);
        let operator = &result[0].clone();
        let dest = &result[2].clone();
        let src = &result[1].clone();
        let name: Vec<char> = match operator {
          AstNode::AddUpdate => "math/add-update".chars().collect(),
          AstNode::SubtractUpdate => "math/subtract-update".chars().collect(),
          AstNode::MultiplyUpdate => "math/multiply-update".chars().collect(),
          AstNode::DivideUpdate => "math/divide-update".chars().collect(),
          AstNode::ExponentUpdate => "math/exponent-update".chars().collect(),
          _ => Vec::new(),
        };
        compiled.push(AstNode::UpdateData{name, children: vec![src.clone(), dest.clone()]});
      },
      ParserNode::SplitData{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::SplitData{children});
      },
      ParserNode::FlattenData{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::FlattenData{children});
      },
      ParserNode::Column{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::TableColumn{children});
      },
      ParserNode::Empty => compiled.push(AstNode::Empty),
      ParserNode::Binding{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::Binding{children});
      },
      ParserNode::FunctionBinding{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::FunctionBinding{children});
      },
      ParserNode::Transformation{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            // Ignore irrelevant nodes like spaces and operators
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        if !children.is_empty() {
          compiled.push(AstNode::Transformation{children});
        }
      },
      ParserNode::SelectExpression{children} => compiled.push(AstNode::SelectExpression{children: self.compile_nodes(children)}),
      ParserNode::InlineTable{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::InlineTable{children});
      },
      ParserNode::AnonymousTable{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::AnonymousTableDefine{children});
      },
      ParserNode::EmptyTable{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::EmptyTable{children});
      },
      ParserNode::TableHeader{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::TableHeader{children});
      },
      ParserNode::TableRow{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::TableRow{children});
      },
      ParserNode::MathExpression{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        let mut new_node = false;
        for node in result {
          match node {
            // Ignore irrelevant nodes like spaces and operators
            AstNode::Token{..} => (),
            AstNode::Function{..} => {
              new_node = true;
              children.push(node);
            },
            /*AstNode::Quantity{..} => {
              new_node = false;
              children.push(node);
            }*/
            _ => children.push(node),
          }
        }
        // TODO I might be able to simplify this now. This doesn't seem to be necessary
        if new_node {
          compiled.push(AstNode::MathExpression{children});
        } else {
          compiled.append(&mut children);
        }
      },
      ParserNode::Infix{children} => {
        let result = self.compile_nodes(children);
        let operator = &result[0];
        let name: Vec<char> = match operator {
          AstNode::Token{token, chars} => chars.to_vec(),
          _ => Vec::new(),
        };
        compiled.push(AstNode::Function{name, children: vec![]});
      },
      ParserNode::VariableDefine{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          // If the node is a naked expression, modify the
          // graph to put it into an anonymous table
          match node {
            AstNode::Token{..} => (),
            AstNode::SelectData{..} => {
              children.push(AstNode::Expression{
                children: vec![AstNode::AnonymousTableDefine{
                  children: vec![AstNode::TableRow{
                    children: vec![AstNode::TableColumn{
                      children: vec![node]}]}]}]});
            },
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::VariableDefine{children});
      },
      ParserNode::TableDefine{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            AstNode::SelectData{..} => {
              children.push(AstNode::Expression{
                children: vec![AstNode::AnonymousTableDefine{
                  children: vec![AstNode::TableRow{
                    children: vec![AstNode::TableColumn{
                      children: vec![node]}]}]}]});
            },
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::TableDefine{children});
      },
      ParserNode::TableSelect{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            AstNode::SelectData{..} => {
              children.push(AstNode::Expression{
                children: vec![AstNode::AnonymousTableDefine{
                  children: vec![AstNode::TableRow{
                    children: vec![AstNode::TableColumn{
                      children: vec![node]}]}]}]});
            },
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::TableSelect{children});
      },      
      ParserNode::AddRow{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::AddRow{children});
      },
      ParserNode::Index{children} => compiled.append(&mut self.compile_nodes(children)),
      ParserNode::ReshapeColumn => compiled.push(AstNode::ReshapeColumn),
      ParserNode::DotIndex{children} => compiled.push(AstNode::DotIndex{children: self.compile_nodes(children)}),
      ParserNode::Swizzle{children} => compiled.push(AstNode::Swizzle{children: self.compile_nodes(children)}),
      ParserNode::SubscriptIndex{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        for node in result {
          match node {
            AstNode::Token{token, ..} => {
              match token {
                Token::Tilde => {
                  children.push(AstNode::WheneverIndex{children: vec![AstNode::Null]});
                }
                _ => (),
              }
            },
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::SubscriptIndex{children});
      },
      ParserNode::Table{children} => {
        let result = self.compile_nodes(children);
        match &result[0] {
          AstNode::Identifier{name, id} => {
            compiled.push(AstNode::Table{name: name.to_vec(), id: *id});
          },
          _ => (),
        };
      },
      // Quantities
      ParserNode::Quantity{children} => compiled.push(AstNode::Quantity{children: self.compile_nodes(children)}),
      ParserNode::Number{children} => {
        let mut word = Vec::new();
        let result = self.compile_nodes(children);
        for node in result {
          match node {
            AstNode::Token{token, mut chars} => word.append(&mut chars),
            _ => (),
          }
        }
        compiled.push(AstNode::String{text: word});
      },
      // String-like nodes
      ParserNode::ParagraphText{children} => {
        let mut result = self.compile_nodes(children);
        let mut paragraph = Vec::new();
        for ref mut node in &mut result {
          match node {
            AstNode::String{ref mut text} => paragraph.append(text),
            _ => (),
          };
        }

        let node = AstNode::ParagraphText{text: paragraph};
        compiled.push(node);
      },
      ParserNode::InlineCode{children} => compiled.push(AstNode::InlineCode{children: self.compile_nodes(children)}),
      ParserNode::CodeBlock{children} => compiled.push(AstNode::CodeBlock{children: self.compile_nodes(children)}),
      ParserNode::MechCodeBlock{children} => compiled.push(AstNode::MechCodeBlock{children: self.compile_nodes(children)}),
      ParserNode::Comment{children} => compiled.push(AstNode::Comment{children: self.compile_nodes(children)}),
      ParserNode::InlineMechCode{children} => compiled.push(AstNode::InlineMechCode{children: self.compile_nodes(children)}),
      ParserNode::Paragraph{children} => compiled.push(AstNode::Paragraph{children: self.compile_nodes(children)}),
      ParserNode::UnorderedList{children} => compiled.push(AstNode::UnorderedList{children: self.compile_nodes(children)}),
      ParserNode::ListItem{children} => compiled.push(AstNode::ListItem{children: self.compile_nodes(children)}),
      ParserNode::Title{children} => {
        let result = self.compile_nodes(children);
        let node = match &result[0] {
          AstNode::String{text} => AstNode::Title{text: text.clone()},
          _ => AstNode::Null,
        };
        compiled.push(node);
      },
      ParserNode::Subtitle{children} => {
        let result = self.compile_nodes(children);
        let node = match &result[0] {
          AstNode::String{text} => AstNode::Title{text: text.clone()},
          _ => AstNode::Null,
        };
        compiled.push(node);
      },
      ParserNode::SectionTitle{children} => {
        let result = self.compile_nodes(children);
        let node = match &result[0] {
          AstNode::String{text} => AstNode::SectionTitle{text: text.clone()},
          _ => AstNode::Null,
        };
        compiled.push(node);
      },
      ParserNode::FormattedText{children} |
      ParserNode::Text{children} => {
        let result = self.compile_nodes(children);
        let mut text_node = Vec::new();
        for node in result {
          match node {
            AstNode::String{mut text} => {
              text_node.append(&mut text)
            },
            AstNode::Token{token, mut chars} => {
              text_node.append(&mut chars)
            },
            _ => (),
          }
        }
        compiled.push(AstNode::String{text: text_node});
      },
      ParserNode::Word{children} => {
        let mut word = Vec::new();
        let result = self.compile_nodes(children);
        for node in result {
          match node {
            AstNode::Token{token, mut chars} => word.append(&mut chars),
            _ => (),
          }
        }
        compiled.push(AstNode::String{text: word});
      },
      ParserNode::TableIdentifier{children} |
      ParserNode::Identifier{children} => {
        let mut word = Vec::new();
        let result = self.compile_nodes(children);
        for node in result {
          match node {
            AstNode::Token{token, mut chars} => word.append(&mut chars),
            AstNode::String{mut text} =>  word.append(&mut text),
            //AstNode::Quantity{value, unit} => word.push_str(&format!("{}", value.to_f32())),
            _ => compiled.push(node),
          }
        }
        let id = hash_chars(&word);
        compiled.push(AstNode::Identifier{name: word, id});
      },
      // Math
      ParserNode::L0{children} |
      ParserNode::L1{children} |
      ParserNode::L2{children} |
      ParserNode::L3{children} |
      ParserNode::L4{children} |
      ParserNode::L5{children} |
      ParserNode::L6{children} => {
        let result = self.compile_nodes(children);
        let mut last = AstNode::Null;
        for node in result {
          match last {
            AstNode::Null => last = node,
            _ => {
              let (name, mut children) = match node {
                AstNode::Function{name, children} => (name.clone(), children.clone()),
                _ => (Vec::new(), vec![]),
              };
              children.push(last);
              children.reverse();
              last = AstNode::Function{name, children};
            },
          };
        }
        compiled.push(last);
      },
      ParserNode::L0Infix{children} |
      ParserNode::L1Infix{children} |
      ParserNode::L2Infix{children} |
      ParserNode::L3Infix{children} |
      ParserNode::L4Infix{children} |
      ParserNode::L5Infix{children} => {
        let result = self.compile_nodes(children);
        let operator = &result[0].clone();
        let input = &result[1].clone();
        let name: Vec<char> = match operator {
          AstNode::Add => "math/add".chars().collect(),
          AstNode::Subtract => "math/subtract".chars().collect(),
          AstNode::Multiply => "math/multiply".chars().collect(),
          AstNode::Divide => "math/divide".chars().collect(),
          AstNode::Exponent => "math/exponent".chars().collect(),
          AstNode::GreaterThan => "compare/greater-than".chars().collect(),
          AstNode::GreaterThanEqual => "compare/greater-than-equal".chars().collect(),
          AstNode::LessThanEqual => "compare/less-than-equal".chars().collect(),
          AstNode::LessThan => "compare/less-than".chars().collect(),
          AstNode::Equal => "compare/equal".chars().collect(),
          AstNode::NotEqual => "compare/not-equal".chars().collect(),
          AstNode::Range => "table/range".chars().collect(),
          AstNode::And => "logic/and".chars().collect(),
          AstNode::Or => "logic/or".chars().collect(),
          AstNode::Xor => "logic/xor".chars().collect(),
          AstNode::Token{token, chars} => chars.to_vec(),
          _ => Vec::new(),
        };
        compiled.push(AstNode::Function{name, children: vec![input.clone()]});
      },
      ParserNode::Not{children} => {
        let result = self.compile_nodes(children);
        compiled.push(AstNode::Function{name: "logic/not".chars().collect(), children: result});
      },
      ParserNode::Negation{children} => {
        let result = self.compile_nodes(children);
        compiled.push(AstNode::Function{name: "math/negate".chars().collect(), children: result});
      },
      ParserNode::Function{children} => {
        let result = self.compile_nodes(children);
        let mut children: Vec<AstNode> = Vec::new();
        let mut function_name = Vec::new();
        for node in result {
          match node {
            AstNode::Token{..} => (),
            AstNode::Identifier{name, id} => function_name = name,
            _ => children.push(node),
          }
        }
        compiled.push(AstNode::Function{name: function_name, children: children.clone()});
      },
      /*ParserNode::Negation{children} => {
        let result = self.compile_nodes(children);
        let mut input = vec![AstNode::Quantity{value: 0, unit: None}];
        input.push(result[0].clone());
        compiled.push(AstNode::Function{ name: "math/subtract".chars().collect(), children: input });
      },*/
      /*ParserNode::Not{children} => {
        let result = self.compile_nodes(children);
        let mut input = vec![AstNode::Quantity{value: Value::from_bool(true), unit: None}];
        input.push(result[0].clone());
        compiled.push(AstNode::Function{ name: "logic/xor".chars().collect(), children: input });
      },*/
      ParserNode::String{children} => {
        let result = self.compile_nodes(children);
        let string = if result.len() > 0 {
          result[0].clone()
        } else {
          AstNode::String{text: Vec::new()}
        };
        compiled.push(string);
      },
      ParserNode::NumberLiteral{children} => {
        let mut result = self.compile_nodes(children);
        // There's a type annotation
        if result.len() > 1 {
          match (&result[0], &result[1]) {
            (AstNode::NumberLiteral{kind,bytes}, AstNode::KindAnnotation{children}) => {
              if let AstNode::Identifier{name, id} = &children[0] {
                result[0] = AstNode::NumberLiteral{kind: *id, bytes: bytes.clone()};
              }
            }
            _ => (),
          }
        }
        compiled.push(result[0].clone());
      },
      ParserNode::True => {
        compiled.push(AstNode::True);
      },
      ParserNode::False => {
        compiled.push(AstNode::False);
      },
      ParserNode::RationalNumber{children} => {
        let result = self.compile_nodes(children);
        compiled.push(AstNode::RationalNumber{children: result});
      },
      ParserNode::KindAnnotation{children} => {
        let result = self.compile_nodes(children);
        compiled.push(AstNode::KindAnnotation{children: result});
      },
      ParserNode::FloatLiteral{chars} => {
        /*let string = chars.iter().cloned().collect::<String>();
        let float = string.parse::<f32>().unwrap();
        let bytes = float.to_be_bytes();*/
        compiled.push(AstNode::NumberLiteral{kind: hash_str("f32-literal"), bytes: chars.to_vec()});
      }
      ParserNode::DecimalLiteral{chars} => {
        /*let mut dec_bytes = chars.iter().map(|c| c.to_digit(10).unwrap() as u8).collect::<Vec<u8>>();
        let mut dec_number: u128 = 0;
        dec_bytes.reverse();
        for (i,byte) in dec_bytes.iter().enumerate() {
          dec_number += *byte as u128 * 10_u128.pow(i as u32);
        }
        use std::mem::transmute;
        let mut bytes: [u8; 16] = unsafe { transmute(dec_number.to_be()) };
        let mut bytes = bytes.to_vec();
        // Remove leading zeros
        while bytes.len() > 1 && bytes[0] == 0 {
          bytes.remove(0);
        }*/
        compiled.push(AstNode::NumberLiteral{kind: *DEC, bytes: chars.to_vec()});
      },
      ParserNode::BinaryLiteral{chars} => {
        //let bin_bytes = chars.iter().map(|c| c.to_digit(2).unwrap() as u8).collect::<Vec<u8>>();
        compiled.push(AstNode::NumberLiteral{kind: *BIN, bytes: chars.to_vec()});
      }
      ParserNode::OctalLiteral{chars} => {
        //let oct_bytes = chars.iter().map(|c| c.to_digit(8).unwrap() as u8).collect::<Vec<u8>>();
        compiled.push(AstNode::NumberLiteral{kind: *OCT, bytes: chars.to_vec()});
      },
      ParserNode::HexadecimalLiteral{chars} => {
        //let hex_bytes = chars.iter().map(|c| c.to_digit(16).unwrap() as u8).collect::<Vec<u8>>();
        compiled.push(AstNode::NumberLiteral{kind: *HEX, bytes: chars.to_vec()});
      },
      ParserNode::True => compiled.push(AstNode::True),
      ParserNode::False => compiled.push(AstNode::False),
      ParserNode::ParentheticalExpression{children} => {
        let result = self.compile_nodes(children);
        compiled.push(result[0].clone());
      },
      ParserNode::GreaterThan => compiled.push(AstNode::GreaterThan),
      ParserNode::LessThan => compiled.push(AstNode::LessThan),
      ParserNode::GreaterThanEqual => compiled.push(AstNode::GreaterThanEqual),
      ParserNode::LessThanEqual => compiled.push(AstNode::LessThanEqual),
      ParserNode::Equal => compiled.push(AstNode::Equal),
      ParserNode::NotEqual => compiled.push(AstNode::NotEqual),
      ParserNode::Range => compiled.push(AstNode::Range),
      ParserNode::Add => compiled.push(AstNode::Add),
      ParserNode::Subtract => compiled.push(AstNode::Subtract),
      ParserNode::Multiply => compiled.push(AstNode::Multiply),
      ParserNode::Divide => compiled.push(AstNode::Divide),
      ParserNode::Exponent => compiled.push(AstNode::Exponent),
      ParserNode::And => compiled.push(AstNode::And),
      ParserNode::Or => compiled.push(AstNode::Or),
      ParserNode::Xor => compiled.push(AstNode::Xor),
      ParserNode::AddUpdate => compiled.push(AstNode::AddUpdate),
      ParserNode::SubtractUpdate => compiled.push(AstNode::SubtractUpdate),
      ParserNode::MultiplyUpdate => compiled.push(AstNode::MultiplyUpdate),
      ParserNode::DivideUpdate => compiled.push(AstNode::DivideUpdate),
      ParserNode::ExponentUpdate => compiled.push(AstNode::ExponentUpdate),
      ParserNode::Comparator{children} => {
        match children[0] {
          ParserNode::LessThan => compiled.push(AstNode::LessThan),
          ParserNode::LessThanEqual => compiled.push(AstNode::LessThanEqual),
          ParserNode::GreaterThanEqual => compiled.push(AstNode::GreaterThanEqual),
          ParserNode::Equal => compiled.push(AstNode::Equal),
          ParserNode::NotEqual => compiled.push(AstNode::NotEqual),
          ParserNode::GreaterThan => compiled.push(AstNode::GreaterThan),
          _ => (),
        }
      },
      ParserNode::LogicOperator{children} => {
        match children[0] {
          ParserNode::And => compiled.push(AstNode::And),
          ParserNode::Or => compiled.push(AstNode::Or),
          ParserNode::Xor => compiled.push(AstNode::Xor),
          _ => (),
        }
      },
      // Pass through nodes. These will just be omitted
      ParserNode::Value{children} |
      ParserNode::Emoji{children} |
      ParserNode::Constant{children} |
      ParserNode::StateMachine{children} |
      ParserNode::StateTransition{children} |
      ParserNode::Body{children} |
      ParserNode::Punctuation{children} |
      ParserNode::DigitOrComma{children} |
      ParserNode::Any{children} |
      ParserNode::Symbol{children} |
      ParserNode::AddOperator{children} |
      ParserNode::Subscript{children} |
      ParserNode::DataOrConstant{children} |
      ParserNode::SpaceOrTab{children} |
      ParserNode::Whitespace{children} |
      ParserNode::NewLine{children} |
      ParserNode::IdentifierOrConstant{children} |
      ParserNode::ProseOrCode{children}|
      ParserNode::StatementOrExpression{children} |
      ParserNode::WatchOperator{children} |
      ParserNode::SetOperator{children} |
      ParserNode::Repeat{children} |
      ParserNode::Alphanumeric{children} |
      ParserNode::BooleanLiteral{children} |
      ParserNode::IdentifierCharacter{children} => {
        compiled.append(&mut self.compile_nodes(children));
      },
      ParserNode::Token{token, chars} => {
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
        compiled.push(AstNode::Token{token: *token, chars: chars.to_vec()});
      },
      ParserNode::Null => (),
      _ => println!("Unhandled Parser AstNode in AST Compiler: {:?}", node),
    }

    //self.constraints = constraints.clone();
    compiled
  }

}
