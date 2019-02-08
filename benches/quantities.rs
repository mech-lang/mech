#![feature(test)]

extern crate test;
extern crate mech_core;

use test::Bencher;
use mech_core::{Quantity, ToQuantity, QuantityMath, make_quantity};

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

#[bench]
fn bench_quantity_add_100000(b:&mut Bencher) {
    let y:i32 = -1;
    let y_quantity = y.to_quantity();
    b.iter(|| {
        for x in (0..100_000).map(|x: i32| x.to_quantity()) {
            test::black_box(x.add(y_quantity));
        }
    });
}

#[bench]
fn bench_quantity_subtract_100000(b:&mut Bencher) {
    let y:i32 = -1;
    let y_quantity = y.to_quantity();
    b.iter(|| {
        for x in (0..100_000).map(|x: i32| x.to_quantity()) {
            test::black_box(x.sub(y_quantity));
        }
    });
}

#[bench]
fn bench_quantity_divide_100000(b:&mut Bencher) {
    let y:i32 = -1;
    let y_quantity = y.to_quantity();
    b.iter(|| {
        for x in (0..100_000).map(|x: i32| x.to_quantity()) {
            test::black_box(x.divide(y_quantity));
        }
    });
}

#[bench]
fn bench_quantity_multiply_100000(b:&mut Bencher) {
    let y:i32 = -1;
    let y_quantity = y.to_quantity();
    b.iter(|| {
        for x in (0..100_000).map(|x: i32| x.to_quantity()) {
            test::black_box(x.multiply(y_quantity));
        }
    });
}

#[bench]
fn bench_quantity_normal_add_100000(b:&mut Bencher) {
    let y:i32 = -1;
    b.iter(|| {
        for x in 0..100_000 {
            test::black_box(x * y);
        }
     });
}