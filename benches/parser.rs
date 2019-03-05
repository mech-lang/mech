#![feature(test)]

extern crate test;
extern crate mech_syntax;

use test::Bencher;
use mech_syntax::parser::Parser;
use mech_syntax::parser2::Parser as Parser2;
use mech_syntax::lexer::Lexer;

#[bench]
fn parse_create_tables(b:&mut Bencher) {
  let mut lexer = Lexer::new();
  let mut parser = Parser::new();
  b.iter(|| {
    let input = String::from("# Bouncing Balls

Define the environment
  #html/event/click = [x: 0 y: 0]
  #ball = [x: 15 y: 9 vx: 40 vy: 9]
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = 5000");
    lexer.add_string(input.clone());
    let tokens = lexer.get_tokens();
    parser.text = input;
    parser.add_tokens(&mut tokens.clone());
    parser.build_parse_tree();
    parser.clear();
  });
}

#[bench]
fn parse_ball_program(b:&mut Bencher) {
  let mut parser = Parser::new(); 
  let mut lexer = Lexer::new();
  b.iter(|| {
    let input = String::from("# Bouncing Balls

Define the environment
  #html/event/click = [x: 0 y: 0]
  #ball = [x: 15 y: 9 vx: 40 vy: 9]
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = 5000

## Update condition

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity

## Boundary Condition

Keep the balls within the y boundary
  ~ #system/timer.tick
  iy = #ball.y > #boundary
  #ball.y{iy} := #boundary
  #ball.vy{iy} := #ball.vy * 80

Keep the balls within the x boundary
  ~ #system/timer.tick
  ix = #ball.x > #boundary
  ixx = #ball.x < 0
  #ball.x{ix} := #boundary
  #ball.x{ixx} := 0
  #ball.vx{ix | ixx} := #ball.vx * 80

## Create More Balls

Create ball on click
  ~ #html/event/click.x
  #ball += [x: 10 y: 10 vx: 40 vy: 0]");

    let text = input.clone();
    lexer.add_string(input.clone());
    let tokens = lexer.get_tokens();
    parser.text = input;
    parser.add_tokens(&mut tokens.clone());
    parser.build_parse_tree();
    parser.clear();
  });
}

#[bench]
fn parse_small(b:&mut Bencher) {
  let mut parser = Parser::new(); 
  let mut lexer = Lexer::new();
  b.iter(|| {
    let input = String::from("# Bouncing Balls


This is a paragraph
Paragraph second
  #table");

    let text = input.clone();
    lexer.add_string(input.clone());
    let tokens = lexer.get_tokens();
    parser.text = input;
    parser.add_tokens(&mut tokens.clone());
    parser.build_parse_tree();
    parser.clear();
  });
}

#[bench]
fn parse_small2(b:&mut Bencher) {
  b.iter(|| {
    let mut parser = Parser2::new();
    parser.parse("# Bouncing Balls


This is a paragraph
Paragraph second
  #table");
  });
}