#![feature(test)]

extern crate test;
extern crate mech_syntax;

use test::Bencher;
use mech_syntax::compiler::Compiler;

#[bench]
fn compile_create_tables(b:&mut Bencher) {
  let mut compiler = Compiler::new();
  b.iter(|| {
    let input = String::from("# Bouncing Balls

Define the environment
  #html/event/click = [x: 0 y: 0]
  #ball = [x: 15 y: 9 vx: 40 vy: 9]
  #time/timer = [period: 15, tick: 0]
  #gravity = 2
  #boundary = 5000");
    compiler.compile_string(input);
    compiler.clear();
  });
}

#[bench]
fn compile_ball_program(b:&mut Bencher) {
  let mut compiler = Compiler::new();
  b.iter(|| {
    let input = String::from("# Bouncing Balls

Define the environment
  #html/event/click = [x: 0 y: 0]
  #ball = [x: 15 y: 9 vx: 40 vy: 9]
  #time/timer = [period: 15, tick: 0]
  #gravity = 2
  #boundary = 5000

## Update condition

Now update the block positions
  ~ #time/timer.tick
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity

## Boundary Condition

Keep the balls within the y boundary
  ~ #time/timer.tick
  iy = #ball.y > #boundary
  #ball.y{iy} := #boundary
  #ball.vy{iy} := #ball.vy * 80

Keep the balls within the x boundary
  ~ #time/timer.tick
  ix = #ball.x > #boundary
  ixx = #ball.x < 0
  #ball.x{ix} := #boundary
  #ball.x{ixx} := 0
  #ball.vx{ix | ixx} := #ball.vx * 80

## Create More Balls

Create ball on click
  ~ #html/event/click.x
  #ball += [x: 10 y: 10 vx: 40 vy: 0]");
    compiler.compile_string(input);
    compiler.clear();
  });
}