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
- `l1`: addition and subtraction (`+`, `-`)
- `l2`: multiplication, division, matrix operations
- `l3`: exponentiation (`^`)
- `l4`: logical operators (e.g., `and`, `or`)
- `l5`: comparisons (e.g., `==`, `<`, `>`)
- `l6`: table operations (e.g., joins)
- `l7`: set operations (e.g., union, intersection)
- `factor`: atomic units (literals, function calls, variables, etc.)
*/

// expression := set-comprehension | range-expression | formula ;
pub fn expression(input: ParseString) -> ParseResult<Expression> {
  let (input, expr) = match set_comprehension(input.clone()) {
    Ok((input, sc)) => (input, Expression::SetComprehension(Box::new(sc))),
    Err(_) => match range_expression(input.clone()) {
      Ok((input, rng)) => (input, Expression::Range(Box::new(rng))),
      Err(_) => match formula(input.clone()) {
        Ok((input, Factor::Expression(expr))) => (input, *expr),
        Ok((input, fctr)) => (input, Expression::Formula(fctr)),
        Err(err) => {
          return Err(err);},
      } 
    }
  };
  Ok((input, expr))
}

// formula := l1 ;
pub fn formula(input: ParseString) -> ParseResult<Factor> {
  let (input, factor) = l1(input)?;
  Ok((input, factor))
}

// l1 := l2, (add-sub-operator, l2)* ;
pub fn l1(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l2(input)?;
  let (input, rhs) = many0(pair(add_sub_operator,cut(l2)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l2 := l3, (mul-div-operator | matrix-operator, l3)* ;
pub fn l2(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l3(input)?;
  let (input, rhs) = many0(pair(alt((mul_div_operator, matrix_operator)),cut(l3)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l3 := l4, (power-operator, l4)* ;
pub fn l3(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l4(input)?;
  let (input, rhs) = many0(pair(power_operator,cut(l4)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l4 := l5, (logic-operator, l5)* ;
pub fn l4(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l5(input)?;
  let (input, rhs) = many0(pair(logic_operator,cut(l5)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l5 := factor, (comparison-operator, factor)* ;
pub fn l5(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l6(input)?;
  let (input, rhs) = many0(pair(comparison_operator,cut(l6)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l6 := factor, (table-operator, factor)* ;
pub fn l6(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l7(input)?;
  let (input, rhs) = many0(pair(table_operator,cut(l7)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l7 := factor, (set-operator, factor)* ;
pub fn l7(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = factor(input)?;
  let (input, rhs) = many0(pair(set_operator,cut(factor)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// factor := parenthetical-term | negate-factor | not-factor | structure | function-call | literal | slice | var ;
pub fn factor(input: ParseString) -> ParseResult<Factor> {
  let parsers: Vec<(&str, Box<dyn Fn(ParseString) -> ParseResult<Factor>>)> = vec![
    ("parenthetical_term", Box::new(|i| parenthetical_term(i))),
    ("negate_factor", Box::new(|i| negate_factor(i))),
    ("not_factor", Box::new(|i| not_factor(i))),
    ("structure", Box::new(|i| structure(i).map(|(i, s)| (i, Factor::Expression(Box::new(Expression::Structure(s))))))),
    ("function_call", Box::new(|i| function_call(i).map(|(i, f)| (i, Factor::Expression(Box::new(Expression::FunctionCall(f))))))),
    ("literal", Box::new(|i| literal(i).map(|(i, l)| (i, Factor::Expression(Box::new(Expression::Literal(l))))))),
    ("slice", Box::new(|i| slice(i).map(|(i, s)| (i, Factor::Expression(Box::new(Expression::Slice(s))))))),
    ("var", Box::new(|i| var(i).map(|(i, v)| (i, Factor::Expression(Box::new(Expression::Var(v))))))),
  ];
  let (input, fctr) = alt_best(input, &parsers)?;
  let (input, transpose) = opt(transpose)(input)?;
  let fctr = match transpose {
    Some(_) => Factor::Transpose(Box::new(fctr)),
    None => fctr,
  };
  Ok((input, fctr))
}

// parenthetical-term := left-parenthesis, space-tab0, formula, space-tab0, right-parenthesis ;
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

// var := identifier, ?kind-annotation ;
pub fn var(input: ParseString) -> ParseResult<Var> {
  let ((input, name)) = identifier(input)?;
  let ((input, kind)) = opt(kind_annotation)(input)?;
  Ok((input, Var{ name, kind }))
}

// statement-separator := ";" ;
pub fn statement_separator(input: ParseString) -> ParseResult<()> {
  let (input,_) = nom_tuple((whitespace0,semicolon,whitespace0))(input)?;
  Ok((input, ()))
}

// Math Expressions
// ----------------------------------------------------------------------------

// add-sub-operator := add | subtract ;
pub fn add_sub_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((add, subtract))(input)?;
  Ok((input, FormulaOperator::AddSub(op)))
}


// mul-div-operator := multiply | divide | modulus ;
pub fn mul_div_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((multiply, divide, modulus))(input)?;
  Ok((input, FormulaOperator::MulDiv(op)))
}

// power-operator := power ;
pub fn power_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = power(input)?;
  Ok((input, FormulaOperator::Power(op)))
}

// negate-factor := "-", factor ;
pub fn negate_factor(input: ParseString) -> ParseResult<Factor> {
  let (input, _) = dash(input)?;
  let (input, expr) = factor(input)?;
  Ok((input, Factor::Negate(Box::new(expr))))
}

// not-factor := not, factor ;
pub fn not_factor(input: ParseString) -> ParseResult<Factor> {
  let (input, _) = not(input)?;
  let (input, expr) = factor(input)?;
  Ok((input, Factor::Not(Box::new(expr))))
}

// add := "+" ;
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

// subtract := "-" ;
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

// multiply := "*" | "×" ;
pub fn multiply(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = pair(is_not(matrix_multiply),alt((tag("*"), tag("×"))))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, MulDivOp::Mul))
}

// divide := "/" | "÷" ;
pub fn divide(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = pair(is_not(comment_sigil),alt((tag("/"),tag("÷"))))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, MulDivOp::Div))
}

// modulus := "%" ;
pub fn modulus(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("%")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, MulDivOp::Mod))
}

// power := "^" ;
pub fn power(input: ParseString) -> ParseResult<PowerOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("^")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, PowerOp::Pow))
}

// Matrix Operations
// ----------------------------------------------------------------------------

// matrix-operator := matrix-multiply | multiply | divide | matrix-solve ;
pub fn matrix_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((matrix_multiply, matrix_solve, dot_product, cross_product))(input)?;
  Ok((input, FormulaOperator::Vec(op)))
}

// matrix-multiply := "**" ;
pub fn matrix_multiply(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("**")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, VecOp::MatMul))
}

// matrix-solve := "\" ;
pub fn matrix_solve(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("\\")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, VecOp::Solve))
}

// dot-product := "·" | "•" ;
pub fn dot_product(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("·"),tag("•")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, VecOp::Dot))
}

// cross-product := "⨯" ;
pub fn cross_product(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⨯")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, VecOp::Cross))
}

// transpose := "'" ;
pub fn transpose(input: ParseString) -> ParseResult<()> {
  let (input, _) = tag("'")(input)?;
  Ok((input, ()))
}

// Range Expressions
// ----------------------------------------------------------------------------

// range := formula, range-operator, formula, (range-operator, formula)? ;
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

// range-inclusive := "..=" ;
pub fn range_inclusive(input: ParseString) -> ParseResult<RangeOp> {
  let (input, _) = tag("..=")(input)?;
  Ok((input, RangeOp::Inclusive))
}

// range-exclusive := ".." ;
pub fn range_exclusive(input: ParseString) -> ParseResult<RangeOp> {
  let (input, _) = tag("..")(input)?;
  Ok((input, RangeOp::Exclusive))
}

// range-operator := range-inclusive | range-exclusive ;
pub fn range_operator(input: ParseString) -> ParseResult<RangeOp> {
  let (input, op) = alt((range_inclusive,range_exclusive))(input)?;
  Ok((input, op))
}

// Comparison expressions
// ----------------------------------------------------------------------------

// comparison-operator := strict-equal | strict-not-equal | not-equal | equal-to | greater-than-equal | greater-than | less-than-equal | less-than ;
pub fn comparison_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((strict_equal, strict_not_equal, not_equal, equal_to, greater_than_equal, greater_than, less_than_equal, less_than))(input)?;
  Ok((input, FormulaOperator::Comparison(op)))
}

// not-equal := "!=" | "¬=" | "≠" ;
pub fn not_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("!="),tag("¬="),tag("≠")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::NotEqual))
}

// equal-to := "==" ;
pub fn equal_to(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("==")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::Equal))
}

// strict-not-equal := "=!=" | "=¬=" ;
pub fn strict_not_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("=!="),tag("=¬=")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::StrictNotEqual))
}

// strict-equal := "=:=" | "≡" ;
pub fn strict_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("=:="),tag("≡")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::StrictEqual))
}

// greater-than := ">" ;
pub fn greater_than(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag(">")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::GreaterThan))
}

// less_than := "<" ;
pub fn less_than(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = is_not(tag("<-"))(input)?;
  let (input, _) = tag("<")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::LessThan))
}

// greater-than-equal := ">=" | "≥" ;
pub fn greater_than_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag(">="),tag("≥")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::GreaterThanEqual))
}

// less-than-equal := "<=" | "≤" ;
pub fn less_than_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("<="),tag("≤")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, ComparisonOp::LessThanEqual))
}

// Logic expressions
// ----------------------------------------------------------------------------

// logic-operator := and | or | xor ;
pub fn logic_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((and, or, xor))(input)?;
  Ok((input, FormulaOperator::Logic(op)))
}

// or := "|" ;
pub fn or(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("||"), tag("∨"), tag("⋁")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, LogicOp::Or))
}

// and := "&" ;
pub fn and(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("&&"), tag("∧"), tag("⋀")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, LogicOp::And))
}

// not := "!" | "¬" ;
pub fn not(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = alt((tag("!"), tag("¬")))(input)?;
  Ok((input, LogicOp::Not))
}

// xor := "xor" | "⊕" | "⊻" ;
pub fn xor(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("^^"), tag("⊕"), tag("⊻")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, LogicOp::Xor))
}

// Table Operations
// ----------------------------------------------------------------------------

// table-operator := join | left-join | right-join | full-join | left-semi-join | left-anti-join ;
fn table_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((join,left_join,right_join,full_join,left_semi_join,left_anti_join))(input)?;
  Ok((input, FormulaOperator::Table(op)))
}

// join := "⋈" ;
fn join(input: ParseString) -> ParseResult<TableOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⋈")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, TableOp::InnerJoin))
}

// left-join := "⟕" ;
fn left_join(input: ParseString) -> ParseResult<TableOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⟕")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, TableOp::LeftOuterJoin))
}

// right-join := "⟖" ;
fn right_join(input: ParseString) -> ParseResult<TableOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⟖")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, TableOp::RightOuterJoin))
}

// full-join := "⟗" ;
fn full_join(input: ParseString) -> ParseResult<TableOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⟗")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, TableOp::FullOuterJoin))
}

// left-semi-join := "⋉" ;
fn left_semi_join(input: ParseString) -> ParseResult<TableOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⋉")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, TableOp::LeftSemiJoin))
}

// left-anti-join := "▷" ;
fn left_anti_join(input: ParseString) -> ParseResult<TableOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("▷")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, TableOp::LeftAntiJoin))
}


// Set Operations
// ----------------------------------------------------------------------------

// set-operator := union | intersection | difference | complement | subset | superset | proper-subset | proper-superset | element-of | not-element-of | symmetric-difference ;
pub fn set_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((union_op,intersection,difference,complement,subset,superset,proper_subset,proper_superset,element_of,not_element_of,symmetric_difference))(input)?;
  Ok((input, FormulaOperator::Set(op)))
}

// union := "∪" ;
pub fn union_op(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("∪")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::Union))
}

// intersection := "∩" ;
pub fn intersection(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("∩")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::Intersection))
}

// difference := "∖" ;
pub fn difference(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("∖")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::Difference))
}

// complement := "∁" ;
pub fn complement(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("∁")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::Complement))
}

// subset := "⊆" ;
pub fn subset(input: ParseString) -> ParseResult<SetOp> { 
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⊆")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::Subset))
}

// superset := "⊇" ;
pub fn superset(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("⊇")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::Superset))
}

// proper-subset := "⊊" ;
pub fn proper_subset(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("⊊"), tag("⊂")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::ProperSubset))
}

// proper-superset := "⊋" ;
pub fn proper_superset(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = alt((tag("⊋"), tag("⊃")))(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::ProperSuperset))
}

// element-of := "∈" ;
pub fn element_of(input: ParseString) -> ParseResult<SetOp> { 
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("∈")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::ElementOf))
}

// not-element-of := "∉" ;
pub fn not_element_of(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws0e(input)?;
  let (input, _) = tag("∉")(input)?;
  let (input, _) = ws0e(input)?;
  Ok((input, SetOp::NotElementOf))
}

// symmetric-difference := "Δ" ;
pub fn symmetric_difference(input: ParseString) -> ParseResult<SetOp> {
  let (input, _) = ws1e(input)?;
  let (input, _) = tag("Δ")(input)?;
  let (input, _) = ws1e(input)?;
  Ok((input, SetOp::SymmetricDifference))
}

// Set Comprehensions
// ----------------------------------------------------------------------------

// set-comprehension := "{", formula, "|", [set-qualifier, ","], "}" ;
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

// set-qualifier := generator | expression | variable-define  ;
pub fn comprehension_qualifier(input: ParseString) -> ParseResult<ComprehensionQualifier> {
  match generator(input.clone()) {
    Ok((input, gen)) => Ok((input, gen)),
    Err(_) => match variable_define(input.clone()) {
      Ok((input, var_def)) => Ok((input, ComprehensionQualifier::Let(var_def))),
      Err(_) => {
        let (input, expr) = expression(input)?;
        Ok((input, ComprehensionQualifier::Filter(expr)))
      }
    }
  }
}

// generator := pattern, "<-", expression ;
pub fn generator(input: ParseString) -> ParseResult<ComprehensionQualifier> {
  let (input, ptrn) = pattern(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, _) = cut(tag("<-"))(input)?;
  let (input, _) = space_tab0(input)?;
  let (input, expr) = expression(input)?;
  Ok((input, ComprehensionQualifier::Generator((ptrn, expr))))
}

// Subscript Operations
// ----------------------------------------------------------------------------

// subscript := (swizzle-subscript | dot-subscript-int | dot-subscript | bracket-subscript | brace-subscript)+ ; 
pub fn subscript(input: ParseString) -> ParseResult<Vec<Subscript>> {
  let (input, subscripts) = many1(alt((swizzle_subscript,dot_subscript,dot_subscript_int,bracket_subscript,brace_subscript)))(input)?;
  Ok((input, subscripts))
}

// slice := identifier, subscript ;
pub fn slice(input: ParseString) -> ParseResult<Slice> {
  let (input, name) = identifier(input)?;
  let (input, ixes) = subscript(input)?;
  Ok((input, Slice{name, subscript: ixes}))
}

// slice-ref := identifier, subscript? ;
pub fn slice_ref(input: ParseString) -> ParseResult<SliceRef> {
  let (input, name) = identifier(input)?;
  let (input, ixes) = opt(subscript)(input)?;
  Ok((input, SliceRef{name, subscript: ixes}))
}

// swizzle-subscript := ".", identifier, ",", list1(",", identifier) ;
pub fn swizzle_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = period(input)?;
  let (input, first) = identifier(input)?;
  let (input, _) = comma(input)?;
  let (input, mut name) = separated_list1(tag(","),identifier)(input)?;
  let mut subscripts = vec![first];
  subscripts.append(&mut name);
  Ok((input, Subscript::Swizzle(subscripts)))
}

// dot-subscript := ".", identifier ;
pub fn dot_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = period(input)?;
  let (input, name) = identifier(input)?;
  Ok((input, Subscript::Dot(name)))
}

// dot-subscript-int := ".", integer-literal ;
pub fn dot_subscript_int(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = period(input)?;
  let (input, name) = integer_literal(input)?;
  Ok((input, Subscript::DotInt(name)))
}

// bracket-subscript := "[", list1(",", select-all | range-subscript | formula-subscript), "]" ;
pub fn bracket_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = left_bracket(input)?;
  let (input, subscripts) = separated_list1(list_separator,alt((select_all,range_subscript,formula_subscript)))(input)?;
  let (input, _) = right_bracket(input)?;
  Ok((input, Subscript::Bracket(subscripts)))
}

// brace-subscript := "{", list1(",", select-all | range-subscript | formula-subscript), "}" ;
pub fn brace_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = left_brace(input)?;
  let (input, subscripts) = separated_list1(list_separator,alt((select_all,range_subscript,formula_subscript)))(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Subscript::Brace(subscripts)))
}

// select-all := ":" ;
pub fn select_all(input: ParseString) -> ParseResult<Subscript> {
  let (input, lhs) = colon(input)?;
  Ok((input, Subscript::All))
}

// formula-subscript := formula ;
pub fn formula_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, factor) = formula(input)?;
  Ok((input, Subscript::Formula(factor)))
}

// range-subscript := range-expression ;
pub fn range_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, rng) = range_expression(input)?;
  Ok((input, Subscript::Range(rng)))
}