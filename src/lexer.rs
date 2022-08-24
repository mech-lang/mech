// use unicode_segmentation::UnicodeSegmentation;
// 
// use std::fs;
// 
// #[derive(Debug, Clone, PartialEq)]
// pub enum TokenType {
//   Plus,
//   Minus,
//   Star,
//   Slash,
//   Equal,
//   LeftParen,
//   RightParen,
//   SemiColon,
// 
//   Set,
//   Print,
// 
//   Identifier,
//   Number,
// 
//   Newline,
//   Eof,
// }
// 
// #[derive(Debug, Clone)]
// pub struct Token {
//   pub token_type: TokenType,
//   pub lexeme: String,
//   pub len: usize,       // # of graphemes that makes up the token
//   pub row: usize,       // row # of the token (in grapheme)
//   pub col: usize,       // col # of the token (in grapheme)
// }
// 
// #[derive(Clone)]
// pub struct TokenStream {
//   cursor: usize,
//   tokens: Vec<Token>,
//   filename: String,
//   source: String,
//   line_rngs: Vec<(usize, usize)>,
// }
// 
// impl TokenStream {
//   /// Peek at the current token.
//   pub fn peek(&self) -> &Token
//   {
//     &self.tokens[self.cursor]
//   }
// 
//   /// Peek the token at index.
//   pub fn at(&self, i: usize) -> &Token
//   {
//     &self.tokens[i]
//   }
// 
//   /// There are more tokens to parse.
//   pub fn has_next(&self) -> bool
//   {
//     self.peek().token_type != TokenType::Eof
//   }
// 
//   /// Consume the current token.
//   pub fn next(&mut self) -> Token
//   {
//     let t = self.peek().clone();
//     self.cursor += 1;
//     t
//   }
// 
//   /// Get the internal token index that points to the current token.
//   pub fn index(&self) -> usize
//   {
//     self.cursor
//   }
// 
//   /// Get a line of source code.
//   pub fn get_src_line(&self, line: usize) -> &str
//   {
//     let (a, b) = self.line_rngs[line-1];
//     &self.source[a..b]
//   }
// 
//   /// Get # of graphemes that comprises one line of source code.
//   pub fn get_line_len(&self, line: usize) -> usize
//   {
//     let (a, b) = self.line_rngs[line-1];
//     UnicodeSegmentation::graphemes(
//       &self.source[a..b], true).collect::<Vec<&str>>().len()
//   }
// 
//   /// A TokenStream may be constructed from lexing a file. Get the filename.
//   pub fn filename(&self) -> &str
//   {
//     &self.filename
//   }
// }
// 
// /// For nom
// impl nom::InputLength for TokenStream {
//   fn input_len(&self) -> usize
//   {
//     self.tokens.len() - self.cursor
//   }
// }
// 
// #[derive(Debug)]
// pub struct LexerError {
//   pub row: usize,
//   pub col: usize,
//   pub message: String,
// }
// 
// struct LexingContext<'a> {
//   line_start: usize,
//   cursor: usize,
//   probe: usize,
//   row: usize,
//   col: usize,
//   result: Vec<Token>,
//   errors: Vec<LexerError>,
//   line_rngs: Vec<(usize, usize)>,
//   graphemes: Vec<&'a str>,
// }
// 
// impl<'a> LexingContext<'a> {
//   /// A new LexingContext is created at the beginning of lexing procedure.
//   fn new(input: &'a String) -> Self
//   {
//     LexingContext {
//       line_start: 0,
//       cursor: 0,
//       probe: 0,
//       row: 1,
//       col: 1,
//       result: vec![],
//       errors: vec![],
//       line_rngs: vec![],
//       graphemes: UnicodeSegmentation::graphemes(
//                    input.as_str(), true).collect::<Vec<&'a str>>(),
//     }
//   }
// 
//   /// Check if the lexing process has encountered errors.
//   fn had_error(&self) -> bool
//   {
//     !self.errors.is_empty()
//   }
// 
//   /// Check if there are more graphemes to be scanned.
//   fn has_next(&self) -> bool
//   {
//     self.probe < self.graphemes.len()
//   }
// 
//   /// Peek at the next grapheme to scan, without modifying any state.
//   fn peek_next(&mut self) -> &'a str
//   {
//     self.graphemes[self.probe]
//   }
// 
//   /// Advance the probe pointer to the next grapheme, and returns the
//   /// one that's skipped.
//   fn next(&mut self) -> &'a str
//   {
//     let curr = self.graphemes[self.probe];
//     self.probe += 1;
//     curr
//   }
// 
//   /// Inspect the string made up by all graphemes between cursor and
//   /// probe, without modifying any state. The same string will be commited
//   /// if `commit` is called.
//   fn inspect(&self) -> String
//   {
//     let mut s = String::from("");
//     let mut i = self.cursor;
//     while i < self.probe {
//       s.push_str(self.graphemes[i]);
//       i += 1;
//     }
//     return s;
//   }
// 
//   /// Commit the string made up by all graphemes between cursor and
//   /// probe into a Token in result vector, then advance cursor to
//   /// probe, and finally update the column count.
//   fn commit(&mut self, token_type: TokenType)
//   {
//     let len = self.probe - self.cursor;
//     let mut lexeme = String::from("");  // TODO: unnecessary heap allocation
//     while self.cursor < self.probe {
//       lexeme.push_str(self.graphemes[self.cursor]);
//       self.cursor += 1;
//     }
//     self.result.push(Token {
//       token_type,
//       lexeme,
//       len,
//       row: self.row,
//       col: self.col,
//     });
//     self.col += len;
//   }
// 
//   /// (Only) update the row/col count information.
//   fn newline(&mut self)
//   {
//     self.line_rngs.push((self.line_start, self.probe));
//     self.line_start = self.probe;
//     self.col = 1;
//     self.row += 1;
//   }
// 
//   /// The same as `commit`, except nothing is pushed into result.
//   fn skip(&mut self)
//   {
//     self.col += self.probe - self.cursor;
//     self.cursor = self.probe;
//   }
// 
//   /// The same as `commit`, except instead of pushing a Token into
//   /// result vector, an error message is pushed into errors vector.
//   fn error(&mut self, grapheme: &str)
//   {
//     self.errors.push(LexerError {
//       row: self.row,
//       col: self.col,
//       message: format!("Unexpected character: {}", grapheme),
//     });
//     self.skip();
//   }
// }
// 
// /// Check if the grapheme represents newline. This may need further
// /// work to be supported over different platforms.
// fn is_newline(grapheme: &str) -> bool
// {
//   for c in grapheme.chars() {
//     if c == '\n' {
//       return true;
//     }
//   }
//   false
// }
// 
// /// Check if the grapheme represents whitespace. Note that newlines
// /// are also considered as whitespace characters.
// fn is_whitespace(grapheme: &str) -> bool
// {
//   for c in grapheme.chars() {
//     if !c.is_whitespace() {
//       return false;
//     }
//   }
//   true
// }
// 
// /// Check if the grapheme represents an 'alpha character'.
// /// In this case we want emojis to be valid.
// fn is_alpha(grapheme: &str) -> bool
// {
//   if grapheme.len() == 1 {
//     return grapheme.chars().next().unwrap().is_alphabetic();
//   }
//   // accept everything else for now
//   true
// }
// 
// /// Check if the grapheme is a digit in ascii table.
// fn is_number(grapheme: &str) -> bool
// {
//   if grapheme.len() != 1 {
//     return false;
//   }
//   grapheme.chars().next().unwrap().is_digit(10)
// }
// 
// /// The major `lex` function, called by public interfaces
// fn lex(mut input: String) -> Result<TokenStream, Vec<LexerError>>
// {
//   if input.len() != 0 && input.chars().last().unwrap() != '\n' {
//     input.push('\n');  // make sure the last line ends with '\n'
//   }
//   let mut ctx = LexingContext::new(&input);
// 
//   while ctx.has_next() {
//     match ctx.next() {
//       "+" => ctx.commit(TokenType::Plus),
//       "-" => ctx.commit(TokenType::Minus),
//       "*" => ctx.commit(TokenType::Star),
//       "/" => ctx.commit(TokenType::Slash),
//       "=" => ctx.commit(TokenType::Equal),
//       "(" => ctx.commit(TokenType::LeftParen),
//       ")" => ctx.commit(TokenType::RightParen),
//       ";" => ctx.commit(TokenType::SemiColon),
//       g => {
//         if is_number(g) {
//           while is_number(ctx.peek_next()) {
//             ctx.next();
//           }
//           ctx.commit(TokenType::Number);
//         } else if is_alpha(g) {
//           while is_alpha(ctx.peek_next()) {
//             ctx.next();
//           }
//           match &ctx.inspect()[..] {
//             "set" => ctx.commit(TokenType::Set),
//             "print" => ctx.commit(TokenType::Print),
//             _ => ctx.commit(TokenType::Identifier),
//           }
//         } else if is_newline(g) {
//           ctx.commit(TokenType::Newline);
//           ctx.newline();
//         } else if is_whitespace(g) {
//           ctx.skip();
//         } else {
//           ctx.error(g);
//         }
//       },
//     }
//   }
// 
//   if ctx.had_error() {
//     return Err(ctx.errors);
//   }
// 
//   ctx.commit(TokenType::Eof);
// 
//   Ok(TokenStream {
//     cursor: 0,
//     tokens: ctx.result,
//     line_rngs: ctx.line_rngs,
//     filename: String::from("<filename>"),
//     source: input,
//   })
// }
// 
// /// public interface for file input
// pub fn lex_file(filename: &String) -> Result<TokenStream, String>
// {
//   match fs::read_to_string(&filename) {
//     Ok(content) => {
//       match lex(content) {
//         Ok(mut tokens) => {
//           tokens.filename = filename.clone();
//           Ok(tokens)
//         },
//         Err(_) => Err(String::from("Lexer error")),
//       }
//     },
//     Err(_) => Err(format!("Unable to read file: {}", filename)),
//   }
// }
//  



















































// # Lexer

// A list of possible tokens. The lexer used to be its own thing, but most of 
// the work done here has been moved into the parser. This is all that remains
// of that distant past.

// ## Tokens

#[derive(Clone, Copy, Debug, PartialEq)]
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
  Backslash,
  Quote,
  Ampersand,
  Percent,
  Newline,
  CarriageReturn,
  Tab,
  EndOfStream,
  Emoji,
}
