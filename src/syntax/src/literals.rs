#[macro_use]
use crate::*;
use nom::{
  multi::separated_list0,
  sequence::tuple as nom_tuple,
};
use crate::nodes::Kind;

// Literals
// =============================================================================

// literal := (number | string | atom | boolean | empty | kind-annotation), kind-annotation? ;
pub fn literal(input: ParseString) -> ParseResult<Literal> {
  let (input, result) = match number(input.clone()) {
    Ok((input, num)) => (input, Literal::Number(num)),
    _ => match string(input.clone()) {
      Ok((input, s)) => (input, Literal::String(s)),
      _ => match atom(input.clone()) {
        Ok((input, atm)) => (input, Literal::Atom(atm)),
        _ => match boolean(input.clone()) {
          Ok((input, boolean)) => (input, Literal::Boolean(boolean)),
          _ => match empty(input.clone()) {
            Ok((input, empty)) => (input, Literal::Empty(empty)), 
            Err(err) => match kind_annotation(input.clone()) {
              Ok((input, knd)) => (input, Literal::Kind(knd.kind)),
              Err(err) => return Err(err),
            }
          }
        }
      }
    }
  };
  let (input, result) = match opt(kind_annotation)(input.clone()) {
    Ok((input, Some(knd))) => ((input, Literal::TypedLiteral((Box::new(result),knd)))),
    Ok((input, None)) => (input,result),
    Err(err) => {return Err(err);}
  };
  Ok((input, result))
}

// atom := "`", identifier ;
pub fn atom(input: ParseString) -> ParseResult<Atom> {
  let (input, _) = grave(input)?;
  let (input, name) = identifier(input)?;
  Ok((input, Atom{name}))
}

// string := quote, (!quote, text)*, quote ;
pub fn string(input: ParseString) -> ParseResult<MechString> {
  let msg = "Character not allowed in string";
  let (input, _) = quote(input)?;
  let (input, matched) = many0(nom_tuple((is_not(quote), alt((text,new_line)))))(input)?;
  let (input, _) = quote(input)?;
  let (_, mut text): ((), Vec<_>) = matched.into_iter().unzip();
  let mut merged = Token::merge_tokens(&mut text).unwrap();
  merged.kind = TokenKind::String;
  Ok((input, MechString { text: merged }))
}

// Boolean
// ----------------------------------------------------------------------------

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

// Number
// ----------------------------------------------------------------------------

// number := real_number, "i"? | ("+", real_number, "i")? ;
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

// real_number := dash?, (hexadecimal_literal | decimal_literal | octal_literal | binary_literal | scientific_literal | rational_literal | float_literal | integer_literal) ;
pub fn real_number(input: ParseString) -> ParseResult<RealNumber> {
  let (input, neg) = opt(dash)(input)?;
  let (input, result) = alt((hexadecimal_literal, decimal_literal, octal_literal, binary_literal, scientific_literal, rational_literal, float_literal, integer_literal))(input)?;
  let result = match neg {
    Some(_) => RealNumber::Negated(Box::new(result)),
    None => result,
  };
  Ok((input, result))
}

// rational_literal := integer_literal, "/", integer_literal ;
pub fn rational_literal(input: ParseString) -> ParseResult<RealNumber> {
  let (input, RealNumber::Integer(numerator)) = integer_literal(input)? else { unreachable!() };
  let (input, _) = slash(input)?;
  let (input, RealNumber::Integer(denominator)) = integer_literal(input)? else { unreachable!() };
  Ok((input, RealNumber::Rational((numerator,denominator))))
}

// scientific_literal := (float_literal | integer_literal), ("e" | "E"), plus?, dash?, (float_literal | integer_literal) ;
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

// float_literal := float_decimal_start | float_full;
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

// Kind Annotations
// ----------------------------------------------------------------------------

// kind_annotation := left_angle, kind, right_angle ;
pub fn kind_annotation(input: ParseString) -> ParseResult<KindAnnotation> {
  let msg2 = "Expects at least one unit in kind annotation";
  let msg3 = "Expects right angle";
  let (input, (_, r)) = range(left_angle)(input)?;
  let (input, kind) = kind(input)?;
  let (input, _) = label!(right_angle, msg3, r)(input)?;
  Ok((input, KindAnnotation{ kind }))
}

// kind := kind-fxn | kind-empty | kind-atom | kind-tuple | kind-scalar | kind-matrix | kind-map | kind-brace ;
pub fn kind(input: ParseString) -> ParseResult<Kind> {
  let (input, kind) = alt((kind_fxn,kind_empty,kind_atom,kind_tuple,kind_scalar,kind_matrix,kind_map,kind_brace,kind_any))(input)?;
  Ok((input, kind))
}

// kind-any := "*";
pub fn kind_any(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = asterisk(input)?;
  Ok((input, Kind::Any))
}

// kind-empty := underscore+ ;
pub fn kind_empty(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = many1(underscore)(input)?;
  Ok((input, Kind::Empty))
}

// kind-atom := "`", identifier ;
pub fn kind_atom(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = grave(input)?;
  let (input, atm) = identifier(input)?;
  Ok((input, Kind::Atom(atm)))
}

// kind-map := "{", kind, ":", kind, "}" ;
pub fn kind_map(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_brace(input)?;
  let (input, key_kind) = kind(input)?;
  let (input, _) = colon(input)?;
  let (input, value_kind) = kind(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Kind::Map(Box::new(key_kind),Box::new(value_kind))))
}

// kind-fxn := "(", list0(list_separator, kind), ")", "=", "(", list0(list_separator, kind), ")" ;
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

// kind-brace := "{", list1(",", kind), "}", ":"?, list0("," , literal) ;
pub fn kind_brace(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_brace(input)?;
  let (input, kinds) = separated_list1(list_separator,kind)(input)?;
  let (input, _) = right_brace(input)?;
  let (input, _) = opt(colon)(input)?;
  let (input, size) = separated_list0(list_separator,literal)(input)?;
  Ok((input, Kind::Brace((kinds,size))))
}

// kind-matrox := "[", list1(",",kind), "]", ":"?, list0(",", literal) ;
pub fn kind_matrix(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_bracket(input)?;
  let (input, kind) = kind(input)?;
  let (input, _) = right_bracket(input)?;
  let (input, _) = opt(colon)(input)?;
  let (input, size) = separated_list0(list_separator,literal)(input)?;
  Ok((input, Kind::Matrix((Box::new(kind),size))))
}

// kind-tuple := "(", list1(",", kind), ")" ;
pub fn kind_tuple(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_parenthesis(input)?;
  let (input, kinds) = separated_list1(list_separator, kind)(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, Kind::Tuple(kinds)))
}

// kind-scalar := identifier, [":", range_expression] ;
pub fn kind_scalar(input: ParseString) -> ParseResult<Kind> {
  let (input, kind) = identifier(input)?;
  let (input, range) = opt(tuple((colon,range_expression)))(input)?;
  Ok((input, Kind::Scalar(kind)))
}