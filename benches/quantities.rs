#![feature(test)]

extern crate test;
extern crate mech_core;

use test::Bencher;
use mech_core::{Quantity, QuantityMath, make_quantity};

#[bench]
fn quantity_add(b:&mut Bencher) {
    let x = make_quantity(1, 3, 1);
    let y = make_quantity(1, -3, 1);
    b.iter(|| {
        let add = x.add(y);
    });
}

#[bench]
fn quantity_subtract(b:&mut Bencher) {
    let x = make_quantity(1, 3, 1);
    let y = make_quantity(1, -3, 1);
    b.iter(|| {
        let sub = x.sub(y);
    });
}

#[bench]
fn quantity_multiply(b:&mut Bencher) {
    let x = make_quantity(1, 3, 1);
    let y = make_quantity(1, -3, 1);
    b.iter(|| {
        let sub = x.multiply(y);
    });
}

#[bench]
fn quantity_divide(b:&mut Bencher) {
    let x = make_quantity(1, 3, 1);
    let y = make_quantity(1, -3, 1);
    b.iter(|| {
        let sub = x.divide(y);
    });
}