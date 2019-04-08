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
  LeftBrace,
  RightBrace,
  Caret,
  Semicolon,
  Space,
  Plus,
  Dash,
  Underscore,
  Asterisk,
  Slash,
  Apostrophe,
  Equal,
  LessThan,
  GreaterThan,
  Exclamation,
  Question,
  Period,
  Colon,
  Comma,
  Tilde,
  Grave,
  Bar,
  Quote,
  Ampersand,
  Percent,
  Newline,
  CarriageReturn,
  Tab,
  EndOfStream,
}