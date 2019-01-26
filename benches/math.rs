#![feature(test)]

extern crate test;
extern crate mech;

use mech::{Compiler, Core};
use test::Bencher;

#[bench]
fn add_two_vectors_10(b:&mut Bencher) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("
block
  x = 1:10
  z = x + x");
  compiler.compile_string(input);
  b.iter(|| {
    core.register_blocks(compiler.blocks.clone());
    core.step();
    core.clear();
  });
}

#[bench]
fn add_two_vectors_10e2(b:&mut Bencher) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("
block
  x = 1:100
  z = x + x");
  compiler.compile_string(input);
  b.iter(|| {
    core.register_blocks(compiler.blocks.clone());
    core.step();
    core.clear();
  });
}

#[bench]
fn add_two_vectors_10e3(b:&mut Bencher) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("
block
  x = 1:1,000
  z = x + x");
  compiler.compile_string(input);
  b.iter(|| {
    core.register_blocks(compiler.blocks.clone());
    core.step();
    core.clear();
  });
}

#[bench]
fn add_two_vectors_10e4(b:&mut Bencher) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("
block
  x = 1:10,000
  z = x + x");
  compiler.compile_string(input);
  b.iter(|| {
    core.register_blocks(compiler.blocks.clone());
    core.step();
    core.clear();
  });
}

#[bench]
fn add_two_vectors_10e5(b:&mut Bencher) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("
block
  x = 1:100,000
  z = x + x");
  compiler.compile_string(input);
  b.iter(|| {
    core.register_blocks(compiler.blocks.clone());
    core.step();
    core.clear();
  });
}

#[bench]
fn add_two_vectors_10e6(b:&mut Bencher) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("
block
  x = 1:1,000,000
  z = x + x");
  compiler.compile_string(input);
  b.iter(|| {
    core.register_blocks(compiler.blocks.clone());
    core.step();
    core.clear();
  });
}

#[bench]
fn add_two_scalars(b:&mut Bencher) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("
block
  z = 3 + 3");
  compiler.compile_string(input);
  b.iter(|| {
    core.register_blocks(compiler.blocks.clone());
    core.step();
    core.clear();
  });
}

#[bench]
fn bouncing_balls(b:&mut Bencher) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("# Bouncing Balls

Define the environment
  #html/event/click = [x: 0 y: 0]
  #ball = [x: 50 y: 9 vx: 40 vy: 9]
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = 60

## Update condition

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity

## Boundary Condition

Keep the balls within the y boundary
  ~ #ball.y
  iy = #ball.y > #boundary
  #ball.y{iy} := #boundary
  #ball.vy{iy} := -#ball.vy * 80 / 100

Keep the balls within the x boundary
  ~ #ball.x
  ix = #ball.x > #boundary
  ixx = #ball.x < 0
  #ball.x{ix} := #boundary
  #ball.x{ixx} := 0
  #ball.vx{ix | ixx} := -#ball.vx * 80 / 100

## Create More Balls

Create ball on click
  ~ #html/event/click.x
  #ball += [x: 10 y: 10 vx: 40 vy: 0]");
  compiler.compile_string(input);
  b.iter(|| {
    core.register_blocks(compiler.blocks.clone());
    core.step();
    core.clear();
  });
}