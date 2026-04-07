use crate::*;

// Patterns
// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PatternMatchSemantics {
    Standard,
    OptionGuard,
}

pub fn pattern_matches_arguments(pattern: &Pattern, args: &Vec<Value>, env: &mut Environment, p: &Interpreter) -> MResult<bool> {
  if args.len() == 1 {
    return pattern_matches_value(pattern, &args[0], env, p);
  }
  match pattern {
    Pattern::Tuple(pattern_tuple) => {
      if pattern_tuple.0.len() != args.len() {
        return Ok(false);
      }
      for (pat, arg) in pattern_tuple.0.iter().zip(args.iter()) {
        if !pattern_matches_value(pat, arg, env, p)? {
          return Ok(false);
        }
      }
      Ok(true)
    }
    _ => Ok(false),
  }
}

pub fn pattern_matches_value(pattern: &Pattern, value: &Value, env: &mut Environment, p: &Interpreter) -> MResult<bool> {
  pattern_matches_value_with_semantics(pattern, value, env, p, PatternMatchSemantics::Standard)
}

pub fn pattern_matches_value_with_semantics(pattern: &Pattern, value: &Value, env: &mut Environment, p: &Interpreter, semantics: PatternMatchSemantics) -> MResult<bool> {
  let detached_value = deep_detach_value(value);
  match pattern {
    Pattern::Wildcard => Ok(true),
    #[cfg(feature = "tuple")]
    Pattern::Tuple(pattern_tuple) => {
      match detached_value {
        Value::Tuple(tuple) => {
          let tuple_brrw = tuple.borrow();
          if pattern_tuple.0.len() != tuple_brrw.elements.len() {
            return Ok(false);
          }
          for (pat, val) in pattern_tuple.0.iter().zip(tuple_brrw.elements.iter()) {
            if !pattern_matches_value_with_semantics(pat, val, env, p, semantics)? {
              return Ok(false);
            }
          }
          return Ok(true);
        }
        _ => {return Ok(false);},
      }
      return Ok(false);
    }
    #[cfg(feature = "matrix")]
    Pattern::Array(pattern_array) => {
      let values = match matrix_like_values(&detached_value) {
        Some(values) => values,
        None => return Ok(false),
      };
      if values.len() < pattern_array.prefix.len() + pattern_array.suffix.len() {
        return Ok(false);
      }
      for (pat, val) in pattern_array.prefix.iter().zip(values.iter()) {
        if !pattern_matches_value_with_semantics(pat, val, env, p, semantics)? {
          return Ok(false);
        }
      }
      let suffix_start = values.len() - pattern_array.suffix.len();
      for (pat, val) in pattern_array
          .suffix
          .iter()
          .zip(values[suffix_start..].iter())
      {
        if !pattern_matches_value_with_semantics(pat, val, env, p, semantics)? {
          return Ok(false);
        }
      }

      // If there's no spread, the number of values must match exactly. 
      // If there is a spread, there can be any number of values in the middle.
      if pattern_array.spread.is_none() && values.len() != pattern_array.prefix.len() + pattern_array.suffix.len()
      {
        return Ok(false);
      }
      if let Some(spread) = &pattern_array.spread {
        if let Some(binding) = &spread.binding {
          let middle = values[pattern_array.prefix.len()..suffix_start].to_vec();
          #[cfg(feature = "matrix")]
          let captured = Value::MatrixValue(Matrix::from_vec(
            middle,
            1,
            suffix_start.saturating_sub(pattern_array.prefix.len()),
          ));
          let captured = {
            let _ = middle;
            return Ok(false);
          };
          if !pattern_matches_value_with_semantics(binding, &captured, env, p, semantics)?
          {
            return Ok(false);
          }
        }
      }
      Ok(true)
    }
    Pattern::Expression(Expression::Var(var)) => {
      let var_id = var.name.hash();
      if let Some(existing) = env.get(&var_id) {
        Ok(existing == &detached_value)
      } else {
        env.insert(var_id, detached_value);
        Ok(true)
      }
    }
    Pattern::Expression(expr) => {
      if let Some(var_id) = extract_pattern_variable_id(expr) {
        if let Some(existing) = env.get(&var_id) {
          return Ok(existing == &detached_value);
        }
        env.insert(var_id, detached_value);
        return Ok(true);
      }
      let expected = expression(expr, Some(env), p)?;
      if semantics == PatternMatchSemantics::OptionGuard {
        #[cfg(feature = "bool")]
        if let Value::Bool(flag) = &expected {
          return Ok(*flag.borrow());
        }
      }
      Ok(values_match(&deep_detach_value(&expected), &detached_value))
    }
    #[cfg(all(feature = "tuple", feature = "atom"))]
    Pattern::TupleStruct(pat_struct) => {
      match detached_value {
        Value::Tuple(tuple) => {
          let tuple_brrw = tuple.borrow();
          if tuple_brrw.elements.len() != pat_struct.patterns.len() + 1 {
            return Ok(false);
          }
          let expected_state = atom(&Atom {name: pat_struct.name.clone(),},p,
          );
          if !values_match(&expected_state, &deep_detach_value(&tuple_brrw.elements[0])) {
            return Ok(false);
          }
          for (pat, val) in pat_struct
              .patterns
              .iter()
              .zip(tuple_brrw.elements.iter().skip(1))
          {
            if !pattern_matches_value_with_semantics(pat, val, env, p, semantics)? {
              return Ok(false);
            }
          }
          return Ok(true);
        }
        _ => return Ok(false),
      }
      return Ok(false);
    } 
  }
}

pub fn clear_pattern_bindings(pattern: &Pattern, env: &mut Environment) {
  let mut ids = Vec::new();
  collect_pattern_variable_ids(pattern, &mut ids);
  for var_id in ids {
    env.remove(&var_id);
  }
}

pub fn pattern_to_value(pattern: &Pattern, env: &Environment, p: &Interpreter) -> MResult<Value> {
  match pattern {
    Pattern::Wildcard => Ok(Value::Empty),
    Pattern::Expression(expr) => expression(expr, Some(env), p),
    #[cfg(not(feature = "tuple"))]
    Pattern::Tuple(pattern_tuple) => {
      let mut values = Vec::with_capacity(pattern_tuple.0.len());
      for inner in &pattern_tuple.0 {
        values.push(pattern_to_value(inner, env, p)?);
      }
      return Ok(Value::Tuple(Ref::new(MechTuple::from_vec(values))));
    }
    #[cfg(feature = "matrix")]
    Pattern::Array(array) => {
      let mut values = Vec::new();
      for inner in &array.prefix {
        values.push(pattern_to_value(inner, env, p)?);
      }
      if let Some(spread) = &array.spread {
        if let Some(binding) = &spread.binding {
          let bound = pattern_to_value(binding, env, p)?;
          match bound {
            Value::MatrixValue(ref matrix) => values.extend(matrix.as_vec()),
            ref other => values.push(other.clone()),
          }
          values.push(bound.clone());
        }
      }
      for inner in &array.suffix {
        values.push(pattern_to_value(inner, env, p)?);
      }
      return Ok(Value::MatrixValue(Matrix::from_vec(values.clone(), 1, values.len())));
    }
    #[cfg(all(feature = "tuple", feature = "atom"))]
    Pattern::TupleStruct(pattern_tuple_struct) => {
      let mut values = Vec::with_capacity(pattern_tuple_struct.patterns.len() + 1);
      values.push(atom(&Atom {name: pattern_tuple_struct.name.clone()}, p));
      for inner in &pattern_tuple_struct.patterns {
        values.push(pattern_to_value(inner, env, p)?);
      }
      return Ok(Value::Tuple(Ref::new(MechTuple::from_vec(values))));
    }
    _ => Err(MechError::new(FeatureNotEnabledError, None).with_compiler_loc()),
  }
}

fn deep_detach_value(value: &Value) -> Value {
  match value {
    Value::MutableReference(reference) => deep_detach_value(&reference.borrow()),
    _ => value.clone(),
  }
}

fn collect_pattern_variable_ids(pattern: &Pattern, ids: &mut Vec<u64>) {
  match pattern {
    Pattern::Expression(Expression::Var(var)) => ids.push(var.name.hash()),
    #[cfg(feature = "tuple")]
    Pattern::Tuple(tuple) => {
      for item in &tuple.0 {
        collect_pattern_variable_ids(item, ids);
      }
    }
    #[cfg(feature = "matrix")]
    Pattern::Array(array) => {
      for item in &array.prefix {
        collect_pattern_variable_ids(item, ids);
      }
      if let Some(spread) = &array.spread {
        if let Some(binding) = &spread.binding {
          collect_pattern_variable_ids(binding, ids);
        }
      }
      for item in &array.suffix {
        collect_pattern_variable_ids(item, ids);
      }
    }
    #[cfg(all(feature = "tuple", feature = "atom"))]
    Pattern::TupleStruct(tuple_struct) => {
      for item in &tuple_struct.patterns {
        collect_pattern_variable_ids(item, ids);
      }
    }
    _ => {}
  }
}

fn matrix_like_values(value: &Value) -> Option<Vec<Value>> {
  match value {
    #[cfg(feature = "matrix")]
    Value::MatrixIndex(matrix) => Some(
      matrix
          .as_vec()
          .into_iter()
          .map(|value| Value::Index(Ref::new(value)))
          .collect(),
    ),
    #[cfg(all(feature = "matrix", feature = "bool"))]
    Value::MatrixBool(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "u8"))]
    Value::MatrixU8(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "u16"))]
    Value::MatrixU16(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "u32"))]
    Value::MatrixU32(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "u64"))]
    Value::MatrixU64(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "u128"))]
    Value::MatrixU128(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "i8"))]
    Value::MatrixI8(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "i16"))]
    Value::MatrixI16(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "i32"))]
    Value::MatrixI32(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "i64"))]
    Value::MatrixI64(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "i128"))]
    Value::MatrixI128(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "f32"))]
    Value::MatrixF32(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "f64"))]
    Value::MatrixF64(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "string"))]
    Value::MatrixString(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "rational"))]
    Value::MatrixR64(matrix) => Some(
      matrix
          .as_vec()
          .into_iter()
          .map(|value| value.to_value())
          .collect(),
    ),
    #[cfg(all(feature = "matrix", feature = "complex"))]
    Value::MatrixC64(matrix) => Some(
      matrix
          .as_vec()
          .into_iter()
          .map(|value| value.to_value())
          .collect(),
    ),
    #[cfg(feature = "matrix")]
    Value::MatrixValue(matrix) => Some(matrix.as_vec()),
    _ => None,
  }
}

fn extract_pattern_variable_id(expr: &Expression) -> Option<u64> {
  match expr {
    Expression::Var(var) => Some(var.name.hash()),
    Expression::Formula(factor) => match factor {
      Factor::Expression(inner_expr) => extract_pattern_variable_id(inner_expr),
      Factor::Term(term) if term.rhs.is_empty() => extract_pattern_variable_id_from_term(&term.lhs),
      _ => None,
    },
    _ => None,
  }
}

fn extract_pattern_variable_id_from_term(factor: &Factor) -> Option<u64> {
  match factor {
    Factor::Expression(expr) => extract_pattern_variable_id(expr),
    Factor::Parenthetical(inner) => extract_pattern_variable_id_from_term(inner),
    _ => None,
  }
}

fn values_match(expected: &Value, actual: &Value) -> bool {
  if expected == actual {
    return true;
  }
  match (expected, actual) {
    #[cfg(all(feature = "u64", feature = "f64"))]
    (Value::F64(x), Value::U64(y)) => return (*x.borrow() as u64) == *y.borrow(),
    #[cfg(all(feature = "u64", feature = "f64"))]
    (Value::U64(x), Value::F64(y)) => return *x.borrow() == (*y.borrow() as u64),
    _ => {}
  }
  false
}