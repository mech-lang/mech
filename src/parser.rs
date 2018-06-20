// # Parer

// ## Prelude

use lexer::Token;
use lexer::Token::{HashTag, Alpha, Period, LeftBracket, RightBracket, Digit, Space, Equal, Plus, EndOfStream, Dash, Asterisk, Backslash};
use mech::{Hasher, Function};
use alloc::{String, Vec, fmt};

// ## Node

#[derive(Clone)]
pub enum Node {
  Root{ children: Vec<Node> },
  Block{ children: Vec<Node> },
  Constraint{ children: Vec<Node> },
  Select { children: Vec<Node> },
  Insert { children: Vec<Node> },
  ColumnDefine { children: Vec<Node> },
  Table { children: Vec<Node> },
  Number { children: Vec<Node> },
  MathExpression { children: Vec<Node> },
  InfixOperation { children: Vec<Node>},
  Repeat{ children: Vec<Node> },
  Identifier{ children: Vec<Node> },
  Alpha{ children: Vec<Node> },
  Token{token: Token},
}

impl fmt::Debug for Node {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Node::Root{..} => write!(f, "Root").unwrap(),
      Node::Block{..} => write!(f, "Block").unwrap(),
      Node::Constraint{..} => write!(f, "Constraint").unwrap(),
      Node::Select{..} => write!(f, "Select").unwrap(),
      Node::Insert{..} => write!(f, "Insert").unwrap(),
      Node::MathExpression{..} => write!(f, "Math").unwrap(),
      Node::Table{..} => write!(f, "Table").unwrap(),
      Node::Number{..} => write!(f, "Number").unwrap(),
      Node::ColumnDefine{..} => write!(f, "ColumnDefine").unwrap(),
      Node::InfixOperation{..} => write!(f, "Infix").unwrap(),
      Node::Repeat{..} => write!(f, "Repeat").unwrap(),
      Node::Identifier{..} => write!(f, "Identifier").unwrap(),
      Node::Token{token} => write!(f, "Token({:?})", token).unwrap(),
      _ => write!(f, "Unhandled Node").unwrap(),
    }   
    Ok(())
  }
}

// ## Parser

#[derive(Debug, Clone, PartialEq)]
pub enum ParseStatus {
  Waiting,
  Parsing,
  Error(ParseError),
  Complete,
}

#[derive(Debug, Clone)]
pub struct ParseState {
  pub status: ParseStatus,
  pub token_stack: Vec<Token>,
  pub node_stack: Vec<Node>,
  last_match: usize,
  pub position: usize,
  pub committed: usize,
}

impl ParseState {
  pub fn new() -> ParseState {
    ParseState {
      status: ParseStatus::Parsing,
      node_stack: Vec::new(), 
      token_stack: Vec::new(),
      last_match: 0,
      position: 0,
      committed: 0,
    }
  }

  //pub fn and<F>(&mut self, production: F) -> &mut ParseState
    //where F: Fn(&mut ParseState) -> &mut ParseState {
  pub fn and<F>(&mut self, production: F) -> &mut ParseState
    where F: Fn(&mut ParseState) -> &mut ParseState {
    production(self);
    &mut ParseState::new()    
  }

}

/*

pub fn repeat<F>(production: F, s: &mut ParseState) -> &mut ParseState 
  where F: Fn(&mut ParseState) -> &mut ParseState
{
  let mut once = false;
  let mut result = s;
  while result.status == ParseStatus::Parsing {
    let result = production(result);
    once = true;
  }
  if once {
    result.status = ParseStatus::Parsing;
    let node = Node::Repeat{ children: result.node_stack.drain(..).collect() };
    
  }
  result
}*/


#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
  pub line: usize,
  pub token: Token,
  pub code: u64,
}

#[derive(Clone)]
pub struct Parser {
  pub parse_status: ParseStatus,
  pub tokens: Vec<Token>,
  pub ast: Node,
}

impl Parser {

  pub fn new() -> Parser {
    Parser {
      parse_status: ParseStatus::Waiting,
      tokens: Vec::new(),
      ast: Node::Root{ children: Vec::new()  },
    }
  }

  pub fn add_tokens(&mut self, tokens: &mut Vec<Token>) {
    self.tokens.append(tokens);
  }

  pub fn build_ast(&mut self) {
    let mut s = ParseState::new();
    s.token_stack.append(&mut self.tokens);
    table(&mut s);
    println!("The Result: {:?}", s)
  }
   
}

impl fmt::Debug for Parser {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    
    write!(f, "┌───────────────────────────────────────┐\n").unwrap();
    write!(f, "│ Parser\n").unwrap();
    write!(f, "│ Status: {:?}\n", self.parse_status).unwrap();
    write!(f, "│ Length: {:?}\n", self.tokens.len()).unwrap();
    write!(f, "├───────────────────────────────────────┤\n").unwrap();
    for (ix, token) in self.tokens.iter().enumerate() {
      let c1 = " "; //if self.position == ix + 1 { ">" } else { " " };
      let c2 = " "; //if self.last_match == ix + 1 { ">" } else { " " };
      write!(f, "│ {:}{:} {:?}\n", c1, c2, token).unwrap();
    }
    write!(f, "└───────────────────────────────────────┘\n").unwrap();
    Ok(())
  }
}


pub fn optional(s: &mut ParseState) -> &mut ParseState {
  println!("Optional");
  s.status = ParseStatus::Parsing;
  s
}


pub fn table(s: &mut ParseState) -> &mut ParseState {
  println!("Table");
  let result =  hashtag(s).and(identifier);
  if result.status == ParseStatus::Parsing {
    //let node = Node::Identifier{ children: result.node_stack.drain(..).collect() };
    //result.node_stack.push(node);
  }
  result
}

pub fn identifier(s: &mut ParseState) -> &mut ParseState {
  println!("Identifier");
  let result = repeat(alpha, s);
  if result.status == ParseStatus::Parsing {
    let node = Node::Identifier{ children: result.node_stack.drain(..).collect() };
    result.node_stack.push(node);
  }
  result
}

pub fn repeat<F>(production: F, s: &mut ParseState) -> &mut ParseState 
  where F: Fn(&mut ParseState) -> &mut ParseState
{
  let mut once = false;
  let mut result = s;
  while result.status == ParseStatus::Parsing {
    let result = production(result);
    once = true;
  }
  if once {
    result.status = ParseStatus::Parsing;
    let node = Node::Repeat{ children: result.node_stack.drain(..).collect() };
    
  }
  result
}

pub fn alpha(s: &mut ParseState) -> &mut ParseState {
  println!("Alpha");
  let result = token(s, Token::Alpha);
  result
}

pub fn hashtag(s: &mut ParseState) -> &mut ParseState {
  println!("#");
  let result = token(s, Token::HashTag);
  result
}

pub fn end(s: &mut ParseState) -> &mut ParseState {
  println!("End");
  let result = token(s, Token::EndOfStream);
  result.node_stack.pop();
  result
}

pub fn token(s: &mut ParseState, token: Token) -> &mut ParseState {
  println!("Token: {:?} = {:?}?", token, s.token_stack[s.position]);
  if s.token_stack[s.position] == token {
    s.position += 1;
    s.node_stack.push(Node::Token{token});
  } else {
    s.status = ParseStatus::Error(ParseError{code: 0, line: s.position, token });
  }
  s
}



/*
// #student
pub fn table(&mut self) -> bool {
  println!("Table");
  let result = and_combinator!(self.hash_tag(), self.identifier());
  if !result { self.reset(); }
  else { 
    let token = self.token_stack.pop().unwrap();
    match token {
      Identifier{ref name} =>  {
        let id = Hasher::hash_byte_vector(name);
        self.node_stack.push(Node::Table{id, children: vec![], token: token.clone()})
      },
      _ => (),
    }

  }
  result
}*/



/*
// Creates a function that tests for a token
#[macro_export]
macro_rules! production_rule {
  ($func_name:ident, $token:ident) => (
    fn $func_name(&mut s: ParseState) -> ParseState {
      println!("Leaf");
      let token = if s.position < s.tokens.len() {
        &s.tokens[s.position]
      } else { 
        &EndOfStream 
      };
      let last_match = s.last_match;
      let old_position = s.position;
      match token {
        &$token{..} => {
          s.token_stack.push(token.clone());
          s.position += 1;
          s.last_match = s.position;
          true
        },
        _ => {
          s.last_match = last_match;
          s.position = old_position;
          false
        },
      }
    }
  )
}*/


 /*
    self.parse_status = ParseStatus::Parsing;
    'parse_loop: while {
      let result = and_combinator!{
        self.block()
      };
      self.committed = self.last_match;
      // We're at the end of the tokens
      if self.position == self.tokens.len() {
        self.parse_status = ParseStatus::Complete;
        break 'parse_loop
      }
      result
    } { };
    match self.parse_status {
      ParseStatus::Complete => (), 
      _ => self.parse_status = ParseStatus::Waiting,
    }
    // Put each node left on the stack onto a single root node
    match self.ast {
      Node::Root{ref mut children} => {
        children.append(&mut self.node_stack);
      },
      _ => (),
    }*/
  


/*
  pub fn block(&mut self) -> bool {
    println!("Block");
    let result = or_combinator!(
      self.constraint()
    );
    if !result { self.reset(); }
    else {
      let constraint = self.node_stack.pop().unwrap();
      self.node_stack.push(Node::Block{children: vec![constraint]})
    }
    result
  }

  pub fn constraint(&mut self) -> bool {
    println!("Constraint");
    let result = or_combinator!(
      self.column_define()
    );
    if !result { self.reset(); }
    else {
      let constraint = self.node_stack.pop().unwrap();
      self.node_stack.push(Node::Constraint{children: vec![constraint]})
    }
    result
  }

  pub fn select(&mut self) -> bool {
    println!("Select");
    let result = or_combinator!(self.index());
    if !result { self.reset(); }
    else {
      let table = self.node_stack.pop().unwrap();
      self.node_stack.push(Node::Select{children: vec![table]})
    }
    result
  }

  pub fn insert(&mut self) -> bool {
    println!("Insert");
    let result = or_combinator!(
      self.index()
    );
    if !result { self.reset(); }
    else { 
      let index = self.node_stack.pop().unwrap();
      self.node_stack.push(Node::Insert{ children: vec![index] })
    }
    result
  }


  pub fn expression(&mut self) -> bool {
    println!("Expression");
    let result = and_combinator!(
      self.math_expression()
      //,self.dot_select()
    );
    if !result { self.reset(); }
    result
  }

  // #add[3] = #add[1] + #add[2]
  pub fn column_define(&mut self) -> bool {
    println!("Column Define");
    let result = and_combinator!(
      self.insert(),
      self.space(), 
      self.equal(), 
      self.space(), 
      self.expression()   
    );
    if !result { self.reset(); }
    else { 
      let math_expression = self.node_stack.pop().unwrap();
      let sink = self.node_stack.pop().unwrap();
      self.node_stack.push(Node::ColumnDefine{ parts: vec![sink, math_expression] })
    }
    result
  }

  // #add[1] + #add[2]
  pub fn math_expression(&mut self) -> bool {
    println!("Math Expression");
    let result = and_combinator!(
      self.select(), 
      self.space(), 
      self.infix_operator(), 
      self.space(), 
      self.select()
    );
    if !result { self.reset(); }
    else { 
      let right = self.node_stack.pop().unwrap();
      let op = self.node_stack.pop().unwrap();
      let left = self.node_stack.pop().unwrap();
      self.node_stack.push(Node::MathExpression{parameters: vec![left, op, right] })
    }
    result
  }


  pub fn infix_operator(&mut self) -> bool {
    println!("Infix");
    let result = or_combinator!(
      self.plus(),
      self.dash(), 
      self.asterisk(),
      self.backslash()
    );
    if !result { self.reset(); }
    else { 
      let token = self.token_stack.pop().unwrap();
      self.node_stack.push(Node::InfixOperation{token});
    }
    result
  }






  // #student.grade
  pub fn index(&mut self) -> bool {
    println!("Index");
    let result = or_combinator!(
      self.dot_index(), 
      self.bracket_index()
    );
    if !result { self.reset(); }
    else {}
    result
  }


  // #student.grade
  pub fn dot_index(&mut self) -> bool {
    println!("Dot Index");
    let result = and_combinator!(self.table(), self.period(), or_combinator!(self.identifier(), self.digit()));
    if !result { self.reset(); }
    else {
      let digit = self.token_stack.pop().unwrap();
      let value = get_value(&digit).unwrap();
      let ix = self.node_stack.len() - 1;
      match self.node_stack[ix] {
        Node::Table{ref id, ref token,ref mut children} => {
          let value = get_value(&digit).unwrap();
          children.push(Node::Number{value, token: digit.clone()})
        },
        _ => (),
      }
      
    }
    result
  }

  // #student[1]
  pub fn bracket_index(&mut self) -> bool {
    println!("Bracket Index");
    let result = and_combinator!(self.table(), self.left_bracket(), self.digit(), self.right_bracket());
    if !result { self.reset(); }
    else {
      self.token_stack.pop().unwrap();
      let digit = self.token_stack.pop().unwrap();
      let ix = self.node_stack.len() - 1;
      match self.node_stack[ix] {
        Node::Table{ref id, ref token,ref mut children} => {
          let value = get_value(&digit).unwrap();
          children.push(Node::Number{value, token: digit.clone()})
        },
        _ => (),
      }
    }
    result
  }

  production_rule!{plus, Plus}
  production_rule!{dash, Dash}
  production_rule!{asterisk, Asterisk}
  production_rule!{backslash, Backslash}
  production_rule!{equal, Equal}
  production_rule!{space, Space}
  production_rule!{period, Period}
  production_rule!{left_bracket, LeftBracket}
  production_rule!{right_bracket, RightBracket}
  production_rule!{hash_tag, HashTag}
  production_rule!{identifier, Identifier}
  production_rule!{digit, Digit}
*/