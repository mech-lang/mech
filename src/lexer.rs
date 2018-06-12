// # Lexer

// Takes a string, produces a list of tokens. It works incrementally, meaning 
// you can lex an incomplete string to the end, and then add to it to produce
// more tokens.

// ## Tokens

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
  Digit { value: u8 },
  Identifier{ name: Vec<u8> },
  HashTag,
  LeftBracket,
  RightBracket,
  Comma,
  Space,
  Plus,
  Equal,
  Period,
}


// ## Lexer

#[derive(Debug, Clone)]
pub struct Lexer {
  pub string: String,
  pub tokens: Vec<Token>,
  last_token: usize,
  pub position: usize,
}

impl Lexer {

  pub fn new() -> Lexer {
    Lexer {
      string: String::from(""),
      tokens: Vec::new(),
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

  pub fn push_token(&mut self, token: Token) {
    self.tokens.push(token);
    self.last_token = self.position;
  }

  pub fn get_tokens(&mut self) -> &Vec<Token> {
    let bytes = self.string.clone().into_bytes();
    while self.position < self.string.len() {
      if match_digit(&bytes, self) {
        let extracted = extract_bytes(&bytes, self)[0];
        self.push_token(Token::Digit{value: extracted})
      } else if match_identifier(&bytes, self) { 
        let extracted = extract_bytes(&bytes, self);
        self.push_token(Token::Identifier{name: extracted});
      } else if match_char(&bytes, '#', self) { self.push_token(Token::HashTag);
      } else if match_char(&bytes, '[', self) { self.push_token(Token::LeftBracket);
      } else if match_char(&bytes, ']', self) { self.push_token(Token::RightBracket);
      } else if match_char(&bytes, ',', self) { self.push_token(Token::Comma); 
      } else if match_char(&bytes, ' ', self) { self.push_token(Token::Space); 
      } else if match_char(&bytes, '+', self) { self.push_token(Token::Plus); 
      } else if match_char(&bytes, '=', self) { self.push_token(Token::Equal); 
      } else if match_char(&bytes, '.', self) { self.push_token(Token::Period);      
      } else {
        println!("Unknown Byte {:?} {:?}", bytes[self.position] as char, bytes[self.position]);
        break;
      }
    }
    &self.tokens
  }

}

pub fn match_table(bytes: &Vec<u8>, lexer: &mut Lexer) -> bool {
  let byte = bytes[lexer.position];
  if test_match(byte == '#' as u8, lexer) {
    match_identifier(&bytes, lexer)
  } else {
    false
  }
}

pub fn match_identifier(bytes: &Vec<u8>, lexer: &mut Lexer) -> bool {
  let mut matched = false;
  while match_alpha(&bytes, lexer) {
    matched = true;
    if lexer.position >= bytes.len() {
      break;
    }
  }
  matched
}

pub fn match_alpha(bytes: &Vec<u8>, lexer: &mut Lexer) -> bool {
  let byte = bytes[lexer.position];
  test_match((byte >= 'a' as u8 && byte <= 'z' as u8) || (byte >= 'A' as u8 && byte <= 'Z' as u8), lexer)
}

pub fn match_digit(bytes: &Vec<u8>, lexer: &mut Lexer) -> bool {
  let byte = bytes[lexer.position];
  test_match((byte >= '0' as u8 && byte <= '9' as u8), lexer)
}

pub fn match_char(bytes: &Vec<u8>, character: char, lexer: &mut Lexer) -> bool {
  let byte = bytes[lexer.position];
  test_match(byte == character as u8, lexer)
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