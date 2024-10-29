#[macro_use]
use crate::*;

#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use nom::{
  IResult,
  branch::alt,
  sequence::tuple as nom_tuple,
  combinator::{opt, eof},
  multi::{many1, many_till, many0, separated_list1,separated_list0},
  Err,
  Err::Failure
};

use std::collections::HashMap;
use colored::*;

use crate::*;

// #### Expressions

// ##### Math expressions

// parenthetical_expression := left_parenthesis, formula, right_parenthesis ;
pub fn parenthetical_term(input: ParseString) -> ParseResult<Factor> {
  let msg1 = "Expects expression";
  let msg2 = "Expects right parenthesis ')'";
  let (input, (_, r)) = range(left_parenthesis)(input)?;
  let (input, frmla) = label!(formula, msg1)(input)?;
  let (input, _) = label!(right_parenthesis, msg2, r)(input)?;
  Ok((input, frmla))
}

// negate_factor := "-" factor ;
pub fn negate_factor(input: ParseString) -> ParseResult<Factor> {
  let (input, _) = dash(input)?;
  let (input, expr) = factor(input)?;
  Ok((input, Factor::Negate(Box::new(expr))))
}

// not_factor := "not" factor ;
pub fn not_factor(input: ParseString) -> ParseResult<Factor> {
  let (input, _) = not(input)?;
  let (input, expr) = factor(input)?;
  Ok((input, Factor::Not(Box::new(expr))))
}

// add := "+" ;
pub fn add(input: ParseString) -> ParseResult<AddSubOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("+")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, AddSubOp::Add))
}

// subtract := "-" ;
pub fn subtract(input: ParseString) -> ParseResult<AddSubOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("-")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, AddSubOp::Sub))
}

// multiply := "*" ;
pub fn multiply(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("*")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, MulDivOp::Mul))
}

// divide := "/" ;
pub fn divide(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("/")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, MulDivOp::Div))
}

// matrix_multiply := "**" ;
pub fn matrix_multiply(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("**")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, VecOp::MatMul))
}

// matrix_solve := "\" ;
pub fn matrix_solve(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("\\")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, VecOp::Solve))
}

// dot_product := "·" ;
pub fn dot_product(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("·")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, VecOp::Dot))
}

// cross_product := "⨯" ;
pub fn cross_product(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("⨯")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, VecOp::Cross))
}

// exponent := "^" ;
pub fn exponent(input: ParseString) -> ParseResult<ExponentOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("^")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ExponentOp::Exp))
}

// range_inclusive := "..=" ;
pub fn range_inclusive(input: ParseString) -> ParseResult<RangeOp> {
  let (input, _) = tag("..=")(input)?;
  Ok((input, RangeOp::Inclusive))
}

// range_exclusive := ".." ;
pub fn range_exclusive(input: ParseString) -> ParseResult<RangeOp> {
  let (input, _) = tag("..")(input)?;
  Ok((input, RangeOp::Exclusive))
}

// range_operator := range_inclusive | range_exclusive ;
pub fn range_operator(input: ParseString) -> ParseResult<RangeOp> {
  let (input, op) = alt((range_inclusive,range_exclusive))(input)?;
  Ok((input, op))
}

// formula := l1, (range_operator, l1)* ;
pub fn formula(input: ParseString) -> ParseResult<Factor> {
  let (input, factor) = l1(input)?;
  Ok((input, factor))
}

// add_sub_operator := add | subtract ;
pub fn add_sub_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((add, subtract))(input)?;
  Ok((input, FormulaOperator::AddSub(op)))
}

// l1 := l2, (add_sub_operator, l2)* ;
pub fn l1(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l2(input)?;
  let (input, rhs) = many0(nom_tuple((add_sub_operator,l2)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// mul_div_operator := matrix_multiply | multiply | divide | matrix_solve ;
pub fn mul_div_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((multiply, divide))(input)?;
  Ok((input, FormulaOperator::MulDiv(op)))
}

// mul_div_operator := matrix_multiply | multiply | divide | matrix_solve ;
pub fn vec_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((matrix_multiply, matrix_solve, dot_product, cross_product))(input)?;
  Ok((input, FormulaOperator::Vec(op)))
}

// l2 := l3, (mul_div_operator, l3)* ;
pub fn l2(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l3(input)?;
  let (input, rhs) = many0(nom_tuple((alt((mul_div_operator, vec_operator)),l3)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// exponent_operator := exponent ;
pub fn exponent_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = exponent(input)?;
  Ok((input, FormulaOperator::Exponent(op)))
}

// l3 := l4, (exponent_operator, l4)* ;
pub fn l3(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l4(input)?;
  let (input, rhs) = many0(nom_tuple((exponent_operator,l4)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// logic_operator := and | or | xor ;
pub fn logic_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((and, or, xor))(input)?;
  Ok((input, FormulaOperator::Logic(op)))
}

// l4 := l5, (logic_operator, l5)* ;
pub fn l4(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l5(input)?;
  let (input, rhs) = many0(nom_tuple((logic_operator,l5)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l5 := factor, (comparison_operator, factor)* ;
pub fn l5(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = factor(input)?;
  let (input, rhs) = many0(nom_tuple((comparison_operator,factor)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// comparison_operator := not_equal | equal_to | greater_than_equal | greater_than | less_than_equal | less_than ;
pub fn comparison_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((not_equal, equal_to, greater_than_equal, greater_than, less_than_equal, less_than))(input)?;
  Ok((input, FormulaOperator::Comparison(op)))
}

// factor := parenthetical_term | structure | fsm_pipe | function_call | literal | slice | var ;
pub fn factor(input: ParseString) -> ParseResult<Factor> {
  let (input, fctr) = match parenthetical_term(input.clone()) {
    Ok((input, term)) => (input, term),
    Err(_) => match negate_factor(input.clone()) {
      Ok((input, neg)) => (input, neg),
      Err(_) => match not_factor(input.clone()) {
        Ok((input, neg)) => (input, neg),
        Err(_) => match structure(input.clone()) {
          Ok((input, strct)) => (input, Factor::Expression(Box::new(Expression::Structure(strct)))),
          Err(_) => match fsm_pipe(input.clone()) {
            Ok((input, pipe)) => (input, Factor::Expression(Box::new(Expression::FsmPipe(pipe)))),
            Err(_) => match function_call(input.clone()) {
              Ok((input, fxn)) => (input, Factor::Expression(Box::new(Expression::FunctionCall(fxn)))),
              Err(_) => match literal(input.clone()) {
                Ok((input, ltrl)) => (input, Factor::Expression(Box::new(Expression::Literal(ltrl)))),
                Err(_) => match slice(input.clone()) {
                  Ok((input, slc)) => (input, Factor::Expression(Box::new(Expression::Slice(slc)))),
                  Err(_) => match var(input.clone()) {
                    Ok((input, var)) => (input, Factor::Expression(Box::new(Expression::Var(var)))),
                    Err(err) => { return Err(err); },
                  },
                },
              },
            },
          },
        },
      },
    },
  };
  let (input, transpose) = opt(transpose)(input)?;
  let fctr = match transpose {
    Some(_) => Factor::Transpose(Box::new(fctr)),
    None => fctr,
  };
  Ok((input, fctr))
}
// statement_separator := ";" ;
pub fn statement_separator(input: ParseString) -> ParseResult<()> {
  let (input,_) = nom_tuple((whitespace0,semicolon,whitespace0))(input)?;
  Ok((input, ()))
}

// function_define := identifier "(" (function_arg (list_separator function_arg)*)? ")" whitespace0 "=" whitespace0 (function_out_args | function_out_arg) define_operator (statement (whitespace1 | statement_separator)*) period ;
pub fn function_define(input: ParseString) -> ParseResult<FunctionDefine> {
  let ((input, name)) = identifier(input)?;
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, input_args)) = separated_list0(list_separator, function_arg)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  let ((input, _)) = whitespace0(input)?;
  let ((input, _)) = equal(input)?;
  let ((input, _)) = whitespace0(input)?;
  let ((input, output)) = alt((function_out_args,function_out_arg))(input)?;
  let ((input, _)) = define_operator(input)?;
  let ((input, statements)) = separated_list1(alt((whitespace1,statement_separator)), statement)(input)?;
  let ((input, _)) = period(input)?;
  Ok((input,FunctionDefine{name,input: input_args,output,statements}))
}

// function_out_args := "(" function_arg (list_separator function_arg)* ")" ;
pub fn function_out_args(input: ParseString) -> ParseResult<Vec<FunctionArgument>> {
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, args)) = separated_list1(list_separator,function_arg)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  Ok((input, args))
}

// function_out_arg := function_arg ;
pub fn function_out_arg(input: ParseString) -> ParseResult<Vec<FunctionArgument>> {
  let ((input, arg)) = function_arg(input)?;
  Ok((input, vec![arg]))
}

// function_arg := identifier, kind_annotation ;
pub fn function_arg(input: ParseString) -> ParseResult<FunctionArgument> {
  let ((input, name)) = identifier(input)?;
  let ((input, kind)) = kind_annotation(input)?;
  Ok((input, FunctionArgument{ name, kind }))
}

// argument_list := "(", list0(",", call_arg_with_biding | call_arg)
pub fn argument_list(input: ParseString) -> ParseResult<ArgumentList> {
  let (input, _) = left_parenthesis(input)?;
  let (input, args) = separated_list0(list_separator, alt((call_arg_with_binding,call_arg)))(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, args))
}

// function_call := identifier, argument_list
pub fn function_call(input: ParseString) -> ParseResult<FunctionCall> {
  let (input, name) = identifier(input)?;
  let (input, args) = argument_list(input)?;
  Ok((input, FunctionCall{name,args} ))
}

// call_arg_with_binding := identifier, colon, expression ;
pub fn call_arg_with_binding(input: ParseString) -> ParseResult<(Option<Identifier>,Expression)> {
  let (input, arg_name) = identifier(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, expr) = expression(input)?;
  Ok((input, (Some(arg_name), expr)))
}

// call_arg := expression ;
pub fn call_arg(input: ParseString) -> ParseResult<(Option<Identifier>,Expression)> {
  let (input, expr) = expression(input)?;
  Ok((input, (None, expr)))
}

// var := identifier, kind_annotation? ;
pub fn var(input: ParseString) -> ParseResult<Var> {
  let ((input, name)) = identifier(input)?;
  let ((input, kind)) = opt(kind_annotation)(input)?;
  Ok((input, Var{ name, kind }))
}

// ##### Filter expressions

// not_equal := "!=" | "¬=" | "≠" ;
pub fn not_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = alt((tag("!="),tag("¬="),tag("≠")))(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ComparisonOp::NotEqual))
}

// equal_to := "==" ;
pub fn equal_to(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("==")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ComparisonOp::Equal))
}

// greater_than := ">" ;
pub fn greater_than(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag(">")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ComparisonOp::GreaterThan))
}

// less_than := "<" ;
pub fn less_than(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("<")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ComparisonOp::LessThan))
}

// greater_than_equal := ">=" | "≥" ;
pub fn greater_than_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = alt((tag(">="),tag("≥")))(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ComparisonOp::GreaterThanEqual))
}

// less_than_equal := "<=" | "≤" ;
pub fn less_than_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = alt((tag("<="),tag("≤")))(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ComparisonOp::LessThanEqual))
}

// ##### Logic expressions

// or := "|" ;
pub fn or(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("|")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, LogicOp::Or))
}

// and := "&" ;
pub fn and(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("&")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, LogicOp::And))
}

// not := "!" | "¬" ;
pub fn not(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = alt((tag("!"), tag("¬")))(input)?;
  Ok((input, LogicOp::Not))
}

// xor := "xor" | "⊕" | "⊻" ;
pub fn xor(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = alt((tag("xor"), tag("⊕"), tag("⊻")))(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, LogicOp::Xor))
}

// ##### Other expressions

// string := quote, (!quote, <text>)*, quote ;
pub fn string(input: ParseString) -> ParseResult<MechString> {
  let msg = "Character not allowed in string";
  let (input, _) = quote(input)?;
  let (input, matched) = many0(nom_tuple((is_not(quote), label!(text, msg))))(input)?;
  let (input, _) = quote(input)?;
  let (_, mut text): ((), Vec<_>) = matched.into_iter().unzip();
  let mut merged = merge_tokens(&mut text).unwrap();
  merged.kind = TokenKind::String;
  Ok((input, MechString { text: merged }))
}

// transpose := "'" ;
pub fn transpose(input: ParseString) -> ParseResult<()> {
  let (input, _) = tag("'")(input)?;
  Ok((input, ()))
}


// literal := number | string | atom | boolean | empty, kind_annotation? ;
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
            Err(err) => {return Err(err);}
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

// slice := identifier, subscript ;
pub fn slice(input: ParseString) -> ParseResult<Slice> {
  let (input, name) = identifier(input)?;
  let (input, ixes) = subscript(input)?;
  Ok((input, Slice{name, subscript: ixes}))
}

// slice_ref := identifier, subscript ;
pub fn slice_ref(input: ParseString) -> ParseResult<SliceRef> {
  let (input, name) = identifier(input)?;
  let (input, ixes) = opt(subscript)(input)?;
  Ok((input, SliceRef{name, subscript: ixes}))
}

// subscript := (swizzle_subscript | dot_subscript_int | dot_subscript | bracket_subscript | brace_subscript)+ ; 
pub fn subscript(input: ParseString) -> ParseResult<Vec<Subscript>> {
  let (input, subscripts) = many1(alt((swizzle_subscript,dot_subscript,dot_subscript_int,bracket_subscript,brace_subscript)))(input)?;
  Ok((input, subscripts))
}

// swizzle_subscript := ".", identifier, "," , list1(",", identifier) ;
pub fn swizzle_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = period(input)?;
  let (input, first) = identifier(input)?;
  let (input, _) = comma(input)?;
  let (input, mut name) = separated_list1(tag(","),identifier)(input)?;
  let mut subscripts = vec![first];
  subscripts.append(&mut name);
  Ok((input, Subscript::Swizzle(subscripts)))
}

// dot_subscript := ".", identifier ;
pub fn dot_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = period(input)?;
  let (input, name) = identifier(input)?;
  Ok((input, Subscript::Dot(name)))
}

// dot_subscript_int := ".", integer_literal ;
pub fn dot_subscript_int(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = period(input)?;
  let (input, name) = integer_literal(input)?;
  Ok((input, Subscript::DotInt(name)))
}

// bracket_subscript := "[", list1(",", select_all | formula_subscript) "]" ;
pub fn bracket_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = left_bracket(input)?;
  let (input, subscripts) = separated_list1(list_separator,alt((select_all,range_subscript,formula_subscript)))(input)?;
  let (input, _) = right_bracket(input)?;
  Ok((input, Subscript::Bracket(subscripts)))
}

// brace_subscript := "{", list1(",", select_all | formula_subscript) "}" ;
pub fn brace_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = left_brace(input)?;
  let (input, subscripts) = separated_list1(list_separator,alt((select_all,formula_subscript)))(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Subscript::Brace(subscripts)))
}

// select_all := ":" ;
pub fn select_all(input: ParseString) -> ParseResult<Subscript> {
  let (input, lhs) = colon(input)?;
  Ok((input, Subscript::All))
}

// formula_subscript := formula ;
pub fn formula_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, factor) = l1(input)?;
  Ok((input, Subscript::Formula(factor)))
}

// formula_subscript := formula ;
pub fn range_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, rng) = range_expression(input)?;
  Ok((input, Subscript::Range(rng)))
}

// range
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

// expression := formula, transpose? ;
pub fn expression(input: ParseString) -> ParseResult<Expression> {
  let (input, expr) = match range_expression(input.clone()) {
    Ok((input, rng)) => (input, Expression::Range(Box::new(rng))),
    Err(_) => match formula(input.clone()) {
      Ok((input, Factor::Expression(expr))) => (input, *expr),
      Ok((input, fctr)) => (input, Expression::Formula(fctr)),
      Err(err) => {return Err(err);},
    } 
  };
  Ok((input, expr))
}