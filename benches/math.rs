#![feature(test)]

extern crate test;
extern crate mech_syntax;
extern crate mech_core;

use mech_syntax::compiler::Compiler;
use mech_core::{Hasher, Core, Index};
use test::Bencher;

#[bench]
fn add_two_vectors_10e3(b:&mut Bencher) {
  b.iter(|| {
    let mut core = Core::new(100, 100);
    let mut compiler = Compiler::new();
    let input = String::from("
block
  x = 1:1000
  z = x + x");
    compiler.compile_string(input);
    core.register_blocks(compiler.blocks.clone());
    core.step();
  });
}

#[bench]
fn add_two_vectors_10e5(b:&mut Bencher) {
  b.iter(|| {
    let mut core = Core::new(100, 100);
    let mut compiler = Compiler::new();
    let input = String::from("
block
  x = 1:100000
  z = x + x");
    compiler.compile_string(input);
    core.register_blocks(compiler.blocks.clone());
    core.step();
  });
}

#[bench]
fn add_two_scalars(b:&mut Bencher) {
  b.iter(|| {
    let mut core = Core::new(100, 100);
    let mut compiler = Compiler::new();
    let input = String::from("
block
  z = 3 + 3");
    compiler.compile_string(input);
    core.register_blocks(compiler.blocks.clone());
    core.step();
  });
}