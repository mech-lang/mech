#[macro_use]
use crate::*;

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

// spread-operator := "..." | "…" ;
fn spread_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = alt((spread_operator_a, spread_operator_u))(input)?;
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
  Pipe,
  Item(Pattern),
}

fn pattern_array_token(input: ParseString) -> ParseResult<PatternArrayToken> {
  if let Ok((input, _)) = spread_operator(input.clone()) {
    return Ok((input, PatternArrayToken::Spread));
  }
  if let Ok((input, _)) = enum_separator(input.clone()) {
    return Ok((input, PatternArrayToken::Pipe));
  }
  let (input, item) = pattern_array_item(input)?;
  Ok((input, PatternArrayToken::Item(item)))
}

// pattern_array := "[", [pattern_array_item|spread], "]" ;
pub fn pattern_array(input: ParseString) -> ParseResult<PatternArray> {
  let (mut input, _) = left_bracket(input)?;
  let (next_input, _) = whitespace0(input)?;
  input = next_input;

  let mut tokens = Vec::new();
  loop {
    if let Ok((next_input, _)) = right_bracket(input.clone()) {
      input = next_input;
      break;
    }

    let (next_input, token) = pattern_array_token(input.clone())?;
    input = next_input;
    tokens.push(token);

    let (next_input, _) = whitespace0(input)?;
    input = next_input;
    if let Ok((next_input, _)) = list_separator(input.clone()) {
      input = next_input;
      continue;
    }
  }

  let pipe_positions = tokens
    .iter()
    .enumerate()
    .filter_map(|(ix, token)| match token {
      PatternArrayToken::Pipe => Some(ix),
      _ => None,
    })
    .collect::<Vec<usize>>();

  if pipe_positions.len() > 1 {
    return Err(nom::Err::Error(ParseError::new(
      input,
      "Only one | rest binding is allowed in an array pattern",
    )));
  }

  if let Some(pipe_ix) = pipe_positions.first().copied() {
    if tokens.iter().any(|token| matches!(token, PatternArrayToken::Spread)) {
      return Err(nom::Err::Error(ParseError::new(
        input,
        "Cannot mix … spread and | rest binding in an array pattern",
      )));
    }

    let mut prefix = vec![];
    for token in tokens[..pipe_ix].iter() {
      match token {
        PatternArrayToken::Item(pattern) => prefix.push(pattern.clone()),
        _ => {
          return Err(nom::Err::Error(ParseError::new(
            input.clone(),
            "Only patterns are allowed before | in an array pattern",
          )));
        }
      }
    }

    let rest_tokens = &tokens[pipe_ix + 1..];
    if rest_tokens.len() != 1 {
      return Err(nom::Err::Error(ParseError::new(
        input,
        "Array rest binding must be exactly one pattern after |",
      )));
    }

    let binding = match &rest_tokens[0] {
      PatternArrayToken::Item(pattern) => pattern.clone(),
      _ => {
        return Err(nom::Err::Error(ParseError::new(
          input,
          "Array rest binding must be a pattern after |",
        )));
      }
    };

    return Ok((
      input,
      PatternArray {
        prefix,
        spread: Some(PatternArraySpread {
          binding: Some(Box::new(binding)),
        }),
        suffix: vec![],
      },
    ));
  }

  let spread_positions = tokens
    .iter()
    .enumerate()
    .filter_map(|(ix, token)| match token {
      PatternArrayToken::Spread => Some(ix),
      _ => None,
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
        _ => None,
      })
      .collect();
    suffix = tokens[ix + 1..]
      .iter()
      .filter_map(|token| match token {
        PatternArrayToken::Item(pattern) => Some(pattern.clone()),
        _ => None,
      })
      .collect();
  } else {
    prefix = tokens
      .iter()
      .filter_map(|token| match token {
        PatternArrayToken::Item(pattern) => Some(pattern.clone()),
        _ => None,
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
