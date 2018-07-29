// # Lexer

// Takes a string, produces a list of tokens. It works incrementally, meaning 
// you can lex an incomplete string to the end, and then add to it to produce
// more tokens.

// ## Tokens

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
  Alpha,
  Digit,
  HashTag,
  LeftBracket,
  RightBracket,
  LeftParenthesis,
  RightParenthesis,
  Caret,
  Comma,
  Semicolon,
  Space,
  Plus,
  Dash,
  Asterisk,
  Slash,
  Equal,
  Period,
  Colon,
  Comma,
  Newline,
  EndOfStream,
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
      if match_alpha(&bytes, self) { self.push_token(Token::Alpha);
      } else if match_digit(&bytes, self) { self.push_token(Token::Digit);
      } else if match_char(&bytes, '#', self) { self.push_token(Token::HashTag);
      } else if match_char(&bytes, '[', self) { self.push_token(Token::LeftBracket);
      } else if match_char(&bytes, ']', self) { self.push_token(Token::RightBracket);
      } else if match_char(&bytes, '(', self) { self.push_token(Token::LeftParenthesis);
      } else if match_char(&bytes, ')', self) { self.push_token(Token::RightParenthesis);
      } else if match_char(&bytes, ',', self) { self.push_token(Token::Comma); 
      } else if match_char(&bytes, ' ', self) { self.push_token(Token::Space); 
      } else if match_char(&bytes, '+', self) { self.push_token(Token::Plus); 
      } else if match_char(&bytes, '-', self) { self.push_token(Token::Dash); 
      } else if match_char(&bytes, '*', self) { self.push_token(Token::Asterisk); 
      } else if match_char(&bytes, '/', self) { self.push_token(Token::Slash); 
      } else if match_char(&bytes, '^', self) { self.push_token(Token::Caret); 
      } else if match_char(&bytes, '=', self) { self.push_token(Token::Equal); 
      } else if match_char(&bytes, '.', self) { self.push_token(Token::Period);      
      } else if match_char(&bytes, ',', self) { self.push_token(Token::Comma);
      } else if match_char(&bytes, ';', self) { self.push_token(Token::Semicolon);
      } else if match_char(&bytes, ':', self) { self.push_token(Token::Colon);
      } else if match_char(&bytes, '\n', self) { self.push_token(Token::Newline);
      } else {
        println!("Unknown Byte {:?} {:?}", bytes[self.position] as char, bytes[self.position]);
        break;
      }
    }
    self.push_token(Token::EndOfStream);
    &self.tokens
  }

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