use nom::alpha1 as nom_alpha1;
use nom::digit1 as nom_digit1;
use nom::AtEof as eof;
use nom::types::CompleteStr;
use lexer::{Lexer, Token};
use parser::{ParseStatus, Node};
use lexer::Token::{HashTag, Alpha, Period, LeftBracket, RightBracket, Newline,
                   Digit, Space, Equal, Plus, EndOfStream, Dash, Asterisk, Slash};
use mech_core::{Hasher, Function};
use alloc::fmt;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone)]
pub struct Parser {
  pub tokens: Vec<Token>,
  pub parse_tree: Node,
  pub unparsed: String,
  pub text: String,
}

impl Parser {

  pub fn new() -> Parser {
    Parser {
      text: String::from(""),
      tokens: Vec::new(),
      unparsed: String::from(""),
      parse_tree: Node::Root{ children: Vec::new()  },
    }
  }

  pub fn add_tokens(&mut self, tokens: &mut Vec<Token>) {
    self.tokens.append(tokens);
  }

  pub fn parse(&mut self, text: &str) {
    let parse_tree = parse_mech(CompleteStr(text));
    match parse_tree {
      Ok((rest, tree)) => {
        self.unparsed = rest.to_string();
        self.parse_tree = tree;
      },
      _ => (), 
    }
  }
   
}

impl fmt::Debug for Parser {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    
    write!(f, "┌───────────────────────────────────────┐\n").unwrap();
    write!(f, "│ Parser\n").unwrap();
    write!(f, "│ Length: {:?}\n", self.tokens.len()).unwrap();
    write!(f, "├───────────────────────────────────────┤\n").unwrap();
    for (ix, token) in self.tokens.iter().enumerate() {
      let c1 = " "; //if self.position == ix + 1 { ">" } else { " " };
      let c2 = " "; //if self.last_match == ix + 1 { ">" } else { " " };
      write!(f, "│ {:}{:} {:?}\n", c1, c2, token).unwrap();
    }
    write!(f, "├───────────────────────────────────────┤\n").unwrap();
    write!(f, "{:?}", self.parse_tree);
    write!(f, "└───────────────────────────────────────┘\n").unwrap();
    Ok(())
  }
}

macro_rules! leaf {
  ($name:ident, $byte:expr, $token:expr) => (
    named!($name<CompleteStr, Node>,
      do_parse!(
        byte: tag!($byte) >> 
        (Node::Token{token: $token, byte: (byte.as_bytes())[0]})
      )
    );
  )
}

leaf!{hashtag, "#", Token::HashTag}
leaf!{period, ".", Token::Period}
leaf!{colon, ":", Token::Colon}
leaf!{comma, ",", Token::Comma}
leaf!{left_bracket, "[", Token::LeftBracket}
leaf!{right_bracket, "]", Token::RightBracket}
leaf!{left_parenthesis, "(", Token::LeftParenthesis}
leaf!{right_parenthesis, ")", Token::RightParenthesis}
leaf!{left_brace, "{", Token::LeftBrace}
leaf!{right_brace, "}", Token::RightBrace}
leaf!{equal, "=", Token::Equal}
leaf!{left_angle, "<", Token::LessThan}
leaf!{right_angle, ">", Token::GreaterThan}
leaf!{exclamation, "!", Token::Exclamation}
leaf!{question, "?", Token::Question}
leaf!{plus, "+", Token::Plus}
leaf!{dash, "-", Token::Dash}
leaf!{asterisk, "*", Token::Asterisk}
leaf!{slash, "/", Token::Slash}
leaf!{caret, "^", Token::Caret}
leaf!{space, " ", Token::Space}
leaf!{tab, "\t", Token::Tab}
leaf!{tilde, "~", Token::Tilde}
leaf!{grave, "`", Token::Grave}
leaf!{bar, "|", Token::Bar}
leaf!{quote, "\"", Token::Quote}
leaf!{ampersand, "&", Token::Ampersand}
leaf!{semicolon, ";", Token::Semicolon}
leaf!{new_line_char, "\n", Token::Newline}
leaf!{carriage_return, "\r", Token::CarriageReturn}

// ## The Basics

named!(word<CompleteStr, Node>,
  do_parse!(
    bytes: nom_alpha1 >>
    (Node::Word{children: bytes.chars().map(|b| Node::Token{token: Token::Alpha, byte: b as u8}).collect()})));

named!(number<CompleteStr, Node>,
  do_parse!(
    bytes: nom_digit1 >>
    (Node::Number{children: bytes.chars().map(|b| Node::Token{token: Token::Digit, byte: b as u8}).collect()})));

named!(text<CompleteStr, Node>,
  do_parse!(
    word: many1!(alt!(word | space | number)) >>
    (Node::Text{children: word})));

named!(identifier<CompleteStr, Node>,
  do_parse!(
    identifier: map!(tuple!(count!(word,1), many0!(alt!(dash | slash | word | number))), |tuple| {
      let (mut word, mut rest) = tuple;
      word.append(&mut rest);
      word
    }) >>
    (Node::Identifier{children: identifier})));

named!(whitespace<CompleteStr, Node>,
  do_parse!(
    many0!(space) >> new_line_char >>
    (Node::Null)));

named!(floating_point<CompleteStr, Node>,
  do_parse!(
    period >> bytes: nom_digit1 >>
    (Node::FloatingPoint{children: bytes.chars().map(|b| Node::Token{token: Token::Digit, byte: b as u8}).collect()})));

named!(quantity<CompleteStr, Node>,
  do_parse!(
    quantity: map!(tuple!(number, opt!(floating_point)),|tuple| {
      let (front, floating_point) = tuple;
      let mut quantity = vec![front];
      match floating_point {
        Some(point) => quantity.push(point),
        _ => (),
      };
      quantity
    }) >>
    (Node::Quantity{children: quantity})));

named!(constant<CompleteStr, Node>,
  do_parse!(
    constant: alt!(string | quantity) >>
    (Node::Constant{children: vec![constant]})));

// ## Blocks

// ### Data

named!(select_all<CompleteStr, Node>,
  do_parse!(
    colon >> 
    (Node::SelectAll{children: vec![]})));

named!(subscript<CompleteStr, Node>,
  do_parse!(
    subscript: alt!(select_all | constant | expression) >> many0!(space) >> opt!(comma) >> many0!(space) >>
    (Node::Subscript{children: vec![subscript]})));

named!(subscript_index<CompleteStr, Node>,
  do_parse!(
    left_brace >> subscripts: many1!(subscript) >> right_brace >>
    (Node::SubscriptIndex{children: subscripts})));

named!(dot_index<CompleteStr, Node>,
  do_parse!(
    period >> column_name: identifier >> 
    (Node::DotIndex{children: vec![column_name]})));

named!(index<CompleteStr, Node>,
  do_parse!(
    index: alt!(dot_index | subscript_index) >>
    (Node::Index{children: vec![index]})));

named!(data<CompleteStr, Node>,
  do_parse!(
    data: map!(tuple!(alt!(table | identifier), many0!(index)), |tuple| {
      let (mut source, mut indices) = tuple;
      let mut data = vec![source];
      data.append(&mut indices);
      data
    }) >>
    (Node::Data { children: data })));

// ### Tables

named!(table<CompleteStr, Node>,
  do_parse!(
    hashtag >> table_identifier: identifier >>
    (Node::Table { children: vec![table_identifier] })));

named!(binding<CompleteStr, Node>,
  do_parse!(
    binding_id: identifier >> colon >> many0!(space) >> 
    bound: alt!(identifier | constant) >> many0!(space) >> opt!(comma) >> many0!(space) >>
    (Node::Binding { children: vec![binding_id, bound] })));

named!(inline_table<CompleteStr, Node>,
  do_parse!(
    left_bracket >> bindings: many1!(binding) >> right_bracket >>
    (Node::InlineTable { children: bindings })));

// ### Statements

named!(add_row_operator<CompleteStr, Node>,
  do_parse!(
    tag!("+=") >>
    (Node::Null)));

named!(add_row<CompleteStr, Node>,
  do_parse!(
    table: data >> space >> add_row_operator >> space >> inline: inline_table >>
    (Node::AddRow { children: vec![table, inline] })));

named!(set_operator<CompleteStr, Node>,
  do_parse!(
    tag!(":=") >> 
    (Node::Null)));

named!(set_data<CompleteStr, Node>,
  do_parse!(
    table: data >> space >> set_operator >> space >> expression: expression >>
    (Node::SetData { children: vec![table, expression] })));

named!(variable_define<CompleteStr, Node>,
  do_parse!(
    variable: identifier >> space >> equal >> space >> expression: expression >>
    (Node::VariableDefine { children: vec![variable, expression] })));

named!(table_define<CompleteStr, Node>,
  do_parse!(
    table: table >> space >> equal >> space >> expression: expression >>
    (Node::TableDefine { children: vec![table, expression] })));

named!(watch_operator<CompleteStr, Node>,
  do_parse!(
    tilde >> 
    (Node::Null)));

named!(data_watch<CompleteStr, Node>,
  do_parse!(
    watch_operator >> space >> watch: alt!(variable_define | data) >>
    (Node::DataWatch { children: vec![watch] })));

named!(statement<CompleteStr, Node>,
  do_parse!(
    statement: alt!(table_define | variable_define | data_watch | set_data | add_row) >>
    (Node::Statement { children: vec![statement] })));

// ### Expressions

// #### Math Expressions

named!(l1_infix<CompleteStr, Node>,
  do_parse!(
    space >> op: alt!(plus | dash) >> space >> l2: l2 >>
    (Node::L1Infix { children: vec![op, l2] })));

named!(l2_infix<CompleteStr, Node>,
  do_parse!(
    space >> op: alt!(asterisk | slash) >> space >> l3: l3 >>
    (Node::L2Infix { children: vec![op, l3] })));

named!(l3_infix<CompleteStr, Node>,
  do_parse!(
    space >> op: caret >> space >> l4: l4 >>
    (Node::L3Infix { children: vec![op, l4] })));

named!(l4<CompleteStr, Node>,
  do_parse!(
    l4: alt!(data | quantity) >>
    (Node::L4 { children: vec![l4] })));

named!(l3<CompleteStr, Node>,
  do_parse!(
    l4: map!(tuple!(l4, many0!(l3_infix)), |tuple| {
      let (mut l, mut infix) = tuple;
      let mut math = vec![l];
      math.append(&mut infix);
      math
    }) >>
    (Node::L3 { children: l4 })));

named!(l2<CompleteStr, Node>,
  do_parse!(
    l3: map!(tuple!(l3, many0!(l2_infix)), |tuple| {
      let (mut l, mut infix) = tuple;
      let mut math = vec![l];
      math.append(&mut infix);
      math
    }) >>
    (Node::L2 { children: l3 })));

named!(l1<CompleteStr, Node>,
  do_parse!(
    l2: map!(tuple!(l2, many0!(l1_infix)), |tuple| {
      let (mut l, mut infix) = tuple;
      let mut math = vec![l];
      math.append(&mut infix);
      math
    }) >>
    (Node::L1 { children: l2 })));

named!(math_expression<CompleteStr, Node>,
  do_parse!(
    l1: l1 >>
    (Node::MathExpression { children: vec![l1] })));

// #### Filter Expressions

named!(less_than<CompleteStr, Node>,
  do_parse!(
    tag!("<") >> 
    (Node::LessThan)));

named!(greater_than<CompleteStr, Node>,
  do_parse!(
    tag!(">") >> 
    (Node::GreaterThan)));

named!(comparator<CompleteStr, Node>,
  do_parse!(
    comparator: alt!(less_than | greater_than) >>
    (Node::Comparator { children: vec![comparator] })));

named!(filter_expression<CompleteStr, Node>,
  do_parse!(
    lhs: alt!(data | constant) >> space >> comp: comparator >> space >> rhs: alt!(data | constant) >>
    (Node::FilterExpression { children: vec![lhs, comp, rhs] })));

// #### Logic Expressions

named!(or<CompleteStr, Node>,
  do_parse!(
    bar >>
    (Node::Or)));

named!(and<CompleteStr, Node>,
  do_parse!(
    ampersand >>
    (Node::And)));

named!(logic_operator<CompleteStr, Node>,
  do_parse!(
    operator: alt!(and | or) >>
    (Node::LogicOperator { children: vec![operator] })));

named!(logic_expression<CompleteStr, Node>,
  do_parse!(
    start: math_expression >> many0!(space) >> colon >> many0!(space) >> end: math_expression >>
    (Node::Range { children: vec![start,end] })));

// #### Other Expressions

named!(range<CompleteStr, Node>,
  do_parse!(
    quote >> text: text >> quote >>
    (Node::String { children: vec![text] })));

named!(string<CompleteStr, Node>,
  do_parse!(
    quote >> text: text >> quote >>
    (Node::String { children: vec![text] })));

named!(expression<CompleteStr, Node>,
  do_parse!(
    expression: alt!(string | range | filter_expression | logic_expression | inline_table | math_expression) >>
    (Node::Expression { children: vec![expression] })));

// ### Block Basics

named!(constraint<CompleteStr, Node>,
  do_parse!(
    space >> space >>
    statement_or_expression: statement >> opt!(new_line_char) >>
    (Node::Constraint { children: vec![statement_or_expression] })));

named!(block<CompleteStr, Node>,
  do_parse!(
    constraints: many1!(constraint) >>
    (Node::Block { children: constraints })));

// ## Markdown

named!(title<CompleteStr, Node>,
  do_parse!(
    hashtag >> space >> text: text >> many0!(whitespace) >>
    (Node::Title { children: vec![text] })));

named!(subtitle<CompleteStr, Node>,
  do_parse!(
    hashtag >> hashtag >> space >> text: text >> many0!(whitespace) >>
    (Node::Subtitle { children: vec![text] })));

named!(paragraph<CompleteStr, Node>,
  do_parse!(
    text: text >> many0!(whitespace) >>
    (Node::Paragraph { children: vec![text] })));

named!(section<CompleteStr, Node>,
  do_parse!(
    section: map!(tuple!(opt!(subtitle), many0!(alt!(block | paragraph))), |tuple| {
      let (mut section_title, mut section_body) = tuple;
      let mut section = vec![];
      match section_title {
        Some(subtitle) => section.push(subtitle),
        _ => (),
      };
      section.append(&mut section_body);
      section
    }) >> many0!(whitespace) >>
    (Node::Section { children: section })));

named!(body<CompleteStr, Node>,
  do_parse!(
    many0!(whitespace) >>
    sections: many1!(section) >>
    (Node::Body { children: sections })));

// ## Start Here

named!(fragment<CompleteStr, Node>,
  do_parse!(
    statement: statement >>
    (Node::Fragment { children: vec![statement] })));

named!(program<CompleteStr, Node>,
  do_parse!(
    program: map!(tuple!(opt!(title),body), |tuple| {
      let (title, body) = tuple;
      let mut program = vec![];
      match title {
        Some(title) => program.push(title),
        None => (),
      };
      program.push(body);
      program
    } ) >> opt!(whitespace) >>
    (Node::Program { children: program })));

named!(parse_mech<CompleteStr, Node>,
  do_parse!(
    program: alt!(fragment | program) >>
    (Node::Root { children: vec![program] })));