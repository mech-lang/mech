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
