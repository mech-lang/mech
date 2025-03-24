#[macro_use]
use crate::*;

#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use nom::{
  branch::alt,
  bytes::complete::tag,
  character::complete::{anychar, char as nom_char, digit1, satisfy},
  combinator::{map, map_res, opt},
  multi::{many0, many1, separated_list1},
  sequence::{delimited, preceded, tuple as nom_tuple},
  IResult,
};


use std::collections::HashMap;
use colored::*;

use crate::*;

// grammar := +rule ;
pub fn grammar(input: ParseString) -> ParseResult<Grammar> {
  let ((input, rules)) = many1(rule)(input)?;
  let (input, _) = new_line(input)?;
  Ok((input, Grammar { rules }))
}

// grammar-identifier := alpha_token, *(alpha_token | digit_token | dash) ;
fn grammar_identifier(input: ParseString) -> ParseResult<GrammarIdentifier> {
  let (input, first) = alpha_token(input)?;
  let (input, mut rest) = many0(alt((alpha_token, digit_token, dash)))(input)?;
  let mut id = vec![first];
  id.extend(rest);
  let name = Token::merge_tokens(&mut id).unwrap();
  Ok((input, GrammarIdentifier{name}))
}

// rule := grammar-identifier, define_operator, grammar_expression, semicolon ;
fn rule(input: ParseString) -> ParseResult<Rule> {
  let ((input, name)) = grammar_identifier(input)?;
  let ((input, _)) = define_operator(input)?;
  let ((input, expr)) = grammar_expression(input)?;
  let ((input, _)) = semicolon(input)?;
  Ok((input, Rule { name, expr }))
}

// grammar-expression := term, *( "|" term ) ;
fn grammar_expression(input: ParseString) -> ParseResult<GrammarExpression> {
  let (input, first) = term(input)?;
  let (input, rest) = many0(nom_tuple((bar, term)))(input)?;
  if rest.len() == 0 {
    Ok((input,first))
  } else {
    let mut choice = vec![first];
    choice.extend(rest.into_iter().map(|(_, term)| term));
    Ok((input, GrammarExpression::Choice(choice)))
  }
}

// term := factor, *( "," factor ) ;
fn term(input: ParseString) -> ParseResult<GrammarExpression> {
  let (input, first) = factor(input)?;
  let (input, rest) = many0(nom_tuple((comma, factor)))(input)?;
  let mut seq: Vec<GrammarExpression> = vec![first];
  seq.extend(rest.into_iter().map(|(_, factor)| factor));
  if seq.len() == 1 {
    return Ok((input, seq.pop().unwrap()));
  }
  Ok((input, GrammarExpression::Sequence(seq)))
}

// definition := grammar_identifier ;
fn definition(input: ParseString) -> ParseResult<GrammarExpression> {
  let (input, id) = grammar_identifier(input)?;
  Ok((input, GrammarExpression::Definition(id)))
}

// repeat0 := "*", factor ;
fn repeat0(input: ParseString) -> ParseResult<GrammarExpression> {
  let (input, _) = asterisk(input)?;
  let (input, expr) = factor(input)?;
  Ok((input, GrammarExpression::Repeat0(Box::new(expr))))
}

// repeat1 := "+", factor ;
fn repeat1(input: ParseString) -> ParseResult<GrammarExpression> {
  let (input, _) = plus(input)?;
  let (input, expr) = factor(input)?;
  Ok((input, GrammarExpression::Repeat1(Box::new(expr))))
}

// optional := "?", factor ;
fn optional(input: ParseString) -> ParseResult<GrammarExpression> {
  let (input, _) = question(input)?;
  let (input, expr) = factor(input)?;
  Ok((input, GrammarExpression::Optional(Box::new(expr))))
}

// peek := ">", factor ;
fn peek(input: ParseString) -> ParseResult<GrammarExpression> {
  let (input, _) = right_angle(input)?;
  let (input, expr) = factor(input)?;
  Ok((input, GrammarExpression::Peek(Box::new(expr))))
}

// not := "¬", factor ;
fn not(input: ParseString) -> ParseResult<GrammarExpression> {
  let (input, _) = negate(input)?;
  let (input, expr) = factor(input)?;
  Ok((input, GrammarExpression::Not(Box::new(expr))))
}

// g-range := terminal, "..", terminal ;
fn g_range(input: ParseString) -> ParseResult<GrammarExpression> {
  let (input, start) = terminal_token(input)?;
  let (input, _) = tuple((period,period))(input)?;
  let (input, end) = terminal_token(input)?;
  Ok((input, GrammarExpression::Range(start, end)))
}

// factor := repeat0 | repeat1 | optional | peek | not | group | definition | terminal ;
fn factor(input: ParseString) -> ParseResult<GrammarExpression> {
  alt((
    repeat0,
    repeat1,
    optional,
    peek,
    not,
    group,
    definition,
    g_range,
    terminal,
  ))(input)
  
}
  
// group := "(", GrammarExpression, ")" ;
fn group(input: ParseString) -> ParseResult<GrammarExpression> {
  let (input, expr) = delimited(left_parenthesis, grammar_expression, right_parenthesis)(input)?;
  Ok((input, GrammarExpression::Group(Box::new(expr))))
}

// terminal := quote, +any_token, quote ;
fn terminal(input: ParseString) -> ParseResult<GrammarExpression> {
  let (input, trminl) = terminal_token(input)?;
  Ok((input, GrammarExpression::Terminal(trminl)))
}

// terminal := quote, +any_token, quote ;
fn terminal_token(input: ParseString) -> ParseResult<Token> {
  let (input, _) = quote(input)?;
  let (input, mut t) = many0(tuple((is_not(quote),any_token)))(input)?;
  let (input, _) = quote(input)?;
  let mut t = t.into_iter().map(|(_,b)| b).collect::<Vec<Token>>();
  let token =  Token::merge_tokens(&mut t).unwrap();
  Ok((input,token))
}