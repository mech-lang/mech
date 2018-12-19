#![feature(test)]

extern crate test;
extern crate mech;

use mech::{Compiler, Core};
use test::Bencher;

#[bench]
fn add_two_vectors_10e3(b:&mut Bencher) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("
block
  x = 1:1000
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
  x = 1:100000
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