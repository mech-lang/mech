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

leaf!{at, "@", TokenKind::At}
leaf!{hashtag, "#", TokenKind::HashTag}
leaf!{period, ".", TokenKind::Period}
leaf!{colon, ":", TokenKind::Colon}
leaf!{comma, ",", TokenKind::Comma}
leaf!{percent, "%", TokenKind::Percent}
leaf!{apostrophe, "'", TokenKind::Apostrophe}
leaf!{left_bracket, "[", TokenKind::LeftBracket}
leaf!{right_bracket, "]", TokenKind::RightBracket}
leaf!{left_parenthesis, "(", TokenKind::LeftParenthesis}
leaf!{right_parenthesis, ")", TokenKind::RightParenthesis}
leaf!{left_brace, "{", TokenKind::LeftBrace}
leaf!{right_brace, "}", TokenKind::RightBrace}
leaf!{dollar, "$", TokenKind::Dollar}
leaf!{equal, "=", TokenKind::Equal}
leaf!{left_angle, "<", TokenKind::LeftAngle}
leaf!{right_angle, ">", TokenKind::RightAngle}
leaf!{exclamation, "!", TokenKind::Exclamation}
leaf!{question, "?", TokenKind::Question}
leaf!{plus, "+", TokenKind::Plus}
leaf!{dash, "-", TokenKind::Dash}
leaf!{underscore, "_", TokenKind::Underscore}
leaf!{asterisk, "*", TokenKind::Asterisk}
leaf!{slash, "/", TokenKind::Slash}
leaf!{backslash, "\\", TokenKind::Backslash}
leaf!{caret, "^", TokenKind::Caret}
leaf!{space, " ", TokenKind::Space}
leaf!{tab, "\t", TokenKind::Tab}
leaf!{tilde, "~", TokenKind::Tilde}
leaf!{grave, "`", TokenKind::Grave}
leaf!{bar, "|", TokenKind::Bar}
leaf!{quote, "\"", TokenKind::Quote}
leaf!{ampersand, "&", TokenKind::Ampersand}
leaf!{semicolon, ";", TokenKind::Semicolon}
leaf!{new_line_char, "\n", TokenKind::Newline}
leaf!{carriage_return, "\r", TokenKind::CarriageReturn}
leaf!{carriage_return_new_line, "\r\n", TokenKind::CarriageReturn}
leaf!{english_true_literal, "true", TokenKind::True}
leaf!{english_false_literal, "false", TokenKind::False}
leaf!{check_mark, "✓", TokenKind::True}
leaf!{cross, "✗", TokenKind::False}
leaf!{box_tl_round, "╭", TokenKind::BoxDrawing}
leaf!{box_tr_round, "╮", TokenKind::BoxDrawing}
leaf!{box_bl_round, "╰", TokenKind::BoxDrawing}
leaf!{box_br_round, "╯", TokenKind::BoxDrawing}
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

// emoji_grapheme := ?emoji_grapheme_literal? ;
pub fn emoji_grapheme(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_emoji() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
  }
}

// alpha := ?alpha_literal? ;
pub fn alpha(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_alpha() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
  }
}

// digit := ?digit_literal? ;
pub fn digit(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_digit() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
  }
}

// any := ?any_character? ;
pub fn any(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_one() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected eof")))
  }
}

// forbidden_emoji := box_drawing | other_forbidden_shapes ;
pub fn forbidden_emoji(input: ParseString) -> ParseResult<Token> {
  alt((box_tl, box_br, box_bl, box_tr, box_t_left,box_tl_round,box_br_round, box_tr_round, box_bl_round, box_vert, box_cross, box_horz, box_t_right, box_t_top, box_t_bottom))(input)
}

// emoji := emoji_grapheme+ ;
pub fn emoji(input: ParseString) -> ParseResult<Token> {
  let msg1 = "Cannot be a box-drawing emoji";
  let start = input.loc();
  let (input, _) = is_not(forbidden_emoji)(input)?;
  let (input, g) = emoji_grapheme(input)?;
  let end = input.loc();
  let src_range = SourceRange { start, end };
  Ok((input, Token{kind: TokenKind::Emoji, chars: g.chars().collect::<Vec<char>>(), src_range}))
}

// alpha_token := alpha_literal_token ;
pub fn alpha_token(input: ParseString) -> ParseResult<Token> {
  let (input, (g, src_range)) = range(alpha)(input)?;
  Ok((input, Token{kind: TokenKind::Alpha, chars: g.chars().collect::<Vec<char>>(), src_range}))
}

// digit_token := digit_literal_token ;
pub fn digit_token(input: ParseString) -> ParseResult<Token> {
  let (input, (g, src_range)) = range(digit)(input)?;
  Ok((input, Token{kind: TokenKind::Digit, chars: g.chars().collect::<Vec<char>>(), src_range}))
}

// underscore_digit := underscore, digit ;
pub fn underscore_digit(input: ParseString) -> ParseResult<Token> {
  let (input, _) = underscore(input)?;
  let (input, digit) = digit_token(input)?;
  Ok((input,digit))
}

// digit_sequence := digit, (underscore_digit | digit)*
pub fn digit_sequence(input: ParseString) -> ParseResult<Vec<Token>> {
  let (input, mut start) = digit_token(input)?;
  let (input, mut tokens) = many0(alt((underscore_digit,digit_token)))(input)?;
  let mut all = vec![start];
  all.append(&mut tokens);
  Ok((input,all))
}

// grouping_symbol := left_parenthesis | right_parenthesis | left_angle | right_angle | left_brace | right_brace | left_bracket | right_bracket
pub fn grouping_symbol(input: ParseString) -> ParseResult<Token> {
  let (input, grouping) = alt((left_parenthesis, right_parenthesis, left_angle, right_angle, left_brace, right_brace, left_bracket, right_bracket))(input)?;
  Ok((input, grouping))
}

// punctuation := period | exclamation | question | comma | colon | semicolon | quote | apostrophe ;
pub fn punctuation(input: ParseString) -> ParseResult<Token> {
  let (input, punctuation) = alt((period, exclamation, question, comma, colon, semicolon, quote, apostrophe))(input)?;
  Ok((input, punctuation))
}

// escaped_char := "\" ,  symbol | punctuation ;
pub fn escaped_char(input: ParseString) -> ParseResult<Token> {
  let (input, _) = backslash(input)?;
  let (input, symbol) = alt((symbol, punctuation))(input)?;
  Ok((input, symbol))
}

// symbol := ampersand | bar | at | slash | hashtag | equal | backslash | tilde | plus | dash | asterisk | caret | underscore ;
pub fn symbol(input: ParseString) -> ParseResult<Token> {
  let (input, symbol) = alt((ampersand, bar, at, slash, hashtag, equal, backslash, tilde, plus, dash, asterisk, caret, underscore))(input)?;
  Ok((input, symbol))
}

// text := (alpha | digit | space | tabe | escaped_char | punctuation | grouping_symbol | symbol)+ ;
pub fn text(input: ParseString) -> ParseResult<Token> {
  let (input, text) = alt((alpha_token, digit_token, space, tab, escaped_char, punctuation, grouping_symbol, symbol))(input)?;
  Ok((input, text))
}

// identifier := (alpha | emoji), (alpha | digit | symbol | emoji)* ;
pub fn identifier(input: ParseString) -> ParseResult<Identifier> {
  let (input, (first, mut rest)) = nom_tuple((alt((alpha_token, emoji)), many0(alt((alpha_token, digit_token, symbol, emoji)))))(input)?;
  let mut tokens = vec![first];
  tokens.append(&mut rest);
  let mut merged = Token::merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Identifier; 
  Ok((input, Identifier{name: merged}))
}

// boolean_literal := true_literal | false_literal ;
pub fn boolean(input: ParseString) -> ParseResult<Token> {
  let (input, boolean) = alt((true_literal, false_literal))(input)?;
  Ok((input, boolean))
}

// true_literal := english_true_literal | check_mark ;
pub fn true_literal(input: ParseString) -> ParseResult<Token> {
  let (input, token) = alt((english_true_literal, check_mark))(input)?;
  Ok((input, token))
}

// false_literal := english_false_literal | cross ;
pub fn false_literal(input: ParseString) -> ParseResult<Token> {
  let (input, token) = alt((english_false_literal, cross))(input)?;
  Ok((input, token))
}

// new_line := new_line_char | carriage_new_line ;
pub fn new_line(input: ParseString) -> ParseResult<Token> {
  let (input, result) = alt((carriage_return_new_line,new_line_char,carriage_return, ))(input)?;
  Ok((input, result))
}

// whitespace := space | new_line | carriage_return | tabe ;
pub fn whitespace(input: ParseString) -> ParseResult<Token> {
  let (input, space) = alt((space,tab,new_line))(input)?;
  Ok((input, space))
}

// whitespace0 := whitespace* ;
pub fn whitespace0(input: ParseString) -> ParseResult<()> {
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, ()))
}

// whitespace1 := one_or_more_whitespaces ;
pub fn whitespace1(input: ParseString) -> ParseResult<()> {
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, ()))
}

// space_tab := space | tab ;
pub fn space_tab(input: ParseString) -> ParseResult<Token> {
  let (input, space) = alt((space,tab))(input)?;
  Ok((input, space))
}

// list_separator := whitespace*, ",", whitespace* ;
pub fn list_separator(input: ParseString) -> ParseResult<()> {
  let (input,_) = nom_tuple((whitespace0,tag(","),whitespace0))(input)?;
  Ok((input, ()))
}

// enum_separator := whitespace*, "|", whitespace* ;
pub fn enum_separator(input: ParseString) -> ParseResult<()> {
  let (input,_) = nom_tuple((whitespace0,tag("|"),whitespace0))(input)?;
  Ok((input, ()))
}


// number-literal := (integer | hexadecimal | octal | binary | decimal | float | rational | scientific) ;

pub fn number(input: ParseString) -> ParseResult<Number> {
  let (input, real_num) = real_number(input)?;
  match tag("i")(input.clone()) {
    Ok((input,_)) => {
      return Ok((input, Number::Imaginary(
        ComplexNumber{
          real: None, 
          imaginary: ImaginaryNumber{number: real_num}
        })));
      }
    _ => match nom_tuple((plus,real_number,tag("i")))(input.clone()) {
      Ok((input, (_,imaginary_num,_))) => {
        return Ok((input, Number::Imaginary(
          ComplexNumber{
            real: Some(real_num), 
            imaginary: ImaginaryNumber{number: imaginary_num},
          })));
        }
      _ => ()
    }
  }
  Ok((input, Number::Real(real_num)))
}

// real_number := optional_dash (hexadecimal_literal | decimal_literal | octal_literal | binary_literal | scientific_literal | rational_literal | float_literal | integer_literal) ;
pub fn real_number(input: ParseString) -> ParseResult<RealNumber> {
  let (input, neg) = opt(dash)(input)?;
  let (input, result) = alt((hexadecimal_literal, decimal_literal, octal_literal, binary_literal, scientific_literal, rational_literal, float_literal, integer_literal))(input)?;
  let result = match neg {
    Some(_) => RealNumber::Negated(Box::new(result)),
    None => result,
  };
  Ok((input, result))
}

// rational_literal := integer_literal "/" integer_literal ;
pub fn rational_literal(input: ParseString) -> ParseResult<RealNumber> {
  let (input, RealNumber::Integer(numerator)) = integer_literal(input)? else { unreachable!() };
  let (input, _) = slash(input)?;
  let (input, RealNumber::Integer(denominator)) = integer_literal(input)? else { unreachable!() };
  Ok((input, RealNumber::Rational((numerator,denominator))))
}

// scientific_literal := (float_literal | integer_literal) ("e" | "E") (optional_plus | ε) (optional_dash | ε) (float_literal | integer_literal) ;
pub fn scientific_literal(input: ParseString) -> ParseResult<RealNumber> {
  let (input, base) = match float_literal(input.clone()) {
    Ok((input, RealNumber::Float(base))) => {
      (input, base)
    }
    _ => match integer_literal(input.clone()) {
      Ok((input, RealNumber::Integer(base))) => {
        (input, (base, Token::default()))
      }
      Err(err) => {return Err(err);}
      _ => unreachable!(),
    }
  };
  let (input, _) = alt((tag("e"), tag("E")))(input)?;
  let (input, _) = opt(plus)(input)?;
  let (input, neg) = opt(dash)(input)?;
  let (input, (ex_whole,ex_part)) = match float_literal(input.clone()) {
    Ok((input, RealNumber::Float(exponent))) => {
      (input, exponent)
    }
    _ => match integer_literal(input.clone()) {
      Ok((input, RealNumber::Integer(exponent))) => {
        (input, (exponent, Token::default()))
      }
      Err(err) => {return Err(err);}
      _ => unreachable!(),
    }
  };
  let ex_sign = match neg {
    Some(_) => true,
    None => false,
  };
  Ok((input, RealNumber::Scientific((base,(ex_sign,ex_whole,ex_part)))))
}

// float_decimal_start := ".", digit_sequence ;
pub fn float_decimal_start(input: ParseString) -> ParseResult<RealNumber> {
  let (input, _) = period(input)?;
  let (input, part) = digit_sequence(input)?;
  let mut tokens2 = part.clone();
  let mut merged = Token::merge_tokens(&mut tokens2).unwrap();
  merged.kind = TokenKind::Number;
  Ok((input, RealNumber::Float((Token::default(),merged))))
}

// float_full := digit_sequence, ".", digit_sequnce ;
pub fn float_full(input: ParseString) -> ParseResult<RealNumber> {
  let (input, mut whole) = digit_sequence(input)?;
  let (input, _) = period(input)?;
  let (input, mut part) = digit_sequence(input)?;
  let mut whole = Token::merge_tokens(&mut whole).unwrap();
  let mut part = Token::merge_tokens(&mut part).unwrap();
  whole.kind = TokenKind::Number;
  part.kind = TokenKind::Number;
  Ok((input, RealNumber::Float((whole,part))))
}

// float_literal := "."?, digit1, "."?, digit0 ;
pub fn float_literal(input: ParseString) -> ParseResult<RealNumber> {
  let (input, result) = alt((float_decimal_start,float_full))(input)?;
  Ok((input, result))
}

// integer := digit1 ;
pub fn integer_literal(input: ParseString) -> ParseResult<RealNumber> {
  let (input, mut digits) = digit_sequence(input)?;
  let mut merged = Token::merge_tokens(&mut digits).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, RealNumber::Integer(merged)))
}

// decimal_literal := "0d", <digit1> ;
pub fn decimal_literal(input: ParseString) -> ParseResult<RealNumber> {
  let msg = "Expects decimal digits after \"0d\"";
  let input = tag("0d")(input);
  let (input, _) = input?;
  let (input, mut tokens) = label!(digit_sequence, msg)(input)?;
  let mut merged = Token::merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, RealNumber::Decimal(merged)))
}

// hexadecimal_literal := "0x", <hex_digit+> ;
pub fn hexadecimal_literal(input: ParseString) -> ParseResult<RealNumber> {
  let msg = "Expects hexadecimal digits after \"0x\"";
  let input = tag("0x")(input);
  let (input, _) = input?;
  let (input, mut tokens) = label!(many1(alt((digit_token,underscore,alpha_token))), msg)(input)?;
  let mut merged = Token::merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, RealNumber::Hexadecimal(merged)))
}

// octal_literal := "0o", <oct_digit+> ;
pub fn octal_literal(input: ParseString) -> ParseResult<RealNumber> {
  let msg = "Expects octal digits after \"0o\"";
  let input = tag("0o")(input);
  let (input, _) = input?;
  let (input, mut tokens) = label!(many1(alt((digit_token,underscore,alpha_token))), msg)(input)?;
  let mut merged = Token::merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, RealNumber::Octal(merged)))
}

// binary_literal := "0b", <bin_digit+> ;
pub fn binary_literal(input: ParseString) -> ParseResult<RealNumber> {
  let msg = "Expects binary digits after \"0b\"";
  let input = tag("0b")(input);
  let (input, _) = input?;
  let (input, mut tokens) = label!(many1(alt((digit_token,underscore,alpha_token))), msg)(input)?;
  let mut merged = Token::merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, RealNumber::Binary(merged)))
}

// empty := underscore+ ;
pub fn empty(input: ParseString) -> ParseResult<Token> {
  let (input, (g, src_range)) = range(many1(tag("_")))(input)?;
  Ok((input, Token{kind: TokenKind::Empty, chars: g.join("").chars().collect(), src_range}))
}

// #### Kind Annotations

// kind_annotation := left_angle, kind, right_angle ;
pub fn kind_annotation(input: ParseString) -> ParseResult<KindAnnotation> {
  let msg2 = "Expects at least one unit in kind annotation";
  let msg3 = "Expects right angle";
  let (input, (_, r)) = range(left_angle)(input)?;
  let (input, kind) = kind(input)?;
  let (input, _) = label!(right_angle, msg3, r)(input)?;
  Ok((input, KindAnnotation{ kind }))
}

// kind := empty | atom | tuple | scalar | bracket | map | brace ;
pub fn kind(input: ParseString) -> ParseResult<Kind> {
  let (input, kind) = alt((kind_fxn,kind_empty,kind_atom,kind_tuple, kind_scalar, kind_bracket, kind_map, kind_brace))(input)?;
  Ok((input, kind))
}

// kind_empty := underscore+ ;
pub fn kind_empty(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = many1(underscore)(input)?;
  Ok((input, Kind::Empty))
}

// kind_atom := "`", identifier ;
pub fn kind_atom(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = grave(input)?;
  let (input, atm) = identifier(input)?;
  Ok((input, Kind::Atom(atm)))
}

// kind_map = "{", kind, ":", kind, "}" ;
pub fn kind_map(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_brace(input)?;
  let (input, key_kind) = kind(input)?;
  let (input, _) = colon(input)?;
  let (input, value_kind) = kind(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Kind::Map(Box::new(key_kind),Box::new(value_kind))))
}

// kind_fxn := "(" kind (list_separator kind)* ")" "=" "(" kind (list_separator kind)* ")" ;
pub fn kind_fxn(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_parenthesis(input)?;
  let (input, input_kinds) = separated_list0(list_separator,kind)(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = equal(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, output_kinds) = separated_list0(list_separator,kind)(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, Kind::Function(input_kinds,output_kinds)))
}

// kind_brace = "{", list1(",",kind) "}", [":"], list0(",",literal) ;
pub fn kind_brace(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_brace(input)?;
  let (input, kinds) = separated_list1(list_separator,kind)(input)?;
  let (input, _) = right_brace(input)?;
  let (input, _) = opt(colon)(input)?;
  let (input, size) = separated_list0(list_separator,literal)(input)?;
  Ok((input, Kind::Brace((kinds,size))))
}

// kind_bracket = "[", list1(",",kind) "]", [":"], list0(",",literal) ;
pub fn kind_bracket(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_bracket(input)?;
  let (input, kinds) = separated_list1(list_separator,kind)(input)?;
  let (input, _) = right_bracket(input)?;
  let (input, _) = opt(colon)(input)?;
  let (input, size) = separated_list0(list_separator,literal)(input)?;
  Ok((input, Kind::Bracket((kinds,size))))
}

// kind_tuple = "(", list1(",",kind) ")" ;
pub fn kind_tuple(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_parenthesis(input)?;
  let (input, kinds) = separated_list1(list_separator, kind)(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, Kind::Tuple(kinds)))
}

// kind_scalar := identifier ;
pub fn kind_scalar(input: ParseString) -> ParseResult<Kind> {
  let (input, kind) = identifier(input)?;
  Ok((input, Kind::Scalar(kind)))
}