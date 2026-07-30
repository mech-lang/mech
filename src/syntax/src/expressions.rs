#[macro_use]
use crate::*;
use crate::structures::tuple;

#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use nom::{
  IResult,
  branch::alt,
  sequence::{tuple as nom_tuple, preceded, pair},
  combinator::{opt, eof, cut},
  multi::{many1, many_till, many0, separated_list1,separated_list0},
  Err,
  Err::Failure
};

use std::collections::HashMap;
use colored::*;

use crate::*;

// Expressions
// ============================================================================

/*
Defines how expressions are parsed using a recursive structure hat reflects 
operator precedence. Parsing begins at the top-level (`formula`) and proceeds 
through increasingly tightly-binding operations, down to the basic elements 
like literals and variables.

- `formula`: entry point
- `l1`: logical operators (e.g., `and`, `or`)
- `l2`: comparisons (e.g., `==`, `<`, `>`)
- `l3`: addition and subtraction (`+`, `-`)
- `l4`: multiplication, division, matrix operations
- `l5`: exponentiation (`^`)
- `l6`: table operations (e.g., joins)
- `l7`: set operations (e.g., union, intersection)
- `factor`: atomic units (literals, function calls, variables, etc.)
*/

// Grammar: docs/design/specification.mec, `expression`.
pub fn expression(input: ParseString) -> ParseResult<Expression> {
  let (input, expr) = match fsm_pipe(input.clone()) {
    Ok((input, pipe)) => (input, Expression::FsmPipe(pipe)),
    Err(_) => match set_comprehension(input.clone()) {
      Ok((input, sc)) => (input, Expression::SetComprehension(Box::new(sc))),
      Err(_) => match matrix_comprehension(input.clone()) {
        Ok((input, mc)) => (input, Expression::MatrixComprehension(Box::new(mc))),
        Err(_) => match range_expression(input.clone()) {
          Ok((input, rng)) => (input, Expression::Range(Box::new(rng))),
          Err(_) => match formula(input.clone()) {
            Ok((input, source_factor)) => {
              let source_expression = match source_factor.clone() {
                Factor::Expression(expr) => *expr,
                fctr => Expression::Formula(fctr),
              };
              if let Ok((input, _)) = question(input.clone()) {
                let (input, _) = whitespace0(input)?;
                let (input, arms) = many1(match_arm)(input)?;
                let (input, _) = opt(period)(input)?;
                (input, Expression::Match(Box::new(MatchExpression { source: source_expression, arms })))
              } else {
                match source_factor {
                  Factor::Expression(expr) => (input, *expr),
                  fctr => (input, Expression::Formula(fctr)),
                }
              }
            }
            Err(err) => return Err(err),
          }
        }
      }
    }
  };
  Ok((input, expr))
}

// Grammar: docs/design/specification.mec, `match-expression`.
pub fn match_expression(input: ParseString) -> ParseResult<MatchExpression> {
  let (input, source) = factor(input)?;
  let source = match source {
    Factor::Expression(expr) => *expr,
    fctr => Expression::Formula(fctr),
  };
  let (input, _) = question(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, arms) = many1(match_arm)(input)?;
  let (input, _) = opt(period)(input)?;
  Ok((input, MatchExpression { source, arms }))
}

// Grammar: docs/design/specification.mec, `match-arm`.
pub fn match_arm(input: ParseString) -> ParseResult<MatchArm> {
  let (input, _) = crate::state_machines::guard_operator(input)?;
  let (input, pattern) = crate::patterns::pattern(input)?;
  let (input, guard) = opt(preceded(
    list_separator,
    preceded(whitespace0, expression),
  ))(input)?;
  let (input, _) = output_operator(input)?;
  let (input, expr) = expression(input)?;
  let (input, _) = opt(alt((whitespace1, statement_separator)))(input)?;
  Ok((input, MatchArm {
    pattern,
    guard,
    expression: expr,
  }))
}

// Grammar: docs/design/specification.mec, `formula`.
pub fn formula(input: ParseString) -> ParseResult<Factor> {
  let (input, factor) = l1(input)?;
  Ok((input, factor))
}

// Grammar: docs/design/specification.mec, `l1`.
pub fn l1(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l2(input)?;
  let (input, rhs) = many0(pair(logic_operator,cut(l2)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// Grammar: docs/design/specification.mec, `l2`.
pub fn l2(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l3(input)?;
  let (input, rhs) = many0(pair(comparison_operator,cut(l3)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// Grammar: docs/design/specification.mec, `l3`.
pub fn l3(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l4(input)?;
  let (input, rhs) = many0(pair(add_sub_operator,cut(l4)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// Grammar: docs/design/specification.mec, `l4`.
pub fn l4(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l5(input)?;
  let (input, rhs) = many0(pair(alt((mul_div_operator, matrix_operator)),cut(l5)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// Grammar: docs/design/specification.mec, `l5`.
pub fn l5(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l6(input)?;
  let (input, rhs) = many0(pair(power_operator,cut(l6)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// Grammar: docs/design/specification.mec, `l6`.
pub fn l6(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l7(input)?;
  let (input, rhs) = many0(pair(table_operator,cut(l7)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// Grammar: docs/design/specification.mec, `l7`.
pub fn l7(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = factor(input)?;
  let (input, rhs) = many0(pair(set_operator,cut(factor)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// Grammar: docs/design/specification.mec, `factor`.
pub fn factor(input: ParseString) -> ParseResult<Factor> {
  let (input, fctr) = if let Ok((input, fctr)) = parenthetical_term(input.clone()) {
    (input, fctr)
  } else if let Ok((input, fctr)) = negate_factor(input.clone()) {
    (input, fctr)
  } else if let Ok((input, fctr)) = not_factor(input.clone()) {
    (input, fctr)
  } else if let Ok((input, m)) = matrix_comprehension(input.clone()) {
    (input, Factor::Expression(Box::new(Expression::MatrixComprehension(Box::new(m)))))
  } else if let Ok((input, s)) = structure(input.clone()) {
    (input, Factor::Expression(Box::new(Expression::Structure(s))))
  } else if let Ok((input, f)) = function_call(input.clone()) {
    (input, Factor::Expression(Box::new(Expression::FunctionCall(f))))
  } else if let Ok((input, l)) = literal(input.clone()) {
    (input, Factor::Expression(Box::new(Expression::Literal(l))))
  } else if let Ok((input, s)) = slice(input.clone()) {
    (input, Factor::Expression(Box::new(Expression::Slice(s))))
  } else {
    match var(input.clone()) {
      Ok((input, v)) => (input, Factor::Expression(Box::new(Expression::Var(v)))),
      Err(err) => return Err(err),
    }
  };
  let (input, transpose) = opt(transpose)(input)?;
  let fctr = match transpose {
    Some(_) => Factor::Transpose(Box::new(fctr)),
    None => fctr,
  };
  Ok((input, fctr))
}

// Grammar: docs/design/specification.mec, `parenthetical-term`.
pub fn parenthetical_term(input: ParseString) -> ParseResult<Factor> {
  let msg1 = "parenthetical_term: Expects expression";
  let msg2 = "parenthetical_term: Expects right parenthesis `)`";
  let (input, (_, r)) = range(left_parenthesis)(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, frmla) = label!(formula, msg1)(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, _) = label!(right_parenthesis, msg2, r)(input)?;
  Ok((input, Factor::Parenthetical(Box::new(frmla))))
}

fn context_address_path_token(input: ParseString) -> ParseResult<Token> {
  alt((alpha_token, digit_token, dash, slash, underscore, period))(input)
}

fn context_address_path(input: ParseString) -> ParseResult<Identifier> {
  let (input, mut tokens) = many1(context_address_path_token)(input)?;
  let mut merged = Token::merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Identifier;
  Ok((input, Identifier { name: merged }))
}

fn prefixed_context_path(input: ParseString) -> ParseResult<(Identifier, Identifier)> {
  let (input, _) = at(input)?;
  let (input, context) = identifier_path_segment(input)?;
  let (input, _) = slash(input)?;
  let (input, name) = context_address_path(input)?;
  Ok((input, (context, name)))
}

// Grammar: docs/design/specification.mec, `var`.
pub fn var(input: ParseString) -> ParseResult<Var> {
  if let Ok((input, (context, name))) = prefixed_context_path(input.clone()) {
    let ((input, kind)) = opt(kind_annotation)(input)?;
    return Ok((input, Var{ name, context: Some(context), kind }));
  }
  let ((input, name)) = identifier(input)?;
  let ((input, kind)) = opt(kind_annotation)(input)?;
  Ok((input, Var{ name, context: None, kind }))
}

// Grammar: docs/design/specification.mec, `statement-separator`.
pub fn statement_separator(input: ParseString) -> ParseResult<()> {
  let (input,_) = nom_tuple((whitespace0,semicolon,whitespace0))(input)?;
  Ok((input, ()))
}

// Math Expressions
// ----------------------------------------------------------------------------

// Grammar: docs/design/specification.mec, `add-sub-operator`.
pub fn add_sub_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((add, subtract))(input)?;
  Ok((input, FormulaOperator::AddSub(op)))
}


// Grammar: docs/design/specification.mec, `mul-div-operator`.
pub fn mul_div_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((multiply, divide, modulus))(input)?;
  Ok((input, FormulaOperator::MulDiv(op)))
}

// Grammar: docs/design/specification.mec, `power-operator`.
pub fn power_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = power(input)?;
  Ok((input, FormulaOperator::Power(op)))
}

// Grammar: docs/design/specification.mec, `negate-factor`.
pub fn negate_factor(input: ParseString) -> ParseResult<Factor> {
  let (input, _) = dash(input)?;
  let (input, expr) = factor(input)?;
  Ok((input, Factor::Negate(Box::new(expr))))
}

// Grammar: docs/design/specification.mec, `not-factor`.
pub fn not_factor(input: ParseString) -> ParseResult<Factor> {
  let (input, _) = not(input)?;
  let (input, expr) = factor(input)?;
  Ok((input, Factor::Not(Box::new(expr))))
}

// Grammar: docs/design/specification.mec, `add`.
pub fn add(input: ParseString) -> ParseResult<AddSubOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("+")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, AddSubOp::Add))
}

pub fn subtract(input: ParseString) -> ParseResult<AddSubOp> {
  let (input, _) = alt((spaced_subtract, raw_subtract))(input)?;
  Ok((input, AddSubOp::Sub))
}

// Grammar: docs/design/specification.mec, `raw-subtract`.
pub fn raw_subtract(input: ParseString) -> ParseResult<AddSubOp> {
  let (input, _) = pair(is_not(comment_sigil), tag("-"))(input)?;
  Ok((input, AddSubOp::Sub))
}

pub fn spaced_subtract(input: ParseString) -> ParseResult<AddSubOp> {
  let (input, _) = ws1e(input)?;
  let (input, _) = raw_subtract(input)?;
  let (input, _) = ws1e(input)?;
  Ok((input, AddSubOp::Sub))
}

// Grammar: docs/design/specification.mec, `multiply`.
pub fn multiply(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = pair(is_not(matrix_multiply),alt((tag("*"), tag("×"))))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, MulDivOp::Mul))
}

// Grammar: docs/design/specification.mec, `divide`.
pub fn divide(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = pair(is_not(comment_sigil),alt((tag("/"),tag("÷"))))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, MulDivOp::Div))
}

// Grammar: docs/design/specification.mec, `modulus`.
pub fn modulus(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("%")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, MulDivOp::Mod))
}

// Grammar: docs/design/specification.mec, `power`.
pub fn power(input: ParseString) -> ParseResult<PowerOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("^")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, PowerOp::Pow))
}

// Matrix Operations
// ----------------------------------------------------------------------------

// Grammar: docs/design/specification.mec, `matrix-operator`.
pub fn matrix_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((matrix_multiply, matrix_solve, dot_product, cross_product))(input)?;
  Ok((input, FormulaOperator::Vec(op)))
}

// Grammar: docs/design/specification.mec, `matrix-multiply`.
pub fn matrix_multiply(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("**")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, VecOp::MatMul))
}

// Grammar: docs/design/specification.mec, `matrix-solve`.
pub fn matrix_solve(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("\\")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, VecOp::Solve))
}

// Grammar: docs/design/specification.mec, `dot-product`.
pub fn dot_product(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("·"),tag("•")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, VecOp::Dot))
}

// Grammar: docs/design/specification.mec, `cross-product`.
pub fn cross_product(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⨯")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, VecOp::Cross))
}

// Grammar: docs/design/specification.mec, `transpose`.
pub fn transpose(input: ParseString) -> ParseResult<()> {
  let (input, _) = tag("'")(input)?;
  Ok((input, ()))
}

// Range Expressions
// ----------------------------------------------------------------------------

// Grammar: docs/design/specification.mec, `range-expression`.
pub fn range_expression(input: ParseString) -> ParseResult<RangeExpression> {
  let (input, start) = formula(input)?;
  let (input, op) = range_operator(input)?;
  let (input, x) = formula(input)?;
  let (input, y) = opt(nom_tuple((range_operator,formula)))(input)?;
  let range = match y {
    Some((op2,terminal)) => RangeExpression{start, increment: Some((op,x)), operator: op2, terminal},
    None => RangeExpression{start, increment: None, operator: op, terminal: x},
  };
  Ok((input, range))
}

// Grammar: docs/design/specification.mec, `range-inclusive`.
pub fn range_inclusive(input: ParseString) -> ParseResult<RangeOp> {
  let (input, _) = tag("..=")(input)?;
  Ok((input, RangeOp::Inclusive))
}

// Grammar: docs/design/specification.mec, `range-exclusive`.
pub fn range_exclusive(input: ParseString) -> ParseResult<RangeOp> {
  let (input, _) = tag("..")(input)?;
  Ok((input, RangeOp::Exclusive))
}

// Grammar: docs/design/specification.mec, `range-operator`.
pub fn range_operator(input: ParseString) -> ParseResult<RangeOp> {
  let (input, op) = alt((range_inclusive,range_exclusive))(input)?;
  Ok((input, op))
}

// Comparison expressions
// ----------------------------------------------------------------------------

// Grammar: docs/design/specification.mec, `comparison-operator`.
pub fn comparison_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((strict_equal, strict_not_equal, not_equal, equal_to, greater_than_equal, greater_than, less_than_equal, less_than))(input)?;
  Ok((input, FormulaOperator::Comparison(op)))
}

// Grammar: docs/design/specification.mec, `not-equal`.
pub fn not_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("!="),tag("¬="),tag("≠")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::NotEqual))
}

// Grammar: docs/design/specification.mec, `equal-to`.
pub fn equal_to(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("=="),tag("⩵")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::Equal))
}

// Grammar: docs/design/specification.mec, `strict-not-equal`.
pub fn strict_not_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("!=="),tag("!≡"),tag("¬≡"),tag("¬==")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::StrictNotEqual))
}

// Grammar: docs/design/specification.mec, `strict-equal`.
pub fn strict_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("==="),tag("≡")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::StrictEqual))
}

// Grammar: docs/design/specification.mec, `greater-than`.
pub fn greater_than(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag(">")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::GreaterThan))
}

// Grammar: docs/design/specification.mec, `less-than`.
pub fn less_than(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = is_not(tag("<-"))(input)?;
  let (input, _) = tag("<")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::LessThan))
}

// Grammar: docs/design/specification.mec, `greater-than-equal`.
pub fn greater_than_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag(">="),tag("≥")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::GreaterThanEqual))
}

// Grammar: docs/design/specification.mec, `less-than-equal`.
pub fn less_than_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("<="),tag("≤")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::LessThanEqual))
}

// Logic expressions
// ----------------------------------------------------------------------------

// Grammar: docs/design/specification.mec, `logic-operator`.
pub fn logic_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((and, or, xor))(input)?;
  Ok((input, FormulaOperator::Logic(op)))
}

// Grammar: docs/design/specification.mec, `or`.
pub fn or(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("||"), tag("∨"), tag("⋁")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, LogicOp::Or))
}

// Grammar: docs/design/specification.mec, `and`.
pub fn and(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("&&"), tag("∧"), tag("⋀")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, LogicOp::And))
}

// Grammar: docs/design/specification.mec, `not`.
pub fn not(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = alt((tag("!"), tag("¬")))(input)?;
  Ok((input, LogicOp::Not))
}

// Grammar: docs/design/specification.mec, `xor`.
pub fn xor(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("^^"), tag("⊕"), tag("⊻")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, LogicOp::Xor))
}

// Table Operations
// ----------------------------------------------------------------------------

// Grammar: docs/design/specification.mec, `table-operator`.
fn table_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((join,left_join,right_join,full_join,left_semi_join,left_anti_join))(input)?;
  Ok((input, FormulaOperator::Table(op)))
}

// Grammar: docs/design/specification.mec, `join`.
fn join(input: ParseString) -> ParseResult<TableOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⋈")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, TableOp::InnerJoin))
}

// Grammar: docs/design/specification.mec, `left-join`.
fn left_join(input: ParseString) -> ParseResult<TableOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⟕")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, TableOp::LeftOuterJoin))
}

// Grammar: docs/design/specification.mec, `right-join`.
fn right_join(input: ParseString) -> ParseResult<TableOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⟖")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, TableOp::RightOuterJoin))
}

// Grammar: docs/design/specification.mec, `full-join`.
fn full_join(input: ParseString) -> ParseResult<TableOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⟗")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, TableOp::FullOuterJoin))
}

// Grammar: docs/design/specification.mec, `left-semi-join`.
fn left_semi_join(input: ParseString) -> ParseResult<TableOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⋉")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, TableOp::LeftSemiJoin))
}

// Grammar: docs/design/specification.mec, `left-anti-join`.
fn left_anti_join(input: ParseString) -> ParseResult<TableOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("▷")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, TableOp::LeftAntiJoin))
}


// Set Operations
// ----------------------------------------------------------------------------

// Grammar: docs/design/specification.mec, `set-operator`.
pub fn set_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((union_op,intersection,difference,complement,subset,superset,proper_subset,proper_superset,element_of,not_element_of,symmetric_difference))(input)?;
  Ok((input, FormulaOperator::Set(op)))
}

// Grammar: docs/design/specification.mec, `union-op`.
pub fn union_op(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("∪")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::Union))
}

// Grammar: docs/design/specification.mec, `intersection`.
pub fn intersection(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("∩")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::Intersection))
}

// Grammar: docs/design/specification.mec, `difference`.
pub fn difference(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("∖")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::Difference))
}

// Grammar: docs/design/specification.mec, `complement`.
pub fn complement(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("∁")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::Complement))
}

// Grammar: docs/design/specification.mec, `subset`.
pub fn subset(input: ParseString) -> ParseResult<SetOp> { 
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⊆")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::Subset))
}

// Grammar: docs/design/specification.mec, `superset`.
pub fn superset(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⊇")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::Superset))
}

// Grammar: docs/design/specification.mec, `proper-subset`.
pub fn proper_subset(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("⊊"), tag("⊂")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::ProperSubset))
}

// Grammar: docs/design/specification.mec, `proper-superset`.
pub fn proper_superset(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("⊋"), tag("⊃")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::ProperSuperset))
}

// Grammar: docs/design/specification.mec, `element-of`.
pub fn element_of(input: ParseString) -> ParseResult<SetOp> { 
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("∈")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::ElementOf))
}

// Grammar: docs/design/specification.mec, `not-element-of`.
pub fn not_element_of(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("∉")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::NotElementOf))
}

// Grammar: docs/design/specification.mec, `symmetric-difference`.
pub fn symmetric_difference(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws1e(input)?;
  let (input, _) = tag("Δ")(input)?;
  let (input, _) = ws1e(input)?;
  Ok((input, SetOp::SymmetricDifference))
}

// Set Comprehensions
// ----------------------------------------------------------------------------

// Grammar: docs/design/specification.mec, `set-comprehension`.
pub fn set_comprehension(input: ParseString) -> ParseResult<SetComprehension> {
  let (input, _) = left_brace(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, expr) = expression(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, _) = bar(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, quals) = separated_list1(list_separator, comprehension_qualifier)(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, SetComprehension{ expression: expr, qualifiers: quals }))
}

// Grammar: docs/design/specification.mec, `matrix-comprehension`.
pub fn matrix_comprehension(input: ParseString) -> ParseResult<MatrixComprehension> {
  let (input, _) = left_bracket(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, expr) = expression(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, _) = bar(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, quals) = separated_list1(list_separator, comprehension_qualifier)(input)?;
  if !quals.iter().any(|q| matches!(q, ComprehensionQualifier::Generator(_) | ComprehensionQualifier::Let(_))) {
    return Err(nom::Err::Error(ParseError::new(
      input,
      "Matrix comprehension requires at least one generator (<-) or let (:=) qualifier",
    )));
  }
  let (input, _) = space_tab0(input)?;
  let (input, _) = right_bracket(input)?;
  Ok((input, MatrixComprehension{ expression: expr, qualifiers: quals }))
}

// Grammar: docs/design/specification.mec, `comprehension-qualifier`.
pub fn comprehension_qualifier(input: ParseString) -> ParseResult<ComprehensionQualifier> {
  match generator(input.clone()) {
    Ok((input, generator)) => Ok((input, generator)),
    Err(_) => match variable_define(input.clone()) {
      Ok((input, var_def)) => Ok((input, ComprehensionQualifier::Let(var_def))),
      Err(_) => {
        let (input, expr) = expression(input)?;
        Ok((input, ComprehensionQualifier::Filter(expr)))
      }
    }
  }
}

// Grammar: docs/design/specification.mec, `generator`.
pub fn generator(input: ParseString) -> ParseResult<ComprehensionQualifier> {
  let (input, ptrn) = pattern(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, _) = cut(alt((generator_arrow, generator_arrow_u)))(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, expr) = expression(input)?;
  Ok((input, ComprehensionQualifier::Generator((ptrn, expr))))
}

// Subscript Operations
// ----------------------------------------------------------------------------

// Grammar: docs/design/specification.mec, `subscript`.
pub fn subscript(input: ParseString) -> ParseResult<Vec<Subscript>> {
  let (input, subscripts) = many1(alt((swizzle_subscript,dot_subscript,dot_subscript_int,bracket_subscript,brace_subscript)))(input)?;
  Ok((input, subscripts))
}

// Grammar: docs/design/specification.mec, `slice`.
pub fn slice(input: ParseString) -> ParseResult<Slice> {
  if let Ok((input, (context, name))) = prefixed_context_path(input.clone()) {
    let (input, ixes) = subscript(input)?;
    return Ok((input, Slice{name, context: Some(context), subscript: ixes}));
  }
  let (input, name) = identifier(input)?;
  let (input, ixes) = subscript(input)?;
  Ok((input, Slice{name, context: None, subscript: ixes}))
}

// Grammar: docs/design/specification.mec, `slice-ref`.
pub fn slice_ref(input: ParseString) -> ParseResult<SliceRef> {
  if let Ok((input, (context, name))) = prefixed_context_path(input.clone()) {
    let (input, ixes) = opt(subscript)(input)?;
    return Ok((input, SliceRef{name, context: Some(context), subscript: ixes}));
  }
  let (input, name) = identifier(input)?;
  let (input, ixes) = opt(subscript)(input)?;
  Ok((input, SliceRef{name, context: None, subscript: ixes}))
}

// Grammar: docs/design/specification.mec, `swizzle-subscript`.
pub fn swizzle_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = period(input)?;
  let (input, first) = identifier(input)?;
  let (input, _) = comma(input)?;
  let (input, mut name) = separated_list1(tag(","),identifier)(input)?;
  let mut subscripts = vec![first];
  subscripts.append(&mut name);
  Ok((input, Subscript::Swizzle(subscripts)))
}

// Grammar: docs/design/specification.mec, `dot-subscript`.
pub fn dot_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = period(input)?;
  let (input, name) = identifier(input)?;
  Ok((input, Subscript::Dot(name)))
}

// Grammar: docs/design/specification.mec, `dot-subscript-int`.
pub fn dot_subscript_int(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = period(input)?;
  let (input, name) = integer_literal(input)?;
  Ok((input, Subscript::DotInt(name)))
}

// Grammar: docs/design/specification.mec, `bracket-subscript`.
pub fn bracket_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = left_bracket(input)?;
  let (input, subscripts) = separated_list1(list_separator,alt((select_all,range_subscript,formula_subscript)))(input)?;
  let (input, _) = right_bracket(input)?;
  Ok((input, Subscript::Bracket(subscripts)))
}

// Grammar: docs/design/specification.mec, `brace-subscript`.
pub fn brace_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = left_brace(input)?;
  let (input, subscripts) = separated_list1(list_separator,alt((select_all,range_subscript,formula_subscript)))(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Subscript::Brace(subscripts)))
}

// Grammar: docs/design/specification.mec, `select-all`.
pub fn select_all(input: ParseString) -> ParseResult<Subscript> {
  let (input, lhs) = colon(input)?;
  Ok((input, Subscript::All))
}

// Grammar: docs/design/specification.mec, `formula-subscript`.
pub fn formula_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, factor) = formula(input)?;
  Ok((input, Subscript::Formula(factor)))
}

// Grammar: docs/design/specification.mec, `range-subscript`.
pub fn range_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, rng) = range_expression(input)?;
  Ok((input, Subscript::Range(rng)))
}

#[cfg(test)]
#[path = "expressions/operator_parity.rs"]
mod canonical_phase_2d_operator_parity;

#[cfg(test)]
mod canonical_phase_2c_context_path_tests {
  use super::*;

  use std::panic::{catch_unwind, AssertUnwindSafe};

  use mech_core::{SourceLocation, SourceRange, TokenKind};

  use crate::document::ast::paths::{
    ContextAddressPathSyntax, PrefixedContextPathSyntax,
  };
  use crate::document::parser::canonical::parse_canonical_phase_2c_rule_for_test;
  use crate::document::parser::rules;
  use crate::document::{
    AstNode, DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextRange,
    TextSize, TextSnapshot, lower_legacy_context_address_path,
    lower_legacy_prefixed_context_path,
    reconstruct_source_range,
  };

  #[derive(Clone, Copy, Debug, Eq, PartialEq)]
  struct LegacyPrefix {
    consumed: TextSize,
    remaining: TextSize,
  }

  fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(925), Revision(0), text).unwrap()
  }

  fn parse(
    text: &str,
    rule: RuleId,
  ) -> crate::document::parser::canonical::CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2c_rule_for_test(source(text), rule, ParseConfig::default())
      .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2C direct rule"))
  }

  fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
      return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
  }

  fn legacy_prefix<Output>(
    input: &str,
    parser: for<'source> fn(ParseString<'source>) -> ParseResult<'source, Output>,
  ) -> Option<LegacyPrefix> {
    let graphemes = crate::graphemes::init_tag(input);
    parser(ParseString::new(&graphemes)).ok().map(|(remaining, _)| {
      let consumed = graphemes[..remaining.cursor]
        .iter()
        .map(|grapheme| grapheme.len())
        .sum::<usize>();
      let remaining = graphemes[remaining.cursor..]
        .iter()
        .map(|grapheme| grapheme.len())
        .sum::<usize>();
      LegacyPrefix {
        consumed: TextSize(consumed as u32),
        remaining: TextSize(remaining as u32),
      }
    })
  }

  fn legacy_value<Output>(
    input: &str,
    parser: for<'source> fn(ParseString<'source>) -> ParseResult<'source, Output>,
  ) -> Output {
    let graphemes = crate::graphemes::init_tag(input);
    let (remaining, value) = parser(ParseString::new(&graphemes)).unwrap();
    assert_eq!(remaining.cursor, graphemes.len(), "{input:?}");
    assert!(remaining.error_log.is_empty(), "{input:?}");
    value
  }

  fn assert_prefix_contract<Output>(
    input: &str,
    rule: RuleId,
    parser: for<'source> fn(ParseString<'source>) -> ParseResult<'source, Output>,
  ) {
    let canonical = parse(input, rule);
    let legacy = legacy_prefix(input, parser);
    assert_eq!(canonical.matched, legacy.is_some(), "{rule:?} on {input:?}");

    if let Some(legacy) = legacy {
      assert!(canonical.is_strictly_clean(), "{rule:?} on {input:?}");
      assert_eq!(canonical.consumed.start, TextSize::ZERO, "{input:?}");
      assert_eq!(canonical.consumed.end, legacy.consumed, "{input:?}");
      assert_eq!(
        canonical.source.byte_len().0 - canonical.consumed.end.0,
        legacy.remaining.0,
        "{input:?}",
      );
    } else {
      assert!(canonical.diagnostics.is_empty(), "{rule:?} on {input:?}");
      assert_eq!(
        canonical.consumed,
        TextRange::empty(TextSize::ZERO),
        "{input:?}",
      );
    }
  }

  fn legacy_token_kind(kind: SyntaxKind) -> TokenKind {
    match kind {
      SyntaxKind::Alpha => TokenKind::Alpha,
      SyntaxKind::Digit => TokenKind::Digit,
      SyntaxKind::Dash => TokenKind::Dash,
      SyntaxKind::Slash => TokenKind::Slash,
      SyntaxKind::Underscore => TokenKind::Underscore,
      SyntaxKind::Period => TokenKind::Period,
      other => panic!("unexpected context-address-path token {other:?}"),
    }
  }

  #[test]
  fn private_context_address_path_tokens_match_canonical_values_and_extents() {
    for input in ["a", "3", "-", "/", "_", "."] {
      assert_prefix_contract(input, rules::CONTEXT_ADDRESS_PATH_TOKEN, context_address_path_token);

      let canonical = parse(input, rules::CONTEXT_ADDRESS_PATH_TOKEN);
      let token = canonical.syntax().tokens().into_iter().next().unwrap();
      let legacy = legacy_value(input, context_address_path_token);
      assert_eq!(legacy.kind, legacy_token_kind(token.kind()), "{input:?}");
      assert_eq!(
        legacy.chars,
        token.text().unwrap().chars().collect::<Vec<_>>(),
        "{input:?}",
      );
      assert_eq!(
        legacy.src_range,
        SourceRange {
          start: SourceLocation { row: 1, col: 1 },
          end: SourceLocation { row: 1, col: 2 },
        },
        "{input:?}",
      );
    }
  }

  #[test]
  fn private_context_address_paths_match_canonical_lowering_and_extents() {
    for input in ["path", "path/to.value_1", "x-y"] {
      assert_prefix_contract(input, rules::CONTEXT_ADDRESS_PATH, context_address_path);

      let canonical = parse(input, rules::CONTEXT_ADDRESS_PATH);
      let node = find_node(&canonical.syntax(), SyntaxKind::ContextAddressPath).unwrap();
      let canonical_value = lower_legacy_context_address_path(
        &ContextAddressPathSyntax::cast(node).unwrap(),
      )
      .unwrap();
      let legacy = legacy_value(input, context_address_path);
      assert_eq!(canonical_value, legacy, "{input:?}");
    }
  }

  #[test]
  fn private_prefixed_context_paths_match_canonical_lowering_and_extents() {
    for input in [
      "@context/path",
      "@ctx/path/to.value_1",
      "@💡/x-y",
    ] {
      assert_prefix_contract(input, rules::PREFIXED_CONTEXT_PATH, prefixed_context_path);

      let canonical = parse(input, rules::PREFIXED_CONTEXT_PATH);
      let node = find_node(&canonical.syntax(), SyntaxKind::PrefixedContextPath).unwrap();
      let canonical_value = lower_legacy_prefixed_context_path(
        &PrefixedContextPathSyntax::cast(node).unwrap(),
      )
      .unwrap();
      let legacy = legacy_value(input, prefixed_context_path);
      assert_eq!(canonical_value, legacy, "{input:?}");
    }
  }

  #[test]
  fn incomplete_prefixed_context_paths_remain_noncommitting() {
    for input in ["@", "@ctx", "@ctx/", "@/path"] {
      assert_prefix_contract(input, rules::PREFIXED_CONTEXT_PATH, prefixed_context_path);
    }
  }

  #[derive(Clone, Copy, Debug, Eq, PartialEq)]
  enum LegacyContractOutcome {
    Matched(LegacyPrefix),
    NoMatch,
    Panicked,
  }

  type LegacyContractParser = fn(&str) -> LegacyContractOutcome;

  #[derive(Clone, Copy)]
  struct Phase2CContract {
    rule: RuleId,
    name: &'static str,
    legacy: LegacyContractParser,
    // The five columns are minimal success, representative success, valid
    // prefix with remainder, boundary failure, and ambiguous alternative.
    inputs: [&'static str; 5],
    known_panic_inputs: &'static [&'static str],
  }

  const BOUNDARY_CASE_KINDS: [&str; 5] = [
    "minimal success",
    "representative success",
    "valid prefix with remainder",
    "boundary failure",
    "ambiguous alternative",
  ];

  fn legacy_contract<Output>(
    input: &str,
    parser: for<'source> fn(ParseString<'source>) -> ParseResult<'source, Output>,
  ) -> LegacyContractOutcome {
    let result = catch_unwind(AssertUnwindSafe(|| legacy_prefix(input, parser)));
    match result {
      Ok(Some(prefix)) => LegacyContractOutcome::Matched(prefix),
      Ok(None) => LegacyContractOutcome::NoMatch,
      Err(_) => LegacyContractOutcome::Panicked,
    }
  }

  macro_rules! legacy_contract_parser {
    ($name:ident, $parser:path) => {
      fn $name(input: &str) -> LegacyContractOutcome {
        legacy_contract(input, $parser)
      }
    };
  }

  legacy_contract_parser!(legacy_empty_contract, crate::empty);
  legacy_contract_parser!(legacy_atom_contract, crate::atom);
  legacy_contract_parser!(legacy_string_contract, crate::string);
  legacy_contract_parser!(legacy_utf8_string_contract, crate::utf8_string);
  legacy_contract_parser!(legacy_raw_string_contract, crate::raw_string);
  legacy_contract_parser!(legacy_boolean_contract, crate::boolean);
  legacy_contract_parser!(legacy_true_literal_contract, crate::true_literal);
  legacy_contract_parser!(legacy_false_literal_contract, crate::false_literal);
  legacy_contract_parser!(legacy_number_contract, crate::number);
  legacy_contract_parser!(legacy_complex_number_contract, crate::complex_number);
  legacy_contract_parser!(legacy_real_number_contract, crate::real_number);
  legacy_contract_parser!(legacy_untyped_real_number_contract, crate::untyped_real_number);
  legacy_contract_parser!(legacy_rational_literal_contract, crate::rational_literal);
  legacy_contract_parser!(legacy_scientific_literal_contract, crate::scientific_literal);
  legacy_contract_parser!(legacy_float_decimal_start_contract, crate::float_decimal_start);
  legacy_contract_parser!(legacy_float_full_contract, crate::float_full);
  legacy_contract_parser!(legacy_float_literal_contract, crate::float_literal);
  legacy_contract_parser!(legacy_integer_literal_contract, crate::integer_literal);
  legacy_contract_parser!(legacy_typed_integer_contract, crate::typed_integer);
  legacy_contract_parser!(legacy_untyped_integer_contract, crate::untyped_integer);
  legacy_contract_parser!(legacy_decimal_literal_contract, crate::decimal_literal);
  legacy_contract_parser!(legacy_hexadecimal_literal_contract, crate::hexadecimal_literal);
  legacy_contract_parser!(legacy_octal_literal_contract, crate::octal_literal);
  legacy_contract_parser!(legacy_binary_literal_contract, crate::binary_literal);
  legacy_contract_parser!(legacy_context_address_path_token_contract, context_address_path_token);
  legacy_contract_parser!(legacy_context_address_path_contract, context_address_path);
  legacy_contract_parser!(legacy_prefixed_context_path_contract, prefixed_context_path);
  legacy_contract_parser!(legacy_kind_any_contract, crate::kind_any);
  legacy_contract_parser!(legacy_kind_empty_contract, crate::kind_empty);
  legacy_contract_parser!(legacy_kind_atom_contract, crate::kind_atom);

  fn phase_2c_contracts() -> [Phase2CContract; 30] {
    [
      Phase2CContract {
        rule: rules::EMPTY,
        name: "empty",
        legacy: legacy_empty_contract,
        inputs: ["_", "___", "___tail", "x", "__"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::ATOM,
        name: "atom",
        legacy: legacy_atom_contract,
        inputs: [":a", ":💡", ":a/tail", ":", ":a-b"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::STRING,
        name: "string",
        legacy: legacy_string_contract,
        inputs: ["\"\"", "\"text\"", "\"text\"tail", "plain", "\"\"\"raw\"\"\""],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::UTF8_STRING,
        name: "utf8-string",
        legacy: legacy_utf8_string_contract,
        inputs: ["\"\"", "\"text\"", "\"text\"tail", "plain", "\"\"\"\""],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::RAW_STRING,
        name: "raw-string",
        legacy: legacy_raw_string_contract,
        inputs: ["\"\"\"\"\"\"", "\"\"\"raw\"\"\"", "\"\"\"raw\"\"\"tail", "plain", "\"\""],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::BOOLEAN,
        name: "boolean",
        legacy: legacy_boolean_contract,
        inputs: ["true", "false", "truex", "x", "✓tail"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::TRUE_LITERAL,
        name: "true-literal",
        legacy: legacy_true_literal_contract,
        inputs: ["true", "✓", "truex", "false", "true-value"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::FALSE_LITERAL,
        name: "false-literal",
        legacy: legacy_false_literal_contract,
        inputs: ["false", "✗", "falsehood", "true", "false-value"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::NUMBER,
        name: "number",
        legacy: legacy_number_contract,
        inputs: ["1", "0xG_", "1tail", ".", "1u8/2u16"],
        known_panic_inputs: &["1.0e3u8"],
      },
      Phase2CContract {
        rule: rules::COMPLEX_NUMBER,
        name: "complex-number",
        legacy: legacy_complex_number_contract,
        inputs: ["2i", "1+-2i", "2itail", "1", "1+2i"],
        known_panic_inputs: &["1.0e3u8"],
      },
      Phase2CContract {
        rule: rules::REAL_NUMBER,
        name: "real-number",
        legacy: legacy_real_number_contract,
        inputs: ["1", "-0xFF", "1tail", ".", "1u8/2u16"],
        known_panic_inputs: &["1.0e3u8"],
      },
      Phase2CContract {
        rule: rules::UNTYPED_REAL_NUMBER,
        name: "untyped-real-number",
        legacy: legacy_untyped_real_number_contract,
        inputs: ["1", "-0o9", "1tail", ".", "1/2"],
        known_panic_inputs: &["1.0e3u8"],
      },
      Phase2CContract {
        rule: rules::RATIONAL_LITERAL,
        name: "rational-literal",
        legacy: legacy_rational_literal_contract,
        inputs: ["1/2", "1_0/2_0", "1/2tail", "1/", "1u8/2u16"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::SCIENTIFIC_LITERAL,
        name: "scientific-literal",
        legacy: legacy_scientific_literal_contract,
        inputs: ["1.0e3", "1.0e+-3", "1.0e3+tail", "1.0e", "1e3"],
        known_panic_inputs: &["1.0e3u8"],
      },
      Phase2CContract {
        rule: rules::FLOAT_DECIMAL_START,
        name: "float-decimal-start",
        legacy: legacy_float_decimal_start_contract,
        inputs: [".5", ".٣", ".5tail", ".", ".5.2"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::FLOAT_FULL,
        name: "float-full",
        legacy: legacy_float_full_contract,
        inputs: ["1.0", "1.٣", "1.0tail", "1.", "1.2.3"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::FLOAT_LITERAL,
        name: "float-literal",
        legacy: legacy_float_literal_contract,
        inputs: [".5", "1.0", ".5tail", "1.", "1.2.3"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::INTEGER_LITERAL,
        name: "integer-literal",
        legacy: legacy_integer_literal_contract,
        inputs: ["1", "1u8", "1tail", "x", "1.0"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::TYPED_INTEGER,
        name: "typed-integer",
        legacy: legacy_typed_integer_contract,
        inputs: ["1a", "1u8", "1foo/2", "1", "1e3"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::UNTYPED_INTEGER,
        name: "untyped-integer",
        legacy: legacy_untyped_integer_contract,
        inputs: ["1", "1_000", "1tail", "x", "1u8"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::DECIMAL_LITERAL,
        name: "decimal-literal",
        legacy: legacy_decimal_literal_contract,
        inputs: ["0d1", "0d٣", "0d1tail", "0x1", "0d1_2"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::HEXADECIMAL_LITERAL,
        name: "hexadecimal-literal",
        legacy: legacy_hexadecimal_literal_contract,
        inputs: ["0x0", "0xG_", "0xG_tail", "0d1", "0xF"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::OCTAL_LITERAL,
        name: "octal-literal",
        legacy: legacy_octal_literal_contract,
        inputs: ["0o1", "0o9", "0o9tail", "0d1", "0o1_2"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::BINARY_LITERAL,
        name: "binary-literal",
        legacy: legacy_binary_literal_contract,
        inputs: ["0b1", "0b9", "0b9tail", "0d1", "0b1_2"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::CONTEXT_ADDRESS_PATH_TOKEN,
        name: "context-address-path-token",
        legacy: legacy_context_address_path_token_contract,
        inputs: ["a", "3", "a/", "💡", "-"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::CONTEXT_ADDRESS_PATH,
        name: "context-address-path",
        legacy: legacy_context_address_path_contract,
        inputs: ["a", "path/to.value_1", "path/to!", "💡", "a-b"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::PREFIXED_CONTEXT_PATH,
        name: "prefixed-context-path",
        legacy: legacy_prefixed_context_path_contract,
        inputs: ["@ctx/path", "@💡/x-y", "@ctx/path!", "@ctx/", "@ctx/path/to.value_1"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::KIND_ANY,
        name: "kind-any",
        legacy: legacy_kind_any_contract,
        inputs: ["*", "*", "*tail", "_", "**"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::KIND_EMPTY,
        name: "kind-empty",
        legacy: legacy_kind_empty_contract,
        inputs: ["_", "___", "___tail", "x", "__"],
        known_panic_inputs: &[],
      },
      Phase2CContract {
        rule: rules::KIND_ATOM,
        name: "kind-atom",
        legacy: legacy_kind_atom_contract,
        inputs: [":a", ":💡", ":a/tail", ":", ":a-b"],
        known_panic_inputs: &[],
      },
    ]
  }

  fn assert_phase_2c_contract(contract: Phase2CContract, input: &str, case_kind: &str) {
    let canonical = parse(input, contract.rule);
    match (contract.legacy)(input) {
      LegacyContractOutcome::Matched(legacy) => {
        assert!(canonical.matched, "{} {case_kind} must match {input:?}", contract.name);
        assert!(canonical.is_strictly_clean(), "{} {case_kind} on {input:?}", contract.name);
        assert_eq!(canonical.consumed.start, TextSize::ZERO, "{} {case_kind}", contract.name);
        assert_eq!(
          canonical.consumed.end,
          legacy.consumed,
          "{} {case_kind} consumed extent mismatch for {input:?}",
          contract.name,
        );
        assert_eq!(
          canonical.source.byte_len().0 - canonical.consumed.end.0,
          legacy.remaining.0,
          "{} {case_kind} remaining extent mismatch for {input:?}",
          contract.name,
        );
        assert_eq!(
          reconstruct_source_range(&canonical.root, &canonical.source, canonical.consumed).unwrap(),
          &input[..legacy.consumed.0 as usize],
          "{} {case_kind} must preserve consumed source for {input:?}",
          contract.name,
        );
      }
      LegacyContractOutcome::NoMatch => {
        assert!(!canonical.matched, "{} {case_kind} must reject {input:?}", contract.name);
        assert!(canonical.diagnostics.is_empty(), "{} {case_kind} on {input:?}", contract.name);
        assert_eq!(
          canonical.consumed,
          TextRange::empty(TextSize::ZERO),
          "{} {case_kind} on {input:?}",
          contract.name,
        );
      }
      LegacyContractOutcome::Panicked => {
        assert!(
          contract.known_panic_inputs.contains(&input),
          "unexpected legacy panic in {} {case_kind} for {input:?}",
          contract.name,
        );
        let repeat = parse(input, contract.rule);
        assert_eq!(canonical.matched, repeat.matched, "{} {case_kind} on {input:?}", contract.name);
        assert_eq!(canonical.consumed, repeat.consumed, "{} {case_kind} on {input:?}", contract.name);
        assert_eq!(
          canonical.diagnostics.len(),
          repeat.diagnostics.len(),
          "{} {case_kind} on {input:?}",
          contract.name,
        );
      }
    }
  }

  #[test]
  fn all_phase_2c_direct_rules_match_legacy_boundaries() {
    let contracts = phase_2c_contracts();
    assert_eq!(contracts.len(), 30);
    for contract in contracts {
      for (case_kind, input) in BOUNDARY_CASE_KINDS.into_iter().zip(contract.inputs) {
        assert_phase_2c_contract(contract, input, case_kind);
      }
    }
  }

  #[test]
  fn typed_scientific_exponent_legacy_panics_are_explicitly_characterized() {
    for contract in phase_2c_contracts() {
      for input in contract.known_panic_inputs {
        assert_eq!(
          (contract.legacy)(input),
          LegacyContractOutcome::Panicked,
          "{} on {input:?}",
          contract.name,
        );
        let canonical = parse(input, contract.rule);
        let repeat = parse(input, contract.rule);
        assert_eq!(canonical.matched, repeat.matched, "{} on {input:?}", contract.name);
        assert_eq!(canonical.consumed, repeat.consumed, "{} on {input:?}", contract.name);
      }
    }
  }
}
