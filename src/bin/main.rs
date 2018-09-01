#![feature(test)]

extern crate test;
extern crate mech_core;
extern crate core;
extern crate rand;

use test::Bencher;
use mech_core::{Core, Transaction, Change};
use mech_core::{Value, Table};
use mech_core::Hasher;
use mech_core::{Function, Plan, Comparator};
use mech_core::{Runtime, Block, Constraint, Register};
use rand::{Rng};

fn main() {

  Core::new(100,100);

}
