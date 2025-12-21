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

// atom := ":", identifier ;
pub fn atom(input: ParseString) -> ParseResult<Atom> {
  let (input, _) = colon(input)?;
  let (input, name) = identifier(input)?;
  Ok((input, Atom{name}))
}


// string := quote, *(¬quote, (text | new-line)), quote ;
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

// number := complex-number | real-number ;
pub fn number(input: ParseString) -> ParseResult<Number> {
  match complex_number(input.clone()) {
    Ok((input, complex_num)) => Ok((input, Number::Complex(complex_num))),
    _ => match real_number(input.clone()) {
      Ok((input, real_num)) => Ok((input, Number::Real(real_num))),
      Err(err) => return Err(err),
    },
  }
}

// complex-number := real-number, ("i"|"j")? | (("+"|"-"), real-number, ("i"|"j")) ;
pub fn complex_number(input: ParseString) -> ParseResult<C64Node> {
  let (input, real_num) = real_number(input)?;
  if let Ok((input, _)) = alt((tag("i"), tag("j")))(input.clone()) {
    return Ok((
      input,
      C64Node {
        real: None,
        imaginary: ImaginaryNumber { number: real_num },
      },
    ));
  }
  if let Ok((input, (sign, imaginary_num, _))) = 
    nom_tuple((alt((plus, dash)), real_number, alt((tag("i"), tag("j")))))(input.clone())
  {
    let imaginary = match sign.kind {
      TokenKind::Plus => imaginary_num,
      TokenKind::Dash => RealNumber::Negated(Box::new(imaginary_num)),
      _ => unreachable!(),
    };
    return Ok((
      input,
      C64Node {
        real: Some(real_num),
        imaginary: ImaginaryNumber { number: imaginary },
      },
    ));
  } else {
    return Err(nom::Err::Error(nom::error::make_error(
      input,
      nom::error::ErrorKind::Alt,
    )));
  }
}

// real-number := ?dash, (hexadecimal-literal | decimal-literal | octal-literal | binary-literal | scientific-literal | rational-literal | float-literal | integer-literal) ;
pub fn real_number(input: ParseString) -> ParseResult<RealNumber> {
  let (input, neg) = opt(dash)(input)?;
  let (input, result) = alt((hexadecimal_literal, decimal_literal, octal_literal, binary_literal, scientific_literal, rational_literal, float_literal, integer_literal))(input)?;
  let result = match neg {
    Some(_) => RealNumber::Negated(Box::new(result)),
    None => result,
  };
  Ok((input, result))
}

// rational-literal := integer-literal, slash, integer-literal ;
pub fn rational_literal(input: ParseString) -> ParseResult<RealNumber> {
  let (input, RealNumber::Integer(numerator)) = integer_literal(input)? else { unreachable!() };
  let (input, _) = slash(input)?;
  let (input, RealNumber::Integer(denominator)) = integer_literal(input)? else { unreachable!() };
  Ok((input, RealNumber::Rational((numerator,denominator))))
}

// scientific-literal := (float-literal | integer-literal), ("e" | "E"), ?plus, ?dash, (float-literal | integer-literal) ;
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

// float-decimal-start := ".", digit-sequence ;
pub fn float_decimal_start(input: ParseString) -> ParseResult<RealNumber> {
  let (input, _) = period(input)?;
  let (input, part) = digit_sequence(input)?;
  let mut tokens2 = part.clone();
  let mut merged = Token::merge_tokens(&mut tokens2).unwrap();
  merged.kind = TokenKind::Number;
  Ok((input, RealNumber::Float((Token::default(),merged))))
}

// float-full := digit-sequence, ".", digit-sequnce ;
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

// float-literal := float-decimal-start | float-full;
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

// kind_annotation := left_angle, kind, ?question, right_angle ;
pub fn kind_annotation(input: ParseString) -> ParseResult<KindAnnotation> {
  let msg2 = "Expects at least one unit in kind annotation";
  let msg3 = "Expects right angle";
  let (input, (_, r)) = range(left_angle)(input)?;
  let (input, kind) = kind(input)?;
  let (input, optional) = opt(question)(input)?;
  let (input, _) = label!(right_angle, msg3, r)(input)?;
  let kind = match optional {
    Some(_) => Kind::Option(Box::new(kind)),
    None => kind,
  };
  Ok((input, KindAnnotation{ kind }))
}

// kind := kind-fxn | kind-empty | kind-record | kind-atom | kind-tuple | kind-scalar | kind-matrix | kind-map ;
pub fn kind(input: ParseString) -> ParseResult<Kind> {
  let (input, kind) = alt((
    kind_any,
    kind_atom,
    kind_empty,
    kind_map,
    kind_matrix,
    kind_record,
    kind_scalar,
    kind_set,
    kind_table, 
    kind_tuple,
  ))(input)?;
  Ok((input, kind))
}

// kind-table := "|" , list1(",", (identifier, kind)), "|", ":", list0(",", literal) ;
pub fn kind_table(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = bar(input)?;
  let (input, elements) = separated_list1(alt((null(list_separator),null(many1(space_tab)))), nom_tuple((identifier, kind_annotation)))(input)?;
  let (input, _) = bar(input)?;
  let (input, size) = opt(tuple((colon,literal)))(input)?;
  let size = size.map(|(_, ltrl)| ltrl).unwrap_or_else(|| Literal::Empty(Token::default()));
  let elements = elements.into_iter().map(|(id, knd)| (id, knd.kind)).collect();
  Ok((input, Kind::Table((elements, Box::new(size)))))
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

// kind-set := "{", kind, "}", (":", literal)? ;
pub fn kind_set(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_brace(input)?;
  let (input, kind) = kind(input)?;
  let (input, _) = right_brace(input)?;
  let (input, opt_lit) = opt(nom_tuple((colon, literal)))(input)?;
  let (input, notation) = opt(tag(":N"))(input)?;
  let ltrl = match opt_lit {
    Some((_, ltrl)) => Some(Box::new(ltrl)),
    None => match notation {
      Some(_) => Some(Box::new(Literal::Empty(Token::new(TokenKind::Empty, SourceRange::default(), vec!['N'])))),
      None => None,
    },
  };
  Ok((input, Kind::Set(Box::new(kind), ltrl)))
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

// kind-record := "{", list1(",", (identifier, kind)), "}" ;
pub fn kind_record(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_brace(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, elements) = separated_list1(alt((list_separator,space_tab1)), nom_tuple((identifier, kind_annotation)))(input)?;
  let (input, _) = opt(tag(",..."))(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, _) = right_brace(input)?;
  let elements = elements.into_iter().map(|(id, knd)| (id, knd.kind)).collect();
  Ok((input, Kind::Record(elements)))
}

// kind-fxn := "(", list0(list_separator, kind), ")", "=", "(", list0(list_separator, kind), ")" ;
/*pub fn kind_fxn(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_parenthesis(input)?;
  let (input, input_kinds) = separated_list0(list_separator,kind)(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = equal(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, output_kinds) = separated_list0(list_separator,kind)(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, Kind::Function(input_kinds,output_kinds)))
}*/

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