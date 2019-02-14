extern crate mech_core;

use mech_core::{Quantity, ToQuantity, QuantityMath, make_quantity};

#[test]
fn quantities_base() {
    let x = make_quantity(1, 3, 1);
    let y = make_quantity(1, -3, 1);
    let added = x.add(y);
    assert!(x.is_number());
    assert!(!x.is_other());
    assert_eq!(x.mantissa(), 1);
    assert_eq!(y.mantissa(), 1);
    assert_eq!(x.range(), 3);
    assert_eq!(y.range(), -3);
    assert_eq!(added.mantissa(), 1000001);
    assert_eq!(added.range(), -3);
    let added_reverse = y.add(x);
    assert_eq!(added_reverse.mantissa(), 1000001);
    assert_eq!(added_reverse.range(), -3);
}

#[test]
fn quantities_base_sub() {
    let x = make_quantity(1, 3, 1);
    let y = make_quantity(1, -3, 1);
    let sub = x.sub(y);
    assert_eq!(sub.mantissa(), 999999);
    assert_eq!(sub.range(), -3);
}

#[test]
fn quantities_base_multiply() {
    let x = make_quantity(1, 3, 1);
    let y = make_quantity(1, -3, 1);
    let sub = x.multiply(y);
    assert_eq!(sub.mantissa(), 1);
    assert_eq!(sub.range(), 0);
}

#[test]
fn quantities_base_add_float() {
    let x = make_quantity(1, -1, 0);
    let y = make_quantity(2, -1, 0);
    assert_eq!(x.add(y), make_quantity(3, -1, 0));
}

#[test]
fn quantities_base_add_01_02_03() {
    let x = make_quantity(1, -1, 0);
    let y = make_quantity(2, -1, 0);
    let z = make_quantity(3, -1, 0);
    assert_eq!(x.add(y.add(z)), make_quantity(6, -1, 0));
}

#[test]
fn quantities_base_associativity() {
    let x = make_quantity(1, -1, 0);
    let y = make_quantity(2, -1, 0);
    let z = make_quantity(3, -1, 0);
    assert_eq!(z.add(x.add(y)), make_quantity(6, -1, 0));
}

#[test]
fn quantities_base_add_subtract() {
    let x = make_quantity(1, -1, 0);
    let y = make_quantity(2, -1, 0);
    let z = make_quantity(3, -1, 0);
    assert_eq!((z.add(x.add(y))).sub(z).sub(y), make_quantity(1, -1, 0));
}

#[test]
fn quantities_base_add_big_little() {
  let x = make_quantity(275251200000000,-12,0);
  let y = make_quantity(7864320000000,-12,0);
    assert_eq!(x.add(y), make_quantity(28311552000000, -11, 0));
}

#[test]
fn quantities_base_multiply_small() {
  let w = make_quantity(0,0,0);
  let x = make_quantity(14336512000000,-12,0);
  let y = make_quantity(8,-1,0);
  let z = make_quantity(1,0,0);
  let q = w.sub(x);
  let r = q.multiply(y);
  assert_eq!(r, make_quantity(-114692096000000, -13, 0));
}

#[test]
fn quantities_base_float() {
    let x = 1.2;
    let y = 1.1;
    let z = 0.5;
    assert_eq!(x.to_quantity().to_float(), x);
    assert_eq!(y.to_quantity().to_float(), y);
    assert_eq!(z.to_quantity().to_float(), z);
    println!("{}", x.to_quantity().to_string());
    println!("{}", y.to_quantity().to_string());
    println!("{}", z.to_quantity().to_string());
}