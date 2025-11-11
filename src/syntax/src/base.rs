#[macro_use]
use crate::parser::*;
use crate::*;
use crate::label;
use crate::labelr;
use nom::{
  multi::separated_list0,
  sequence::tuple as nom_tuple,
};
use crate::nodes::Kind;

// Lexical Elements
// ----------------------------------------------------------------------------
// Ref: #58393432045966419

macro_rules! leaf {
  ($name:ident, $byte:expr, $token:expr) => (
    pub fn $name(input: ParseString) -> ParseResult<Token> {
      if input.is_empty() {
        return Err(nom::Err::Error(ParseError::new(input, "Unexpected eof")))
      }
      let start = input.loc();
      let byte = input.graphemes[input.cursor];
      let (input, _) = tag($byte)(input)?;
      let end = input.loc();
      let src_range = SourceRange { start, end };
      Ok((input, Token{kind: $token, chars: $byte.chars().collect::<Vec<char>>(), src_range}))
    }
  )
}

macro_rules! ws0_leaf {
  ($name:ident, $byte:expr, $token:expr) => (
    pub fn $name(input: ParseString) -> ParseResult<Token> {
      if input.is_empty() {
        return Err(nom::Err::Error(ParseError::new(input, "Unexpected eof")))
      }
      let start = input.loc();
      let byte = input.graphemes[input.cursor];
      let (input, _) = whitespace0(input)?;
      let (input, _) = tag($byte)(input)?;
      let (input, _) = whitespace0(input)?;
      let end = input.loc();
      let src_range = SourceRange { start, end };
      Ok((input, Token{kind: $token, chars: $byte.chars().collect::<Vec<char>>(), src_range}))
    }
  )
}

macro_rules! ws1_leaf {
  ($name:ident, $byte:expr, $token:expr) => (
    pub fn $name(input: ParseString) -> ParseResult<Token> {
      if input.is_empty() {
        return Err(nom::Err::Error(ParseError::new(input, "Unexpected eof")))
      }
      let (input, _) = whitespace1(input)?;
      let start = input.loc();
      let byte = input.graphemes[input.cursor];
      let (input, _) = tag($byte)(input)?;
      let end = input.loc();
      let (input, _) = whitespace1(input)?;
      let src_range = SourceRange { start, end };
      Ok((input, Token{kind: $token, chars: $byte.chars().collect::<Vec<char>>(), src_range}))
    }
  )
}

// Tokens
// ----------------------------------------------------------------------------
// Ref: 39003557984811317

leaf!{ampersand, "&", TokenKind::Ampersand}
leaf!{apostrophe, "'", TokenKind::Apostrophe}
leaf!{asterisk, "*", TokenKind::Asterisk}
leaf!{at, "@", TokenKind::At}
leaf!{bar, "|", TokenKind::Bar}
leaf!{backslash, "\\", TokenKind::Backslash}
leaf!{caret, "^", TokenKind::Caret}
leaf!{colon, ":", TokenKind::Colon}
leaf!{comma, ",", TokenKind::Comma}
leaf!{dash, "-", TokenKind::Dash}
leaf!{dollar, "$", TokenKind::Dollar}
leaf!{equal, "=", TokenKind::Equal}
leaf!{exclamation, "!", TokenKind::Exclamation}
leaf!{grave, "`", TokenKind::Grave}
leaf!{hashtag, "#", TokenKind::HashTag}
leaf!{negate, "¬", TokenKind::Not}
leaf!{percent, "%", TokenKind::Percent}
leaf!{period, ".", TokenKind::Period}
leaf!{plus, "+", TokenKind::Plus}
leaf!{question, "?", TokenKind::Question}
leaf!{quote, "\"", TokenKind::Quote}
leaf!{semicolon, ";", TokenKind::Semicolon}
leaf!{slash, "/", TokenKind::Slash}
leaf!{tilde, "~", TokenKind::Tilde}
leaf!{underscore, "_", TokenKind::Underscore}

leaf!{check_mark, "✓", TokenKind::True}
leaf!{cross, "✗", TokenKind::False}
leaf!{english_true_literal, "true", TokenKind::True}
leaf!{english_false_literal, "false", TokenKind::False}

leaf!{space, " ", TokenKind::Space}
leaf!{new_line_char, "\n", TokenKind::Newline}
leaf!{carriage_return, "\r", TokenKind::CarriageReturn}
leaf!{carriage_return_new_line, "\r\n", TokenKind::CarriageReturn}
leaf!{tab, "\t", TokenKind::Tab}

leaf!{left_bracket, "[", TokenKind::LeftBracket}
leaf!{left_parenthesis, "(", TokenKind::LeftParenthesis}
leaf!{left_brace, "{", TokenKind::LeftBrace}
leaf!{left_angle, "<", TokenKind::LeftAngle}

leaf!{right_bracket, "]", TokenKind::RightBracket}
leaf!{right_parenthesis, ")", TokenKind::RightParenthesis}
leaf!{right_brace, "}", TokenKind::RightBrace}
leaf!{right_angle, ">", TokenKind::RightAngle}

leaf!{box_tl_round, "╭", TokenKind::BoxDrawing}
leaf!{box_tr_round, "╮", TokenKind::BoxDrawing}
leaf!{box_bl_round, "╰", TokenKind::BoxDrawing}
leaf!{box_br_round, "╯", TokenKind::BoxDrawing}

leaf!{box_tl_bold, "┏", TokenKind::BoxDrawing}
leaf!{box_tr_bold, "┓", TokenKind::BoxDrawing} 
leaf!{box_bl_bold, "┗", TokenKind::BoxDrawing}
leaf!{box_br_bold, "┛", TokenKind::BoxDrawing}

leaf!{box_tl, "┌", TokenKind::BoxDrawing}
leaf!{box_tr, "┐", TokenKind::BoxDrawing}
leaf!{box_bl, "└", TokenKind::BoxDrawing}
leaf!{box_br, "┘", TokenKind::BoxDrawing}

leaf!{box_cross, "┼", TokenKind::BoxDrawing}
leaf!{box_horz, "─", TokenKind::BoxDrawing}
leaf!{box_t_left, "├", TokenKind::BoxDrawing}
leaf!{box_t_right, "┤", TokenKind::BoxDrawing}
leaf!{box_t_top, "┬", TokenKind::BoxDrawing}
leaf!{box_t_bottom, "┴", TokenKind::BoxDrawing}
leaf!{box_vert, "│", TokenKind::BoxDrawing}
leaf!{box_vert_bold, "┃", TokenKind::BoxDrawing}

leaf!(abstract_sigil, "%%", TokenKind::AbstractSigil);
leaf!(emphasis_sigil, "*", TokenKind::EmphasisSigil);
leaf!(equation_sigil, "$$", TokenKind::EquationSigil);
leaf!(footnote_prefix, "[^", TokenKind::FootnotePrefix);
leaf!(float_left, "<<", TokenKind::FloatLeft);
leaf!(float_right, ">>", TokenKind::FloatRight);
leaf!(http_prefix, "http", TokenKind::HttpPrefix);
leaf!(highlight_sigil, "!!", TokenKind::HighlightSigil);
leaf!(img_prefix, "![", TokenKind::ImgPrefix);
leaf!(question_sigil, "(?)>", TokenKind::QuestionSigil);
leaf!(quote_sigil, ">", TokenKind::QuoteSigil);
leaf!(info_sigil, "(!)>", TokenKind::InfoSigil);
leaf!(strike_sigil, "~~", TokenKind::StrikeSigil);
leaf!(strong_sigil, "**", TokenKind::StrongSigil);
leaf!(grave_codeblock_sigil, "```", TokenKind::GraveCodeBlockSigil);
leaf!(tilde_codeblock_sigil, "~~~", TokenKind::TildeCodeBlockSigil);
leaf!(underline_sigil, "__", TokenKind::UnderlineSigil);

ws0_leaf!(assign_operator, "=", TokenKind::AssignOperator);
ws0_leaf!(async_transition_operator, "~>", TokenKind::AsyncTransitionOperator);
ws0_leaf!(define_operator, ":=", TokenKind::DefineOperator);
ws0_leaf!(output_operator, "=>", TokenKind::OutputOperator);
ws0_leaf!(transition_operator, "->", TokenKind::TransitionOperator);


// emoji-grapheme := ?emoji-grapheme-literal? ;
pub fn emoji_grapheme(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_emoji() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
  }
}

// alpha := ?alpha-literal? ;
pub fn alpha(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_alpha() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
  }
}

// digit := ?digit-literal? ;
pub fn digit(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_digit() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
  }
}

// any := ?any-character? ;
pub fn any(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_one() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected eof")))
  }
}

pub fn any_token(mut input: ParseString) -> ParseResult<Token> {
  if input.is_empty() {
    return Err(nom::Err::Error(ParseError::new(input, "Unexpected eof")))
  }
  let start = input.loc();
  let byte = input.graphemes[input.cursor];
  if let Some(matched) = input.consume_one() {
    let end = input.loc();
    let src_range = SourceRange { start, end };
    Ok((input, Token{kind: TokenKind::Any, chars: byte.chars().collect::<Vec<char>>(), src_range}))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected eof")))
  }
}

// forbidden-emoji := box-drawing | other-forbidden-shapes ;
pub fn forbidden_emoji(input: ParseString) -> ParseResult<Token> {
  alt((box_tl, box_br, box_bl, box_tr, box_tr_bold, box_tl_bold, box_br_bold, box_bl_bold, box_t_left,box_tl_round,box_br_round, box_tr_round, box_bl_round, box_vert, box_cross, box_horz, box_t_right, box_t_top, box_t_bottom))(input)
}

// emoji := (!forbidden-emoji, emoji-grapheme) ;
pub fn emoji(input: ParseString) -> ParseResult<Token> {
  let msg1 = "Cannot be a box-drawing emoji";
  let start = input.loc();
  let (input, _) = is_not(forbidden_emoji)(input)?;
  let (input, g) = emoji_grapheme(input)?;
  let end = input.loc();
  let src_range = SourceRange { start, end };
  Ok((input, Token{kind: TokenKind::Emoji, chars: g.chars().collect::<Vec<char>>(), src_range}))
}

// alpha-token := alpha-literal-token ;
pub fn alpha_token(input: ParseString) -> ParseResult<Token> {
  let (input, (g, src_range)) = range(alpha)(input)?;
  Ok((input, Token{kind: TokenKind::Alpha, chars: g.chars().collect::<Vec<char>>(), src_range}))
}

// digit-token := digit-literal-token ;
pub fn digit_token(input: ParseString) -> ParseResult<Token> {
  let (input, (g, src_range)) = range(digit)(input)?;
  Ok((input, Token{kind: TokenKind::Digit, chars: g.chars().collect::<Vec<char>>(), src_range}))
}

// alphanumeric := alpha | digit ;
pub fn alphanumeric(input: ParseString) -> ParseResult<Token> {
  let (input, token) = alt((alpha_token, digit_token))(input)?; 
  Ok((input, token))
}

// underscore-digit := underscore, digit ;
pub fn underscore_digit(input: ParseString) -> ParseResult<Token> {
  let (input, _) = underscore(input)?;
  let (input, digit) = digit_token(input)?;
  Ok((input,digit))
}

// digit-sequence := digit, (underscore-digit | digit)* ;
pub fn digit_sequence(input: ParseString) -> ParseResult<Vec<Token>> {
  let (input, mut start) = digit_token(input)?;
  let (input, mut tokens) = many0(alt((underscore_digit,digit_token)))(input)?;
  let mut all = vec![start];
  all.append(&mut tokens);
  Ok((input,all))
}

// grouping-symbol := left-parenthesis | right-parenthesis | left-angle | right-angle | left-brace | right-brace | left-bracket | right-bracket ;
pub fn grouping_symbol(input: ParseString) -> ParseResult<Token> {
  let (input, grouping) = alt((left_parenthesis, right_parenthesis, left_angle, right_angle, left_brace, right_brace, left_bracket, right_bracket))(input)?;
  Ok((input, grouping))
}

// punctuation := period | exclamation | question | comma | colon | semicolon | quote | apostrophe ;
pub fn punctuation(input: ParseString) -> ParseResult<Token> {
  let (input, punctuation) = alt((period, exclamation, question, comma, colon, semicolon, quote, apostrophe))(input)?;
  Ok((input, punctuation))
}

// escaped-char := "\" ,  alpha | symbol | punctuation ;
pub fn escaped_char(input: ParseString) -> ParseResult<Token> {
  let (input, _) = backslash(input)?;
  let (input, symbol) = alt((alpha_token, symbol, punctuation))(input)?;
  Ok((input, symbol))
}

// symbol := ampersand | dollar | bar | percent | at | slash | hashtag | equal | backslash | tilde | plus | dash | asterisk | caret | underscore ;
pub fn symbol(input: ParseString) -> ParseResult<Token> {
  let (input, symbol) = alt((ampersand, grave, dollar, bar, percent, at, slash, hashtag, equal, backslash, tilde, plus, dash, asterisk, caret, underscore))(input)?;
  Ok((input, symbol))
}

// identifier-symbol := ampersand | dollar | bar | percent | at | slash | hashtag | backslash | tilde | plus | dash | asterisk | caret ;
pub fn identifier_symbol(input: ParseString) -> ParseResult<Token> {
  let (input, symbol) = alt((ampersand, dollar, bar, percent, at, slash, hashtag, backslash, tilde, plus, dash, asterisk, caret))(input)?;
  Ok((input, symbol))
}

// text := alpha | digit | space | tab | escaped_char | punctuation | grouping_symbol | symbol ;
pub fn text(input: ParseString) -> ParseResult<Token> {
  let (input, text) = alt((alpha_token, digit_token, emoji, forbidden_emoji, space, tab, escaped_char, punctuation, grouping_symbol, symbol))(input)?;
  Ok((input, text))
}

// Whitespace
// ============================================================================
// Ref: #35070717845239353

// new-line := (carriage-return, new-line) | new-line-char | carriage-return ;
pub fn new_line(input: ParseString) -> ParseResult<Token> {
  let (input, result) = alt((carriage_return_new_line,new_line_char,carriage_return))(input)?;
  Ok((input, result))
}

// whitespace := space | new-line | tab ;
pub fn whitespace(input: ParseString) -> ParseResult<Token> {
  let (input, space) = alt((space,tab,new_line))(input)?;
  Ok((input, space))
}

// ws0 := *whitespace ;
pub fn whitespace0(input: ParseString) -> ParseResult<()> {
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, ()))
}

// ws1 := +whitespace ;
pub fn whitespace1(input: ParseString) -> ParseResult<()> {
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, ()))
}

// newline-indent := new-line, *space-tab ;
pub fn newline_indent(input: ParseString) -> ParseResult<()> {
  let (input, _) = new_line(input)?;
  let (input, _) = many0(space_tab)(input)?;
  Ok((input, ()))
}

// ws0e := ws0, newline-indent? ;
pub fn ws1e(input: ParseString) -> ParseResult<()> {
  let (input, _) = many1(space_tab)(input)?;
  let (input, _) = opt(newline_indent)(input)?;
  Ok((input, ()))
}

// space-tab := space | tab ;
pub fn space_tab(input: ParseString) -> ParseResult<Token> {
  let (input, space) = alt((space,tab))(input)?;
  Ok((input, space))
}

// list-separator := ws0, ",", ws0 ;
pub fn list_separator(input: ParseString) -> ParseResult<()> {
  let (input,_) = nom_tuple((whitespace0,tag(","),whitespace0))(input)?;
  Ok((input, ()))
}

// enum-separator := ws0*, "|", ws0 ;
pub fn enum_separator(input: ParseString) -> ParseResult<()> {
  let (input,_) = nom_tuple((whitespace0,tag("|"),whitespace0))(input)?;
  Ok((input, ()))
}

// Identifiers
// ----------------------------------------------------------------------------
// Ref: #40075932908181571

// identifier := (alpha | emoji), (alpha | digit | identifier_symbol | emoji)* ;
pub fn identifier(input: ParseString) -> ParseResult<Identifier> {
  let (input, (first, mut rest)) = nom_tuple((alt((alpha_token, emoji)), many0(alt((alpha_token, digit_token, identifier_symbol, emoji)))))(input)?;
  let mut tokens = vec![first];
  tokens.append(&mut rest);
  let mut merged = Token::merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Identifier; 
  Ok((input, Identifier{name: merged}))
}