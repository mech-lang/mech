// # Lexer

// A list of possible tokens. The lexer used to be its own thing, but most of 
// the work done here has been moved into the parser. This is all that remains
// of that distant past.

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
  At,
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