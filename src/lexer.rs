// # Lexer

// Takes a string, produces a list of tokens. It works incrementally, meaning 
// you can lex an incomplete string to the end, and then add to it to produce
// more tokens.

// ## Tokens

#[derive(Clone, Debug)]
pub enum Token {
  Table { name: Vec<u8> },
  LeftBracket,
  RightBracket,
}


// ## Lexer

#[derive(Debug, Clone)]
pub struct Lexer {
  pub string: String,
  last_token: usize,
  pub position: usize,
}

impl Lexer {

  pub fn new() -> Lexer {
    Lexer {
      string: String::from(""),
      last_token: 0,
      position: 0,
    }
  }

  pub fn add_chars(&mut self, chars: &str) {
    self.string.push_str(chars);
  }

  pub fn add_string(&mut self, string: String) {
    self.string.push_str(&string);
  }

  pub fn advance_token(&mut self) {
    self.last_token = self.position;
  }

  pub fn get_tokens(&mut self) -> Vec<Token>{
    let mut tokens = Vec::new();
    let bytes = self.string.clone().into_bytes();
    while self.position < self.string.len() {
      if match_table(&bytes, self) {
        self.last_token += 1;
        tokens.push(Token::Table{name: extract_bytes(&bytes, self)});
        self.advance_token();
      } else if match_left_bracket(&bytes, self) {
        tokens.push(Token::LeftBracket);
        self.advance_token();
      } else if match_right_bracket(&bytes, self) {
        tokens.push(Token::RightBracket);
        self.advance_token();
      } else {
        println!("Unknown Byte Sequence {:?}", self);
        self.position = self.string.len();
      }
    }
    tokens
  }

}

pub fn match_table(bytes: &Vec<u8>, lexer: &mut Lexer) -> bool {
  let byte = bytes[lexer.position];
  if test_match(bytes[lexer.position] == '#' as u8, lexer) {
    while match_alpha(&bytes, lexer){
    }
    true
  } else {
    false
  }
}

pub fn match_alpha(bytes: &Vec<u8>, lexer: &mut Lexer) -> bool {
  let byte = bytes[lexer.position];
  test_match((byte >= 'a' as u8 && byte <= 'z' as u8) || (byte >= 'A' as u8 && byte <= 'Z' as u8), lexer)
}

pub fn match_left_bracket(bytes: &Vec<u8>, lexer: &mut Lexer) -> bool {
  let byte = bytes[lexer.position];
  test_match(byte == '[' as u8, lexer)
}

pub fn match_right_bracket(bytes: &Vec<u8>, lexer: &mut Lexer) -> bool {
  let byte = bytes[lexer.position];
  test_match(byte == ']' as u8, lexer)
}

pub fn test_match(test: bool, lexer: &mut Lexer) -> bool {
  if test {
    lexer.position += 1;
  }
  test
}

pub fn extract_bytes(bytes: &Vec<u8>, lexer: &mut Lexer) -> Vec<u8> {
  let mut v = Vec::new();
  for i in lexer.last_token .. lexer.position {
    v.push(bytes[i]);
  }
  v
}
