#![feature(test)]

extern crate test;
extern crate mech;

use test::Bencher;
use mech::eav::{Entity, Value};

#[bench]
fn make_entity(b: &mut Bencher) {
    b.iter(|| {
         let raw = vec![("tag", Value::from_str("#keyboard/event/keydown")),
                        ("key", Value::from_str("A")),
                        ("code", Value::from_u64(42))];
        Entity::from_raw(raw)
    });
}