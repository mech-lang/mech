#[macro_use]
use crate::*;
use nom::{multi::many0, sequence::{preceded, terminated}};

// pattern := pattern_atom_struct | pattern_tuple_struct | wildcard | pattern_array | pattern_tuple | expression ;
pub fn pattern(input: ParseString) -> ParseResult<Pattern> {
  match pattern_atom_struct(input.clone()) {
    Ok((input, tpl)) => {return Ok((input, Pattern::TupleStruct(tpl)))},
    _ => ()
  }
  match pattern_tuple_struct(input.clone()) {
    Ok((input, tpl)) => {return Ok((input, Pattern::TupleStruct(tpl)))},
    _ => ()
  }
  match wildcard(input.clone()) {
    Ok((input, _)) => {return Ok((input, Pattern::Wildcard))},
    _ => ()
  }
  match pattern_array(input.clone()) {
    Ok((input, arr)) => {return Ok((input, Pattern::Array(arr)))},
    _ => ()
  }
  match pattern_tuple(input.clone()) {
    Ok((input, tpl)) => {return Ok((input, Pattern::Tuple(tpl)))},
    _ => ()
  }
  match expression(input.clone()) {
    Ok((input, expr)) => {return Ok((input, Pattern::Expression(expr)))},
    Err(err) => {return Err(err)},
  }
}

// wildcard := "*" ;
pub fn wildcard(input: ParseString) -> ParseResult<Pattern> {
  let ((input, _)) = asterisk(input)?;
  Ok((input, Pattern::Wildcard))
}

// pattern_tuple_struct := grave, identifier, "(", list1(",", pattern), ")" ;
pub fn pattern_tuple_struct(input: ParseString) -> ParseResult<PatternTupleStruct> {
  let (input, _) = grave(input)?;
  let (input, id) = identifier(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, patterns) = separated_list1(list_separator, pattern)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, PatternTupleStruct{name: id, patterns}))
}

fn spread_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = alt((tag("..."), tag("…")))(input)?;
  Ok((input, ()))
}

fn pattern_array_item(input: ParseString) -> ParseResult<Pattern> {
  if let Ok((input, _)) = wildcard(input.clone()) {
    return Ok((input, Pattern::Wildcard));
  }
  let (input, expr) = expression(input)?;
  Ok((input, Pattern::Expression(expr)))
}

#[derive(Clone)]
enum PatternArrayToken {
  Spread,
  Item(Pattern),
}

fn pattern_array_token(input: ParseString) -> ParseResult<PatternArrayToken> {
  if let Ok((input, _)) = spread_operator(input.clone()) {
    return Ok((input, PatternArrayToken::Spread));
  }
  let (input, item) = pattern_array_item(input)?;
  Ok((input, PatternArrayToken::Item(item)))
}

// pattern_array := "[", [pattern_array_item|spread], "]" ;
pub fn pattern_array(input: ParseString) -> ParseResult<PatternArray> {
  let (input, _) = left_bracket(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, tokens) = many0(terminated(preceded(whitespace0, pattern_array_token), whitespace0))(input)?;
  let (input, _) = right_bracket(input)?;

  let spread_positions = tokens
    .iter()
    .enumerate()
    .filter_map(|(ix, token)| match token {
      PatternArrayToken::Spread => Some(ix),
      PatternArrayToken::Item(_) => None,
    })
    .collect::<Vec<usize>>();

  if spread_positions.len() > 1 {
    return Err(nom::Err::Error(ParseError::new(
      input,
      "Only one spread operator is allowed in an array pattern",
    )));
  }

  let spread_ix = spread_positions.first().copied();
  let mut prefix = vec![];
  let mut suffix = vec![];

  if let Some(ix) = spread_ix {
    prefix = tokens[..ix]
      .iter()
      .filter_map(|token| match token {
        PatternArrayToken::Item(pattern) => Some(pattern.clone()),
        PatternArrayToken::Spread => None,
      })
      .collect();
    suffix = tokens[ix + 1..]
      .iter()
      .filter_map(|token| match token {
        PatternArrayToken::Item(pattern) => Some(pattern.clone()),
        PatternArrayToken::Spread => None,
      })
      .collect();
  } else {
    prefix = tokens
      .iter()
      .filter_map(|token| match token {
        PatternArrayToken::Item(pattern) => Some(pattern.clone()),
        PatternArrayToken::Spread => None,
      })
      .collect();
  }

  let spread = spread_ix.map(|_| {
    let prefix_ends_wildcard = matches!(prefix.last(), Some(Pattern::Wildcard));
    let mut spread_binding: Option<Box<Pattern>> = None;
    if prefix_ends_wildcard {
      if suffix.len() == 1 {
        spread_binding = suffix.pop().map(Box::new);
      } else if suffix.len() >= 2 {
        spread_binding = Some(Box::new(suffix.remove(0)));
      }
    }
    PatternArraySpread { binding: spread_binding }
  });

  Ok((input, PatternArray { prefix, spread, suffix }))
}

// pattern_atom_struct := ":", identifier, "(", list1(",", pattern), ")" ;
pub fn pattern_atom_struct(input: ParseString) -> ParseResult<PatternTupleStruct> {
  let (input, _) = colon(input)?;
  let (input, id) = identifier(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, patterns) = separated_list1(list_separator, pattern)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, PatternTupleStruct{name: id, patterns}))
}

// pattern-tuple := "(", [pattern, ","], ")" ;
pub fn pattern_tuple(input: ParseString) -> ParseResult<PatternTuple> {
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, patterns) = separated_list1(list_separator, pattern)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, PatternTuple(patterns)))
}
