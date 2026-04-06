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

// pattern_array := "[", [pattern_array_item|spread], "]" ;
pub fn pattern_array(input: ParseString) -> ParseResult<PatternArray> {
  let (mut input, _) = left_bracket(input)?;
  let (i, _) = whitespace0(input)?;
  input = i;

  let mut items: Vec<Pattern> = vec![];
  let mut spread_ix: Option<usize> = None;

  if let Ok((i, _)) = right_bracket(input.clone()) {
    return Ok((i, PatternArray { prefix: vec![], spread: None, suffix: vec![] }));
  }

  loop {
    let (next_input, _) = whitespace0(input.clone())?;
    input = next_input;

    if spread_ix.is_none() {
      if let Ok((after_spread, _)) = spread_operator(input.clone()) {
        spread_ix = Some(items.len());
        input = after_spread;
      } else {
        let (after_item, item) = pattern_array_item(input.clone())?;
        items.push(item);
        input = after_item;
      }
    } else {
      let (after_item, item) = pattern_array_item(input.clone())?;
      items.push(item);
      input = after_item;
    }

    let (after_ws, _) = whitespace0(input.clone())?;
    if let Ok((after_rb, _)) = right_bracket(after_ws.clone()) {
      let split = spread_ix.unwrap_or(items.len());
      let mut suffix = if split < items.len() { items.split_off(split) } else { vec![] };
      let prefix = items;
      let spread = if spread_ix.is_some() {
        let prefix_ends_wildcard = matches!(prefix.last(), Some(Pattern::Wildcard));
        let mut spread_binding: Option<Box<Pattern>> = None;
        if prefix_ends_wildcard {
          if suffix.len() == 1 {
            spread_binding = suffix.pop().map(Box::new);
          } else if suffix.len() >= 2 {
            spread_binding = Some(Box::new(suffix.remove(0)));
          }
        }
        Some(PatternArraySpread { binding: spread_binding })
      } else {
        None
      };
      return Ok((after_rb, PatternArray { prefix, spread, suffix }));
    }
    input = after_ws;
  }
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
