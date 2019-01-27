#![feature(test)]

extern crate test;
extern crate mech;

use mech::{Compiler, Core};
use test::Bencher;

#[bench]
fn balls_10e2(b:&mut Bencher) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("# Bouncing Balls

Define the environment
  #ball = [x y |
           1:100 1:100]
  #vx = 3
  #vy = 3
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = 60

## Update condition

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #vx
  #ball.y := #ball.y + #vy

## Boundary Condition

Keep the balls within the y boundary
  ~ #ball.y
  iy = #ball.y > #boundary
  #ball.y{iy} := #boundary

Keep the balls within the x boundary
  ~ #ball.x
  ix = #ball.x > #boundary
  ixx = #ball.x < 0
  #ball.x{ix} := #boundary
  #ball.x{ixx} := 0");
  compiler.compile_string(input);
  b.iter(|| {
    core.register_blocks(compiler.blocks.clone());
    core.step();
    core.clear();
  });
}

#[bench]
fn balls_10e3(b:&mut Bencher) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("# Bouncing Balls

Define the environment
  #ball = [x y |
           1:1,000 1:1,000]
  #vx = 3
  #vy = 3
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = 60

## Update condition

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #vx
  #ball.y := #ball.y + #vy

## Boundary Condition

Keep the balls within the y boundary
  ~ #ball.y
  iy = #ball.y > #boundary
  #ball.y{iy} := #boundary

Keep the balls within the x boundary
  ~ #ball.x
  ix = #ball.x > #boundary
  ixx = #ball.x < 0
  #ball.x{ix} := #boundary
  #ball.x{ixx} := 0");
  compiler.compile_string(input);
  b.iter(|| {
    core.register_blocks(compiler.blocks.clone());
    core.step();
    core.clear();
  });
}

#[bench]
fn balls_10e4(b:&mut Bencher) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("# Bouncing Balls

Define the environment
  #ball = [x y |
           1:10,000 1:10,000]
  #vx = 3
  #vy = 3
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = 60

## Update condition

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #vx
  #ball.y := #ball.y + #vy

## Boundary Condition

Keep the balls within the y boundary
  ~ #ball.y
  iy = #ball.y > #boundary
  #ball.y{iy} := #boundary

Keep the balls within the x boundary
  ~ #ball.x
  ix = #ball.x > #boundary
  ixx = #ball.x < 0
  #ball.x{ix} := #boundary
  #ball.x{ixx} := 0");
  compiler.compile_string(input);
  b.iter(|| {
    core.register_blocks(compiler.blocks.clone());
    core.step();
    core.clear();
  });
}

#[bench]
fn balls_10e5(b:&mut Bencher) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("# Bouncing Balls

Define the environment
  #ball = [x y |
           1:100,000 1:100,000]
  #vx = 3
  #vy = 3
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = 60

## Update condition

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #vx
  #ball.y := #ball.y + #vy

## Boundary Condition

Keep the balls within the y boundary
  ~ #ball.y
  iy = #ball.y > #boundary
  #ball.y{iy} := #boundary

Keep the balls within the x boundary
  ~ #ball.x
  ix = #ball.x > #boundary
  ixx = #ball.x < 0
  #ball.x{ix} := #boundary
  #ball.x{ixx} := 0");
  compiler.compile_string(input);
  b.iter(|| {
    core.register_blocks(compiler.blocks.clone());
    core.step();
    core.clear();
  });
}