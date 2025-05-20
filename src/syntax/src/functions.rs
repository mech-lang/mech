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

// function_define := identifier, "(", list0(list_separator, function_arg), ")", "=", (function_out_args | function_out_arg), define_operator, list1((whitespace1 | statement_separator), statement), period ;
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

// function_out_args := "(", list1(list_separator, function_arg), ")" ;
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

// argument_list := "(", list0(",", call_arg_with_biding | call_arg), ")" ;
pub fn argument_list(input: ParseString) -> ParseResult<ArgumentList> {
  let (input, _) = left_parenthesis(input)?;
  let (input, args) = separated_list0(list_separator, alt((call_arg_with_binding,call_arg)))(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, args))
}

// function_call := identifier, argument_list ;
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